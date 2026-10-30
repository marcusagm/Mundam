//! Regression checks for events queued before a monitored tree was removed.

use super::*;
use crate::infra::events::TokioEventBus;

#[tokio::test]
async fn folder_removal_unwatches_adopted_roots_and_discards_pending_events() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let child = directory.path().join("Child");
    tokio::fs::create_dir(&child).await?;
    let watcher = WatcherService::new(Arc::new(TokioEventBus::new()));
    watcher
        .watch(child.clone(), CancellationToken::new())
        .await?;
    watcher
        .watch(directory.path().to_path_buf(), CancellationToken::new())
        .await?;
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
