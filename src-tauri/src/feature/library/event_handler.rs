//! Real-time filesystem event handler for the Library Indexer.
//!
//! Processes individual `DomainEvent` items emitted by the debouncer/watcher
//! and translates them into targeted Ledger commands (add, rename, delete, move).
//! This is the V2 equivalent of the V1 synchronous event loop, designed to avoid
//! full directory re-scans for single-file operations.

use crate::core::error::AppResult;
use crate::core::events::AppEventBus;
use crate::core::ledger::command::{CreateAssetPayload, CreateFolderPayload, LedgerCommand};
use crate::core::models::asset::AssetState;
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use unicode_normalization::UnicodeNormalization;

use super::indexer::LibraryIndexer;

impl LibraryIndexer {
    /// Starts a background loop that listens for domain events and handles
    /// individual file system changes through direct ledger commands.
    ///
    /// This mirrors the V1 behavior where each event (add, rename, delete)
    /// was processed as a single, targeted operation — NOT a full re-scan.
    pub fn start_event_listener(
        self: Arc<Self>,
        event_bus: Arc<dyn AppEventBus>,
        token: tokio_util::sync::CancellationToken,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let mut receiver = event_bus.subscribe();
        tauri::async_runtime::spawn(async move {
            info!("IndexerEventListener: started and listening for filesystem events");

            loop {
                tokio::select! {
                    recv_result = receiver.recv() => {
                        match recv_result {
                            Ok(event) => {
                                self.handle_single_event(event).await;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                                warn!(
                                    "IndexerEventListener: lagged behind {} events, continuing",
                                    skipped_count
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                info!("IndexerEventListener: channel closed, shutting down");
                                break;
                            }
                        }
                    }
                    _ = token.cancelled() => {
                        info!("IndexerEventListener: cancelled, shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Handle a single DomainEvent with a targeted ledger operation.
    ///
    /// Instead of calling scan_directory() (which walks 42k+ files), this
    /// performs the minimal database operation for each event type.
    async fn handle_single_event(&self, event: crate::core::events::DomainEvent) {
        use crate::core::events::DomainEvent;

        match event {
            DomainEvent::FsFileDiscovered { path, .. } => {
                self.handle_file_discovered(path).await;
            }

            DomainEvent::FsPathDeleted { path } => {
                self.handle_path_deleted(path).await;
            }

            DomainEvent::FsPathRenamed { from, to } => {
                self.handle_path_renamed(from, to).await;
            }

            DomainEvent::FsDirectoryDiscovered { path } => {
                self.handle_directory_discovered(path).await;
            }

            DomainEvent::FsDirectoryDeleted { path } => {
                self.handle_directory_deleted(path).await;
            }

            _ => {}
        }
    }

    /// Handles a new file discovery event, potentially recovering a rename or move.
    ///
    /// Uses a 500ms strategic delay to allow `handle_path_deleted` to populate
    /// `recent_removals` first. Then attempts a "Fast Match" by fingerprinting
    /// the new file against recently removed entries (size + created_at).
    /// If no match is found, falls through to normal asset creation.
    async fn handle_file_discovered(&self, path: String) {
        let entry_path = PathBuf::from(&path);

        if self.path_guard.is_marked(&entry_path) {
            debug!("Indexer: Ignoring ledger-initiated create for {}", path);
            return;
        }
        // Strategic Delay: On macOS, a Rename is often emitted as Delete then Create.
        // 500ms gives enough time for handle_path_deleted to populate recent_removals.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let disk_metadata = std::fs::metadata(&entry_path).ok();
        let disk_size = disk_metadata
            .as_ref()
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(0);
        let disk_created_at: Option<DateTime<Utc>> = disk_metadata
            .as_ref()
            .and_then(|metadata| metadata.created().ok())
            .map(DateTime::<Utc>::from);

        let from_path = self
            .recent_removals
            .iter()
            .find(|entry| {
                if *entry.key() == entry_path {
                    return false;
                }

                let (_, (_, removed_size, removed_created_at)) = entry.pair();
                let size_matches = *removed_size as i64 == disk_size && disk_size > 0;
                if !size_matches {
                    return false;
                }
                match (removed_created_at, &disk_created_at) {
                    (Some(database_created), Some(filesystem_created)) => {
                        (*database_created - *filesystem_created)
                            .num_seconds()
                            .abs()
                            < 2
                    }
                    _ => true,
                }
            })
            .map(|entry| entry.key().clone());

        if let Some(from_path) = from_path {
            info!(
                "Indexer: Fast Match (Size: {}) - Treating as Move: {} -> {}",
                disk_size,
                from_path.to_string_lossy(),
                path
            );

            self.recent_removals.remove(&from_path);

            let update_payload = crate::core::ledger::command::UpdateAssetPayload {
                asset_id: None,
                old_path: Some(from_path),
                new_path: entry_path.clone(),
            };
            let _ = self
                .ledger
                .execute(LedgerCommand::UpdateAsset(update_payload))
                .await;
            return;
        }

        let _ = self.handle_file_creation(entry_path).await;
    }

    /// Extracts metadata and persists a new file to the ledger.
    ///
    /// Performs a collision check against the database before creating the asset
    /// to prevent duplicates when the same event is processed multiple times.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Io` if filesystem metadata cannot be read.
    async fn handle_file_creation(&self, entry_path: PathBuf) -> AppResult<()> {
        let path_str = entry_path.to_string_lossy().to_string();

        // The DB may hold the path in either Unicode form (accented names on macOS);
        // checking only one form would insert a duplicate asset.
        let nfc_path: String = path_str.nfc().collect();
        let nfd_path: String = path_str.nfd().collect();
        for candidate in [&path_str, &nfc_path, &nfd_path] {
            if let Ok(Some(_)) = self.query_handler.find_asset_by_path(candidate).await {
                debug!(
                    "Indexer: Asset already exists for path '{}', skipping duplicate creation",
                    entry_path.display()
                );
                return Ok(());
            }
        }

        let extension = entry_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        if !self.registry.is_supported_extension(extension) {
            return Ok(());
        }

        let (format_name, family_name) = self
            .registry
            .detect(&entry_path)
            .map(|format| (format.name.to_string(), format.type_category.to_string()))
            .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

        let metadata = std::fs::metadata(&entry_path).map_err(crate::core::error::AppError::Io)?;
        let size_bytes = metadata.len();
        let created_at = metadata.created().ok().map(DateTime::<Utc>::from);
        let modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);

        let folder_id = if let Some(parent) = entry_path.parent() {
            self.ensure_folder_hierarchy(parent).await?
        } else {
            None
        };

        let create_payload = CreateAssetPayload {
            path: entry_path,
            file_size: size_bytes,
            format_type: format_name,
            family: family_name,
            state_init: AssetState::Indexed,
            folder_id,
            created_at,
            modified_at,
        };

        self.ledger
            .execute(LedgerCommand::CreateAsset(create_payload))
            .await?;
        Ok(())
    }

    /// Handles a path deletion event.
    ///
    /// The debouncer has already applied a 3-second deletion guard before emitting
    /// `FsPathDeleted`. The indexer trusts this judgment and executes immediately,
    /// but maintains a `recent_removals` cache so that `handle_file_discovered`
    /// can still pair late renames/moves.
    async fn handle_path_deleted(&self, path: String) {
        let entry_path = PathBuf::from(&path);

        // A delete event for a path that is ledger-owned or exists again is stale
        // (e.g. a trashed file that was already restored) and must not touch the DB.
        if self.path_guard.is_marked(&entry_path) || entry_path.exists() {
            debug!("Indexer: Ignoring stale/ledger-initiated DELETE for {}", path);
            return;
        }
        info!("Indexer: Processing DELETE for: {}", path);

        let mut old_size = 0;
        let mut old_created_at: Option<DateTime<Utc>> = None;
        if let Ok(Some(asset)) = self.query_handler.find_asset_by_path(&path).await {
            old_size = asset.file_size;
            old_created_at = asset.created_at;
        }

        self.recent_removals
            .insert(entry_path.clone(), (Utc::now(), old_size, old_created_at));

        let delete_command = LedgerCommand::DeleteAsset {
            asset_id: None,
            path: Some(entry_path.clone()),
            physical_delete: false,
        };

        if let Err(error) = self.ledger.execute(delete_command).await {
            if !matches!(error, crate::core::error::AppError::NotFound(_)) {
                error!("Indexer: DELETE failed for {}: {}", path, error);
            }
        }

        let recent_removals_clone = self.recent_removals.clone();
        let entry_path_clone = entry_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            recent_removals_clone.remove(&entry_path_clone);
        });
    }

    /// Handles an explicit path rename, cleaning up rename caches for the source.
    ///
    /// For directories, this also handles the macOS "Pasta Sem Título" flow where
    /// the Finder creates a temporary folder and then renames it to the final name.
    /// If the `from` path is not found in the database, the rename is treated as
    /// a new folder creation with the `to` path.
    async fn handle_path_renamed(&self, from: String, to: String) {
        let from_path = PathBuf::from(&from);
        let to_path = PathBuf::from(&to);

        if self.path_guard.is_marked(&from_path) || self.path_guard.is_marked(&to_path) {
            debug!("Indexer: Ignoring ledger-initiated rename {} -> {}", from, to);
            return;
        }

        self.recent_removals.remove(&from_path);

        if to_path.is_dir() {
            self.handle_directory_rename(from_path, to_path).await;
        } else {
            let update_result = self
                .ledger
                .execute(LedgerCommand::UpdateAsset(
                    crate::core::ledger::command::UpdateAssetPayload {
                        asset_id: None,
                        old_path: Some(from_path.clone()),
                        new_path: to_path.clone(),
                    },
                ))
                .await;

            if let Err(crate::core::error::AppError::NotFound(_)) = update_result {
                info!(
                    "Indexer: Rename source '{}' not in DB — treating as new file creation for '{}'",
                    from_path.display(),
                    to_path.display()
                );
                self.handle_file_discovered(to_path.to_string_lossy().to_string())
                    .await;
            }
        }
    }

    /// Handles a directory rename, including the macOS new-folder creation flow.
    async fn handle_directory_rename(&self, from_path: PathBuf, to_path: PathBuf) {
        let found_folder = self
            .query_handler
            .find_folder_by_path(&from_path.to_string_lossy())
            .await
            .unwrap_or(None);

        if let Some(folder_id) = found_folder {
            let _ = self
                .ledger
                .execute(LedgerCommand::RenameFolder(
                    crate::core::ledger::command::RenameFolderPayload {
                        folder_id,
                        old_path: from_path,
                        new_path: to_path,
                    },
                ))
                .await;
        } else {
            info!(
                "Indexer: Rename target '{}' not in DB — treating as new folder creation for '{}'",
                from_path.display(),
                to_path.display()
            );

            let to_path_str = to_path.to_string_lossy().to_string();
            if self
                .query_handler
                .find_folder_by_path(&to_path_str)
                .await
                .unwrap_or(None)
                .is_none()
            {
                self.create_and_scan_folder(to_path).await;
            }
        }
    }

    /// Handles discovery of a new directory on the filesystem.
    ///
    /// Creates the folder in the database and immediately scans it
    /// to discover any files that were dragged or restored inside it.
    async fn handle_directory_discovered(&self, path: String) {
        let dir_path = PathBuf::from(&path);
        let dir_path_str = dir_path.to_string_lossy().to_string();

        if let Ok(Some(_)) = self.query_handler.find_folder_by_path(&dir_path_str).await {
            return;
        }

        self.create_and_scan_folder(dir_path).await;
    }

    /// Handles deletion of a directory from the filesystem.
    async fn handle_directory_deleted(&self, path: String) {
        if let Ok(Some(folder_id)) = self.query_handler.find_folder_by_path(&path).await {
            let _ = self
                .ledger
                .execute(LedgerCommand::RemoveFolder(
                    crate::core::ledger::command::RemoveFolderPayload { folder_id },
                ))
                .await;
        }
    }

    /// Creates a folder in the database and immediately scans it for contents.
    ///
    /// Resolves the parent folder ID from the database, creates the folder via
    /// the Ledger, then triggers a `scan_directory` using the newly created
    /// folder's ID (not the parent's) to ensure correct asset ownership.
    async fn create_and_scan_folder(&self, folder_path: PathBuf) {
        let parent_id = if let Some(parent) = folder_path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            self.query_handler
                .find_folder_by_path(&parent_str)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        let folder_name = folder_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let create_result = self
            .ledger
            .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
                parent_id: parent_id.clone(),
                name: folder_name,
                path: folder_path.clone(),
            }))
            .await;

        if let Ok(new_folder_asset) = create_result {
            let _ = self
                .scan_directory(folder_path, Some(new_folder_asset.id))
                .await;
        } else {
            let _ = self.scan_directory(folder_path, parent_id).await;
        }
    }

    /// Ensures that the complete folder hierarchy for a given path exists in the database.
    ///
    /// Recursively walks up the path, creating any missing folders along the way.
    /// Returns the ID of the leaf folder.
    ///
    /// # Errors
    ///
    /// Logs a warning and returns `Ok(None)` if a folder creation fails,
    /// allowing the caller to proceed without a folder ID.
    #[async_recursion]
    pub(crate) async fn ensure_folder_hierarchy(
        &self,
        path: &std::path::Path,
    ) -> AppResult<Option<String>> {
        let path_str = path.to_string_lossy().to_string();
        if let Some(id) = self.query_handler.find_folder_by_path(&path_str).await? {
            return Ok(Some(id));
        }

        let parent_id = if let Some(parent) = path.parent() {
            self.ensure_folder_hierarchy(parent).await?
        } else {
            None
        };

        let folder_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let create_folder_command = LedgerCommand::CreateFolder(CreateFolderPayload {
            parent_id,
            name: folder_name,
            path: path.to_path_buf(),
        });

        match self.ledger.execute(create_folder_command).await {
            Ok(folder_asset) => Ok(Some(folder_asset.id)),
            Err(error) => {
                warn!(
                    "Failed to ensure folder hierarchy for {:?}: {}",
                    path, error
                );
                Ok(None)
            }
        }
    }
}
