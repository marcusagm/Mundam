use super::*;
use crate::core::ledger::{command::LedgerCommand, port::TransactionalAssetLedger};
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_regeneration_is_recovered_without_viewport_or_event_delivery() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("old.webp")).await?;
    sqlx::query("UPDATE assets SET state = 'Thumbnailed'")
        .execute(&fixture.pool)
        .await?;
    fixture
        .ledger
        .execute(LedgerCommand::RegenerateThumbnail {
            asset_id: "asset".into(),
        })
        .await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Thumbnail, 10)
            .await?,
        ["asset"]
    );
    Ok(())
}

#[tokio::test]
async fn processing_excludes_trash_and_offline_from_all_stages() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    for identifier in ["active", "trash", "offline"] {
        fixture.asset(identifier, None).await?;
    }
    sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP WHERE id = 'trash'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET state = 'Offline' WHERE id = 'offline'")
        .execute(&fixture.pool)
        .await?;
    for kind in [ProcessingKind::Thumbnail, ProcessingKind::Fingerprint] {
        assert_eq!(fixture.processing.pending(kind, 10).await?, ["active"]);
    }
    sqlx::query("UPDATE assets SET thumbnail_path = 'asset.webp'")
        .execute(&fixture.pool)
        .await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Colors, 10)
            .await?,
        ["active"]
    );
    Ok(())
}

#[tokio::test]
async fn processing_retries_are_bounded_persistent_and_do_not_starve_other_assets() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("bad", None).await?;
    fixture.asset("good", None).await?;
    for attempt in 1..=3 {
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, "bad")
            .await?
            .ok_or("Missing attempt")?;
        fixture
            .processing
            .finish(&ticket, ProcessingOutcome::Failed("decoder failure".into()))
            .await?;
        assert_eq!(
            fixture
                .processing
                .pending(ProcessingKind::Thumbnail, 10)
                .await?,
            ["good"]
        );
        let state: (String, i64) =
            sqlx::query_as("SELECT state, attempts FROM processing_tasks WHERE asset_id = 'bad'")
                .fetch_one(&fixture.pool)
                .await?;
        assert_eq!(state.1, attempt);
        assert_eq!(state.0, if attempt == 3 { "failed" } else { "retry_wait" });
        sqlx::query("UPDATE processing_tasks SET next_attempt_at = 0 WHERE asset_id = 'bad'")
            .execute(&fixture.pool)
            .await?;
    }
    let reopened = SqliteProcessingRepository::new(fixture.pool.clone());
    assert!(reopened
        .begin(ProcessingKind::Thumbnail, "bad")
        .await?
        .is_none());
    fixture
        .ledger
        .execute(LedgerCommand::RegenerateThumbnail {
            asset_id: "bad".into(),
        })
        .await?;
    assert!(reopened
        .begin(ProcessingKind::Thumbnail, "bad")
        .await?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn processing_due_retries_are_selected_before_untouched_backlog() -> TestResult {
    for kind in [
        ProcessingKind::Thumbnail,
        ProcessingKind::Colors,
        ProcessingKind::Fingerprint,
    ] {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("retry", Some("retry.webp")).await?;
        if kind == ProcessingKind::Thumbnail {
            sqlx::query("UPDATE assets SET thumbnail_path = NULL")
                .execute(&fixture.pool)
                .await?;
        }
        let ticket = fixture
            .processing
            .begin(kind, "retry")
            .await?
            .ok_or("Missing attempt")?;
        fixture
            .processing
            .finish(&ticket, ProcessingOutcome::Failed("decoder failure".into()))
            .await?;
        fixture.asset("untouched", Some("untouched.webp")).await?;
        if kind == ProcessingKind::Thumbnail {
            sqlx::query("UPDATE assets SET thumbnail_path = NULL WHERE id = 'untouched'")
                .execute(&fixture.pool)
                .await?;
        }
        assert_eq!(fixture.processing.pending(kind, 1).await?, ["untouched"]);
        sqlx::query("UPDATE processing_tasks SET next_attempt_at = unixepoch() - 1")
            .execute(&fixture.pool)
            .await?;
        assert_eq!(fixture.processing.pending(kind, 1).await?, ["retry"]);
    }
    Ok(())
}

#[tokio::test]
async fn processing_new_request_is_not_overwritten_by_an_old_completion() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("asset.webp")).await?;
    let old = fixture
        .processing
        .begin(ProcessingKind::Colors, "asset")
        .await?
        .ok_or("Missing attempt")?;
    fixture
        .ledger
        .execute(LedgerCommand::ReextractColors {
            asset_id: "asset".into(),
        })
        .await?;
    assert!(!fixture.processing.is_current(&old).await?);
    fixture
        .processing
        .finish(&old, ProcessingOutcome::Completed)
        .await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Colors, 1)
            .await?,
        ["asset"]
    );
    let next = fixture
        .processing
        .begin(ProcessingKind::Colors, "asset")
        .await?
        .ok_or("Missing newer attempt")?;
    assert!(next.generation > old.generation);
    Ok(())
}

#[tokio::test]
async fn processing_source_change_releases_a_terminal_failure_and_removal_cascades_tasks(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("Missing attempt")?;
    fixture
        .processing
        .finish(&ticket, ProcessingOutcome::Skipped("unsupported".into()))
        .await?;
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 1)
        .await?
        .is_empty());
    sqlx::query("UPDATE assets SET file_size = 9 WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    assert!(!fixture.processing.is_current(&ticket).await?);
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Thumbnail, 1)
            .await?,
        ["asset"]
    );
    sqlx::query("DELETE FROM assets WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    fixture
        .processing
        .finish(&ticket, ProcessingOutcome::Completed)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn processing_empty_thumbnail_is_rejected_and_does_not_publish_success() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    use crate::core::events::AppEventBus;
    let mut events = fixture.event_bus.subscribe();
    assert!(fixture
        .ledger
        .execute(LedgerCommand::UpdateThumbnail {
            asset_id: "asset".into(),
            thumbnail_path: String::new()
        })
        .await
        .is_err());
    assert!(events.try_recv().is_err());
    let asset: (String, Option<String>) =
        sqlx::query_as("SELECT state, thumbnail_path FROM assets WHERE id = 'asset'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(asset, ("Indexed".into(), None));
    Ok(())
}
