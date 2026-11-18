//! IPC HLS requests and sessions owned by a single lifecycle-supervised worker.

mod session;
use crate::core::error::{AppError, AppResult};
use session::{start_child, StreamSession};
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    process::Child,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::CancellationToken;

struct StreamInput {
    asset_id: String,
    original_path: PathBuf,
    mime_type: Option<String>,
}
struct StreamRequest {
    input: StreamInput,
    reply: oneshot::Sender<AppResult<PathBuf>>,
}

/// Serializes starts, expiration and shutdown without transferring child ownership to callers.
pub struct HlsManager {
    commands: mpsc::Sender<StreamRequest>,
    receiver: Mutex<Option<mpsc::Receiver<StreamRequest>>>,
    cancellation: CancellationToken,
    streams_dir: PathBuf,
}

impl HlsManager {
    /// Creates an inactive manager; admit its lifecycle owner before publishing it.
    pub fn new(app_data_dir: &Path, cancellation: CancellationToken) -> Arc<Self> {
        let (commands, receiver) = mpsc::channel(32);
        Arc::new(Self {
            commands,
            receiver: Mutex::new(Some(receiver)),
            cancellation,
            streams_dir: app_data_dir.join("streams").join("ipc"),
        })
    }

    /// Requests an existing stream or a supervised start.
    /// # Errors
    /// Returns cancellation after admission closes, or the setup/owner error.
    /// Dropping this future never drops a child or an admitted setup operation.
    pub async fn get_or_start_stream(
        &self,
        asset_id: &str,
        original_path: &Path,
        mime_type: Option<&str>,
    ) -> AppResult<PathBuf> {
        let (reply, response) = oneshot::channel();
        let request = StreamRequest {
            input: StreamInput {
                asset_id: asset_id.into(),
                original_path: original_path.into(),
                mime_type: mime_type.map(str::to_owned),
            },
            reply,
        };
        tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => return Err(AppError::Cancelled),
            sent = self.commands.send(request) => sent.map_err(|_| self.owner_closed())?,
        }
        tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => Err(AppError::Cancelled),
            result = response => result.map_err(|_| self.owner_closed())?,
        }
    }

    fn owner_closed(&self) -> AppError {
        if self.cancellation.is_cancelled() {
            AppError::Cancelled
        } else {
            AppError::Internal("IPC HLS owner is unavailable".into())
        }
    }

    /// Runs the sole owner, then awaits children and directory cleanup on cancellation.
    /// # Errors
    /// Returns duplicate-owner, process-wait or directory-cleanup failures.
    /// Already-started native setup is awaited under the lifecycle's shared deadline.
    pub async fn run(&self, timeout_secs: u64) -> AppResult<()> {
        self.run_with_factory(
            Duration::from_secs(timeout_secs),
            Duration::from_secs(10),
            start_child,
        )
        .await
    }

    async fn run_with_factory<Factory, Operation>(
        &self,
        timeout: Duration,
        sweep_period: Duration,
        factory: Factory,
    ) -> AppResult<()>
    where
        Factory: Fn(StreamInput, PathBuf, CancellationToken) -> Operation,
        Operation: Future<Output = AppResult<Child>>,
    {
        let mut receiver = self
            .receiver
            .lock()
            .map_err(|_| AppError::Internal("IPC HLS receiver lock failed".into()))?
            .take()
            .ok_or_else(|| AppError::Internal("IPC HLS owner already started".into()))?;
        let mut sessions = HashMap::<String, StreamSession>::new();
        let mut failed = false;
        let mut interval = tokio::time::interval(sweep_period);
        tracing::info!("IPC HLS session owner started");
        loop {
            tokio::select! {
                biased;
                _ = self.cancellation.cancelled() => break,
                _ = interval.tick() => {
                    if self.cancellation.is_cancelled() {
                        break;
                    }
                    let expired = sessions.iter()
                        .filter(|(_, session)| session.last_accessed.elapsed() > timeout)
                        .map(|(key, _)| key.clone())
                        .collect();
                    failed |= !cleanup_sessions(&mut sessions, expired).await;
                }
                request = receiver.recv() => {
                    let Some(request) = request else { break };
                    if self.cancellation.is_cancelled() {
                        let _ = request.reply.send(Err(AppError::Cancelled));
                        break;
                    }
                    if request.reply.is_closed() {
                        continue;
                    }
                    let key = request.input.asset_id.clone();
                    let result = self.start_or_touch(request.input, &mut sessions, &factory).await;
                    if result.is_err() {
                        failed |= !cleanup_sessions(&mut sessions, vec![key]).await;
                    }
                    let _ = request.reply.send(result);
                }
            }
        }
        self.cancellation.cancel();
        receiver.close();
        while let Some(request) = receiver.recv().await {
            let _ = request.reply.send(Err(AppError::Cancelled));
        }
        let session_count = sessions.len();
        tracing::info!(session_count, "IPC HLS shutdown started; admission closed");
        let keys = sessions.keys().cloned().collect();
        failed |= !cleanup_sessions(&mut sessions, keys).await;
        tracing::info!(
            session_count,
            remaining_sessions = sessions.len(),
            clean = !failed,
            "IPC HLS shutdown completed; process waits and directory cleanup finished"
        );
        if failed {
            Err(AppError::Internal(
                "IPC HLS resource cleanup could not be verified".into(),
            ))
        } else {
            Ok(())
        }
    }

    async fn start_or_touch<Factory, Operation>(
        &self,
        input: StreamInput,
        sessions: &mut HashMap<String, StreamSession>,
        factory: &Factory,
    ) -> AppResult<PathBuf>
    where
        Factory: Fn(StreamInput, PathBuf, CancellationToken) -> Operation,
        Operation: Future<Output = AppResult<Child>>,
    {
        if self.cancellation.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        if let Some(session) = sessions.get_mut(&input.asset_id) {
            if session.process.is_none() {
                return Err(AppError::Internal(
                    "Previous IPC HLS cleanup is incomplete".into(),
                ));
            }
            session.last_accessed = Instant::now();
            return Ok(session.output_dir.clone());
        }
        // Internally generated directories cannot alias originals or replacement sessions.
        let output_dir = self.streams_dir.join(uuid::Uuid::new_v4().to_string());
        let key = input.asset_id.clone();
        sessions.insert(
            key.clone(),
            StreamSession {
                process: None,
                last_accessed: Instant::now(),
                output_dir: output_dir.clone(),
            },
        );
        tokio::fs::create_dir_all(&output_dir).await?;
        if self.cancellation.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        let child = factory(input, output_dir.clone(), self.cancellation.clone()).await?;
        // No await between obtaining the child and assigning it to its sole owner.
        if let Some(session) = sessions.get_mut(&key) {
            session.process = Some(child);
        }
        tracing::info!(asset_id = %key, "IPC HLS session started");
        Ok(output_dir)
    }
}

async fn cleanup_sessions(
    sessions: &mut HashMap<String, StreamSession>,
    keys: Vec<String>,
) -> bool {
    let mut clean = true;
    for key in keys {
        if let Some(session) = sessions.get_mut(&key) {
            if let Err(error) = session.stop().await {
                tracing::error!(asset_id = %key, %error, "IPC HLS cleanup failed; resource retained");
                clean = false;
                continue;
            }
        }
        sessions.remove(&key);
    }
    clean
}

#[cfg(test)]
#[path = "hls_manager/tests.rs"]
mod tests;
