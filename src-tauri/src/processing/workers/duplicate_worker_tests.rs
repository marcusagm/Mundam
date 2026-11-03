use super::super::test_support::{ProcessingFixture, TestResult};
use super::*;

#[tokio::test]
async fn processing_scheduled_comparison_survives_pause_and_finishes_after_resume() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    image::RgbImage::from_pixel(8, 8, image::Rgb([30, 120, 200]))
        .save(fixture.data.path().join("asset.png"))?;
    let control = Arc::new(ProcessingControl::default());
    let settings = SettingsService::new(Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    ));
    settings
        .update_settings(crate::core::settings::AppSettings {
            auto_detect_duplicates: true,
            ..Default::default()
        })
        .await?;
    let worker = DuplicateWorker::new(
        Arc::new(
            crate::infra::database::duplicates::SqliteDuplicatesRepository::new(
                fixture.pool.clone(),
            ),
        ),
        fixture.event_bus.clone(),
        settings,
        fixture.registry.clone(),
        fixture.processing.clone(),
        fixture.queries.clone(),
        Arc::new(DuplicateProcessingCoordinator::default()),
    )
    .with_control(control.clone());
    let token = CancellationToken::new();
    let _shutdown = token.clone().drop_guard();
    let handle = Arc::new(worker).start(token.clone());
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        while !control.snapshot()[2].scheduled {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await?;
    control.set_paused(WorkerKind::Duplicates, true);
    tokio::time::sleep(std::time::Duration::from_millis(1700)).await;
    assert!(control.snapshot()[2].scheduled);
    assert_eq!(control.snapshot()[2].active, 0);
    control.set_paused(WorkerKind::Duplicates, false);
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        while control.snapshot()[2].scheduled {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await?;
    token.cancel();
    handle.await?;
    assert!(!control.snapshot()[2].online);
    Ok(())
}

#[tokio::test]
async fn processing_automatic_duplicates_pause_retains_hashing_and_matching_intent() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let control = Arc::new(ProcessingControl::default());
    let settings = SettingsService::new(Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    ));
    let worker = DuplicateWorker::new(
        Arc::new(
            crate::infra::database::duplicates::SqliteDuplicatesRepository::new(
                fixture.pool.clone(),
            ),
        ),
        fixture.event_bus.clone(),
        settings,
        fixture.registry.clone(),
        fixture.processing.clone(),
        fixture.queries.clone(),
        Arc::new(DuplicateProcessingCoordinator::default()),
    )
    .with_control(control.clone());
    let token = CancellationToken::new();
    control.set_paused(WorkerKind::Duplicates, true);
    assert_eq!(worker.process_batch(&token).await?, (0, false));
    assert!(!worker.compare(&token, true).await?);
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processing_tasks")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(attempts, 0);
    control.set_paused(WorkerKind::Duplicates, false);
    worker.process_batch(&token).await?;
    let counts = fixture
        .processing
        .queue_counts(ProcessingKind::Fingerprint, &[])
        .await?;
    assert_eq!(counts.retry_wait, 1);
    assert_eq!(control.snapshot()[2].active, 0);
    Ok(())
}
