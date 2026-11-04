use super::*;
use crate::core::formats::{capabilities::ThumbnailCapability, provider::FormatProvider};
use crate::core::ledger::command::LedgerCommand;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_thumbnail_publishes_immutable_versions_and_reclaims_previous_cache(
) -> TestResult {
    struct RasterProvider(Vec<u8>);
    impl FormatProvider for RasterProvider {
        fn name(&self) -> &'static str {
            "RASTER_LEASE_TEST"
        }
        fn supported_extensions(&self) -> Vec<&'static str> {
            vec!["png"]
        }
        fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
            Some(self)
        }
    }
    #[async_trait::async_trait]
    impl ThumbnailCapability for RasterProvider {
        async fn generate(
            &self,
            _path: &std::path::Path,
            _asset_id: &str,
            _size_hint: u32,
        ) -> AppResult<Vec<u8>> {
            Ok(self.0.clone())
        }
    }
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let source = fixture.data.path().join("asset.png");
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(32, 32, image::Rgb([25, 75, 150]))
        .write_to(&mut bytes, image::ImageFormat::Png)?;
    let original = bytes.into_inner();
    tokio::fs::write(&source, &original).await?;
    let mut registry = FormatRegistry::new();
    registry.register(Arc::new(RasterProvider(original.clone())));
    let directory = fixture.data.path().join("thumbnails");
    let worker = ThumbnailWorker::new(
        Arc::new(registry),
        fixture.ledger.clone(),
        fixture.queries.clone(),
        Arc::new(ThumbnailPriorityState::default()),
        directory.clone(),
        fixture.processing.clone(),
    );
    worker.process_single("asset".into(), false).await?;
    let first = fixture
        .processing
        .thumbnail_statuses(&["asset".into()])
        .await?
        .remove(0);
    assert_eq!(
        first.state,
        crate::core::processing::ThumbnailProcessingState::Succeeded
    );
    let first_name = first.thumbnail_path.ok_or("No first thumbnail")?;
    assert!(first_name.starts_with("asset."));
    assert_ne!(first_name, "asset.webp");
    assert!(image::load_from_memory(&tokio::fs::read(directory.join(&first_name)).await?).is_ok());
    fixture
        .ledger
        .execute(LedgerCommand::RegenerateThumbnail {
            asset_id: "asset".into(),
        })
        .await?;
    worker.process_single("asset".into(), false).await?;
    let second = fixture
        .processing
        .thumbnail_statuses(&["asset".into()])
        .await?
        .remove(0);
    let second_name = second.thumbnail_path.ok_or("No second thumbnail")?;
    assert_ne!(first_name, second_name);
    assert!(second.generation > first.generation);
    fixture.ledger.reconcile_processing_artifacts(None).await?;
    assert!(!directory.join(first_name).exists());
    assert!(directory.join(second_name).exists());
    assert_eq!(tokio::fs::read(source).await?, original);
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 1)
        .await?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn processing_thumbnail_pause_retains_durable_work_until_resume() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let control = Arc::new(ProcessingControl::default());
    let worker = ThumbnailWorker::new(
        fixture.registry.clone(),
        fixture.ledger.clone(),
        fixture.queries.clone(),
        Arc::new(ThumbnailPriorityState::default()),
        fixture.data.path().join("thumbnails"),
        fixture.processing.clone(),
    )
    .with_control(control.clone());
    control.set_paused(WorkerKind::Thumbnails, true);
    worker.process_single("asset".into(), true).await?;
    assert_eq!(
        fixture
            .processing
            .pending(ProcessingKind::Thumbnail, 1)
            .await?,
        ["asset"]
    );
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(attempts, 0);
    control.set_paused(WorkerKind::Thumbnails, false);
    worker.process_single("asset".into(), false).await?;
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 1)
        .await?
        .is_empty());
    Ok(())
}

struct FailingThumbnailProvider;

impl FormatProvider for FailingThumbnailProvider {
    fn name(&self) -> &'static str {
        "FAILING_TEST_PROVIDER"
    }
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["heic"]
    }
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl ThumbnailCapability for FailingThumbnailProvider {
    async fn generate(
        &self,
        _path: &std::path::Path,
        _asset_id: &str,
        _size_hint: u32,
    ) -> AppResult<Vec<u8>> {
        Err(AppError::Transcoding(
            "FFmpeg image thumbnail extraction failed: Invalid argument".into(),
        ))
    }
}

#[tokio::test]
async fn processing_thumbnail_failed_extraction_retries_through_backlog_and_reaches_fallback(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("corrupt", None).await?;
    let source = fixture.data.path().join("corrupt.heic");
    tokio::fs::write(&source, b"corrupt HEIC fixture").await?;
    sqlx::query("UPDATE assets SET path = ? WHERE id = 'corrupt'")
        .bind(source.to_string_lossy().as_ref())
        .execute(&fixture.pool)
        .await?;
    let mut registry = FormatRegistry::new();
    registry.register(Arc::new(FailingThumbnailProvider));
    let worker = ThumbnailWorker::new(
        Arc::new(registry),
        fixture.ledger.clone(),
        fixture.queries.clone(),
        Arc::new(ThumbnailPriorityState::default()),
        fixture.data.path().join("thumbnails"),
        fixture.processing.clone(),
    );
    assert!(worker.process_single("corrupt".into(), true).await.is_err());
    fixture.asset("untouched", None).await?;
    for _ in 0..2 {
        sqlx::query("UPDATE processing_tasks SET next_attempt_at = unixepoch() - 1 WHERE asset_id = 'corrupt'")
            .execute(&fixture.pool).await?;
        let next = fixture.queries.get_assets_needing_thumbnails(1).await?;
        assert_eq!(next, ["corrupt"]);
        assert!(worker.process_single(next[0].clone(), false).await.is_err());
    }
    let status = fixture
        .processing
        .thumbnail_statuses(&["corrupt".into()])
        .await?;
    assert_eq!(
        status[0].state,
        crate::core::processing::ThumbnailProcessingState::Failed
    );
    assert_eq!(
        fixture.queries.get_assets_needing_thumbnails(1).await?,
        ["untouched"]
    );
    let persisted: (String, Option<String>, i64) = sqlx::query_as(
        "SELECT a.state, a.thumbnail_path, t.attempts FROM assets a JOIN processing_tasks t ON t.asset_id = a.id WHERE a.id = 'corrupt' AND t.kind = 'thumbnail'"
    ).fetch_one(&fixture.pool).await?;
    assert_eq!(persisted, ("Indexed".into(), None, 3));
    assert_eq!(tokio::fs::read(source).await?, b"corrupt HEIC fixture");
    Ok(())
}

#[tokio::test]
async fn processing_thumbnail_unsupported_is_not_recorded_as_a_successful_file() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let worker = ThumbnailWorker::new(
        fixture.registry.clone(),
        fixture.ledger.clone(),
        fixture.queries.clone(),
        Arc::new(ThumbnailPriorityState::default()),
        fixture.data.path().join("thumbnails"),
        fixture.processing.clone(),
    );
    worker.process_single("asset".into(), false).await?;
    let asset: (String, Option<String>) =
        sqlx::query_as("SELECT state, thumbnail_path FROM assets WHERE id = 'asset'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(asset, ("Indexed".into(), None));
    assert!(fixture
        .processing
        .pending(ProcessingKind::Thumbnail, 1)
        .await?
        .is_empty());
    Ok(())
}

#[test]
fn processing_thumbnail_rejects_invalid_output_and_bounds_generated_images() -> TestResult {
    assert!(prepare_thumbnail(Vec::new()).is_err());
    assert!(prepare_thumbnail(b"invalid data".to_vec()).is_err());
    let image = image::DynamicImage::new_rgb8(1000, 500);
    let mut encoded = std::io::Cursor::new(Vec::new());
    image.write_to(&mut encoded, image::ImageFormat::Png)?;
    let prepared = prepare_thumbnail(encoded.into_inner())?;
    let decoded = image::load_from_memory(&prepared)?;
    assert_eq!(decoded.width(), 300);
    assert_eq!(decoded.height(), 150);
    Ok(())
}

#[test]
fn processing_thumbnail_reservation_is_released_when_a_child_panics() {
    let active = Arc::new(std::sync::Mutex::new(std::collections::HashSet::from([
        "asset".to_owned(),
    ])));
    let active_clone = active.clone();
    let result = std::panic::catch_unwind(move || {
        let _reservation = ThumbnailReservation {
            asset_id: "asset".into(),
            in_flight: active_clone,
        };
        panic!("Simulated provider panic");
    });
    assert!(result.is_err());
    assert!(active
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
}
