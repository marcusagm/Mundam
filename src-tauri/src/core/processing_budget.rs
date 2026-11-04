//! Fair, drainable admission for media operations; no database leases are held while waiting.

use super::processing_control::WorkerKind;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Operation capacity, not an operating-system thread or memory limit.
#[derive(Clone, Debug, Serialize)]
pub struct ProcessingCapacity {
    pub limit: usize,
    pub active: usize,
    pub waiting: usize,
    pub thumbnail_limit: usize,
}

struct BudgetState {
    limit: usize,
    thumbnails: usize,
    active: BTreeMap<WorkerKind, usize>,
    waiting: BTreeMap<u64, WorkerKind>,
    next: u64,
}

/// FIFO among eligible workers; a capped worker cannot block other workers' free slots.
pub struct ProcessingBudget {
    state: Mutex<BudgetState>,
    changed: Notify,
}

impl Default for ProcessingBudget {
    fn default() -> Self {
        Self::new(
            std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(2),
        )
    }
}

impl ProcessingBudget {
    /// Builds a bounded automatic budget, with serial color and duplicate operations.
    pub fn new(processors: usize) -> Self {
        let automatic = processors.saturating_sub(1).clamp(1, 8);
        Self {
            state: Mutex::new(BudgetState {
                limit: automatic,
                thumbnails: automatic,
                active: BTreeMap::new(),
                waiting: BTreeMap::new(),
                next: 0,
            }),
            changed: Notify::new(),
        }
    }

    /// Applies persisted limits to future admissions; active work always drains normally.
    pub fn configure(&self, shared: usize, thumbnails: usize) {
        let automatic = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(2)
            .saturating_sub(1)
            .clamp(1, 8);
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.limit = if shared == 0 {
            automatic
        } else {
            shared.clamp(1, 8)
        };
        state.thumbnails = if thumbnails == 0 {
            state.limit
        } else {
            thumbnails.clamp(1, 32)
        };
        drop(state);
        self.changed.notify_waiters();
    }

    /// Captures actual resource ownership independently of long-lived manual scan activity.
    pub fn snapshot(&self) -> ProcessingCapacity {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        ProcessingCapacity {
            limit: state.limit,
            active: state.active.values().sum(),
            waiting: state.waiting.len(),
            thumbnail_limit: state.thumbnails.min(state.limit),
        }
    }

    /// Waits without claiming durable work. Cancellation and dropped futures remove their queue entry.
    pub async fn acquire(
        self: &Arc<Self>,
        worker: WorkerKind,
        token: &CancellationToken,
    ) -> Option<ResourcePermit> {
        let request = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.next += 1;
            let request = state.next;
            state.waiting.insert(request, worker);
            request
        };
        let mut waiting = WaitingRequest {
            budget: self.clone(),
            request: Some(request),
        };
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if token.is_cancelled() {
                return None;
            }
            if self.try_acquire(request, worker) {
                waiting.request = None;
                self.changed.notify_waiters();
                return Some(ResourcePermit {
                    budget: self.clone(),
                    worker,
                });
            }
            tokio::select! { _ = token.cancelled() => return None, _ = changed => {} }
        }
    }

    fn try_acquire(&self, request: u64, worker: WorkerKind) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.active.values().sum::<usize>() >= state.limit {
            return false;
        }
        let eligible = state
            .waiting
            .iter()
            .find(|(_, candidate)| {
                let limit = if **candidate == WorkerKind::Thumbnails {
                    state.thumbnails
                } else {
                    1
                };
                state.active.get(candidate).copied().unwrap_or(0) < limit
            })
            .map(|(request, _)| *request);
        if eligible != Some(request) {
            return false;
        }
        state.waiting.remove(&request);
        *state.active.entry(worker).or_default() += 1;
        true
    }
}

struct WaitingRequest {
    budget: Arc<ProcessingBudget>,
    request: Option<u64>,
}
impl Drop for WaitingRequest {
    fn drop(&mut self) {
        if let Some(request) = self.request {
            self.budget
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .waiting
                .remove(&request);
            self.budget.changed.notify_waiters();
        }
    }
}

/// Releases CPU/I/O operation capacity after its current result has drained.
pub struct ResourcePermit {
    budget: Arc<ProcessingBudget>,
    worker: WorkerKind,
}
impl Drop for ResourcePermit {
    fn drop(&mut self) {
        let mut state = self
            .budget
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *state.active.entry(self.worker).or_default() -= 1;
        drop(state);
        self.budget.changed.notify_waiters();
    }
}

#[cfg(test)]
#[path = "processing_budget_tests.rs"]
mod tests;
