use super::*;
use crate::core::{
    events::AppEventBus,
    ledger::{
        command::{LedgerCommand, UpdateTechnicalMetadataPayload},
        port::TransactionalAssetLedger,
    },
    processing::ProcessingResult,
};
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_leases_serialize_admission_and_recover_without_reusing_ownership() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let other = SqliteProcessingRepository::new(fixture.pool.clone());
    let (first, second) = tokio::join!(
        fixture.processing.begin(ProcessingKind::Thumbnail, "asset"),
        other.begin(ProcessingKind::Thumbnail, "asset"),
    );
    let first = first?;
    let second = second?;
    assert_ne!(first.is_some(), second.is_some());
    let old = first.or(second).ok_or("No reservation")?;
    assert!(other
        .pending(ProcessingKind::Thumbnail, 1)
        .await?
        .is_empty());
    assert_eq!(
        other
            .queue_counts(ProcessingKind::Thumbnail, &[])
            .await?
            .pending,
        0
    );
    sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
        .execute(&fixture.pool)
        .await?;
    assert!(!other.renew(&old).await?);
    assert!(!other.is_current(&old).await?);
    assert_eq!(
        other.pending(ProcessingKind::Thumbnail, 1).await?,
        ["asset"]
    );
    assert_eq!(
        other
            .queue_counts(ProcessingKind::Thumbnail, &[])
            .await?
            .pending,
        1
    );
    let current = other
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No reclaimed reservation")?;
    assert_eq!(old.generation, current.generation);
    assert_ne!(old.lease_id, current.lease_id);
    other.release(&old).await?;
    other
        .finish(&old, ProcessingOutcome::Failed("late failure".into()))
        .await?;
    assert!(other.is_current(&current).await?);
    assert!(!other.renew(&old).await?);
    other.release(&current).await?;
    let attempts: i64 = sqlx::query_scalar("SELECT attempts FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(attempts, 0);
    Ok(())
}

#[tokio::test]
async fn processing_lease_survives_reopening_the_database_and_only_expires_once() -> TestResult {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    let fixture = ProcessingFixture::new().await?;
    let options = SqliteConnectOptions::new()
        .filename(fixture.data.path().join("leases.db"))
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options.clone())
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size) VALUES ('asset', 'asset', 'asset.png', 'Indexed', 'png', 'IMAGE', 8)")
        .execute(&pool).await?;
    let repository = SqliteProcessingRepository::new(pool.clone());
    let old = repository
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("Missing lease")?;
    pool.close().await;
    let reopened = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    let first = SqliteProcessingRepository::new(reopened.clone());
    let second = SqliteProcessingRepository::new(reopened.clone());
    assert!(first
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .is_none());
    sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
        .execute(&reopened)
        .await?;
    let (left, right) = tokio::join!(
        first.begin(ProcessingKind::Thumbnail, "asset"),
        second.begin(ProcessingKind::Thumbnail, "asset")
    );
    let left = left?;
    let right = right?;
    assert_ne!(left.is_some(), right.is_some());
    assert!(!first.is_current(&old).await?);
    reopened.close().await;
    Ok(())
}

fn thumbnail_result(ticket: &ProcessingTicket) -> ProcessingResult {
    ProcessingResult::Thumbnail {
        thumbnail_path: Some(format!("{}.{}.webp", ticket.asset_id, ticket.lease_id)),
        preview_path: Some(format!("{}.{}.glb", ticket.asset_id, ticket.lease_id)),
        metadata: Some(UpdateTechnicalMetadataPayload {
            asset_id: ticket.asset_id.clone(),
            width: Some(300),
            height: Some(200),
            duration_secs: None,
            technical_payload: None,
            semantic_payload: None,
        }),
        unsupported_reason: None,
    }
}

#[tokio::test]
async fn processing_fenced_thumbnail_commit_is_atomic_and_emits_nothing_on_rollback() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No attempt")?;
    let mut events = fixture.event_bus.subscribe();
    sqlx::query("CREATE TRIGGER reject_processing_completion BEFORE UPDATE ON processing_tasks WHEN NEW.state = 'succeeded' BEGIN SELECT RAISE(ABORT, 'test failure'); END")
        .execute(&fixture.pool).await?;
    assert!(fixture
        .ledger
        .commit_processing(&ticket, thumbnail_result(&ticket))
        .await
        .is_err());
    assert!(events.try_recv().is_err());
    let rows: (Option<String>, i64, i64) = sqlx::query_as("SELECT thumbnail_path, (SELECT COUNT(*) FROM asset_metadata_envelope), (SELECT COUNT(*) FROM processing_tasks WHERE kind = 'colors') FROM assets WHERE id = 'asset'")
        .fetch_one(&fixture.pool).await?;
    assert_eq!(rows, (None, 0, 0));
    assert!(fixture.processing.preview_path("asset").await?.is_none());
    assert!(fixture.processing.is_current(&ticket).await?);
    sqlx::query("DROP TRIGGER reject_processing_completion")
        .execute(&fixture.pool)
        .await?;
    assert!(
        fixture
            .ledger
            .commit_processing(&ticket, thumbnail_result(&ticket))
            .await?
    );
    let saved: (String, String, Option<String>) = sqlx::query_as("SELECT a.thumbnail_path, task.state, task.lease_id FROM assets a JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = 'thumbnail'")
        .fetch_one(&fixture.pool).await?;
    assert_eq!(
        saved,
        (
            format!("asset.{}.webp", ticket.lease_id),
            "succeeded".into(),
            None
        )
    );
    assert!(events.try_recv().is_ok());
    assert!(events.try_recv().is_ok());
    assert!(events.try_recv().is_err());
    assert!(
        !fixture
            .ledger
            .commit_processing(&ticket, thumbnail_result(&ticket))
            .await?
    );
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn processing_fencing_rejects_superseded_inactive_and_expired_results() -> TestResult {
    for change in [
        "request", "source", "offline", "trash", "removed", "expired",
    ] {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("asset", Some("old.webp")).await?;
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Colors, "asset")
            .await?
            .ok_or("No attempt")?;
        match change {
            "request" => {
                fixture
                    .ledger
                    .execute(LedgerCommand::ReextractColors {
                        asset_id: "asset".into(),
                    })
                    .await?;
            }
            "source" => {
                sqlx::query("UPDATE assets SET file_size = 99")
                    .execute(&fixture.pool)
                    .await?;
            }
            "offline" => {
                sqlx::query("UPDATE assets SET state = 'Offline'")
                    .execute(&fixture.pool)
                    .await?;
            }
            "trash" => {
                sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP")
                    .execute(&fixture.pool)
                    .await?;
            }
            "removed" => {
                sqlx::query("DELETE FROM assets")
                    .execute(&fixture.pool)
                    .await?;
            }
            _ => {
                sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
                    .execute(&fixture.pool)
                    .await?;
            }
        }
        let mut events = fixture.event_bus.subscribe();
        assert!(
            !fixture
                .ledger
                .commit_processing(&ticket, ProcessingResult::Colors(Vec::new()))
                .await?,
            "{change}"
        );
        assert!(events.try_recv().is_err(), "{change}");
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_colors")
            .fetch_one(&fixture.pool)
            .await?;
        assert_eq!(rows, 0);
    }
    Ok(())
}

#[tokio::test]
async fn processing_old_fingerprint_cannot_replace_a_reclaimed_attempt() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    tokio::fs::write(
        fixture.data.path().join("asset.png"),
        b"exact-only test source",
    )
    .await?;
    let asset = crate::core::repository::AssetQueryHandler::get_asset_by_id(
        fixture.queries.as_ref(),
        "asset",
    )
    .await?;
    let fingerprint = crate::processing::media::fingerprint::generate(
        asset,
        fixture.registry.clone(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await?
    .ok_or("No fingerprint")?;
    let old = fixture
        .processing
        .begin(ProcessingKind::Fingerprint, "asset")
        .await?
        .ok_or("No attempt")?;
    sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
        .execute(&fixture.pool)
        .await?;
    let current = fixture
        .processing
        .begin(ProcessingKind::Fingerprint, "asset")
        .await?
        .ok_or("No reclaimed attempt")?;
    assert!(
        !fixture
            .ledger
            .commit_processing(&old, ProcessingResult::Fingerprint(fingerprint.clone()))
            .await?
    );
    assert!(
        fixture
            .ledger
            .commit_processing(&current, ProcessingResult::Fingerprint(fingerprint))
            .await?
    );
    fixture
        .processing
        .finish(&old, ProcessingOutcome::Failed("late failure".into()))
        .await?;
    let outcome: (String, i64, i64) = sqlx::query_as("SELECT state, attempts, (SELECT COUNT(*) FROM duplicate_fingerprints) FROM processing_tasks WHERE kind = 'fingerprint'").fetch_one(&fixture.pool).await?;
    assert_eq!(outcome, ("succeeded".into(), 0, 1));
    Ok(())
}

#[tokio::test]
async fn processing_cache_pruning_preserves_published_and_leased_files_and_originals() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let directory = fixture.data.path().join("thumbnails");
    let old = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No attempt")?;
    let old_name = format!("asset.{}.webp", old.lease_id);
    tokio::fs::write(directory.join(&old_name), b"old").await?;
    fixture.ledger.reconcile_processing_artifacts(None).await?;
    assert!(directory.join(&old_name).exists());
    sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
        .execute(&fixture.pool)
        .await?;
    let current = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("No reclaimed attempt")?;
    let new_name = format!("asset.{}.webp", current.lease_id);
    let preview_name = format!("asset.{}.glb", current.lease_id);
    tokio::fs::write(directory.join(&new_name), b"new").await?;
    tokio::fs::write(directory.join(&preview_name), b"model").await?;
    assert!(
        fixture
            .ledger
            .commit_processing(&current, thumbnail_result(&current))
            .await?
    );
    assert!(
        !fixture
            .ledger
            .commit_processing(&old, thumbnail_result(&old))
            .await?
    );
    fixture.ledger.reconcile_processing_artifacts(None).await?;
    assert!(!directory.join(&old_name).exists());
    assert_eq!(tokio::fs::read(directory.join(&new_name)).await?, b"new");
    assert_eq!(
        fixture.processing.preview_path("asset").await?.as_deref(),
        Some(preview_name.as_str())
    );
    tokio::fs::write(fixture.data.path().join("asset.png"), b"original").await?;
    crate::infra::asset_cache::remove_asset_artifacts(
        fixture.data.path(),
        &std::collections::HashSet::from(["asset".to_string()]),
    )
    .await?;
    assert!(!directory.join(new_name).exists());
    assert!(!directory.join(preview_name).exists());
    assert_eq!(
        tokio::fs::read(fixture.data.path().join("asset.png")).await?,
        b"original"
    );
    Ok(())
}
