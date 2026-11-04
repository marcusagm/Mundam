use super::*;
use crate::core::processing::diagnostics::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

async fn outcome(
    fixture: &ProcessingFixture,
    identifier: &str,
    outcome: ProcessingOutcome,
) -> TestResult {
    fixture.asset(identifier, Some("preserved.webp")).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Colors, identifier)
        .await?
        .ok_or("No attempt")?;
    fixture.processing.finish(&ticket, outcome).await?;
    Ok(())
}

#[tokio::test]
async fn processing_retry_targets_only_current_terminal_work_and_preserves_results() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    for identifier in [
        "failed",
        "retry",
        "unsupported",
        "offline",
        "trash",
        "changed",
        "leased",
    ] {
        outcome(
            &fixture,
            identifier,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("ffmpeg".into())),
        )
        .await?;
    }
    sqlx::query(
        "UPDATE processing_tasks SET state = 'failed', attempts = 3 WHERE asset_id != 'retry'",
    )
    .execute(&fixture.pool)
    .await?;
    sqlx::query("UPDATE processing_tasks SET state = 'skipped' WHERE asset_id = 'unsupported'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET state = 'Offline' WHERE id = 'offline'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP WHERE id = 'trash'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET file_size = 99 WHERE id = 'changed'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE processing_tasks SET lease_id = 'other-owner', lease_expires_at = unixepoch() + 120 WHERE asset_id = 'leased'").execute(&fixture.pool).await?;
    sqlx::query("INSERT INTO asset_colors (asset_id, hex_color, lab_lightness, lab_green_red, lab_blue_yellow, percentage, rank) VALUES ('failed', '#123456', 1, 1, 1, 1, 0)").execute(&fixture.pool).await?;
    let count = fixture
        .processing
        .retry(ProcessingKind::Colors, ProcessingRetryScope::Failed)
        .await?;
    assert_eq!((count.assets, count.comparisons), (1, 0));
    assert_eq!(
        fixture
            .processing
            .retry(ProcessingKind::Colors, ProcessingRetryScope::Failed)
            .await?
            .assets,
        0
    );
    let preserved: (String, String, i64, i64, Option<String>) = sqlx::query_as("SELECT a.thumbnail_path, c.hex_color, task.attempts, task.generation, task.issue_code FROM assets a JOIN asset_colors c ON c.asset_id = a.id JOIN processing_tasks task ON task.asset_id = a.id WHERE a.id = 'failed'").fetch_one(&fixture.pool).await?;
    assert_eq!(
        preserved,
        ("preserved.webp".into(), "#123456".into(), 0, 1, None)
    );
    assert_eq!(
        fixture
            .processing
            .retry(ProcessingKind::Colors, ProcessingRetryScope::Unsupported)
            .await?
            .assets,
        1
    );
    let wait: String =
        sqlx::query_scalar("SELECT state FROM processing_tasks WHERE asset_id = 'retry'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(wait, "retry_wait");
    Ok(())
}

#[tokio::test]
async fn processing_diagnostics_are_bounded_grouped_and_never_serialize_private_logs() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    for identifier in ["one", "two"] {
        outcome(
            &fixture,
            identifier,
            ProcessingOutcome::failure(&crate::core::AppError::ToolUnavailable("ffmpeg".into())),
        )
        .await?;
    }
    outcome(
        &fixture,
        "unknown",
        ProcessingOutcome::Failed("C:/private/asset.heic SQL SECRET".into()),
    )
    .await?;
    sqlx::query(
        "UPDATE processing_tasks SET issue_code = 'C:/private/SECRET' WHERE asset_id = 'unknown'",
    )
    .execute(&fixture.pool)
    .await?;
    let rows = fixture
        .processing
        .diagnostics(ProcessingKind::Colors)
        .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .find(|row| row.issue == "ffmpeg_unavailable")
            .ok_or("No grouped tool failure")?
            .count,
        2
    );
    assert!(rows.iter().any(|row| row.issue == "unknown_failure"));
    let json = serde_json::to_string(&rows)?;
    assert!(!json.contains("private"));
    assert!(!json.contains("SECRET"));
    assert!(!json.contains("asset.heic"));
    sqlx::query("UPDATE assets SET state = 'Offline'")
        .execute(&fixture.pool)
        .await?;
    assert!(fixture
        .processing
        .diagnostics(ProcessingKind::Colors)
        .await?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn processing_retry_duplicates_rolls_back_assets_and_comparisons_together() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Fingerprint, "asset")
        .await?
        .ok_or("No attempt")?;
    fixture
        .processing
        .finish(&ticket, ProcessingOutcome::Failed("read failed".into()))
        .await?;
    sqlx::query("UPDATE processing_tasks SET state = 'failed', attempts = 3")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE duplicate_matching_jobs SET state = 'failed', attempts = 3")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("CREATE TRIGGER reject_comparison_retry BEFORE UPDATE ON duplicate_matching_jobs WHEN NEW.state = 'pending' BEGIN SELECT RAISE(ABORT, 'rollback test'); END").execute(&fixture.pool).await?;
    assert!(fixture
        .processing
        .retry(ProcessingKind::Fingerprint, ProcessingRetryScope::Failed)
        .await
        .is_err());
    let state: String = sqlx::query_scalar("SELECT state FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(state, "failed");
    sqlx::query("DROP TRIGGER reject_comparison_retry")
        .execute(&fixture.pool)
        .await?;
    let counts = fixture
        .processing
        .retry(ProcessingKind::Fingerprint, ProcessingRetryScope::Failed)
        .await?;
    assert_eq!((counts.assets, counts.comparisons), (1, 2));
    fixture
        .processing
        .finish(&ticket, ProcessingOutcome::Completed)
        .await?;
    let state: String = sqlx::query_scalar("SELECT state FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(state, "pending");
    Ok(())
}

#[tokio::test]
async fn processing_unsupported_diagnostic_remains_terminal_until_explicit_recheck() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No attempt")?;
    fixture
        .processing
        .finish(
            &ticket,
            ProcessingOutcome::Unavailable(ProcessingIssue::ProviderMissing),
        )
        .await?;
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 10)
        .await?
        .is_empty());
    let rows = fixture
        .processing
        .diagnostics(ProcessingKind::Thumbnail)
        .await?;
    assert_eq!(rows[0].issue, "provider_missing");
    assert_eq!(
        fixture
            .processing
            .retry(ProcessingKind::Thumbnail, ProcessingRetryScope::Failed)
            .await?
            .assets,
        0
    );
    assert_eq!(
        fixture
            .processing
            .retry(ProcessingKind::Thumbnail, ProcessingRetryScope::Unsupported)
            .await?
            .assets,
        1
    );
    assert!(fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .is_some());
    Ok(())
}

#[test]
fn processing_diagnostic_causes_use_typed_errors_without_guessing_from_messages() {
    use crate::core::AppError;
    assert_eq!(
        ProcessingIssue::from_error(&AppError::ToolUnavailable("ffmpeg".into())),
        ProcessingIssue::FfmpegUnavailable
    );
    assert_eq!(
        ProcessingIssue::from_error(&AppError::ExtractionProcessTimeout),
        ProcessingIssue::Timeout
    );
    assert_eq!(
        ProcessingIssue::from_error(&AppError::InvalidProcessingOutput),
        ProcessingIssue::InvalidOutput
    );
    assert_eq!(
        ProcessingIssue::from_error(&AppError::Transcoding("binary not found C:/private".into())),
        ProcessingIssue::ExtractionFailed
    );
}
