//! Partial boot boundaries and native work that outlives the bounded shutdown wait.

use super::*;
use crate::lifecycle::LifecycleRegistry;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio_util::sync::CancellationToken;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn cancellation_at_every_boot_boundary_prevents_later_phases() -> TestResult {
    for boundary in 0..=5 {
        let token = CancellationToken::new();
        let stages = Arc::new(Mutex::new(Vec::new()));
        if boundary == 0 {
            token.cancel();
        }
        run_stages(&token, |stage| {
            let token = token.clone();
            let stages = stages.clone();
            async move {
                let mut stages = stages
                    .lock()
                    .map_err(|_| AppError::Internal("Test stages poisoned".into()))?;
                stages.push(stage);
                if stages.len() == boundary {
                    token.cancel();
                }
                Ok(())
            }
        })
        .await?;
        assert_eq!(
            stages.lock().map_err(|_| "Test stages poisoned")?.len(),
            boundary
        );
    }
    Ok(())
}

#[tokio::test]
async fn failure_in_every_phase_stops_boot_without_starting_dependents() -> TestResult {
    for boundary in 1..=5 {
        let token = CancellationToken::new();
        let stages = Arc::new(Mutex::new(Vec::new()));
        let result = run_stages(&token, |stage| {
            let stages = stages.clone();
            async move {
                let mut stages = stages
                    .lock()
                    .map_err(|_| AppError::Internal("Test stages poisoned".into()))?;
                stages.push(stage);
                if stages.len() == boundary {
                    return Err(AppError::Internal("Injected boot failure".into()));
                }
                Ok(())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(
            stages.lock().map_err(|_| "Test stages poisoned")?.len(),
            boundary
        );
    }
    Ok(())
}

#[tokio::test]
async fn native_startup_timeout_retains_owner_skips_pool_close_and_rejects_late_registration(
) -> TestResult {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    let lifecycle = Arc::new(LifecycleRegistry::new());
    let (release, native_receiver) = std::sync::mpsc::channel();
    let (entered, entered_receiver) = tokio::sync::oneshot::channel();
    let native_finished = Arc::new(AtomicBool::new(false));
    let finished = native_finished.clone();
    let late_started = Arc::new(AtomicBool::new(false));
    let late = late_started.clone();
    let registry = lifecycle.clone();
    let startup = StartupTask::new(tauri::async_runtime::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || {
            let _ = entered.send(());
            let _ = native_receiver.recv_timeout(Duration::from_secs(5));
            finished.store(true, Ordering::Release);
        })
        .await;
        registry.spawn_unique("late_boot_resource".into(), async move {
            late.store(true, Ordering::Release);
        });
    }));
    tokio::time::timeout(Duration::from_secs(2), entered_receiver).await??;
    lifecycle.root_token().cancel();
    let boot_clean = startup.wait_within(Duration::from_millis(30)).await;
    assert!(!boot_clean);
    assert!(!native_finished.load(Ordering::Acquire));
    assert!(lifecycle.shutdown_all().await);
    assert!(!super::super::shutdown::finish_pool(boot_clean, Some(&pool)).await);
    assert!(!pool.is_closed());
    release
        .send(())
        .map_err(|_| "Native fixture unexpectedly finished")?;
    assert!(startup.wait_within(Duration::from_secs(2)).await);
    assert!(startup.wait_within(Duration::from_secs(2)).await);
    assert!(native_finished.load(Ordering::Acquire));
    assert!(!late_started.load(Ordering::Acquire));
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn simultaneous_waiters_do_not_report_clean_before_boot_finishes() -> TestResult {
    let (release, receiver) = tokio::sync::oneshot::channel::<()>();
    let startup = StartupTask::new(tauri::async_runtime::spawn(async move {
        let _ = receiver.await;
    }));
    let (first, second) = tokio::join!(
        startup.wait_within(Duration::from_millis(30)),
        startup.wait_within(Duration::from_millis(30))
    );
    assert!(!first);
    assert!(!second);
    release.send(()).map_err(|_| "Boot fixture lost")?;
    assert!(startup.wait_within(Duration::from_secs(2)).await);
    Ok(())
}

#[tokio::test]
async fn supervisor_panic_is_retained_and_partial_boot_without_database_can_close() -> TestResult {
    let startup = StartupTask::new(tauri::async_runtime::spawn(async {
        panic!("Injected supervisor failure");
    }));
    assert!(!startup.wait_within(Duration::from_secs(2)).await);
    assert!(!startup.wait_within(Duration::from_secs(2)).await);
    assert!(super::super::shutdown::finish_pool(true, None).await);
    assert!(!super::super::shutdown::finish_pool(false, None).await);
    Ok(())
}
