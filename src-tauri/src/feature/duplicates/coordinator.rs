//! One admission gate prevents automatic processing from racing manual duplicate scans.

use tokio::sync::{Mutex, MutexGuard};

/// Shared admission across hashing and matching entry points in this application instance.
#[derive(Default)]
pub struct DuplicateProcessingCoordinator {
    gate: Mutex<()>,
    /// Shared media-operation capacity, also used by manual scans.
    pub budget: std::sync::Arc<crate::core::processing::budget::ProcessingBudget>,
    matching: std::sync::Mutex<Option<(String, bool, tokio_util::sync::CancellationToken)>>,
}

impl DuplicateProcessingCoordinator {
    /// Injects the same resource policy used by the other media workers.
    pub fn with_budget(
        mut self,
        budget: std::sync::Arc<crate::core::processing::budget::ProcessingBudget>,
    ) -> Self {
        self.budget = budget;
        self
    }

    /// Acquires an execution slot; callers select cancellation while waiting.
    pub async fn acquire(&self) -> MutexGuard<'_, ()> {
        self.gate.lock().await
    }

    /// Makes recovered manual matching respond immediately to local cancellation controls.
    pub fn watch_matching(
        self: &std::sync::Arc<Self>,
        ticket: &crate::core::duplicates::matching::MatchingTicket,
        token: tokio_util::sync::CancellationToken,
    ) -> MatchingActivity {
        *self
            .matching
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((
            ticket.lease_id.clone(),
            ticket.manual_run_id.is_some(),
            token,
        ));
        MatchingActivity {
            coordinator: self.clone(),
            lease_id: ticket.lease_id.clone(),
        }
    }

    /// Requests cooperative exit without dropping the shared execution admission.
    pub fn cancel_manual_matching(&self) {
        if let Some((_, true, token)) = self
            .matching
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            token.cancel();
        }
    }
}

/// Prevents an old execution from clearing a newer cancellation target.
pub struct MatchingActivity {
    coordinator: std::sync::Arc<DuplicateProcessingCoordinator>,
    lease_id: String,
}
impl Drop for MatchingActivity {
    fn drop(&mut self) {
        let mut matching = self
            .coordinator
            .matching
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if matching
            .as_ref()
            .is_some_and(|(lease_id, _, _)| lease_id == &self.lease_id)
        {
            *matching = None;
        }
    }
}
