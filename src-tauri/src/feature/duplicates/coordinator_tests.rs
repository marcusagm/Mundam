use super::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use crate::infra::database::duplicates::SqliteDuplicatesRepository;

#[tokio::test]
async fn processing_duplicate_cancel_waits_for_exit_and_prevents_a_second_scan() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let coordinator = Arc::new(super::super::coordinator::DuplicateProcessingCoordinator::default());
    let _busy = coordinator.acquire().await;
    let service = DuplicateCommandService::new(
        Arc::new(SqliteDuplicatesRepository::new(fixture.pool.clone())), fixture.ledger.clone(),
        fixture.event_bus.clone(), fixture.queries.clone(), fixture.registry.clone(),
    ).with_processing_coordinator(coordinator.clone());
    let running_service = service.clone();
    let running = tokio::spawn(async move { running_service.start_duplicate_scan().await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !service.get_scan_status().is_running { tokio::task::yield_now().await; }
    }).await?;
    let run_id = service.get_scan_status().run_id;
    service.cancel_duplicate_scan();
    assert!(service.get_scan_status().is_running);
    assert_eq!(service.get_scan_status().phase, "cancel_requested");
    service.start_duplicate_scan().await?;
    assert_eq!(service.get_scan_status().run_id, run_id);
    running.await??;
    assert!(!service.get_scan_status().is_running);
    Ok(())
}

#[test]
fn processing_duplicate_old_guard_cannot_clear_a_new_execution() {
    let status = Arc::new(RwLock::new(DuplicateScanStatus {
        run_id: Some("new".into()), is_running: true, ..Default::default()
    }));
    let event_bus = Arc::new(crate::infra::events::TokioEventBus::new());
    let mut events = event_bus.subscribe();
    drop(ScanRunningGuard { status: status.clone(), run_id: "old".into(), event_bus,
        token: CancellationToken::new(), completed: false });
    assert!(status.read().unwrap_or_else(|poisoned| poisoned.into_inner()).is_running);
    assert!(events.try_recv().is_err());
}
