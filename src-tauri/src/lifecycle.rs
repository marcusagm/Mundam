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

    /// Named handles for every spawned long-running task.
    /// The key is a human-readable identifier (e.g. `"thumbnail_worker"`,
    /// `"watcher:/Users/me/photos"`).
    tasks: Mutex<HashMap<String, (CancellationToken, JoinHandle<()>)>>,
}

impl LifecycleRegistry {
    /// Create a new registry with a fresh root cancellation token.
    pub fn new() -> Self {
        Self {
            root_token: CancellationToken::new(),
            tasks: Mutex::new(HashMap::new()),
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
        if let Some((old_token, _old_handle)) = tasks.insert(name.clone(), (token, handle)) {
            old_token.cancel();
            info!(task = %name, "Replaced existing task");
        } else {
            info!(task = %name, "Registered task");
        }
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
            tasks.remove(name)
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
        info!("Initiating full shutdown...");
        self.root_token.cancel();

        let tasks: HashMap<String, (CancellationToken, JoinHandle<()>)> = {
            let mut guard = self
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::mem::take(&mut *guard)
        };

        let task_count = tasks.len();
        let mut clean = true;
        for (name, (_token, mut handle)) in tasks {
            info!(task = %name, "Awaiting task...");

            // Abort fallback on timeout
            match timeout(Duration::from_secs(5), &mut handle).await {
                Ok(Ok(_)) => info!(task = %name, "Task stopped gracefully"),
                Ok(Err(join_error)) => {
                    clean = false;
                    error!(task = %name, error = %join_error, "Task panicked during shutdown")
                }
                Err(_) => {
                    warn!(task = %name, "Task shutdown timed out after 5s. Forcing abort.");
                    clean = false;
                    handle.abort();
                    if let Err(join_error) = handle.await {
                        info!(task = %name, error = %join_error, "Aborted task finished");
                    }
                }
            }
        }
        info!(task_count, clean, "Registered task shutdown complete");
        clean
    }
}

impl Default for LifecycleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
