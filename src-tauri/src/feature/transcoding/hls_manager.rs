//! HLS Factory and Subprocess Manager
//!
//! Lazily instantiates FFmpeg pipelines for on-the-fly streaming of heavy media.
//! Tracks session activities and implements timeout sweeps to free resources properly.

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

use super::profiles::TranscodingProfile;
use crate::core::error::{AppError, AppResult};

/// A running FFmpeg fragmentation session.
pub struct StreamSession {
    /// The subprocess tracking the FFmpeg binary execution.
    pub process: Child,
    /// Instantiation of the last time the frontend requested a chunk.
    pub last_accessed: Instant,
    /// The directory where the fragments are being temporarily written to.
    pub output_dir: PathBuf,
}

/// The stateful orchestrator of all streaming subprocesses.
#[derive(Clone)]
pub struct HlsManager {
    /// Concurrent hashmap containing active session ID strings and processes.
    pub sessions: Arc<DashMap<String, StreamSession>>,
    /// Base directory where `streams/{asset_id}` fragment folders reside.
    pub streams_dir: PathBuf,
}

/// Implementation of HlsManager.
impl HlsManager {
    /// Initializes a new HLS Subprocess Factory.
    ///
    /// # Arguments
    /// * `app_data_dir` - Reconstructed base configuration directory from Tauri.
    ///
    /// # Returns
    /// An Arc-wrapped configured `HlsManager`.
    pub fn new(app_data_dir: &Path) -> Arc<Self> {
        let streams_dir = app_data_dir.join("streams");
        let _ = std::fs::create_dir_all(&streams_dir);

        Arc::new(Self {
            sessions: Arc::new(DashMap::<String, StreamSession>::new()),
            streams_dir,
        })
    }

    /// Extends the lifetime of a specific running stream.
    /// To be called by the `asset://` handler whenever a segment or playlist is fetched.
    ///
    /// # Arguments
    ///
    /// * `session_id` - ID of the session to touch.
    ///
    /// # Returns
    ///
    /// Ok(()) if the session was touched successfully.
    pub fn touch_session(&self, session_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.last_accessed = Instant::now();
        }
    }

    /// Checks if an ongoing stream session is active, or fires up FFmpeg lazily.
    ///
    /// # Arguments
    /// * `asset_id` - Unique identifier for the media.
    /// * `original_path` - Physical location of the legacy unstreamable video/audio file.
    /// * `mime_type` - Optional MIME detector fallback for audio profiles.
    ///
    /// # Errors
    /// Returns an `AppError` wrapping std::io if directory permissions fail or FFmpeg crashes.
    pub async fn get_or_start_stream(
        &self,
        asset_id: &str,
        original_path: &Path,
        mime_type: Option<&str>,
    ) -> AppResult<PathBuf> {
        let session_dir = self.streams_dir.join(asset_id);

        // If the streaming engine is mapping it currently...
        if self.sessions.contains_key(asset_id) {
            self.touch_session(asset_id);
            return Ok(session_dir);
        }

        // Fresh session setup
        // Nuke previous aborted fragments to avoid H.264 desync crashes.
        let _ = tokio::fs::remove_dir_all(&session_dir).await;
        tokio::fs::create_dir_all(&session_dir).await.map_err(|e| {
            AppError::Generic(format!("Failed to create HLS output directory: {}", e))
        })?;

        let profile = TranscodingProfile::resolve_for_hls(original_path, mime_type);
        let playlist_path = session_dir.join("playlist.m3u8");
        let segment_pattern = session_dir.join("segment_%05d.ts");

        let tools = crate::processing::transcoding::resolve_transcoding_tools::<tauri::Wry>(None)?;
        let mut cmd = Command::new(tools.ffmpeg);

        cmd.args(&profile.input_args)
            .arg("-i")
            .arg(original_path)
            .args(&profile.output_args)
            .arg("-hls_segment_filename")
            .arg(&segment_pattern)
            .arg(&playlist_path);

        // Crucial: hide output pipelines so we do not freeze Tauri RPC threads implicitly.
        // Also ensure children die if the manager drops them.
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Generic(format!("Fatal spawn of FFmpeg subprocess: {}", e)))?;

        self.sessions.insert(
            asset_id.to_string(),
            StreamSession {
                process: child,
                last_accessed: Instant::now(),
                output_dir: session_dir.clone(),
            },
        );

        tracing::info!(
            "HLS Transcoding Factory invoked explicitly on Demand for AssetID: {}",
            asset_id
        );

        Ok(session_dir)
    }

    /// Background task to sweep the tracker locking down orphaned unused processes.
    /// Intended to be invoked via `tokio::spawn(manager.start_cleanup_worker(token, 90))` in boot.
    ///
    /// # Arguments
    ///
    /// * `token` - Cancellation token to signal shutdown.
    /// * `timeout_secs` - Timeout in seconds to wait before killing a session.
    ///
    /// # Returns
    ///
    /// Ok(()) if the cleanup worker was started successfully.
    pub fn start_cleanup_worker(&self, token: CancellationToken, timeout_secs: u64) -> tauri::async_runtime::JoinHandle<()> {
        let sessions = self.sessions.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::info!("HlsManager: Shutdown signal received. Cleaning up active streams.");
                        sessions.clear();
                        break;
                    }
                    _ = interval.tick() => {
                        let now = Instant::now();
                        let mut expired_keys = Vec::new();

                        for entry in sessions.iter() {
                            if now.duration_since(entry.value().last_accessed).as_secs() > timeout_secs {
                                expired_keys.push(entry.key().clone());
                            }
                        }

                        for key in expired_keys {
                            if let Some((_, session)) = sessions.remove(&key) {
                                tracing::info!(
                                    "Sweeping inactive HLS stream for '{}' to free memory...",
                                    key
                                );
                                // Child process killed on drop thanks to .kill_on_drop(true)

                                // Async FS operations gracefully erasing the fragments
                                let out_dir = session.output_dir.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                    let _ = tokio::fs::remove_dir_all(&out_dir).await;
                                });
                            }
                        }
                    }
                }
            }
        })
    }
}
