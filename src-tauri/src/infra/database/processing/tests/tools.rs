use super::*;
use crate::core::processing::tools::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

async fn check(fixture: &ProcessingFixture, available: bool) -> TestResult {
    let checks = fixture
        .processing
        .tool_statuses()
        .await?
        .into_iter()
        .map(|status| ExtractionToolCheck {
            tool: status.tool,
            available,
            expected_revision: status.revision,
        })
        .collect();
    fixture.processing.record_tool_checks(checks).await?;
    Ok(())
}

#[tokio::test]
async fn processing_tools_suspend_only_dependencies_and_recover_without_spending_attempts(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("blocked", Some("preserved.webp")).await?;
    fixture.asset("native", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Colors, "blocked")
        .await?
        .ok_or("No claim")?;
    fixture
        .processing
        .finish(
            &ticket,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("ffmpeg".into())),
        )
        .await?;
    let counts = fixture
        .processing
        .queue_counts(ProcessingKind::Colors, &[])
        .await?;
    assert_eq!(
        (counts.blocked, counts.retry_wait, counts.failed),
        (1, 0, 0)
    );
    assert!(fixture
        .processing
        .pending(ProcessingKind::Colors, 20)
        .await?
        .is_empty());
    assert!(fixture
        .processing
        .begin(ProcessingKind::Colors, "blocked")
        .await?
        .is_none());
    assert!(fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "native")
        .await?
        .is_some());
    assert_eq!(
        fixture
            .processing
            .diagnostics(ProcessingKind::Colors)
            .await?[0]
            .state,
        "blocked"
    );
    let retained: (i64, i64, String) = sqlx::query_as("SELECT attempts, generation, thumbnail_path FROM processing_tasks JOIN assets ON assets.id = asset_id WHERE asset_id = 'blocked'").fetch_one(&fixture.pool).await?;
    assert_eq!(retained, (0, ticket.generation, "preserved.webp".into()));
    check(&fixture, true).await?;
    let recovered = fixture
        .processing
        .begin(ProcessingKind::Colors, "blocked")
        .await?
        .ok_or("Probe must bypass dependency backoff")?;
    assert_eq!(recovered.generation, ticket.generation);
    fixture
        .processing
        .finish(
            &recovered,
            ProcessingOutcome::Failed("Corrupt input".into()),
        )
        .await?;
    let attempts: i64 =
        sqlx::query_scalar("SELECT attempts FROM processing_tasks WHERE asset_id = 'blocked'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(
        attempts, 1,
        "Input errors still consume the bounded retry budget"
    );
    Ok(())
}

#[tokio::test]
async fn processing_tools_reject_stale_probes_and_expired_failure_observations() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let revision = fixture.processing.tool_statuses().await?[0].revision;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No claim")?;
    fixture
        .processing
        .finish(
            &ticket,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("ffmpeg".into())),
        )
        .await?;
    fixture
        .processing
        .record_tool_checks(vec![ExtractionToolCheck {
            tool: ExtractionTool::Ffmpeg,
            available: true,
            expected_revision: revision,
        }])
        .await?;
    assert_eq!(
        fixture.processing.tool_statuses().await?[0].state,
        ToolAvailability::Unavailable
    );
    let status = &fixture
        .processing
        .thumbnail_statuses(&["asset".into()])
        .await?[0];
    assert_eq!(status.state, ThumbnailProcessingState::Blocked);
    check(&fixture, true).await?;
    assert_eq!(
        fixture
            .processing
            .thumbnail_statuses(&["asset".into()])
            .await?[0]
            .state,
        ThumbnailProcessingState::Pending
    );
    let current = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No recovery")?;
    fixture
        .processing
        .finish(
            &ticket,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("ffmpeg".into())),
        )
        .await?;
    assert_eq!(
        fixture.processing.tool_statuses().await?[0].state,
        ToolAvailability::Available
    );
    sqlx::query("UPDATE processing_tasks SET lease_expires_at = 0")
        .execute(&fixture.pool)
        .await?;
    fixture
        .processing
        .finish(
            &current,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("assimp".into())),
        )
        .await?;
    assert!(fixture
        .processing
        .tool_statuses()
        .await?
        .iter()
        .all(|status| status.state == ToolAvailability::Available));
    let json = serde_json::to_string(&fixture.processing.tool_statuses().await?)?;
    assert!(!json.contains("revision"));
    assert!(!json.contains("path"));
    Ok(())
}

struct CountingProbe(AtomicUsize);
#[async_trait::async_trait]
impl ExtractionToolProbe for CountingProbe {
    async fn available(&self, _: ExtractionTool) -> AppResult<bool> {
        self.0.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        Ok(true)
    }
}

#[tokio::test]
async fn processing_tools_coalesce_concurrent_checks_without_touching_queue_intent() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No claim")?;
    fixture
        .processing
        .finish(&ticket, ProcessingOutcome::Skipped("unsupported".into()))
        .await?;
    let probe = Arc::new(CountingProbe(AtomicUsize::new(0)));
    let service = crate::feature::processing::tools::ProcessingToolService::new(
        fixture.processing.clone(),
        probe.clone(),
    );
    let (first, second) = tokio::join!(service.check(), service.check());
    first?;
    second?;
    assert_eq!(probe.0.load(Ordering::SeqCst), 3);
    assert_eq!(
        fixture
            .processing
            .thumbnail_statuses(&["asset".into()])
            .await?[0]
            .state,
        ThumbnailProcessingState::Skipped
    );
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 10)
        .await?
        .is_empty());
    Ok(())
}
