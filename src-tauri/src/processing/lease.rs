//! Runtime renewal complements durable ownership; crashes recover through database expiry.

use crate::core::processing::{ProcessingRepository, ProcessingTicket};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Keeps ownership alive during native work and releases it on normal exit or unwinding.
pub struct ProcessingLease {
    stop: CancellationToken,
}

impl ProcessingLease {
    /// Call immediately after `begin`; a dropped guard never renews or releases another owner.
    pub fn start(repository: Arc<dyn ProcessingRepository>, ticket: ProcessingTicket) -> Self {
        Self::start_with_interval(repository, ticket, std::time::Duration::from_secs(30))
    }

    fn start_with_interval(
        repository: Arc<dyn ProcessingRepository>,
        ticket: ProcessingTicket,
        interval: std::time::Duration,
    ) -> Self {
        let stop = CancellationToken::new();
        let stopped = stop.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stopped.cancelled() => break,
                    _ = tokio::time::sleep(interval) => {}
                }
                match repository.renew(&ticket).await {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(error) => {
                        tracing::warn!(%error, asset_id = ticket.asset_id, "Processing lease renewal failed");
                        break;
                    }
                }
            }
            if let Err(error) = repository.release(&ticket).await {
                tracing::warn!(%error, asset_id = ticket.asset_id, "Processing lease release failed; expiry will recover it");
            }
        });
        Self { stop }
    }
}

impl Drop for ProcessingLease {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::processing::ProcessingKind;
    use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

    #[tokio::test]
    async fn processing_heartbeat_renews_ownership_and_drop_releases_without_retry() -> TestResult {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("asset", None).await?;
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, "asset")
            .await?
            .ok_or("No reservation")?;
        sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() + 10")
            .execute(&fixture.pool)
            .await?;
        let guard = ProcessingLease::start_with_interval(
            fixture.processing.clone(),
            ticket.clone(),
            std::time::Duration::from_millis(10),
        );
        tokio::time::timeout(std::time::Duration::from_secs(4), async {
            loop {
                let remaining: i64 = sqlx::query_scalar(
                    "SELECT lease_expires_at - unixepoch() FROM processing_tasks",
                )
                .fetch_one(&fixture.pool)
                .await?;
                if remaining > 100 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Ok::<(), sqlx::Error>(())
        })
        .await??;
        assert!(fixture.processing.is_current(&ticket).await?);
        drop(guard);
        tokio::time::timeout(std::time::Duration::from_secs(4), async {
            while fixture
                .processing
                .pending(ProcessingKind::Thumbnail, 1)
                .await?
                .is_empty()
            {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Ok::<(), crate::core::AppError>(())
        })
        .await??;
        let attempts: i64 = sqlx::query_scalar("SELECT attempts FROM processing_tasks")
            .fetch_one(&fixture.pool)
            .await?;
        assert_eq!(attempts, 0);
        Ok(())
    }
}
