//! Periodic work joined by the HTTP server owner, including any admitted sweep.

use std::{future::Future, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) async fn run<Cleanup, Operation>(
    name: &'static str,
    cancellation: CancellationToken,
    period: Duration,
    mut cleanup: Cleanup,
) where
    Cleanup: FnMut() -> Operation,
    Operation: Future<Output = ()>,
{
    let mut interval = tokio::time::interval(period);
    loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => break,
            _ = interval.tick() => {}
        }
        if cancellation.is_cancelled() {
            break;
        }
        // Do not drop an operation already manipulating a child or its files.
        cleanup().await;
    }
    tracing::info!(worker = name, "Streaming HTTP maintenance finished");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Notify;

    #[tokio::test]
    async fn cancelled_owner_never_starts_a_sweep() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let calls = AtomicUsize::new(0);
        run("test", cancellation, Duration::from_secs(1), || async {
            calls.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cancellation_waits_for_the_admitted_sweep() -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::new();
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let finished = Arc::new(AtomicUsize::new(0));
        let worker_started = started.clone();
        let worker_release = release.clone();
        let worker_finished = finished.clone();
        let worker_cancellation = cancellation.clone();
        let worker = tokio::spawn(async move {
            run(
                "test",
                worker_cancellation,
                Duration::from_millis(1),
                || async {
                    worker_started.notify_one();
                    worker_release.notified().await;
                    worker_finished.fetch_add(1, Ordering::SeqCst);
                },
            )
            .await;
        });
        tokio::time::timeout(Duration::from_secs(2), started.notified()).await?;
        cancellation.cancel();
        assert!(!worker.is_finished());
        assert_eq!(finished.load(Ordering::SeqCst), 0);
        release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), worker).await??;
        assert_eq!(finished.load(Ordering::SeqCst), 1);
        Ok(())
    }
}
