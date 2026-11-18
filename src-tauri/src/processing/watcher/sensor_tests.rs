//! Regression checks for watcher ownership, shutdown and removed-tree notifications.

use super::*;
use crate::infra::events::TokioEventBus;
use std::time::Duration;

#[tokio::test]
async fn folder_removal_unwatches_adopted_roots_and_discards_pending_events() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let child = directory.path().join("Child");
    tokio::fs::create_dir(&child).await?;
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), CancellationToken::new());
    watcher.watch(child.clone()).await?;
    watcher.watch(directory.path().to_path_buf()).await?;
    let pending_event = DomainEvent::FsDirectoryDiscovered {
        path: child.to_string_lossy().into_owned(),
    };
    assert!(is_event_monitored(
        &pending_event,
        &*watcher.watched_paths.lock().await
    ));

    watcher.unwatch(directory.path().to_path_buf()).await?;

    let registrations = watcher.watched_paths.lock().await;
    assert!(registrations.is_empty());
    assert!(!is_event_monitored(&pending_event, &registrations));
    drop(registrations);
    watcher.shutdown().await?;
    Ok(())
}

#[test]
fn folder_removal_preserves_other_roots_and_detects_moves_across_their_boundary() {
    let registrations = HashSet::from([PathBuf::from("/Other")]);
    assert!(is_event_monitored(
        &DomainEvent::FsPathRenamed {
            from: "/Other/photo.jpg".into(),
            to: "/Removed/photo.jpg".into(),
        },
        &registrations
    ));
    assert!(!is_event_monitored(
        &DomainEvent::FsFileDiscovered {
            path: "/OtherSibling/photo.jpg".into(),
            size_bytes: 8,
        },
        &registrations
    ));
}

#[tokio::test]
async fn native_events_stop_and_late_watch_and_reconcile_are_rejected() -> AppResult<()> {
    use crate::core::library::RootObservation;

    let directory = tempfile::tempdir()?;
    let event_bus = Arc::new(TokioEventBus::new());
    let mut receiver = event_bus.subscribe();
    let watcher = WatcherService::new(event_bus, CancellationToken::new());
    watcher.watch(directory.path().to_path_buf()).await?;
    // Boot/manual setup and reconciliation can both request the same native root.
    watcher.watch(directory.path().to_path_buf()).await?;
    let file = directory.path().join("observed.txt");
    tokio::fs::write(&file, b"before shutdown").await?;
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if matches!(receiver.recv().await, Ok(DomainEvent::FsFileDiscovered { path, .. }) if path.ends_with("observed.txt")) {
                break;
            }
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    tokio::time::timeout(Duration::from_secs(5), watcher.shutdown())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    while receiver.try_recv().is_ok() {}
    tokio::fs::write(&file, b"after shutdown").await?;
    assert!(
        tokio::time::timeout(Duration::from_millis(900), receiver.recv())
            .await
            .is_err()
    );
    assert!(matches!(
        watcher.watch(directory.path().to_path_buf()).await,
        Err(AppError::Cancelled)
    ));
    assert!(matches!(
        watcher
            .reconcile(vec![directory.path().to_path_buf()])
            .await,
        Err(AppError::Cancelled)
    ));
    let state = watcher.state.lock().await;
    assert!(state.completed);
    assert!(state.native_watcher.is_none());
    assert!(state.native_operation.is_none());
    assert!(state.pipeline.is_none());
    assert!(watcher.watched_paths.lock().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn cancelled_empty_service_has_idempotent_concurrent_shutdown() -> AppResult<()> {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), cancellation);
    assert!(matches!(
        watcher.watch(PathBuf::from("unused")).await,
        Err(AppError::Cancelled)
    ));
    let (first, second) = tokio::join!(watcher.shutdown(), watcher.shutdown());
    first?;
    second?;
    assert!(watcher.state.lock().await.completed);
    Ok(())
}

#[tokio::test]
async fn concurrent_duplicate_roots_keep_one_registration_and_stop_cleanly() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), CancellationToken::new());
    let path = directory.path().to_path_buf();
    let (first, second) = tokio::join!(watcher.watch(path.clone()), watcher.watch(path));
    first?;
    second?;
    assert_eq!(watcher.watched_paths.lock().await.len(), 1);
    if cfg!(windows) {
        let case_variant = PathBuf::from(directory.path().to_string_lossy().to_uppercase());
        watcher.watch(case_variant.clone()).await?;
        watcher.watch(case_variant).await?;
        // Exact duplicate requests are suppressed, but spelling variants retain
        // their own registration instead of making an identity claim from case.
        assert_eq!(watcher.watched_paths.lock().await.len(), 2);
    }
    tokio::time::timeout(Duration::from_secs(5), watcher.shutdown())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert!(watcher.state.lock().await.completed);
    Ok(())
}

#[tokio::test]
async fn lifecycle_waits_for_native_observation_and_notification_loops() -> AppResult<()> {
    let lifecycle = Arc::new(crate::lifecycle::LifecycleRegistry::new());
    let cancellation = lifecycle.child_token();
    let watcher = Arc::new(WatcherService::new(
        Arc::new(TokioEventBus::new()),
        cancellation.clone(),
    ));
    let owner = watcher.clone();
    let registry = lifecycle.clone();
    assert!(
        lifecycle.spawn_unique("library_watcher".into(), async move {
            cancellation.cancelled().await;
            if owner.shutdown().await.is_err() {
                registry.report_failure("library watcher");
            }
        })
    );
    let directory = tempfile::tempdir()?;
    watcher.watch(directory.path().to_path_buf()).await?;
    assert!(lifecycle.shutdown_all().await);
    let state = watcher.state.lock().await;
    assert!(state.completed);
    assert!(state.native_watcher.is_none());
    assert!(state.native_operation.is_none());
    assert!(state.pipeline.is_none());
    Ok(())
}

#[tokio::test]
async fn shutdown_racing_native_watch_never_leaves_an_owner_or_registration() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), CancellationToken::new());
    let (watch, shutdown) = tokio::join!(
        watcher.watch(directory.path().to_path_buf()),
        watcher.shutdown()
    );
    assert!(watch.is_ok() || matches!(watch, Err(AppError::Cancelled)));
    shutdown?;
    let state = watcher.state.lock().await;
    assert!(state.completed);
    assert!(state.native_watcher.is_none());
    assert!(state.native_operation.is_none());
    assert!(state.pipeline.is_none());
    assert!(watcher.watched_paths.lock().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn dropping_a_request_and_a_shutdown_wait_preserves_the_native_owner() -> AppResult<()> {
    let watcher = Arc::new(WatcherService::new(
        Arc::new(TokioEventBus::new()),
        CancellationToken::new(),
    ));
    let (started, native_started) = tokio::sync::oneshot::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    watcher.state.lock().await.native_operation = Some(tokio::task::spawn_blocking(move || {
        let _ = started.send(());
        let result = blocked
            .recv()
            .map_err(|error| AppError::Internal(error.to_string()));
        NativeOutcome {
            watcher: None,
            added: Vec::new(),
            removed: Vec::new(),
            result,
            supervision_failed: false,
        }
    }));
    native_started
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let requesting_watcher = watcher.clone();
    let request =
        tokio::spawn(async move { requesting_watcher.watch(PathBuf::from("unused")).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while watcher.state.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    request.abort();
    assert!(request.await.is_err());

    // A cancelled waiter must leave its JoinHandle available to the next drain attempt.
    assert!(
        tokio::time::timeout(Duration::from_millis(30), watcher.shutdown())
            .await
            .is_err()
    );
    assert!(watcher.state.lock().await.native_operation.is_some());
    release
        .send(())
        .map_err(|error| AppError::Internal(error.to_string()))?;
    tokio::time::timeout(Duration::from_secs(3), watcher.shutdown())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    assert!(watcher.state.lock().await.native_operation.is_none());
    Ok(())
}

#[tokio::test]
async fn failed_notification_owner_is_not_reported_as_clean() -> AppResult<()> {
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), CancellationToken::new());
    let pipeline = tokio::spawn(std::future::pending::<PipelineReport>());
    pipeline.abort();
    watcher.state.lock().await.pipeline = Some(pipeline);
    assert!(watcher.shutdown().await.is_err());
    assert!(watcher.shutdown().await.is_err());
    assert!(watcher.state.lock().await.completed);
    Ok(())
}

#[tokio::test]
async fn cancellation_releases_callback_waiting_on_a_full_native_queue() -> AppResult<()> {
    let (sender, receiver) = mpsc::channel(1);
    sender
        .try_send(Event::new(notify::EventKind::Any))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let (started, native_started) = tokio::sync::oneshot::channel();
    let callback = tokio::task::spawn_blocking(move || {
        let _ = started.send(());
        sender.blocking_send(Event::new(notify::EventKind::Any))
    });
    native_started
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let event_bus = Arc::new(TokioEventBus::new());
    let mut events = event_bus.subscribe();
    let report = tokio::time::timeout(
        Duration::from_secs(3),
        runtime::run(
            receiver,
            event_bus,
            Arc::new(Mutex::new(HashSet::new())),
            cancellation,
        ),
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    assert_eq!(report.raw_discarded, 1);
    assert!(callback
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .is_err());
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn shutdown_discards_unsettled_rename_hints_without_filesystem_reads() -> AppResult<()> {
    let (sender, receiver) = mpsc::channel(1);
    let event_bus = Arc::new(TokioEventBus::new());
    let mut events = event_bus.subscribe();
    let cancellation = CancellationToken::new();
    let pipeline = tokio::spawn(runtime::run(
        receiver,
        event_bus,
        Arc::new(Mutex::new(HashSet::new())),
        cancellation.clone(),
    ));
    let mut event = Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Name(
        notify::event::RenameMode::From,
    )))
    .add_path(PathBuf::from("unsettled.txt"));
    event.attrs.set_tracker(21);
    sender
        .send(event)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    tokio::time::timeout(Duration::from_secs(1), async {
        while sender.capacity() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    cancellation.cancel();
    let report = tokio::time::timeout(Duration::from_secs(3), pipeline)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))?;
    assert_eq!(report.pending_discarded, 1);
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn cancellation_drains_a_backpressured_domain_queue_without_publication() -> AppResult<()> {
    let (sender, receiver) = mpsc::channel(256);
    for index in 0..250 {
        let event = Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(PathBuf::from(format!("monitored/before{index}")))
        .add_path(PathBuf::from(format!("monitored/after{index}")));
        sender
            .try_send(event)
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    let watched_paths = Arc::new(Mutex::new(HashSet::from([PathBuf::from("monitored")])));
    let registrations = watched_paths.lock().await;
    let cancellation = CancellationToken::new();
    let event_bus = Arc::new(TokioEventBus::new());
    let mut events = event_bus.subscribe();
    let pipeline = tokio::spawn(runtime::run(
        receiver,
        event_bus,
        watched_paths.clone(),
        cancellation.clone(),
    ));
    // More than 100 receives fill the bounded output while publication is held.
    tokio::time::timeout(Duration::from_secs(3), async {
        while sender.capacity() < 108 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    cancellation.cancel();
    drop(registrations);
    let report = tokio::time::timeout(Duration::from_secs(3), pipeline)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .map_err(|error| AppError::Internal(error.to_string()))?;
    assert!(report.domain_discarded >= 100);
    assert!(report.raw_discarded > 0);
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn failed_native_owner_is_not_reported_as_clean() -> AppResult<()> {
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()), CancellationToken::new());
    let native_operation = tokio::spawn(std::future::pending::<NativeOutcome>());
    native_operation.abort();
    watcher.state.lock().await.native_operation = Some(native_operation);
    assert!(watcher.shutdown().await.is_err());
    assert!(watcher.state.lock().await.completed);
    Ok(())
}
