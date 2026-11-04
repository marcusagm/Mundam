use super::*;
use crate::core::repository::DuplicatesRepository;
use crate::infra::database::duplicates::SqliteDuplicatesRepository;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

fn processor(fixture: &ProcessingFixture) -> FingerprintProcessor {
    FingerprintProcessor::new(
        fixture.ledger.clone(),
        fixture.processing.clone(),
        fixture.queries.clone(),
        fixture.registry.clone(),
    )
}

#[tokio::test]
async fn processing_fingerprint_superseded_during_provider_work_is_not_saved() -> TestResult {
    use crate::core::formats::{capabilities::PreviewCapability, provider::FormatProvider};
    struct BlockedProvider {
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
        bytes: Vec<u8>,
    }
    impl FormatProvider for BlockedProvider {
        fn name(&self) -> &'static str {
            "BLOCKED_FINGERPRINT_TEST"
        }
        fn supported_extensions(&self) -> Vec<&'static str> {
            vec!["png"]
        }
        fn preview(&self) -> Option<&dyn PreviewCapability> {
            Some(self)
        }
    }
    #[async_trait::async_trait]
    impl PreviewCapability for BlockedProvider {
        async fn generate_preview(
            &self,
            _path: &std::path::Path,
            _asset_id: &str,
        ) -> AppResult<(Vec<u8>, String)> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok((self.bytes.clone(), "image/png".into()))
        }
    }
    struct ReleaseOnDrop(Arc<tokio::sync::Notify>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.0.notify_one();
        }
    }
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    tokio::fs::write(
        fixture.data.path().join("asset.png"),
        b"custom test container",
    )
    .await?;
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let _release_guard = ReleaseOnDrop(release.clone());
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 16, image::Rgb([30, 90, 150]))
        .write_to(&mut bytes, image::ImageFormat::Png)?;
    let mut registry = FormatRegistry::new();
    registry.register(Arc::new(BlockedProvider {
        entered: entered.clone(),
        release: release.clone(),
        bytes: bytes.into_inner(),
    }));
    let repository = Arc::new(SqliteDuplicatesRepository::new(fixture.pool.clone()));
    let processor = FingerprintProcessor::new(
        fixture.ledger.clone(),
        fixture.processing.clone(),
        fixture.queries.clone(),
        Arc::new(registry),
    );
    let token = CancellationToken::new();
    let _shutdown = token.clone().drop_guard();
    let running = tokio::spawn(async move { processor.process_asset("asset", &token).await });
    tokio::time::timeout(std::time::Duration::from_secs(4), entered.notified()).await?;
    sqlx::query("UPDATE assets SET file_size = 999 WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    release.notify_one();
    let joined = tokio::time::timeout(std::time::Duration::from_secs(4), running).await?;
    assert!(!joined??);
    assert!(repository.get_fingerprint("asset").await?.is_none());
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Fingerprint, 1)
            .await?,
        ["asset"]
    );
    Ok(())
}

#[tokio::test]
async fn processing_manual_fingerprints_share_eligibility_retries_and_exact_only_completion(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    for identifier in [
        "valid", "missing", "corrupt", "offline", "trashed", "removed",
    ] {
        fixture.asset(identifier, None).await?;
    }
    image::RgbImage::from_pixel(16, 16, image::Rgb([45, 90, 180]))
        .save(fixture.data.path().join("valid.png"))?;
    tokio::fs::write(
        fixture.data.path().join("corrupt.png"),
        b"corrupt test raster",
    )
    .await?;
    sqlx::query("UPDATE assets SET state = 'Offline' WHERE id = 'offline'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET state = 'Trashed' WHERE id = 'trashed'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP WHERE id = 'removed'")
        .execute(&fixture.pool)
        .await?;
    let repository = SqliteDuplicatesRepository::new(fixture.pool.clone());
    repository.backfill_missing_fingerprints().await?;
    let processor = processor(&fixture);
    let (progress, mut receiver) = tokio::sync::watch::channel((0, 0));
    assert_eq!(
        processor
            .rehash_pending(&CancellationToken::new(), Some(progress))
            .await?,
        2
    );
    assert!(receiver.changed().await.is_ok());
    assert_eq!(*receiver.borrow_and_update(), (3, 3));
    assert!(receiver.changed().await.is_err());
    let counts = fixture
        .processing
        .queue_counts(ProcessingKind::Fingerprint, &[])
        .await?;
    assert_eq!(
        (counts.pending, counts.retry_wait, counts.failed),
        (0, 1, 0)
    );
    assert_eq!(
        processor
            .rehash_pending(&CancellationToken::new(), None)
            .await?,
        0
    );
    let inactive: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_fingerprints WHERE asset_id IN ('offline', 'trashed', 'removed')").fetch_one(&fixture.pool).await?;
    assert_eq!(inactive, 0);
    let corrupt = repository
        .get_fingerprint("corrupt")
        .await?
        .ok_or("Missing exact-only result")?;
    assert!(corrupt.content_hash.is_some());
    assert!(corrupt.perceptual_hash.is_none());
    assert_eq!(corrupt.fingerprint_version, 5);
    Ok(())
}

#[tokio::test]
async fn processing_cancel_before_manual_fingerprint_does_not_consume_an_attempt() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let token = CancellationToken::new();
    token.cancel();
    assert!(!processor(&fixture).process_asset("asset", &token).await?);
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(attempts, 0);
    Ok(())
}

#[tokio::test]
async fn processing_rebuild_clears_fingerprint_failures_without_resetting_other_stages(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("asset.webp")).await?;
    let fingerprint = fixture
        .processing
        .begin(ProcessingKind::Fingerprint, "asset")
        .await?
        .ok_or("Missing attempt")?;
    fixture
        .processing
        .finish(
            &fingerprint,
            ProcessingOutcome::Failed("missing source".into()),
        )
        .await?;
    sqlx::query("UPDATE processing_tasks SET state = 'failed', attempts = 3")
        .execute(&fixture.pool)
        .await?;
    fixture
        .processing
        .begin(ProcessingKind::Colors, "asset")
        .await?
        .ok_or("Missing colors request")?;
    let repository = SqliteDuplicatesRepository::new(fixture.pool.clone());
    repository.reset_duplicate_index().await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Fingerprint, 1)
            .await?,
        ["asset"]
    );
    let colors: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM processing_tasks WHERE kind = 'colors'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(colors, 1);
    assert!(fixture.queries.get_by_id("asset").await?.is_some());
    Ok(())
}
