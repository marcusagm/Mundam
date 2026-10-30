//! One admission gate prevents automatic processing from racing manual duplicate scans.

use tokio::sync::{Mutex, MutexGuard};

/// Shared admission across hashing and matching entry points in this application instance.
#[derive(Default)]
pub struct DuplicateProcessingCoordinator {
    gate: Mutex<()>,
}

impl DuplicateProcessingCoordinator {
    /// Acquires an execution slot; callers select cancellation while waiting.
    pub async fn acquire(&self) -> MutexGuard<'_, ()> {
        self.gate.lock().await
    }
}
