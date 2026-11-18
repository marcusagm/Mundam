//! Cooperative scheduling for database maintenance owned by the lifecycle.

use std::future::Future;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Stops scheduling on cancellation while awaiting any cleanup already started.
///
/// The lifecycle's shared budget still applies; dropping a database operation
/// mid-flight here would not prove completion of its filesystem side effects.
pub(super) async fn run_periodic_cleanup<Cleanup, Operation>(
    cancellation: CancellationToken,
    mut cleanup: Cleanup,
) where
    Cleanup: FnMut() -> Operation,
    Operation: Future<Output = ()>,
{
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => break,
            _ = interval.tick() => {}
        }
        if cancellation.is_cancelled() {
            break;
        }
        cleanup().await;
    }
    tracing::info!("Library cleanup worker stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn cancelled_worker_does_not_start_cleanup() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let calls = AtomicUsize::new(0);
        run_periodic_cleanup(cancellation, || async {
            calls.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn shutdown_waits_for_cleanup_and_prevents_another_pass(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lifecycle = crate::lifecycle::LifecycleRegistry::new();
        let cancellation = lifecycle.child_token();
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicUsize::new(0));
        let operation_started = started.clone();
        let operation_release = release.clone();
        let operation_calls = calls.clone();
        let operation_finished = finished.clone();
        assert!(
            lifecycle.spawn_unique("library_cleanup".into(), async move {
                run_periodic_cleanup(cancellation, || async {
                    operation_calls.fetch_add(1, Ordering::SeqCst);
                    operation_started.notify_one();
                    operation_release.notified().await;
                    operation_finished.fetch_add(1, Ordering::SeqCst);
                })
                .await;
            })
        );
        tokio::time::timeout(Duration::from_secs(2), started.notified()).await?;
        let shutdown = lifecycle.shutdown_all();
        tokio::pin!(shutdown);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut shutdown)
                .await
                .is_err()
        );
        assert_eq!(finished.load(Ordering::SeqCst), 0);
        release.notify_one();
        assert!(tokio::time::timeout(Duration::from_secs(2), &mut shutdown).await?);
        assert_eq!(finished.load(Ordering::SeqCst), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        Ok(())
    }
}
