use crate::core::ledger::command::{
    BatchTagsPayload, CreateFolderPayload, CreateSmartFolderPayload, CreateTagPayload,
    DeleteSmartFolderPayload, LedgerCommand, UpdateAssetNotesPayload, UpdateAssetRatingPayload,
    UpdateSmartFolderPayload, UpdateTagPayload, UpdateTagsPayload,
};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::models::{Asset, Tag};
use crate::feature::assets::queries::AssetQueryService;
use crate::feature::library::indexer::LibraryIndexer;
use crate::processing::watcher::WatcherService;
use std::sync::Arc;
use tauri::{Manager, State};

use crate::core::error::AppResult;

/// RPC Command to create a new logical folder.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `payload` - The folder payload.
///
/// # Returns
///
/// The created folder.
#[tauri::command]
pub async fn create_folder(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: CreateFolderPayload,
) -> AppResult<Asset> {
    ledger.execute(LedgerCommand::CreateFolder(payload)).await
}

/// RPC Command to move an asset to a folder.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The asset ID.
/// * `folder_id` - The folder ID.
///
/// # Returns
///
/// The updated asset.
#[tauri::command]
pub async fn set_asset_folder(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
    folder_id: Option<String>,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::SetAssetFolder {
            asset_id,
            folder_id,
        })
        .await
}

/// RPC Command to update asset tags.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `payload` - The tag payload.
///
/// # Returns
///
/// The updated asset.
#[tauri::command]
pub async fn update_asset_tags(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: UpdateTagsPayload,
) -> AppResult<Asset> {
    ledger.execute(LedgerCommand::UpdateTags(payload)).await
}

/// RPC Command to create a new taxonomy tag.
///
/// Creates the tag via the Ledger (mutation) and then queries it back
/// to return the full Tag entity with generated UUID.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `service` - The asset query service for reading back the created tag.
/// * `name` - The display name of the tag (must be unique).
/// * `parent_id` - Optional parent tag ID for hierarchical organization.
/// * `color` - Optional hex color code for the tag.
///
/// # Errors
///
/// Returns `AppError` if the tag name is not unique or the DB operation fails.
#[tauri::command]
pub async fn create_tag(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    service: State<'_, AssetQueryService>,
    name: String,
    parent_id: Option<String>,
    color: Option<String>,
) -> AppResult<Tag> {
    let result = ledger
        .execute(LedgerCommand::CreateTag(CreateTagPayload {
            name,
            parent_id,
            color,
        }))
        .await?;

    // The Ledger returns a dummy Asset with the tag_id in the id field.
    // Query the actual tag from the database to return the real Tag entity.
    let all_tags: Vec<Tag> = service.list_tags().await?;
    let created_tag = all_tags
        .into_iter()
        .find(|tag| tag.id == result.id)
        .ok_or_else(|| {
            crate::core::error::AppError::Internal(
                "Tag was created but could not be read back".to_string(),
            )
        })?;

    Ok(created_tag)
}

/// RPC Command to update an existing tag's properties.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `id` - The unique identifier of the tag.
/// * `name` - Optional new display name.
/// * `color` - Optional new hex color code.
/// * `parent_id` - Optional new parent tag ID.
/// * `order_index` - Optional new sorting order index.
///
/// # Errors
///
/// Returns `AppError` if the tag doesn't exist or the DB operation fails.
#[tauri::command]
pub async fn update_tag(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    id: String,
    name: Option<String>,
    color: Option<String>,
    parent_id: Option<String>,
    order_index: Option<i64>,
) -> AppResult<()> {
    ledger
        .execute(LedgerCommand::UpdateTag(UpdateTagPayload {
            id,
            name,
            color,
            parent_id,
            order_index,
        }))
        .await?;
    Ok(())
}

/// RPC Command to delete a tag, removing it from all assets.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `id` - The unique identifier of the tag to delete.
///
/// # Errors
///
/// Returns `AppError` if the DB operation fails.
#[tauri::command]
pub async fn delete_tag(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    id: String,
) -> AppResult<()> {
    ledger.execute(LedgerCommand::DeleteTag { id }).await?;
    Ok(())
}

/// RPC Command to associate multiple tags with multiple assets in a single transaction.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `asset_ids` - The asset IDs to tag.
/// * `tag_ids` - The tag IDs to apply.
///
/// # Errors
///
/// Returns `AppError` if the DB operation fails.
#[tauri::command]
pub async fn add_tags_to_assets_batch(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: BatchTagsPayload,
) -> AppResult<()> {
    ledger
        .execute(LedgerCommand::AddTagsToAssetsBatch(payload))
        .await?;
    Ok(())
}

/// RPC Command to remove specific tags from multiple assets in a single transaction.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `asset_ids` - The asset IDs to untag.
/// * `tag_ids` - The tag IDs to remove.
///
/// # Errors
///
/// Returns `AppError` if the DB operation fails.
#[tauri::command]
pub async fn remove_tags_from_assets_batch(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: BatchTagsPayload,
) -> AppResult<()> {
    ledger
        .execute(LedgerCommand::RemoveTagsFromAssetsBatch(payload))
        .await?;
    Ok(())
}

/// RPC Command to replace all tags for multiple assets with a new set.
///
/// # Arguments
///
/// * `ledger` - The transactional asset ledger.
/// * `asset_ids` - The asset IDs to update.
/// * `tag_ids` - The new set of tag IDs.
///
/// # Errors
///
/// Returns `AppError` if the DB operation fails.
#[tauri::command]
pub async fn replace_tags_for_assets_batch(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: BatchTagsPayload,
) -> AppResult<()> {
    ledger
        .execute(LedgerCommand::ReplaceTagsForAssetsBatch(payload))
        .await?;
    Ok(())
}

/// RPC Command to remove a location/folder.
///
/// # Arguments
///
/// * `folder_id` - The folder ID to remove.
///
/// # Errors
///
/// Returns `AppError` if the DB operation fails.
#[tauri::command]
pub async fn remove_location(
    folder_id: String,
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    queries: State<'_, AssetQueryService>,
    watcher: State<'_, Arc<WatcherService>>,
) -> AppResult<()> {
    // 1. Get thumbnails before delete
    let thumbnails: Vec<String> = queries
        .get_folder_thumbnails(&folder_id)
        .await
        .unwrap_or_default();

    // 2. Ledger operation
    let result = ledger
        .execute(LedgerCommand::RemoveFolder(
            crate::core::ledger::command::RemoveFolderPayload {
                folder_id: folder_id.clone(),
            },
        ))
        .await?;

    // 3. Cleanup Watcher
    let path = result.path;
    let _ = watcher.unwatch(path).await;

    // 4. Cleanup thumbnails
    for thumb in thumbnails {
        let _ = tokio::fs::remove_file(&thumb).await;
    }

    Ok(())
}

/// RPC Command to manually start indexing a location.
///
/// Also registers a filesystem watcher for the path, restoring V1 behavior
/// where `run_scan` always called `start_watcher` at the end.
///
/// # Arguments
///
/// * `path` - The path to index.
/// * `indexer` - The library indexer.
/// * `watcher` - The watcher service.
/// * `app_handle` - The Tauri app handle for lifecycle access.
///
/// # Errors
///
/// Returns `AppError` if the indexing fails.
#[tauri::command]
pub async fn start_indexing(
    path: String,
    folder_id: Option<String>,
    indexer: State<'_, Arc<LibraryIndexer>>,
    watcher: State<'_, Arc<WatcherService>>,
    app_handle: tauri::AppHandle,
) -> AppResult<()> {
    let path_buf = std::path::PathBuf::from(&path);

    // 1. Register a filesystem watcher for this path
    let lifecycle = app_handle
        .try_state::<std::sync::Arc<crate::lifecycle::LifecycleRegistry>>()
        .ok_or_else(|| {
            crate::core::error::AppError::Internal("Lifecycle not initialized".to_string())
        })?;

    let watcher_token = lifecycle.child_token();
    if let Err(watcher_error) = watcher.watch(path_buf.clone(), watcher_token.clone()).await {
        tracing::error!(
            "Failed to start watcher for {}: {}",
            path_buf.display(),
            watcher_error
        );
    } else {
        lifecycle.register(
            format!("watcher:{}", path_buf.display()),
            watcher_token,
            tauri::async_runtime::spawn(async {}),
        );
        tracing::info!("Watcher registered for: {}", path_buf.display());
    }

    // 2. Start indexing in background
    let indexer_ref = indexer.inner().clone();
    tracing::info!(
        "Started indexing for: {} component=tauriService",
        path_buf.display()
    );
    tauri::async_runtime::spawn(async move {
        let _ = indexer_ref.scan_directory(path_buf, folder_id).await;
    });

    Ok(())
}

/// RPC Command to save a new smart folder.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `name` - The smart folder display name.
/// * `query` - The JSON search query.
///
/// # Returns
///
/// The created smart folder as an Asset placeholder.
#[tauri::command]
pub async fn save_smart_folder(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    name: String,
    query: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::CreateSmartFolder(CreateSmartFolderPayload {
            name,
            query_json: query,
        }))
        .await
}

/// RPC Command to update an existing smart folder.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `id` - The smart folder ID.
/// * `name` - The updated name.
/// * `query` - The updated JSON search query.
///
/// # Returns
///
/// The updated smart folder as an Asset placeholder.
#[tauri::command]
pub async fn update_smart_folder(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    id: String,
    name: String,
    query: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::UpdateSmartFolder(UpdateSmartFolderPayload {
            id,
            name,
            query_json: query,
        }))
        .await
}

/// RPC Command to delete a smart folder.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `id` - The ID of the smart folder to delete.
///
/// # Returns
///
/// A tombstone placeholder Asset.
#[tauri::command]
pub async fn delete_smart_folder(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::DeleteSmartFolder(DeleteSmartFolderPayload {
            id,
        }))
        .await
}

/// RPC Command to update an asset's rating.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `payload` - The payload containing the asset ID and rating.
///
/// # Returns
///
/// The updated asset as an Asset placeholder.
#[tauri::command]
pub async fn update_asset_rating(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: UpdateAssetRatingPayload,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::UpdateAssetRating(payload))
        .await
}

/// RPC Command to update an asset's notes.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `payload` - The payload containing the asset ID and notes.
///
/// # Returns
///
/// The updated asset as an Asset placeholder.
#[tauri::command]
pub async fn update_asset_notes(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    payload: UpdateAssetNotesPayload,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::UpdateAssetNotes(payload))
        .await
}

/// RPC Command to trigger re-extraction of asset colors.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The ID of the asset to re-extract colors from.
///
/// # Returns
///
/// The updated asset as an Asset placeholder.
#[tauri::command]
pub async fn reextract_asset_colors(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::ReextractColors { asset_id })
        .await
}

/// RPC Command to request regeneration of thumbnails for an asset.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The ID of the asset to regenerate thumbnails for.
///
/// # Returns
///
/// The updated asset as an Asset placeholder.
#[tauri::command]
pub async fn request_thumbnail_regenerate(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::RegenerateThumbnail { asset_id })
        .await
}

/// RPC Command to run database maintenance (VACUUM/ANALYZE).
///
/// # Arguments
///
/// * `pool_manager` - The database manager.
///
/// # Returns
///
/// An empty result.
#[tauri::command]
pub async fn run_db_maintenance(
    pool_manager: State<'_, Arc<crate::infra::database::manager::DbManager>>,
) -> AppResult<()> {
    pool_manager.run_maintenance().await
}

/// RPC Command to send a telemetry log from the frontend.
///
/// # Arguments
///
/// * `level` - The log level.
/// * `component` - The component name.
/// * `message` - The log message.
///
/// # Returns
///
/// An empty result.
#[tauri::command]
pub fn send_telemetry_log(level: String, component: String, message: String) {
    match level.to_lowercase().as_str() {
        "error" => tracing::error!(component = %component, "{}", message),
        "warn" => tracing::warn!(component = %component, "{}", message),
        "info" => tracing::info!(component = %component, "{}", message),
        "debug" => tracing::debug!(component = %component, "{}", message),
        _ => tracing::info!(component = %component, "[{}] {}", level, message),
    }
}

/// RPC Command to clean up old cache entries.
#[tauri::command]
pub async fn cleanup_cache(handle: tauri::AppHandle) -> AppResult<()> {
    let app_data = handle
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::core::error::AppError::Generic(e.to_string()))?;

    let hls_dir = app_data.join("hls");
    if hls_dir.exists() {
        // Simple implementation: remove HLS directory
        let _ = tokio::fs::remove_dir_all(&hls_dir).await;
        let _ = tokio::fs::create_dir_all(&hls_dir).await;
    }
    Ok(())
}

/// RPC Command to clear the entire cache (thumbnails and HLS).
#[tauri::command]
pub async fn clear_cache(handle: tauri::AppHandle) -> AppResult<()> {
    let app_data = handle
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::core::error::AppError::Generic(e.to_string()))?;

    let thumb_dir = app_data.join("thumbnails");
    let hls_dir = app_data.join("hls");

    if thumb_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&thumb_dir).await;
        let _ = tokio::fs::create_dir_all(&thumb_dir).await;
    }
    if hls_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&hls_dir).await;
        let _ = tokio::fs::create_dir_all(&hls_dir).await;
    }
    Ok(())
}

/// RPC Command to verify all thumbnails in the cache and delete corrupted ones.
#[tauri::command]
pub async fn verify_thumbnails(handle: tauri::AppHandle) -> AppResult<usize> {
    let app_data = handle
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::core::error::AppError::Generic(e.to_string()))?;

    let thumb_dir = app_data.join("thumbnails");
    if !thumb_dir.exists() {
        return Ok(0);
    }

    let mut corrupted_count = 0;
    let mut dir = tokio::fs::read_dir(thumb_dir)
        .await
        .map_err(crate::core::error::AppError::Io)?;

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Ok(bytes) = tokio::fs::read(&path).await {
                if !crate::processing::media::image_utils::is_valid_image(&bytes) {
                    if let Err(e) = tokio::fs::remove_file(&path).await {
                        tracing::error!("Failed to delete corrupted thumbnail {:?}: {}", path, e);
                    } else {
                        corrupted_count += 1;
                    }
                }
            }
        }
    }

    if corrupted_count > 0 {
        tracing::info!(
            "Maintenance: Deleted {} corrupted thumbnails",
            corrupted_count
        );
    }

    Ok(corrupted_count)
}

/// RPC Command to copy files to the system clipboard (MacOS).
///
/// # Arguments
///
/// * `paths` - A list of file paths to copy to the clipboard.
#[tauri::command]
pub fn copy_files_to_clipboard(paths: Vec<String>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if paths.is_empty() {
            return Ok(());
        }
        let script = String::from("ObjC.import('AppKit');\nvar pb = $.NSPasteboard.generalPasteboard;\npb.clearContents;\nfunction run(argv) {\nvar urls = $.NSMutableArray.alloc.init;\nfor (var i = 0; i < argv.length; i++) {\nurls.addObject($.NSURL.fileURLWithPath(argv[i]));\n}\npb.writeObjects(urls);\n}");

        let mut cmd = std::process::Command::new("osascript");
        cmd.arg("-l").arg("JavaScript").arg("-e").arg(&script);
        for path in paths.iter() {
            cmd.arg(path);
        }

        cmd.output().map_err(|e| e.to_string())?;
    }

    // For Windows/Linux, copying actual files to clipboard natively requires extra crates like `arboard`.
    // We just do MacOS as Mundam is primarily MacOS currently.

    Ok(())
}

/// RPC Command to rename a physical file directly.
/// This bypasses Tauri's frontend FS scope limitations.
#[tauri::command]
pub async fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    tokio::fs::rename(old_path, new_path)
        .await
        .map_err(|e| e.to_string())
}

/// RPC Command to toggle the favorite status of an asset.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The ID of the asset.
///
/// # Returns
///
/// The updated asset as an Asset placeholder.
#[tauri::command]
pub async fn toggle_favorite(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::ToggleFavorite(
            crate::core::ledger::command::ToggleFavoritePayload { asset_id },
        ))
        .await
}

/// RPC Command to move an asset to the trash.
///
/// Dispatches a `MoveToTrash` command to the Asset Ledger, which handles both
/// the database soft-delete (setting `deleted_at`) and the physical file move
/// to the trash directory via its Saga post-commit step. This eliminates
/// the previously duplicated filesystem logic from the delivery layer.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The ID of the asset to trash.
///
/// # Errors
///
/// Returns `AppError::NotFound` if the asset does not exist.
#[tauri::command]
pub async fn move_to_trash(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::MoveToTrash(
            crate::core::ledger::command::MoveToTrashPayload {
                asset_id,
            },
        ))
        .await
}

/// RPC Command to restore an asset from the trash.
///
/// Dispatches a `RestoreFromTrash` command to the Asset Ledger, which handles
/// both the database restoration (clearing `deleted_at`) and the physical file
/// move from the trash directory back to its original path via its Saga
/// post-commit step.
///
/// # Arguments
///
/// * `ledger` - The asset ledger.
/// * `asset_id` - The ID of the asset to restore.
///
/// # Errors
///
/// Returns `AppError::NotFound` if the asset does not exist.
#[tauri::command]
pub async fn restore_from_trash(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    asset_id: String,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::RestoreFromTrash(
            crate::core::ledger::command::RestoreFromTrashPayload {
                asset_id,
            },
        ))
        .await
}

/// RPC Command to permanently delete all items in the trash.
///
/// Resolves the physical trash path for each trashed asset and dispatches
/// a `DeleteAsset` command with `physical_delete: true` to the Ledger.
/// The Ledger's Saga post-commit step handles the physical file removal,
/// eliminating any duplicated filesystem logic from the delivery layer.
///
/// # Arguments
///
/// * `app_handle` - The Tauri app handle.
/// * `ledger` - The asset ledger.
/// * `pool_manager` - The database connection pool manager.
///
/// # Returns
///
/// The number of items successfully deleted.
#[tauri::command]
pub async fn empty_trash(
    app_handle: tauri::AppHandle,
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    pool_manager: State<'_, Arc<crate::infra::database::manager::DbManager>>,
) -> AppResult<usize> {
    use tauri::Manager;
    let pool = pool_manager.pool();

    let trashed = sqlx::query!(
        r#"SELECT id as "id!", path as "path!", deleted_at as "deleted_at?: chrono::DateTime<chrono::Utc>" FROM assets WHERE deleted_at IS NOT NULL"#
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| crate::core::error::AppError::Database(database_error))?;

    let app_directories = app_handle.state::<crate::bootstrap::AppDirectories>();
    let mut deleted_count = 0;

    for record in trashed {
        let original_path = std::path::PathBuf::from(&record.path);

        // Resolve the physical trash path so the Saga deletes the correct file.
        // Tries timestamped format first, then falls back to legacy format.
        let resolved_trash_path = crate::core::trash::resolve_trash_path(
            &app_directories.app_data,
            &record.id,
            &original_path,
            record.deleted_at.as_ref(),
        );

        let _ = ledger.execute(LedgerCommand::DeleteAsset {
            asset_id: Some(record.id.clone()),
            path: Some(resolved_trash_path),
            physical_delete: true,
        }).await;
        deleted_count += 1;
    }

    Ok(deleted_count)
}
