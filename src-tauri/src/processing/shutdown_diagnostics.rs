use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(crate) async fn observe<T>(
    token: &CancellationToken,
    phase: &AtomicU8,
    describe: fn(u8) -> &'static str,
    worker: &'static str,
    work: impl Future<Output = T>,
) -> T {
    tokio::pin!(work);
    tokio::select! {
        result = &mut work => result,
        _ = token.cancelled() => {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    biased;
                    result = &mut work => return result,
                    _ = interval.tick() => tracing::info!(
                        worker,
                        phase = describe(phase.load(Ordering::Relaxed)),
                        "Processing work still active after cancellation"
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_observes_without_dropping_work() -> Result<(), Box<dyn std::error::Error>>
    {
        let token = CancellationToken::new();
        let phase = AtomicU8::new(1);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        token.cancel();
        let task = async { observe(&token, &phase, |_| "test", "test", receiver).await };
        tokio::pin!(task);
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut task)
            .await
            .is_err());
        sender.send(42).map_err(|_| "work was dropped")?;
        assert_eq!(task.await?, 42);
        Ok(())
    }
}
