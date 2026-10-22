//! Core Library Indexer — parallel directory scanning and repair.
//!
//! The `LibraryIndexer` is the Application Layer service responsible for
//! maintaining the asset database in sync with the filesystem. It provides
//! two primary operations:
//!
//! - **`scan_directory`**: A parallel, differential scan using a fan-out
//!   producer-consumer pipeline with bounded concurrency.
//! - **`repair_library`**: A differential repair that fixes assets with
//!   missing format types or thumbnails.
//!
//! Real-time filesystem event handling lives in the sibling `event_handler` module,
//! and pure file classification logic lives in the `classifier` module.

use crate::core::error::AppResult;
use crate::core::events::AppEventBus;
use crate::core::ledger::command::{CreateAssetPayload, CreateFolderPayload, LedgerCommand};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::repository::AssetQueryHandler;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::task::JoinSet;
use tracing::{debug, error, info, instrument, warn};
use walkdir::WalkDir;

use super::classifier::{classify_file_entry, AssetDiscoveryResult};

/// Service responsible for indexing library folders using a parallel
/// producer-consumer pipeline.
///
/// Orchestrates full directory scans and differential repairs via the
/// Ledger (CQRS command side) and QueryHandler (read side). Maintains
/// a `recent_removals` cache shared with the `event_handler` module
/// to support implicit rename/move recovery.
pub struct LibraryIndexer {
    /// Port for read-only asset operations (differential cache).
    pub(crate) query_handler: Arc<dyn AssetQueryHandler>,
    /// Port for state mutations.
    pub(crate) ledger: Arc<dyn TransactionalAssetLedger>,
    /// Event bus for publishing domain events.
    pub(crate) event_bus: Arc<dyn AppEventBus>,
    /// The central format registry for file type detection.
    pub(crate) registry: Arc<crate::core::formats::registry::FormatRegistry>,
    /// Maximum number of concurrent file-processing tasks.
    pub(crate) concurrency_limit: usize,
    /// Cache of recently removed files metadata to support "Implicit Rename" recovery.
    ///
    /// Keeps track of removals for a short window so that a subsequent file discovery
    /// with matching size + created_at can be paired as a move instead of delete+create.
    /// Tuple: (removal_timestamp, file_size, created_at_from_db)
    pub(crate) recent_removals: Arc<DashMap<PathBuf, (DateTime<Utc>, u64, Option<DateTime<Utc>>)>>,
    /// Paths being moved by the Ledger itself (trash/restore); watcher events on them are ignored.
    pub(crate) path_guard: Arc<crate::core::path_guard::LedgerPathGuard>,
}

impl LibraryIndexer {
    /// Creates a new `LibraryIndexer` with default concurrency (200 tasks).
    ///
    /// # Arguments
    ///
    /// * `query_handler` - Read-only port for querying asset/folder state.
    /// * `ledger` - Transactional port for persisting state changes.
    /// * `event_bus` - Bus for publishing scan lifecycle events.
    /// * `registry` - Format registry for detecting supported file types.
    pub fn new(
        query_handler: Arc<dyn AssetQueryHandler>,
        ledger: Arc<dyn TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
        registry: Arc<crate::core::formats::registry::FormatRegistry>,
    ) -> Self {
        Self {
            query_handler,
            ledger,
            event_bus,
            registry,
            concurrency_limit: 200,
            recent_removals: Arc::new(DashMap::new()),
            path_guard: Arc::new(crate::core::path_guard::LedgerPathGuard::new()),
        }
    }

    /// Shares the Ledger's path guard so its own file moves are not re-indexed.
    pub fn with_path_guard(mut self, path_guard: Arc<crate::core::path_guard::LedgerPathGuard>) -> Self {
        self.path_guard = path_guard;
        self
    }

    /// Sets a custom concurrency limit, clamped to [10, 500].
    pub fn with_concurrency_limit(mut self, limit: usize) -> Self {
        self.concurrency_limit = limit.max(10).min(500);
        self
    }

    /// Performs a parallel differential scan of a directory tree.
    ///
    /// Pipeline:
    /// 1. Single `spawn_blocking` walk → collect all entries
    /// 2. Process folders hierarchically (sequential, before fan-out)
    /// 3. Load comparison cache for differential check
    /// 4. Fan-out file classification via `JoinSet` + `Semaphore`
    /// 5. Consumer serializes persistence via `Ledger.BatchCreate`
    /// 6. Prune stale assets and folders not found on disk
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` if the WalkDir thread panics.
    #[instrument(skip(self), fields(path = %path.display()))]
    pub async fn scan_directory(&self, path: PathBuf, folder_id: Option<String>) -> AppResult<()> {
        let scan_start_time = std::time::Instant::now();
        let root_str = path.to_string_lossy().to_string();
        info!(
            "▶ Scan STARTED for: {} (concurrency_limit={})",
            root_str, self.concurrency_limit
        );

        let current_root_id = if let Some(id) = folder_id {
            Some(id)
        } else {
            self.query_handler.find_folder_by_path(&root_str).await?
        };

        let _ = self
            .event_bus
            .publish(crate::core::events::DomainEvent::ScanStarted {
                library_id: current_root_id
                    .clone()
                    .unwrap_or_else(|| "root".to_string()),
            });

        let registry_for_walk = self.registry.clone();
        let walk_path = path.clone();
        let all_entries: Vec<walkdir::DirEntry> = tokio::task::spawn_blocking(move || {
            WalkDir::new(&walk_path)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .collect()
        })
        .await
        .map_err(|join_error| {
            crate::core::error::AppError::Internal(format!("WalkDir join failed: {}", join_error))
        })?;

        let mut directory_entries: Vec<walkdir::DirEntry> = Vec::new();
        let mut file_entries: Vec<walkdir::DirEntry> = Vec::new();

        for entry in all_entries {
            if entry.file_type().is_dir() {
                if entry.path() != path {
                    directory_entries.push(entry);
                }
            } else if entry.file_type().is_file() {
                let is_supported = entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| registry_for_walk.is_supported_extension(ext))
                    .unwrap_or(false);

                if is_supported {
                    file_entries.push(entry);
                }
            }
        }

        let total_files = file_entries.len();
        debug!(
            "Walk completed: {} supported files, {} subdirectories",
            total_files,
            directory_entries.len()
        );

        let folder_cache = self
            .build_folder_cache(&path, &current_root_id, &mut directory_entries)
            .await?;

        let comparison_cache = self
            .query_handler
            .get_all_files_comparison_data(&root_str)
            .await?;
        debug!(
            "Loaded {} entries from comparison cache",
            comparison_cache.len()
        );

        let verified_file_paths: std::collections::HashSet<String> = file_entries
            .iter()
            .map(|entry| entry.path().to_string_lossy().to_string())
            .collect();

        let verified_folder_paths: std::collections::HashSet<String> = directory_entries
            .iter()
            .map(|entry| entry.path().to_string_lossy().to_string())
            .collect();

        let (discovered_count, unchanged_count, error_count) = self
            .fanout_classify_and_persist(
                file_entries,
                &comparison_cache,
                &folder_cache,
                &current_root_id,
                total_files,
            )
            .await;

        let existing_folders = self.query_handler.list_all_subfolders().await?;
        let (pruned_files_count, pruned_folders_count) = self
            .prune_stale_entries(
                &root_str,
                &comparison_cache,
                &verified_file_paths,
                &existing_folders,
                &verified_folder_paths,
            )
            .await;

        let scan_duration = scan_start_time.elapsed();
        info!(
            "■ Scan COMPLETED for: {} — Discovered: {}, Unchanged: {}, Pruned: {} ({} files/{} dirs), Errors: {}, Duration: {:.2}s",
            root_str, discovered_count, unchanged_count, pruned_files_count + pruned_folders_count, pruned_files_count, pruned_folders_count, error_count, scan_duration.as_secs_f64()
        );

        let _ = self
            .event_bus
            .publish(crate::core::events::DomainEvent::ScanCompleted {
                library_id: current_root_id.unwrap_or_else(|| "root".to_string()),
            });

        Ok(())
    }

    /// Builds the folder hierarchy cache, creating missing folders in depth order.
    async fn build_folder_cache(
        &self,
        root_path: &PathBuf,
        current_root_id: &Option<String>,
        directory_entries: &mut Vec<walkdir::DirEntry>,
    ) -> AppResult<Arc<RwLock<HashMap<PathBuf, String>>>> {
        let existing_folders = self.query_handler.list_all_subfolders().await?;
        let folder_cache: Arc<RwLock<HashMap<PathBuf, String>>> = Arc::new(RwLock::new({
            let mut map = HashMap::new();
            for folder in &existing_folders {
                map.insert(folder.path.clone(), folder.id.clone());
            }
            if let Some(ref root_id) = current_root_id {
                map.insert(root_path.clone(), root_id.clone());
            }
            map
        }));

        directory_entries.sort_by_key(|entry| entry.path().components().count());

        for dir_entry in directory_entries.iter() {
            let dir_path = dir_entry.path().to_path_buf();
            let dir_path_str = dir_path.to_string_lossy().to_string();

            {
                let cache_read = folder_cache.read().await;
                if cache_read.contains_key(&dir_path) {
                    continue;
                }
            }

            if let Some(existing_id) = self
                .query_handler
                .find_folder_by_path(&dir_path_str)
                .await?
            {
                let mut cache_write = folder_cache.write().await;
                cache_write.insert(dir_path, existing_id);
                continue;
            }

            let parent_path = dir_path.parent().unwrap_or(root_path);
            let parent_id = {
                let cache_read = folder_cache.read().await;
                cache_read.get(parent_path).cloned()
            };

            let name = dir_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let create_folder_command = LedgerCommand::CreateFolder(CreateFolderPayload {
                parent_id,
                name,
                path: dir_path.clone(),
            });

            match self.ledger.execute(create_folder_command).await {
                Ok(folder_asset) => {
                    let mut cache_write = folder_cache.write().await;
                    cache_write.insert(dir_path, folder_asset.id);
                }
                Err(error) => {
                    warn!("Failed to create folder {:?}: {}", dir_path, error);
                }
            }
        }

        Ok(folder_cache)
    }

    /// Fan-out classification and batch persistence pipeline.
    ///
    /// Returns (discovered_count, unchanged_count, error_count).
    async fn fanout_classify_and_persist(
        &self,
        file_entries: Vec<walkdir::DirEntry>,
        comparison_cache: &HashMap<String, (i64, DateTime<Utc>)>,
        folder_cache: &Arc<RwLock<HashMap<PathBuf, String>>>,
        current_root_id: &Option<String>,
        total_files: usize,
    ) -> (u64, u64, u64) {
        let (result_sender, mut result_receiver) = mpsc::channel::<AssetDiscoveryResult>(2000);
        let mut producer_join_set = JoinSet::new();
        let concurrency_semaphore = Arc::new(Semaphore::new(self.concurrency_limit));

        let shared_comparison_cache = Arc::new(comparison_cache.clone());
        let shared_folder_cache = folder_cache.clone();
        let shared_registry = self.registry.clone();

        for file_entry in file_entries {
            let sender_clone = result_sender.clone();
            let semaphore_clone = concurrency_semaphore.clone();
            let cache_clone = shared_comparison_cache.clone();
            let folder_cache_clone = shared_folder_cache.clone();
            let registry_clone = shared_registry.clone();
            let root_id_clone = current_root_id.clone();

            producer_join_set.spawn(async move {
                let _permit = match semaphore_clone.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return,
                };

                let result = classify_file_entry(
                    &file_entry,
                    &cache_clone,
                    &folder_cache_clone,
                    &registry_clone,
                    &root_id_clone,
                )
                .await;

                let _ = sender_clone.send(result).await;
            });
        }
        drop(result_sender);

        let mut batch_payloads: Vec<CreateAssetPayload> = Vec::new();
        let mut discovered_count: u64 = 0;
        let mut unchanged_count: u64 = 0;
        let mut error_count: u64 = 0;
        let mut processed_count: u64 = 0;
        let batch_size: usize = 100;

        while let Some(discovery) = result_receiver.recv().await {
            processed_count += 1;

            match discovery {
                AssetDiscoveryResult::NewAsset(payload) => {
                    batch_payloads.push(payload);
                    discovered_count += 1;

                    if batch_payloads.len() >= batch_size {
                        if let Err(batch_error) = self
                            .ledger
                            .execute(LedgerCommand::BatchCreate(std::mem::take(
                                &mut batch_payloads,
                            )))
                            .await
                        {
                            error!("BatchCreate failed: {}", batch_error);
                        }
                    }
                }
                AssetDiscoveryResult::ExistingAsset => {
                    unchanged_count += 1;
                }
                AssetDiscoveryResult::Error(error_message) => {
                    warn!("Scan: skipping file: {}", error_message);
                    error_count += 1;
                }
            }

            if processed_count.is_multiple_of(100) || processed_count == total_files as u64 {
                let _ = self
                    .event_bus
                    .publish(crate::core::events::DomainEvent::ScanProgress {
                        total: total_files,
                        processed: processed_count as usize,
                        current_file: format!("{}/{}", processed_count, total_files),
                    });
            }
        }

        if !batch_payloads.is_empty() {
            if let Err(batch_error) = self
                .ledger
                .execute(LedgerCommand::BatchCreate(batch_payloads))
                .await
            {
                error!("Final BatchCreate failed: {}", batch_error);
            }
        }

        while producer_join_set.join_next().await.is_some() {}

        (discovered_count, unchanged_count, error_count)
    }

    /// Prunes stale assets and folders that exist in the database but not on disk.
    ///
    /// Returns (pruned_files_count, pruned_folders_count).
    async fn prune_stale_entries(
        &self,
        root_str: &str,
        comparison_cache: &HashMap<String, (i64, DateTime<Utc>)>,
        verified_file_paths: &std::collections::HashSet<String>,
        existing_folders: &[crate::core::models::asset::Folder],
        verified_folder_paths: &std::collections::HashSet<String>,
    ) -> (u64, u64) {
        let mut pruned_files_count: u64 = 0;
        let mut pruned_folders_count: u64 = 0;

        for cached_path in comparison_cache.keys() {
            if !verified_file_paths.contains(cached_path) {
                if let Err(error) = self
                    .ledger
                    .execute(LedgerCommand::DeleteAsset {
                        asset_id: None,
                        path: Some(PathBuf::from(cached_path)),
                        physical_delete: false,
                    })
                    .await
                {
                    warn!("Failed to prune stale file {}: {}", cached_path, error);
                } else {
                    pruned_files_count += 1;
                }
            }
        }

        for folder in existing_folders {
            let folder_path_str = folder.path.to_string_lossy().to_string();

            if folder_path_str.starts_with(root_str)
                && folder_path_str != root_str
                && !verified_folder_paths.contains(&folder_path_str)
            {
                if let Err(error) = self
                    .ledger
                    .execute(LedgerCommand::RemoveFolder(
                        crate::core::ledger::command::RemoveFolderPayload {
                            folder_id: folder.id.clone(),
                        },
                    ))
                    .await
                {
                    warn!(
                        "Failed to prune stale folder {}: {}",
                        folder_path_str, error
                    );
                } else {
                    pruned_folders_count += 1;
                }
            }
        }

        (pruned_files_count, pruned_folders_count)
    }

    /// Performs a differential repair of the library index.
    ///
    /// Finds assets with missing format types or thumbnails and updates them
    /// based on the format registry.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the query for assets needing repair fails.
    pub async fn repair_library(&self) -> AppResult<()> {
        info!("▶ Repair STARTED");

        let assets_needing_repair = self.query_handler.get_assets_needing_repair().await?;
        let count = assets_needing_repair.len();

        if count == 0 {
            info!("Repair COMPLETED: 0 assets needed repair.");
            return Ok(());
        }

        info!("Found {} assets needing repair.", count);

        for asset in assets_needing_repair {
            if asset.format_type == "unknown" {
                if let Some(supported_format) = self.registry.detect(&asset.path) {
                    let format_name = supported_format.name.to_string();
                    let _ = self
                        .ledger
                        .execute(LedgerCommand::UpdateFormat {
                            asset_id: asset.id.clone(),
                            format: format_name,
                        })
                        .await;
                }
            }

            if asset.thumbnail_path.is_none() {
                let _ = self
                    .ledger
                    .execute(LedgerCommand::RegenerateThumbnail {
                        asset_id: asset.id.clone(),
                    })
                    .await;
            }
        }

        info!("Repair COMPLETED: {} assets processed.", count);
        Ok(())
    }
}
