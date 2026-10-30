use super::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_thumbnail_status_exposes_real_outcomes_without_lifecycle_guessing() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    for identifier in [
        "pending",
        "unsupported",
        "retry",
        "failed",
        "offline",
        "stale",
    ] {
        fixture.asset(identifier, None).await?;
    }
    fixture.asset("ready", Some("ready.webp")).await?;
    for (identifier, outcome) in [
        (
            "unsupported",
            ProcessingOutcome::Skipped("No provider".into()),
        ),
        ("retry", ProcessingOutcome::Failed("decoder error".into())),
        ("failed", ProcessingOutcome::Failed("decoder error".into())),
        ("stale", ProcessingOutcome::Skipped("Old source".into())),
    ] {
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, identifier)
            .await?
            .ok_or("Missing attempt")?;
        fixture.processing.finish(&ticket, outcome).await?;
    }
    sqlx::query(
        "UPDATE processing_tasks SET state = 'failed', attempts = 3 WHERE asset_id = 'failed'",
    )
    .execute(&fixture.pool)
    .await?;
    sqlx::query("UPDATE assets SET state = 'Offline' WHERE id = 'offline'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET file_size = 9 WHERE id = 'stale'")
        .execute(&fixture.pool)
        .await?;
    let identifiers = [
        "pending",
        "unsupported",
        "retry",
        "failed",
        "offline",
        "ready",
        "stale",
        "removed",
    ]
    .map(str::to_owned);
    let statuses = fixture.processing.thumbnail_statuses(&identifiers).await?;
    let by_identifier: std::collections::HashMap<_, _> = statuses
        .into_iter()
        .map(|status| (status.asset_id.clone(), status))
        .collect();
    assert_eq!(
        by_identifier["pending"].state,
        ThumbnailProcessingState::Pending
    );
    assert_eq!(
        by_identifier["unsupported"].state,
        ThumbnailProcessingState::Skipped
    );
    assert_eq!(
        by_identifier["retry"].state,
        ThumbnailProcessingState::RetryWait
    );
    assert_eq!(
        by_identifier["failed"].state,
        ThumbnailProcessingState::Failed
    );
    assert_eq!(
        by_identifier["offline"].state,
        ThumbnailProcessingState::Unavailable
    );
    assert_eq!(
        by_identifier["stale"].state,
        ThumbnailProcessingState::Pending
    );
    assert_eq!(
        by_identifier["ready"].thumbnail_path.as_deref(),
        Some("ready.webp")
    );
    assert!(!by_identifier.contains_key("removed"));
    let serialized = serde_json::to_string(&by_identifier["failed"])?;
    assert!(!serialized.contains("decoder error"));
    Ok(())
}

#[tokio::test]
async fn processing_thumbnail_status_batches_are_bounded_and_deduplicated() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let repeated = vec!["asset".to_owned(); 300];
    let statuses =
        crate::feature::processing::thumbnail_statuses(fixture.processing.as_ref(), repeated)
            .await?;
    assert_eq!(statuses.len(), 1);
    let too_many = (0..201).map(|identifier| identifier.to_string()).collect();
    assert!(
        crate::feature::processing::thumbnail_statuses(fixture.processing.as_ref(), too_many)
            .await
            .is_err()
    );
    assert!(fixture.processing.thumbnail_statuses(&[]).await?.is_empty());
    Ok(())
}
