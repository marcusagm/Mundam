//! Coordinates native observation, admission and the owned notification pipeline.

use super::native::{NativeChange, NativeOutcome, NativeWatcher};
use super::runtime::{self, PipelineReport};
use crate::core::error::{AppError, AppResult};
use crate::core::events::{bus::AppEventBus, DomainEvent};
use notify::Event;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument};
use unicode_normalization::UnicodeNormalization;

/// Singleton service that manages active filesystem watchers.
///
/// It listens for low-level OS events, debounces them, and publishes
/// clean Domain Events to the application's Event Bus.
pub struct WatcherService {
    /// Broadcast bus for publishing domain events.
    event_bus: Arc<dyn AppEventBus>,
    state: Mutex<WatcherState>,
    cancellation: CancellationToken,
    /// Explicit registrations, including roots subsequently adopted by another root.
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

#[derive(Default)]
struct WatcherState {
    native_watcher: Option<NativeWatcher>,
    native_operation: Option<tokio::task::JoinHandle<NativeOutcome>>,
    input: Option<mpsc::Sender<Event>>,
    pipeline: Option<tokio::task::JoinHandle<PipelineReport>>,
    closed: bool,
    supervision_failed: bool,
    completed: bool,
    pending_added: Vec<PathBuf>,
    pending_removed: Vec<PathBuf>,
}

/// Implementation of the WatcherService.
impl WatcherService {
    /// Create a new WatcherService.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - Broadcast bus for publishing domain events.
    /// * `cancellation` - Service lifetime token shared by all roots and reconciliation.
    ///
    /// # Returns
    ///
    /// * `Self` - A new WatcherService instance.
    pub fn new(event_bus: Arc<dyn AppEventBus>, cancellation: CancellationToken) -> Self {
        Self {
            event_bus,
            state: Mutex::new(WatcherState::default()),
            cancellation,
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Admits one root using the lifetime token shared by the whole watcher service.
    /// # Errors
    /// Returns cancellation after admission closes, or native watcher/supervision errors.
    #[instrument(skip_all)]
    pub async fn watch(&self, path: PathBuf) -> AppResult<()> {
        let path = normalized_watch_path(&path);
        let mut state = self.state.lock().await;
        self.require_open(&state)?;
        self.finish_native_operation(&mut state).await?;
        self.require_open(&state)?;
        // Reconciliation can race with boot/manual setup using an older snapshot.
        // Register each root only once: notify's Windows backend replaces the
        // entry without releasing the previous native request on duplicate watch.
        // Use the registered spelling, not forgiving containment/case hints,
        // to avoid merging distinct roots on case-sensitive Windows directories.
        let already_registered = self.watched_paths.lock().await.contains(&path);
        self.require_open(&state)?;
        if already_registered {
            tracing::debug!(path = %path.display(), "Watcher root already registered; native registration skipped");
            return Ok(());
        }
        self.ensure_pipeline(&mut state);
        self.start_native_operation(&mut state, NativeChange::Watch(path.clone()))?;
        self.finish_native_operation(&mut state).await?;
        self.require_open(&state)?;
        info!(path = %path.display(), "Watcher service started");
        Ok(())
    }

    /// Removes a tree and its adopted roots without stopping observation of other roots.
    /// # Errors
    /// Returns cancellation after admission closes, or native watcher/supervision errors.
    #[instrument(skip(self))]
    pub async fn unwatch(&self, path: PathBuf) -> AppResult<()> {
        let path = normalized_watch_path(&path);
        let mut state = self.state.lock().await;
        self.require_open(&state)?;
        self.finish_native_operation(&mut state).await?;
        self.require_open(&state)?;
        let registrations = self.watched_paths.lock().await;
        let removed_paths = registrations
            .iter()
            .filter(|registered| is_within_watch(registered, &path))
            .cloned()
            .collect();
        drop(registrations);
        if state.native_watcher.is_some() {
            self.start_native_operation(&mut state, NativeChange::Unwatch(removed_paths))?;
            self.finish_native_operation(&mut state).await?;
        }
        Ok(())
    }

    fn require_open(&self, state: &WatcherState) -> AppResult<()> {
        if state.closed || self.cancellation.is_cancelled() {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn ensure_pipeline(&self, state: &mut WatcherState) {
        if state.pipeline.is_none() {
            let (sender, receiver) = mpsc::channel(1000);
            state.input = Some(sender);
            state.pipeline = Some(tokio::spawn(runtime::run(
                receiver,
                self.event_bus.clone(),
                self.watched_paths.clone(),
                self.cancellation.clone(),
            )));
            info!("Watcher notification pipeline started");
        }
    }

    fn start_native_operation(
        &self,
        state: &mut WatcherState,
        change: NativeChange,
    ) -> AppResult<()> {
        let sender = state.input.as_ref().cloned().ok_or_else(|| {
            AppError::Internal("Watcher notification channel is unavailable".into())
        })?;
        let watcher = state.native_watcher.take();
        let cancellation = self.cancellation.clone();
        state.native_operation = Some(tokio::task::spawn_blocking(move || {
            super::native::apply(watcher, change, sender, cancellation)
        }));
        Ok(())
    }

    async fn finish_native_operation(&self, state: &mut WatcherState) -> AppResult<()> {
        // Await by reference so a dropped request cannot detach the native owner.
        let result = match state.native_operation.as_mut() {
            Some(operation) => Some(operation.await),
            None => None,
        };
        state.native_operation = None;
        let operation_result = match result {
            Some(Ok(outcome)) => {
                state.supervision_failed |= outcome.supervision_failed;
                state.native_watcher = outcome.watcher;
                state.pending_added.extend(outcome.added);
                state.pending_removed.extend(outcome.removed);
                outcome.result
            }
            Some(Err(error)) => {
                state.supervision_failed = true;
                Err(AppError::Internal(format!(
                    "Native watcher owner failed: {error}"
                )))
            }
            None => Ok(()),
        };
        if !state.pending_added.is_empty() || !state.pending_removed.is_empty() {
            let mut registrations = self.watched_paths.lock().await;
            registrations.extend(state.pending_added.drain(..));
            for path in state.pending_removed.drain(..) {
                registrations.remove(&path);
            }
        }
        operation_result
    }

    /// Closes admission, releases native observation and awaits both notification loops.
    /// Volatile hints are discarded; the existing boot scan reconciles filesystem state.
    /// # Errors
    /// Returns supervision failures instead of reporting a verified clean shutdown.
    /// In-flight native operations remain owned if this future is cancelled by a deadline.
    pub async fn shutdown(&self) -> AppResult<()> {
        self.cancellation.cancel();
        let mut state = self.state.lock().await;
        state.closed = true;
        if state.completed {
            return watcher_shutdown_result(state.supervision_failed);
        }
        info!("Watcher shutdown started; admission closed");
        if let Err(error) = self.finish_native_operation(&mut state).await {
            tracing::debug!(%error, "Watcher native request completed during shutdown");
        }
        if state.native_watcher.is_some() {
            self.start_native_operation(&mut state, NativeChange::Stop)?;
            self.finish_native_operation(&mut state).await?;
            info!("Watcher native observation released");
        }
        state.input = None;
        if let Some(pipeline) = state.pipeline.as_mut() {
            match pipeline.await {
                Ok(report) => info!(
                    raw_discarded = report.raw_discarded,
                    pending_discarded = report.pending_discarded,
                    domain_discarded = report.domain_discarded,
                    publication_failures = report.publication_failures,
                    "Watcher notification loops finished; discarded hints reconciled at next boot"
                ),
                Err(error) => {
                    state.supervision_failed = true;
                    tracing::error!(%error, "Watcher notification owner failed");
                }
            }
        }
        state.pipeline = None;
        self.watched_paths.lock().await.clear();
        state.completed = true;
        let result = watcher_shutdown_result(state.supervision_failed);
        info!(clean = result.is_ok(), "Watcher shutdown completed");
        result
    }
}

fn watcher_shutdown_result(failed: bool) -> AppResult<()> {
    if failed {
        Err(AppError::Internal(
            "Watcher shutdown could not verify every owner".into(),
        ))
    } else {
        Ok(())
    }
}

/// Drops debounced events belonging exclusively to locations already unlinked.
pub(super) fn is_event_monitored(event: &DomainEvent, registrations: &HashSet<PathBuf>) -> bool {
    let contains_path = |path: &str| {
        registrations
            .iter()
            .any(|root| is_within_watch(Path::new(path), root))
    };
    match event {
        DomainEvent::FsFileDiscovered { path, .. }
        | DomainEvent::FsDirectoryDiscovered { path }
        | DomainEvent::FsPathDeleted { path }
        | DomainEvent::FsDirectoryDeleted { path } => contains_path(path),
        DomainEvent::FsPathRenamed { from, to } => contains_path(from) || contains_path(to),
        _ => false,
    }
}

/// Preserves registered spelling; forgiving containment hints never replace catalog identity proofs.
fn normalized_watch_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// Applies component boundaries while accepting Windows separator and case variants.
fn is_within_watch(path: &Path, root: &Path) -> bool {
    let path = PathBuf::from(path.to_string_lossy().nfc().collect::<String>());
    let root = PathBuf::from(root.to_string_lossy().nfc().collect::<String>());
    if cfg!(windows) {
        let comparable_path =
            PathBuf::from(path.to_string_lossy().replace('/', "\\").to_lowercase());
        let comparable_root =
            PathBuf::from(root.to_string_lossy().replace('/', "\\").to_lowercase());
        comparable_path.starts_with(comparable_root)
    } else {
        path.starts_with(root)
    }
}

#[cfg(test)]
#[path = "sensor_tests.rs"]
mod tests;

#[async_trait::async_trait]
impl crate::core::library::RootObservation for WatcherService {
    async fn reconcile(&self, roots: Vec<PathBuf>) -> AppResult<()> {
        if self.cancellation.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        let desired: HashSet<_> = roots
            .into_iter()
            .map(|path| normalized_watch_path(&path))
            .collect();
        let registered = self.watched_paths.lock().await.clone();
        let mut failures = Vec::new();
        for stale in registered.difference(&desired) {
            if let Err(error) = self.unwatch(stale.clone()).await {
                failures.push(error.to_string());
            }
        }
        let registered = self.watched_paths.lock().await.clone();
        for missing in desired.difference(&registered) {
            if let Err(error) = self.watch(missing.clone()).await {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(crate::core::error::AppError::Internal(failures.join("; ")))
        }
    }
}
