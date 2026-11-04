use super::*;
use crate::core::duplicates::matching::MatchingRepository;
use crate::core::processing::ProcessingKind;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_matching_worker_recovers_manual_work_with_automation_off_after_pause(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = Arc::new(
        crate::infra::database::duplicates::SqliteDuplicatesRepository::new(fixture.pool.clone()),
    );
    repository.get_active_rule_set().await?;
    repository.request_matching("interrupted-manual").await?;
    let control = Arc::new(ProcessingControl::default());
    control.set_paused(WorkerKind::Duplicates, true);
    let settings = SettingsService::new(Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    ));
    settings
        .update_settings(crate::core::settings::AppSettings {
            auto_detect_duplicates: false,
            ..Default::default()
        })
        .await?;
    let worker = Arc::new(
        DuplicateWorker::new(
            repository.clone(),
            fixture.ledger.clone(),
            fixture.event_bus.clone(),
            settings,
            fixture.registry.clone(),
            fixture.processing.clone(),
            fixture.queries.clone(),
            Arc::new(DuplicateProcessingCoordinator::default()),
        )
        .with_control(control.clone()),
    );
    let token = CancellationToken::new();
    let _shutdown = token.clone().drop_guard();
    let handle = worker.start(token.clone());
    tokio::time::sleep(std::time::Duration::from_millis(1600)).await;
    assert!(repository.matching_status().await?.manual_pending);
    assert!(control.snapshot()[2].scheduled);
    assert_eq!(control.snapshot()[2].active, 0);
    control.set_paused(WorkerKind::Duplicates, false);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while repository.matching_status().await?.completed.len() < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        Ok::<(), crate::core::AppError>(())
    })
    .await??;
    assert!(!repository.matching_status().await?.manual_pending);
    token.cancel();
    handle.await?;
    Ok(())
}

#[tokio::test]
async fn processing_matching_snapshot_and_cancel_recover_without_runtime_manual_status(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = Arc::new(
        crate::infra::database::duplicates::SqliteDuplicatesRepository::new(fixture.pool.clone()),
    );
    repository.request_matching("interrupted-manual").await?;
    let control = Arc::new(ProcessingControl::default());
    let settings = Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    );
    let service = crate::feature::duplicates::commands::DuplicateCommandService::new(
        repository.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.queries.clone(),
        fixture.registry.clone(),
        fixture.processing.clone(),
    )
    .with_processing_control(control.clone());
    let snapshots = crate::feature::processing::control::ProcessingControlService::new(
        repository.clone(),
        control.clone(),
        fixture.processing.clone(),
        settings,
        Arc::new(service.clone()),
    );
    assert!(!service.get_scan_status().is_running);
    let snapshot = snapshots.snapshot().await?;
    assert!(snapshot.manual_scan.is_running);
    assert_eq!(snapshot.manual_scan.phase, "recovering");
    control.set_paused(WorkerKind::Duplicates, true);
    assert_eq!(snapshots.snapshot().await?.manual_scan.phase, "paused");
    service.cancel_duplicate_scan().await?;
    assert!(!snapshots.snapshot().await?.manual_scan.is_running);
    assert!(repository
        .claim_matching(
            crate::core::duplicates::matching::MatchingKind::Exact,
            false
        )
        .await?
        .is_none());
    Ok(())
}

#[tokio::test]
async fn processing_manual_and_automatic_scans_persist_identical_fingerprints() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let source = fixture.data.path().join("asset.png");
    image::RgbImage::from_fn(32, 32, |column, row| {
        image::Rgb([(column * 6) as u8, (row * 5) as u8, 64])
    })
    .save(&source)?;
    sqlx::query("UPDATE assets SET format_type = 'raw_sensor_alias', family = 'IMAGE'")
        .execute(&fixture.pool)
        .await?;
    let original = tokio::fs::read(&source).await?;
    let repository = Arc::new(
        crate::infra::database::duplicates::SqliteDuplicatesRepository::new(fixture.pool.clone()),
    );
    let coordinator = Arc::new(DuplicateProcessingCoordinator::default());
    let control = Arc::new(ProcessingControl::default());
    let settings = SettingsService::new(Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    ));
    let worker = DuplicateWorker::new(
        repository.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        settings,
        fixture.registry.clone(),
        fixture.processing.clone(),
        fixture.queries.clone(),
        coordinator.clone(),
    )
    .with_control(control.clone());
    assert_eq!(worker.process_batch(&CancellationToken::new()).await?.0, 1);
    let automatic = repository
        .get_fingerprint("asset")
        .await?
        .ok_or("Missing automatic result")?;
    assert!(automatic.perceptual_hash.is_some());
    repository.reset_duplicate_index().await?;
    let manual = crate::feature::duplicates::commands::DuplicateCommandService::new(
        repository.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.queries.clone(),
        fixture.registry.clone(),
        fixture.processing.clone(),
    )
    .with_processing_coordinator(coordinator)
    .with_processing_control(control);
    manual.start_duplicate_scan().await?;
    let manual_result = repository
        .get_fingerprint("asset")
        .await?
        .ok_or("Missing manual result")?;
    let mut expected = serde_json::to_value(automatic)?;
    let mut actual = serde_json::to_value(manual_result)?;
    expected
        .as_object_mut()
        .ok_or("Invalid fingerprint")?
        .remove("updated_at");
    actual
        .as_object_mut()
        .ok_or("Invalid fingerprint")?
        .remove("updated_at");
    assert_eq!(actual, expected);
    assert_eq!(tokio::fs::read(&source).await?, original);
    assert!(!manual.get_scan_status().is_running);
    Ok(())
}

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
        fixture.ledger.clone(),
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
        fixture.ledger.clone(),
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
