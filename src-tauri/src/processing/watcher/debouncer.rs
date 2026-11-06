//! Coalesces native notifications without inferring object identity from names or metadata.
use crate::core::events::payloads::DomainEvent;
use notify::{
    event::{ModifyKind, RemoveKind, RenameMode},
    Event, EventKind,
};
use std::{collections::HashMap, path::PathBuf};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant},
};

/// Aggregates notifications; only native paired renames produce rename hints.
/// Catalog handlers independently verify persisted native identity before transferring metadata.
pub struct EventDebouncer {
    output_sender: mpsc::Sender<DomainEvent>,
    buffer: HashMap<PathBuf, Instant>,
    pending_tracked_renames: HashMap<usize, (PathBuf, Instant)>,
    pending_untracked_removes: HashMap<PathBuf, (Instant, bool)>,
}

impl EventDebouncer {
    /// Creates a bounded notification producer whose receiver must run concurrently.
    pub fn new(output_sender: mpsc::Sender<DomainEvent>) -> Self {
        Self {
            output_sender,
            buffer: HashMap::new(),
            pending_tracked_renames: HashMap::new(),
            pending_untracked_removes: HashMap::new(),
        }
    }

    /// Coalesces raw notifications without pairing unrelated delete/create events.
    pub async fn handle_event(&mut self, event: Event) {
        let now = Instant::now();
        match event.kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 => {
                self.emit_rename(event.paths[0].clone(), event.paths[1].clone())
                    .await;
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                for path in event.paths {
                    self.buffer.remove(&path);
                    if let Some(tracker) = event.attrs.tracker() {
                        self.pending_tracked_renames.insert(tracker, (path, now));
                    } else {
                        self.pending_untracked_removes.insert(path, (now, false));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                for path in event.paths {
                    let source = event
                        .attrs
                        .tracker()
                        .and_then(|tracker| self.pending_tracked_renames.remove(&tracker));
                    if let Some((source, _)) = source {
                        self.emit_rename(source, path).await;
                    } else {
                        self.buffer.insert(path, now);
                    }
                }
            }
            EventKind::Remove(kind) => {
                for path in event.paths {
                    self.buffer.remove(&path);
                    self.pending_untracked_removes
                        .insert(path, (now, kind == RemoveKind::Folder));
                }
            }
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    self.buffer.insert(path, now);
                }
            }
            _ => {}
        }
    }

    async fn emit_rename(&mut self, source: PathBuf, destination: PathBuf) {
        self.buffer.remove(&source);
        self.pending_untracked_removes.remove(&source);
        let _ = self
            .output_sender
            .send(DomainEvent::FsPathRenamed {
                from: source.to_string_lossy().into_owned(),
                to: destination.to_string_lossy().into_owned(),
            })
            .await;
    }

    /// Emits settled discoveries and delayed removals; inaccessible entries remain retryable.
    pub async fn tick(&mut self, debounce_window: Duration) {
        let now = Instant::now();
        let deletion_window = Duration::from_secs(3);
        let expired_trackers: Vec<_> = self
            .pending_tracked_renames
            .iter()
            .filter(|(_, (_, timestamp))| now.duration_since(*timestamp) >= deletion_window)
            .map(|(tracker, _)| *tracker)
            .collect();
        for tracker in expired_trackers {
            if let Some((path, timestamp)) = self.pending_tracked_renames.remove(&tracker) {
                self.pending_untracked_removes
                    .insert(path, (timestamp, false));
            }
        }
        let removals: Vec<_> = self
            .pending_untracked_removes
            .iter()
            .filter(|(_, (timestamp, _))| now.duration_since(*timestamp) >= deletion_window)
            .map(|(path, (_, directory))| (path.clone(), *directory))
            .collect();
        for (path, directory) in removals {
            match tokio::fs::symlink_metadata(&path).await {
                Ok(_) => {
                    self.pending_untracked_removes.remove(&path);
                    self.buffer.insert(path, now);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.pending_untracked_removes.remove(&path);
                    let path = path.to_string_lossy().into_owned();
                    let event = if directory {
                        DomainEvent::FsDirectoryDeleted { path }
                    } else {
                        DomainEvent::FsPathDeleted { path }
                    };
                    let _ = self.output_sender.send(event).await;
                }
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Removal observation will retry")
                }
            }
        }
        let discoveries: Vec<_> = self
            .buffer
            .iter()
            .filter(|(_, timestamp)| now.duration_since(**timestamp) >= debounce_window)
            .map(|(path, _)| path.clone())
            .collect();
        for path in discoveries {
            match tokio::fs::symlink_metadata(&path).await {
                Ok(metadata) => {
                    self.buffer.remove(&path);
                    let path = path.to_string_lossy().into_owned();
                    let event = if metadata.is_dir() {
                        DomainEvent::FsDirectoryDiscovered { path }
                    } else {
                        DomainEvent::FsFileDiscovered {
                            path,
                            size_bytes: metadata.len(),
                        }
                    };
                    let _ = self.output_sender.send(event).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.buffer.remove(&path);
                    self.pending_untracked_removes.insert(path, (now, false));
                }
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "Discovery observation will retry")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{ModifyKind, RenameMode};

    #[tokio::test]
    async fn unrelated_same_extension_files_are_never_paired_as_a_rename(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("old.png");
        let destination = directory.path().join("new.png");
        tokio::fs::write(&destination, b"same size").await?;
        let (sender, mut receiver) = mpsc::channel(10);
        let mut debouncer = EventDebouncer::new(sender);
        debouncer
            .handle_event(
                Event::new(EventKind::Remove(notify::event::RemoveKind::File))
                    .add_path(source.clone()),
            )
            .await;
        debouncer
            .handle_event(
                Event::new(EventKind::Create(notify::event::CreateKind::File))
                    .add_path(destination.clone()),
            )
            .await;
        debouncer.tick(Duration::ZERO).await;
        assert!(
            matches!(receiver.try_recv()?, DomainEvent::FsFileDiscovered { path, .. } if path == destination.to_string_lossy())
        );
        assert!(receiver.try_recv().is_err());
        assert!(debouncer.pending_untracked_removes.contains_key(&source));
        Ok(())
    }

    #[tokio::test]
    async fn native_tracker_survives_ticks_until_its_pair_or_timeout(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = mpsc::channel(10);
        let mut debouncer = EventDebouncer::new(sender);
        let mut from = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::From)))
            .add_path("old.png".into());
        from.attrs.set_tracker(42);
        debouncer.handle_event(from).await;
        debouncer.tick(Duration::ZERO).await;
        assert_eq!(debouncer.pending_tracked_renames.len(), 1);
        let mut to = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::To)))
            .add_path("new.png".into());
        to.attrs.set_tracker(42);
        debouncer.handle_event(to).await;
        assert!(
            matches!(receiver.try_recv()?, DomainEvent::FsPathRenamed { from, to } if from == "old.png" && to == "new.png")
        );
        assert!(debouncer.pending_tracked_renames.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_event_aggregation() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = mpsc::channel(100);
        let mut debouncer = EventDebouncer::new(sender);

        // Create a real temp file so `path.exists()` is true
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "test_event_agg_{}.txt",
            std::time::UNIX_EPOCH.elapsed()?.as_nanos()
        ));
        std::fs::write(&path, "test content")?;

        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.clone());

        // Send 3 events in rapid succession
        debouncer.handle_event(event.clone()).await;
        debouncer.handle_event(event.clone()).await;
        debouncer.handle_event(event.clone()).await;

        assert_eq!(debouncer.buffer.len(), 1);

        // Tick with small window (should not emit)
        debouncer.tick(Duration::from_millis(500)).await;
        assert!(receiver.try_recv().is_err());

        // Tick with expired window
        tokio::time::sleep(Duration::from_millis(600)).await;
        debouncer.tick(Duration::from_millis(500)).await;

        let result = receiver.try_recv()?;
        match result {
            DomainEvent::FsFileDiscovered {
                path: event_path, ..
            } => {
                assert_eq!(event_path, path.to_string_lossy().to_string());
            }
            _ => panic!("Expected FsFileDiscovered"),
        }

        // Cleanup
        let _ = std::fs::remove_file(path);

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_mode_both() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = mpsc::channel(100);
        let mut debouncer = EventDebouncer::new(sender);

        let from = PathBuf::from("/tmp/old.txt");
        let to = PathBuf::from("/tmp/new.txt");

        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(from.clone())
            .add_path(to.clone());

        debouncer.handle_event(event).await;

        let result = receiver.try_recv()?;
        match result {
            DomainEvent::FsPathRenamed {
                from: event_from,
                to: event_to,
            } => {
                assert_eq!(event_from, from.to_string_lossy().to_string());
                assert_eq!(event_to, to.to_string_lossy().to_string());
            }
            _ => panic!("Expected FsPathRenamed"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delayed_deletion_guard() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = mpsc::channel(100);
        let mut debouncer = EventDebouncer::new(sender);

        let path = PathBuf::from("/tmp/nonexistent_test_file_12345.txt");
        let event =
            Event::new(EventKind::Remove(notify::event::RemoveKind::File)).add_path(path.clone());

        debouncer.handle_event(event).await;

        // Immediate tick — should NOT emit (guard period not expired)
        debouncer.tick(Duration::from_millis(200)).await;
        assert!(
            receiver.try_recv().is_err(),
            "Should NOT have emitted yet — deletion guard active"
        );

        // Wait for guard period to expire (debouncer uses 3s deletion guard)
        tokio::time::sleep(Duration::from_secs(4)).await;
        debouncer.tick(Duration::from_millis(200)).await;

        // Now it should emit (path doesn't exist)
        let result = receiver.try_recv()?;
        match result {
            DomainEvent::FsPathDeleted { path: event_path } => {
                assert_eq!(event_path, path.to_string_lossy().to_string());
            }
            _ => panic!("Expected FsPathDeleted"),
        }

        Ok(())
    }
}
