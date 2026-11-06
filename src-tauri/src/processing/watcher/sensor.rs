use super::debouncer::EventDebouncer;
use crate::core::error::AppResult;
use crate::core::events::{bus::AppEventBus, DomainEvent};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
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
    /// Handle to the native watcher (kept alive).
    native_watcher: Mutex<Option<RecommendedWatcher>>,
    /// Explicit registrations, including roots subsequently adopted by another root.
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

/// Implementation of the WatcherService.
impl WatcherService {
    /// Create a new WatcherService.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - Broadcast bus for publishing domain events.
    ///
    /// # Returns
    ///
    /// * `Self` - A new WatcherService instance.
    pub fn new(event_bus: Arc<dyn AppEventBus>) -> Self {
        Self {
            event_bus,
            native_watcher: Mutex::new(None),
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Start watching a directory.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to watch.
    /// * `token` - Cancellation token for the background loop.
    ///
    /// # Returns
    ///
    /// * `AppResult<()>` - Result of the watch operation.
    #[instrument(skip_all)]
    pub async fn watch(&self, path: PathBuf, _token: CancellationToken) -> AppResult<()> {
        let path = normalized_watch_path(&path);
        let mut guard = self.native_watcher.lock().await;

        if guard.is_none() {
            let (tx, mut rx) = mpsc::channel::<Event>(1000);
            let event_bus = self.event_bus.clone();
            let watched_paths = self.watched_paths.clone();

            let watcher = RecommendedWatcher::new(
                move |res: notify::Result<Event>| {
                    if let Ok(mut event) = res {
                        // Preserve native spelling; the catalog adapter verifies aliases by identity.
                        for p in &mut event.paths {
                            *p = normalized_watch_path(p);
                        }
                        let _ = tx.blocking_send(event);
                    }
                },
                Config::default().with_poll_interval(Duration::from_millis(500)),
            )?;

            // Spawn the debouncer loop ONCE for all paths
            tokio::spawn(async move {
                let (out_tx, mut out_rx) = mpsc::channel(100);
                let mut debouncer = EventDebouncer::new(out_tx);
                let mut interval = tokio::time::interval(Duration::from_millis(200));

                // Drain concurrently so a discovery burst cannot block its own receiver.
                tokio::spawn(async move {
                    while let Some(domain_event) = out_rx.recv().await {
                        let registrations = watched_paths.lock().await;
                        if is_event_monitored(&domain_event, &registrations) {
                            let _ = event_bus.publish(domain_event);
                        }
                    }
                });

                loop {
                    tokio::select! {
                        received = rx.recv() => {
                            match received { Some(event) => debouncer.handle_event(event).await, None => break }
                        }
                        _ = interval.tick() => {
                            debouncer.tick(Duration::from_millis(600)).await;
                        }
                    }
                }
            });

            *guard = Some(watcher);
            info!("Watcher background loop initialized");
        }

        if let Some(watcher) = guard.as_mut() {
            watcher.watch(&path, RecursiveMode::Recursive)?;
            self.watched_paths.lock().await.insert(path.clone());
            info!("Watcher service started for: {:?}", path);
        }

        Ok(())
    }

    /// Stop watching a directory.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to unwatch.
    ///
    /// # Returns
    ///
    /// * `AppResult<()>` - Result of the unwatch operation.
    #[instrument(skip(self))]
    pub async fn unwatch(&self, path: PathBuf) -> AppResult<()> {
        let path = normalized_watch_path(&path);
        let mut guard = self.native_watcher.lock().await;
        let registrations = self.watched_paths.lock().await;
        let removed_paths: Vec<PathBuf> = registrations
            .iter()
            .filter(|registered| is_within_watch(registered, &path))
            .cloned()
            .collect();

        drop(registrations);
        let mut failures = Vec::new();
        if let Some(watcher) = guard.as_mut() {
            for removed_path in removed_paths {
                if let Err(error) = watcher.unwatch(&removed_path) {
                    if matches!(&error.kind, notify::ErrorKind::WatchNotFound) {
                        self.watched_paths.lock().await.remove(&removed_path);
                    } else {
                        failures.push(format!("{}: {error}", removed_path.display()));
                    }
                } else {
                    self.watched_paths.lock().await.remove(&removed_path);
                    info!("Watcher stopped for: {:?}", removed_path);
                }
            }
        }
        if !failures.is_empty() {
            return Err(crate::core::error::AppError::Internal(failures.join("; ")));
        }
        Ok(())
    }
}

/// Drops debounced events belonging exclusively to locations already unlinked.
fn is_event_monitored(event: &DomainEvent, registrations: &HashSet<PathBuf>) -> bool {
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
            if let Err(error) = self.watch(missing.clone(), CancellationToken::new()).await {
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
