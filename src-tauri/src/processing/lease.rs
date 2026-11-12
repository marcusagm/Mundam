//! Runtime renewal complements durable ownership; crashes recover through database expiry.

use crate::core::{
    error::{AppError, AppResult},
    processing::{ProcessingRepository, ProcessingTicket},
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Keeps ownership alive during native work and releases it on normal exit or unwinding.
pub struct ProcessingLease {
    stop: CancellationToken,
    task: Option<tokio::task::JoinHandle<AppResult<()>>>,
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
        let task = tokio::spawn(async move {
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
            let result = repository.release(&ticket).await;
            if let Err(error) = &result {
                tracing::warn!(%error, asset_id = ticket.asset_id, "Processing lease release failed; expiry will recover it");
            }
            result
        });
        Self {
            stop,
            task: Some(task),
        }
    }

    /// Stops renewal and waits for release before the owner reports completion.
    pub async fn finish(mut self) -> AppResult<()> {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.await.map_err(|error| {
                AppError::Internal(format!("Processing lease task failed: {error}"))
            })?
        } else {
            Err(AppError::Internal(
                "Processing lease task is missing".into(),
            ))
        }
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
    async fn finish_waits_for_release_before_returning() -> TestResult {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("asset", None).await?;
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, "asset")
            .await?
            .ok_or("No reservation")?;
        let lease = ProcessingLease::start(fixture.processing.clone(), ticket);
        let connection = fixture.pool.acquire().await?;
        let mut finishing = tokio::spawn(lease.finish());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut finishing)
                .await
                .is_err()
        );
        drop(connection);
        tokio::time::timeout(std::time::Duration::from_secs(4), finishing).await???;
        assert_eq!(
            fixture
                .processing
                .pending(ProcessingKind::Thumbnail, 1)
                .await?,
            ["asset"]
        );
        fixture.pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn finish_reports_release_failure() -> TestResult {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("asset", None).await?;
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, "asset")
            .await?
            .ok_or("No reservation")?;
        let lease = ProcessingLease::start(fixture.processing.clone(), ticket);
        fixture.pool.close().await;
        assert!(lease.finish().await.is_err());
        Ok(())
    }

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
