//! Application Lifecycle Management
//!
//! Provides a centralized registry for managing the lifecycle of all long-running
//! background tasks (watchers, workers, servers). Uses hierarchical `CancellationToken`s
//! for cooperative shutdown and retains `JoinHandle`s for graceful await on termination.
//!
//! # Architecture
//!
//! ```text
//! LifecycleRegistry (root CancellationToken)
//!  ├── watcher_token_1 (child)
//!  ├── watcher_token_2 (child)
//!  ├── thumbnail_worker_token (child)
//!  └── streaming_server_token (child)
//!       ├── cleanup_process_token (grandchild)
//!       └── cleanup_linear_token (grandchild)
//! ```
//!
//! Cancelling the root token propagates to all children, enabling full app shutdown.
//! Individual subsystems can also be stopped independently via their child tokens.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Centralized registry for managing the lifecycle of all background tasks.
///
/// Holds a root `CancellationToken` whose cancellation propagates to every
/// child token created from it, plus a map of named `JoinHandle`s so that
/// shutdown can await actual task completion.
pub struct LifecycleRegistry {
    /// Root cancellation token. Cancelling this stops every subsystem.
    root_token: CancellationToken,
    reported_failure: AtomicBool,

    /// Named handles for every spawned long-running task.
    /// The key is a human-readable identifier (e.g. `"thumbnail_worker"`,
    /// `"watcher:/Users/me/photos"`).
    tasks: Mutex<RegistryState>,
}

#[derive(Default)]
struct RegistryState {
    entries: HashMap<String, (CancellationToken, JoinHandle<()>)>,
    /// Set atomically with the final drain so late registrations are rejected.
    closed: bool,
}

impl LifecycleRegistry {
    /// Create a new registry with a fresh root cancellation token.
    pub fn new() -> Self {
        Self {
            root_token: CancellationToken::new(),
            reported_failure: AtomicBool::new(false),
            tasks: Mutex::new(RegistryState::default()),
        }
    }

    /// Returns a clone of the root cancellation token.
    ///
    /// Use this when a subsystem only needs to observe the global shutdown
    /// signal without registering a named task.
    pub fn root_token(&self) -> CancellationToken {
        self.root_token.clone()
    }

    /// Creates a new child token derived from the root.
    ///
    /// The child is automatically cancelled when the root is cancelled,
    /// but can also be cancelled independently without affecting siblings.
    pub fn child_token(&self) -> CancellationToken {
        self.root_token.child_token()
    }

    /// Records a subsystem failure that prevents a verified clean shutdown.
    pub fn report_failure(&self, subsystem: &str) {
        self.reported_failure.store(true, Ordering::Release);
        error!(subsystem, "Subsystem reported an incomplete shutdown");
    }

    /// Register a named background task with its cancellation token and join handle.
    ///
    /// If a task with the same name already exists, its token is cancelled
    /// and its handle is dropped (detached). This makes registration idempotent
    /// for cases like restarting a watcher on the same path.
    pub fn register(&self, name: String, token: CancellationToken, handle: JoinHandle<()>) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if tasks.closed {
            token.cancel();
            handle.abort();
            warn!(task = %name, "Rejected task registration after shutdown started");
            return;
        }
        if let Some((old_token, _old_handle)) = tasks.entries.insert(name.clone(), (token, handle))
        {
            old_token.cancel();
            info!(task = %name, "Replaced existing task");
        } else {
            info!(task = %name, "Registered task");
        }
    }

    /// Atomically admits and starts a uniquely named task before shutdown.
    ///
    /// Returns `false` without spawning if shutdown has begun or the name exists.
    pub fn spawn_unique<F>(&self, name: String, future: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if tasks.closed || self.root_token.is_cancelled() || tasks.entries.contains_key(&name) {
            warn!(task = %name, "Rejected background task admission");
            return false;
        }
        let token = self.child_token();
        let handle = tauri::async_runtime::spawn(future);
        tasks.entries.insert(name.clone(), (token, handle));
        info!(task = %name, "Registered task");
        true
    }

    /// Cancel and await a specific named task.
    ///
    /// Returns `true` if the task was found and stopped, `false` if no task
    /// with that name existed in the registry.
    ///
    /// # Errors
    ///
    /// Logs a warning if the task panicked during shutdown but does not propagate it.
    pub async fn shutdown_by_name(&self, name: &str) -> bool {
        let entry = {
            let mut tasks = self
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            tasks.entries.remove(name)
        };

        if let Some((token, handle)) = entry {
            info!(task = %name, "Shutting down task");
            token.cancel();

            // Wait up to 5 seconds for the task to finish gracefully
            let mut handle = handle;
            match timeout(Duration::from_secs(5), &mut handle).await {
                Ok(Ok(_)) => info!(task = %name, "Task stopped gracefully"),
                Ok(Err(join_error)) => {
                    error!(task = %name, error = %join_error, "Task panicked during shutdown")
                }
                Err(_) => {
                    warn!(task = %name, "Task shutdown timed out after 5s. Forcing abort.");
                    handle.abort();
                }
            }
            true
        } else {
            false
        }
    }

    /// Cancel the root token and await all registered tasks.
    ///
    /// This is the primary shutdown path, called when the application is closing.
    /// All child tokens are cancelled automatically via the root, and each task
    /// handle is awaited to ensure orderly cleanup.
    ///
    /// Returns `false` when a registered task panics or exceeds its timeout.
    /// This does not account for work that was not registered.
    pub async fn shutdown_all(&self) -> bool {
        self.shutdown_within(SHUTDOWN_BUDGET).await
    }

    async fn shutdown_within(&self, budget: Duration) -> bool {
        info!("Initiating full shutdown...");
        self.root_token.cancel();

        let mut tasks: HashMap<String, (CancellationToken, JoinHandle<()>)> = {
            let mut guard = self
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.closed = true;
            std::mem::take(&mut guard.entries)
        };

        let task_count = tasks.len();
        let deadline = tokio::time::Instant::now() + budget;
        let bridge = tasks.remove("event_bridge_frontend");
        let mut drains = tokio::task::JoinSet::new();
        for (name, (_token, handle)) in tasks {
            drains.spawn(drain_task(name, handle, deadline));
        }
        let mut clean = true;
        while let Some(result) = drains.join_next().await {
            match result {
                Ok(task_clean) => clean &= task_clean,
                Err(join_error) => {
                    clean = false;
                    error!(error = %join_error, "Shutdown drain task failed");
                }
            }
        }
        if let Some((token, handle)) = bridge {
            token.cancel();
            let bridge_deadline = deadline.max(tokio::time::Instant::now() + BRIDGE_DRAIN_GRACE);
            if bridge_deadline > deadline {
                info!(
                    grace_ms = BRIDGE_DRAIN_GRACE.as_millis() as u64,
                    "Allowing event bridge additional time to drain"
                );
            }
            clean &= drain_task("event_bridge_frontend".to_string(), handle, bridge_deadline).await;
        }
        clean &= !self.reported_failure.load(Ordering::Acquire);
        info!(task_count, clean, "Registered task shutdown complete");
        clean
    }
}

/// Global time allowed for all registered tasks to stop, drained concurrently.
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);
const BRIDGE_DRAIN_GRACE: Duration = Duration::from_millis(250);

/// Awaits one task until the shared deadline, aborting it afterwards.
async fn drain_task(
    name: String,
    mut handle: JoinHandle<()>,
    deadline: tokio::time::Instant,
) -> bool {
    info!(task = %name, "Awaiting task...");
    match tokio::time::timeout_at(deadline, &mut handle).await {
        Ok(Ok(_)) => {
            info!(task = %name, "Task stopped gracefully");
            true
        }
        Ok(Err(join_error)) => {
            error!(task = %name, error = %join_error, "Task panicked during shutdown");
            false
        }
        Err(_) => {
            warn!(task = %name, "Task did not stop within the shutdown budget. Forcing abort.");
            handle.abort();
            if let Err(join_error) = handle.await {
                info!(task = %name, error = %join_error, "Aborted task finished");
            }
            false
        }
    }
}

impl Default for LifecycleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod shutdown_budget_tests {
    use super::*;

    #[tokio::test]
    async fn slow_tasks_share_one_budget_and_are_aborted() {
        let registry = LifecycleRegistry::new();
        for index in 0..3 {
            let token = registry.child_token();
            let handle = tauri::async_runtime::spawn(async {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            });
            registry.register(format!("slow-{index}"), token, handle);
        }
        let started = tokio::time::Instant::now();
        assert!(!registry.shutdown_within(Duration::from_millis(200)).await);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn registration_after_shutdown_is_rejected_and_aborted() {
        let registry = LifecycleRegistry::new();
        assert!(registry.shutdown_all().await);
        let token = registry.child_token();
        let handle = tauri::async_runtime::spawn(async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        registry.register("late".to_string(), token.clone(), handle);
        assert!(token.is_cancelled());
        assert!(registry
            .tasks
            .lock()
            .map(|state| state.entries.is_empty())
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn cooperative_tasks_report_clean() {
        let registry = LifecycleRegistry::new();
        let token = registry.child_token();
        let waiting = token.clone();
        let handle = tauri::async_runtime::spawn(async move { waiting.cancelled().await });
        registry.register("coop".to_string(), token, handle);
        assert!(registry.shutdown_all().await);
    }

    #[tokio::test]
    async fn reported_failure_degrades_registered_task_shutdown() {
        let registry = std::sync::Arc::new(LifecycleRegistry::new());
        let token = registry.child_token();
        let waiting = token.clone();
        let task_registry = registry.clone();
        let handle = tauri::async_runtime::spawn(async move {
            waiting.cancelled().await;
            task_registry.report_failure("test");
        });
        registry.register("reporter".to_string(), token, handle);
        assert!(!registry.shutdown_all().await);
    }

    #[tokio::test]
    async fn admitted_dynamic_task_is_awaited_and_late_task_never_starts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry = std::sync::Arc::new(LifecycleRegistry::new());
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        assert!(registry.spawn_unique("scan-1".into(), async move {
            let _ = entered_tx.send(());
            let _ = release_rx.await;
        }));
        entered_rx.await?;
        let shutting_down = registry.clone();
        let shutdown = tokio::spawn(async move { shutting_down.shutdown_all().await });
        registry.root_token().cancelled().await;
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        let late_ran = ran.clone();
        assert!(!registry.spawn_unique("scan-2".into(), async move {
            late_ran.store(true, Ordering::SeqCst);
        }));
        assert!(!shutdown.is_finished());
        release_tx.send(()).map_err(|_| "scan already stopped")?;
        assert!(shutdown.await?);
        assert!(!ran.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn duplicate_dynamic_task_name_does_not_replace_running_task() {
        let registry = LifecycleRegistry::new();
        let token = registry.root_token();
        assert!(registry.spawn_unique("scan".into(), async move { token.cancelled().await }));
        assert!(!registry.spawn_unique("scan".into(), async {}));
        assert!(registry.shutdown_all().await);
    }

    #[tokio::test]
    async fn bridge_remains_available_until_registered_producers_stop(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry = LifecycleRegistry::new();
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel();
        let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
        let bridge_token = CancellationToken::new();
        let bridge_cancel = bridge_token.clone();
        let bridge = tauri::async_runtime::spawn(async move {
            let event = events_rx.recv().await;
            let _ = observed_tx.send(event);
            bridge_cancel.cancelled().await;
        });
        registry.register("event_bridge_frontend".into(), bridge_token, bridge);
        let producer_token = registry.child_token();
        let waiting = producer_token.clone();
        let producer = tauri::async_runtime::spawn(async move {
            waiting.cancelled().await;
            let _ = events_tx.send(42);
        });
        registry.register("producer".into(), producer_token, producer);
        assert!(registry.shutdown_all().await);
        assert_eq!(observed_rx.await?, Some(42));
        Ok(())
    }

    #[tokio::test]
    async fn delayed_bridge_still_has_a_bounded_deadline() {
        let registry = LifecycleRegistry::new();
        let token = CancellationToken::new();
        let waiting = token.clone();
        let bridge = tauri::async_runtime::spawn(async move {
            waiting.cancelled().await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        registry.register("event_bridge_frontend".into(), token, bridge);
        let start = tokio::time::Instant::now();
        assert!(!registry.shutdown_within(Duration::from_millis(100)).await);
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn bridge_gets_time_to_drain_after_producer_exceeds_deadline(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry = LifecycleRegistry::new();
        let bridge_token = CancellationToken::new();
        let waiting = bridge_token.clone();
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
        let bridge = tauri::async_runtime::spawn(async move {
            waiting.cancelled().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            let _ = finished_tx.send(());
        });
        registry.register("event_bridge_frontend".into(), bridge_token, bridge);
        let producer_token = registry.child_token();
        let producer = tauri::async_runtime::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        registry.register("slow_producer".into(), producer_token, producer);
        let started = tokio::time::Instant::now();
        assert!(!registry.shutdown_within(Duration::from_millis(100)).await);
        assert!(finished_rx.await.is_ok());
        assert!(started.elapsed() < Duration::from_secs(2));
        Ok(())
    }
}
