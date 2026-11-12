//! Cancellation registry for owned segment transcoding processes.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Why a segment process was asked to stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationReason {
    /// A newer request for the same segment supersedes it.
    Superseded,
    /// The process exceeded the stale threshold.
    Stale,
    /// The application is shutting down.
    Shutdown,
}

struct ProcessInfo {
    key: String,
    started_at: Instant,
    cancellation: watch::Sender<Option<CancellationReason>>,
}

/// Tracks all started processes until their owners finish waiting for them.
#[derive(Default)]
pub struct ProcessManager {
    processes: HashMap<uuid::Uuid, ProcessInfo>,
    closed: bool,
    owner_failures: usize,
}

impl ProcessManager {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation of all still-running processes for a segment.
    pub fn cancel(&mut self, key: &str, reason: CancellationReason) {
        for info in self.processes.values_mut().filter(|info| info.key == key) {
            if info.cancellation.borrow().is_none() {
                info.cancellation.send_replace(Some(reason));
            }
        }
    }

    /// Registers an owner before releasing the process admission lock.
    pub fn register(
        &mut self,
        key: String,
    ) -> Option<(uuid::Uuid, watch::Receiver<Option<CancellationReason>>)> {
        if self.closed {
            return None;
        }
        self.cancel(&key, CancellationReason::Superseded);
        let id = uuid::Uuid::new_v4();
        let (cancellation, receiver) = watch::channel(None);
        self.processes.insert(
            id,
            ProcessInfo {
                key,
                started_at: Instant::now(),
                cancellation,
            },
        );
        Some((id, receiver))
    }

    /// Closes admission and requests cancellation of tracked owners.
    pub fn cancel_all(&mut self) -> usize {
        self.closed = true;
        let mut requested = 0;
        for info in self.processes.values_mut() {
            if info.cancellation.borrow().is_none() {
                info.cancellation
                    .send_replace(Some(CancellationReason::Shutdown));
                requested += 1;
            }
        }
        requested
    }

    /// Removes only the finished process instance.
    pub fn remove(&mut self, id: uuid::Uuid) {
        self.processes.remove(&id);
    }

    /// Records that an owner could not complete its process supervision.
    pub fn record_owner_failure(&mut self) {
        self.owner_failures += 1;
    }

    /// Count of owner supervision failures during this server's lifetime.
    pub fn owner_failures(&self) -> usize {
        self.owner_failures
    }

    /// Requests cancellation of stale processes without dropping their tracking.
    pub fn cleanup_stale(&mut self, timeout_secs: u64) {
        for info in self.processes.values_mut() {
            if info.started_at.elapsed() > Duration::from_secs(timeout_secs)
                && info.cancellation.borrow().is_none()
            {
                info.cancellation
                    .send_replace(Some(CancellationReason::Stale));
            }
        }
    }

    /// Number of processes whose owners have not completed.
    pub fn active_count(&self) -> usize {
        self.processes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superseded_process_remains_tracked_until_its_owner_finishes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = ProcessManager::new();
        let (first, old) = manager
            .register("segment".into())
            .ok_or("admission closed")?;
        let (second, new) = manager
            .register("segment".into())
            .ok_or("admission closed")?;
        assert_eq!(*old.borrow(), Some(CancellationReason::Superseded));
        assert_eq!(*new.borrow(), None);
        manager.remove(first);
        assert_eq!(manager.active_count(), 1);
        assert_eq!(manager.cancel_all(), 1);
        assert_eq!(*new.borrow(), Some(CancellationReason::Shutdown));
        assert!(manager.register("late".into()).is_none());
        manager.remove(second);
        assert_eq!(manager.active_count(), 0);
        Ok(())
    }
}
