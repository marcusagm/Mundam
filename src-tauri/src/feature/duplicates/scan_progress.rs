//! Latest-value progress feeds the shared snapshot without per-asset desktop events.

use super::commands::DuplicateScanStatus;
use std::sync::{Arc, RwLock};
use tokio::sync::watch;

/// Drains the final value before hashing hands off to matching; obsolete runs cannot update state.
pub(super) async fn reconcile(
    mut updates: watch::Receiver<(usize, usize)>,
    status: Arc<RwLock<DuplicateScanStatus>>,
    run_id: String,
) {
    while updates.changed().await.is_ok() {
        let (processed, total) = *updates.borrow_and_update();
        let mut current = status.write().unwrap_or_else(|error| error.into_inner());
        if !current.is_running || current.run_id.as_deref() != Some(&run_id) {
            continue;
        }
        if current.phase != "cancel_requested" {
            current.phase = "hashing".into();
        }
        current.processed = processed;
        current.total = total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(phase: &str) -> Arc<RwLock<DuplicateScanStatus>> {
        Arc::new(RwLock::new(DuplicateScanStatus {
            is_running: true,
            run_id: Some("current".into()),
            phase: phase.into(),
            ..Default::default()
        }))
    }

    #[tokio::test]
    async fn processing_progress_coalesces_a_large_burst_and_drains_the_final_value() {
        let (sender, receiver) = watch::channel((0, 0));
        for processed in 0..=100_000 {
            sender.send_replace((processed, 100_000));
        }
        drop(sender);
        let status = running("preparing");
        reconcile(receiver, status.clone(), "current".into()).await;
        let current = status.read().unwrap();
        assert_eq!((current.processed, current.total), (100_000, 100_000));
        assert_eq!(current.phase, "hashing");
    }

    #[tokio::test]
    async fn processing_progress_preserves_cancellation_and_rejects_obsolete_runs() {
        let status = running("cancel_requested");
        let (sender, receiver) = watch::channel((0, 0));
        sender.send_replace((7, 10));
        drop(sender);
        reconcile(receiver, status.clone(), "current".into()).await;
        assert_eq!(status.read().unwrap().phase, "cancel_requested");
        assert_eq!(status.read().unwrap().processed, 7);
        let (sender, receiver) = watch::channel((0, 0));
        sender.send_replace((9, 10));
        drop(sender);
        reconcile(receiver, status.clone(), "obsolete".into()).await;
        assert_eq!(status.read().unwrap().processed, 7);
    }
}
