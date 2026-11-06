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
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

use super::indexer::LibraryIndexer;

#[cfg(test)]
#[path = "event_handler_tests.rs"]
mod tests;

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
        let mut repair_interval = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(600),
            std::time::Duration::from_secs(600),
        );
        tauri::async_runtime::spawn(async move {
            let mut last_reconciliation: Option<tokio::time::Instant> = None;
            info!("IndexerEventListener: started and listening for filesystem events");

            loop {
                tokio::select! {
                    recv_result = receiver.recv() => {
                        match recv_result {
                            Ok(event) => {
                                self.handle_single_event(event).await;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                                if last_reconciliation.is_none_or(|instant| instant.elapsed() >= std::time::Duration::from_secs(60)) { repair_interval.reset_immediately(); }
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
                    _ = repair_interval.tick() => {
                        self.reconcile_observed_roots(&token).await;
                        last_reconciliation = Some(tokio::time::Instant::now());
                    }
                    _ = token.cancelled() => {
                        info!("IndexerEventListener: cancelled, shutting down");
                        break;
                    }
                }
            }
        })
    }

    async fn reconcile_observed_roots(&self, token: &tokio_util::sync::CancellationToken) {
        let roots = match self.query_handler.list_folders(None).await {
            Ok(roots) => roots,
            Err(error) => {
                error!(%error, "Cannot reconcile library roots");
                return;
            }
        };
        for folder in roots {
            if token.is_cancelled() {
                break;
            }
            if !tokio::fs::try_exists(&folder.path).await.unwrap_or(false) {
                continue;
            }
            if let Err(error) = self.scan_directory(folder.path, Some(folder.id)).await {
                warn!(%error, "Library reconciliation will retry");
            }
        }
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
                match self.query_handler.find_folder_by_path(&path).await {
                    Ok(Some(_)) => self.handle_directory_deleted(path).await,
                    Ok(None) => self.handle_path_deleted(path).await,
                    Err(error) => error!(%error, "Cannot classify removal observation"),
                }
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

    /// Discovers files using native identity recovery in the transactional creation handler.
    async fn handle_file_discovered(&self, path: String) {
        if let Err(error) = self.handle_file_creation(PathBuf::from(&path)).await {
            error!(%path, %error, "File discovery failed; reconciliation can retry");
        }
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
        if !self.has_indexed_ancestor(&entry_path).await? {
            return Ok(());
        }
        let path_str = entry_path.to_string_lossy().to_string();

        if self
            .query_handler
            .find_asset_by_path(&path_str)
            .await?
            .is_some()
        {
            return Ok(());
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

        let metadata = tokio::fs::metadata(&entry_path).await?;
        let size_bytes = metadata.len();
        let created_at = metadata.created().ok().map(DateTime::<Utc>::from);
        let modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);

        let folder_id = if let Some(parent) = entry_path.parent() {
            self.ensure_folder_hierarchy(parent).await?
        } else {
            None
        };

        let create_payload = CreateAssetPayload {
            path: entry_path.clone(),
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

    /// Defers catalog removal so native-identity move recovery retains user metadata.
    async fn handle_path_deleted(&self, path: String) {
        let entry_path = PathBuf::from(&path);
        if tokio::fs::try_exists(&entry_path).await.unwrap_or(true) {
            return;
        }
        let asset = match self.query_handler.find_deleted_asset(&path).await {
            Ok(Some(asset)) => asset,
            Ok(None) => return,
            Err(error) => {
                error!(%error, %path, "Cannot resolve deleted asset");
                return;
            }
        };
        let generation = uuid::Uuid::new_v4();
        self.pending_removals.insert(entry_path.clone(), generation);
        let removals = self.pending_removals.clone();
        let ledger = self.ledger.clone();
        let queries = self.query_handler.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if removals
                .remove_if(&entry_path, |_, current| *current == generation)
                .is_none()
            {
                return;
            }
            if tokio::fs::try_exists(&entry_path).await.unwrap_or(true) {
                return;
            }
            if !observed_root_online(queries.as_ref(), asset.folder_id.clone())
                .await
                .unwrap_or(false)
            {
                return;
            }
            // A recovered rename has changed this row's path; never delete it by stale ID.
            match queries.find_deleted_asset(&path).await {
                Ok(Some(current)) if current.id == asset.id => {
                    if let Err(error) = ledger
                        .execute(LedgerCommand::DeleteAsset {
                            asset_id: Some(asset.id),
                            path: Some(asset.path.clone()),
                            physical_delete: false,
                        })
                        .await
                    {
                        error!(%error, %path, "Deferred deletion failed");
                    }
                }
                Err(error) => {
                    error!(%error, "Deferred deletion lookup failed");
                }
                _ => {}
            }
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

        self.pending_removals.remove(&from_path);

        if tokio::fs::metadata(&to_path)
            .await
            .is_ok_and(|metadata| metadata.is_dir())
        {
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

            if let Err(error) = update_result {
                warn!(%error, "Rename hint was not verified; reconciling separate observations");
                self.handle_path_deleted(from).await;
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
        let found_folder = match self
            .query_handler
            .find_folder_by_path(&from_path.to_string_lossy())
            .await
        {
            Ok(Some(identifier)) => Some(identifier),
            Ok(None) => match self
                .query_handler
                .find_missing_folder_at_destination(&to_path)
                .await
            {
                Ok(identifier) => identifier,
                Err(error) => {
                    error!(%error, "Cannot correlate directory rename");
                    return;
                }
            },
            Err(error) => {
                error!(%error, "Cannot resolve directory rename");
                return;
            }
        };

        if let Some(folder_id) = found_folder {
            let original = match self.query_handler.get_folder_by_id(&folder_id).await {
                Ok(Some(folder)) => folder.path,
                Ok(None) => return,
                Err(error) => {
                    error!(%error, "Cannot load directory rename source; reconciliation will retry");
                    return;
                }
            };
            if let Err(error) = self
                .ledger
                .execute(LedgerCommand::RenameFolder(
                    crate::core::ledger::command::RenameFolderPayload {
                        folder_id,
                        old_path: original,
                        new_path: to_path.clone(),
                    },
                ))
                .await
            {
                warn!(%error, "Directory rename was not verified; reconciling destination");
                self.handle_directory_discovered(to_path.to_string_lossy().into_owned())
                    .await;
            }
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

        match self.query_handler.find_folder_by_path(&dir_path_str).await {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => {
                error!(%error, "Cannot resolve discovered folder; reconciliation will retry");
                return;
            }
        }

        self.create_and_scan_folder(dir_path).await;
    }

    /// Handles deletion of a directory from the filesystem.
    async fn handle_directory_deleted(&self, path: String) {
        if tokio::fs::try_exists(&path).await.unwrap_or(true) {
            return;
        }
        if let Ok(Some(folder_id)) = self.query_handler.find_folder_by_path(&path).await {
            if self
                .query_handler
                .get_folder_by_id(&folder_id)
                .await
                .ok()
                .flatten()
                .is_none_or(|folder| folder.parent_id.is_none())
            {
                return;
            }
            let _ = self
                .ledger
                .execute(LedgerCommand::RemoveFolder(
                    crate::core::ledger::command::RemoveFolderPayload {
                        folder_id,
                        expected_path: Some(PathBuf::from(&path)),
                    },
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
        match self.has_indexed_ancestor(&folder_path).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                error!(%error, "Cannot resolve folder ownership; reconciliation will retry");
                return;
            }
        }
        let parent_id = if let Some(parent) = folder_path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            match self.query_handler.find_folder_by_path(&parent_str).await {
                Ok(parent) => parent,
                Err(error) => {
                    error!(%error, "Cannot resolve discovered folder parent; reconciliation will retry");
                    return;
                }
            }
        } else {
            None
        };

        if parent_id.is_none() {
            return;
        }
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

        match create_result {
            Ok(new_folder_asset) => {
                if let Err(error) = self
                    .scan_directory(folder_path, Some(new_folder_asset.id))
                    .await
                {
                    warn!(%error, "Discovered folder scan will retry through reconciliation");
                }
            }
            Err(error) => {
                error!(%error, "Cannot register discovered folder; reconciliation will retry");
            }
        }
    }

    async fn has_indexed_ancestor(&self, path: &std::path::Path) -> AppResult<bool> {
        for ancestor in path.ancestors() {
            if self
                .query_handler
                .find_folder_by_path(&ancestor.to_string_lossy())
                .await?
                .is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Ensures the observed hierarchy belongs to an indexed ancestor before creating missing nodes.
    ///
    /// Walking ancestors first avoids recursive future boxing and keeps parent-before-child writes.
    /// The returned ID identifies the requested leaf, including an already indexed folder.
    ///
    /// # Errors
    /// Returns missing ownership or persistence failures instead of creating unscoped assets.
    pub(crate) async fn ensure_folder_hierarchy(
        &self,
        path: &std::path::Path,
    ) -> AppResult<Option<String>> {
        let mut missing_folders = Vec::new();
        let mut indexed_ancestor = None;
        for ancestor in path.ancestors() {
            if let Some(identifier) = self
                .query_handler
                .find_folder_by_path(&ancestor.to_string_lossy())
                .await?
            {
                indexed_ancestor = Some(identifier);
                break;
            }
            missing_folders.push(ancestor.to_path_buf());
        }
        let mut parent_id = indexed_ancestor.ok_or_else(|| {
            crate::core::error::AppError::NotFound(
                "No indexed ancestor remains for this observation".into(),
            )
        })?;
        for folder_path in missing_folders.into_iter().rev() {
            let folder_name = folder_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let command = LedgerCommand::CreateFolder(CreateFolderPayload {
                parent_id: Some(parent_id),
                name: folder_name,
                path: folder_path.clone(),
            });
            match self.ledger.execute(command).await {
                Ok(folder) => parent_id = folder.id,
                Err(error) => {
                    warn!(%error, path = %folder_path.display(), "Failed to ensure folder hierarchy");
                    return Err(error);
                }
            }
        }
        Ok(Some(parent_id))
    }
}

async fn observed_root_online(
    queries: &dyn crate::core::repository::AssetQueryHandler,
    mut identifier: Option<String>,
) -> AppResult<bool> {
    let mut visited = std::collections::HashSet::new();
    while let Some(current) = identifier {
        if !visited.insert(current.clone()) {
            return Ok(false);
        }
        let Some(folder) = queries.get_folder_by_id(&current).await? else {
            return Ok(false);
        };
        if folder.parent_id.is_none() {
            return Ok(tokio::fs::try_exists(folder.path).await?);
        }
        identifier = folder.parent_id;
    }
    Ok(false)
}
