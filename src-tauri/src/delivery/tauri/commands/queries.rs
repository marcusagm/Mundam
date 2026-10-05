use crate::core::formats::{FormatRegistry, SupportedFormat};
use crate::core::models::{
    Asset, AssetColor, AssetFilter, Folder, LibraryStats, PageParams, PaginatedAssetsDto,
    SmartFolder, Tag,
};
use crate::feature::assets::queries::AssetQueryService;
use std::sync::Arc;
use tauri::{Manager, State};

use crate::core::error::AppResult;

/// Holds the session token used to authenticate streaming server requests.
pub struct StreamingSessionToken(pub String);

/// Returns the streaming session token to the frontend.
#[tauri::command]
pub fn get_streaming_token(token_state: State<'_, StreamingSessionToken>) -> String {
    token_state.0.clone()
}

/// RPC Command to list assets with filters and pagination.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `filter` - The filter to apply to the assets.
/// * `page` - The pagination parameters.
///
/// # Returns
///
/// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
/// * `Err(AppError)` if the assets could not be found.
#[tauri::command]
pub async fn get_assets(
    service: State<'_, AssetQueryService>,
    filter: AssetFilter,
    page: PageParams,
) -> AppResult<PaginatedAssetsDto> {
    service.list_assets(filter, page).await
}

/// RPC Command to get a single asset with full metadata.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `id` - The ID of the asset to retrieve.
///
/// # Returns
///
/// * `Ok(Option<Asset>)` if the asset was found successfully.
/// * `Err(AppError)` if the asset could not be found.
#[tauri::command]
pub async fn get_asset(
    service: State<'_, AssetQueryService>,
    id: String,
) -> AppResult<Option<Asset>> {
    service.get_asset(&id).await
}

/// RPC Command to retrieve a list of assets by their unique identifiers in a single batch query.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `asset_ids` - The list of unique asset identifiers to retrieve.
///
/// # Returns
///
/// * `Ok(Vec<Asset>)` containing the assets found.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_assets_by_ids(
    service: State<'_, AssetQueryService>,
    asset_ids: Vec<String>,
) -> AppResult<Vec<Asset>> {
    service.get_assets_by_ids(&asset_ids).await
}

/// RPC Command to retrieve tags associated with multiple assets in a single batch query.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `asset_ids` - The list of unique asset identifiers whose tags to retrieve.
///
/// # Returns
///
/// * `Ok(HashMap<String, Vec<Tag>>)` mapping each asset ID to its list of tags.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_tags_for_assets(
    service: State<'_, AssetQueryService>,
    asset_ids: Vec<String>,
) -> AppResult<std::collections::HashMap<String, Vec<Tag>>> {
    service.get_tags_for_assets(&asset_ids).await
}

/// RPC Command to list folders under a parent.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `parent_id` - The ID of the parent folder.
///
/// # Returns
///
/// * `Ok(Vec<Folder>)` if the folders were found successfully.
/// * `Err(AppError)` if the folders could not be found.
#[tauri::command]
pub async fn list_folders(
    service: State<'_, AssetQueryService>,
    parent_id: Option<String>,
) -> AppResult<Vec<Folder>> {
    service.list_folders(parent_id).await
}

/// RPC Command to list all tags.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(Vec<Tag>)` if the tags were found successfully.
/// * `Err(AppError)` if the tags could not be found.
#[tauri::command]
pub async fn list_tags(service: State<'_, AssetQueryService>) -> AppResult<Vec<Tag>> {
    service.list_tags().await
}

/// RPC Command to perform an advanced search.
///
/// # Arguments
///
/// * `service` - The search query handler.
/// * `criteria` - The advanced search criteria.
/// * `page` - The pagination parameters.
///
/// # Returns
///
/// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
/// * `Err(AppError)` if the search fails.
#[tauri::command]
pub async fn search_assets(
    service: State<'_, crate::feature::search::SearchQueryHandler>,
    criteria: crate::core::models::SearchCriteria,
    page: PageParams,
) -> AppResult<PaginatedAssetsDto> {
    service.search(criteria, page).await
}

/// RPC Command to get all tags associated with a specific asset.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `asset_id` - The unique identifier of the asset.
///
/// # Returns
///
/// * `Ok(Vec<Tag>)` if the tags were found successfully.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_tags_for_asset(
    service: State<'_, AssetQueryService>,
    asset_id: String,
) -> AppResult<Vec<Tag>> {
    service.get_tags_for_asset(&asset_id).await
}

/// RPC Command to get all subfolders.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(Vec<Folder>)` if the folders were found successfully.
/// * `Err(AppError)` if the folders could not be found.
#[tauri::command]
pub async fn get_all_subfolders(service: State<'_, AssetQueryService>) -> AppResult<Vec<Folder>> {
    service.list_all_subfolders().await
}

/// RPC Command to get subfolder asset counts.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
/// * `Err(AppError)` if the counts could not be found.
#[tauri::command]
pub async fn get_subfolder_counts(
    service: State<'_, AssetQueryService>,
) -> AppResult<Vec<(String, i64)>> {
    service.get_subfolder_asset_counts().await
}

/// RPC Command to get root location counts.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
/// * `Err(AppError)` if the counts could not be found.
#[tauri::command]
pub async fn get_location_root_counts(
    service: State<'_, AssetQueryService>,
) -> AppResult<Vec<(String, i64)>> {
    service.get_location_root_counts().await
}

/// RPC Command to get all smart folders.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(Vec<SmartFolder>)` if the smart folders were found successfully.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_smart_folders(
    service: State<'_, AssetQueryService>,
) -> AppResult<Vec<SmartFolder>> {
    service.list_smart_folders().await
}

/// RPC Command to get the total number of assets matching a filter.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `filter` - The asset filter parameters.
///
/// # Returns
///
/// * `Ok(i64)` count of assets.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_asset_count_filtered(
    service: State<'_, AssetQueryService>,
    filter: AssetFilter,
) -> AppResult<i64> {
    service.get_asset_count(filter).await
}

/// RPC Command to get library wide statistics.
///
/// # Arguments
///
/// * `service` - The asset query service.
///
/// # Returns
///
/// * `Ok(LibraryStats)` object containing aggregates.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_library_stats(service: State<'_, AssetQueryService>) -> AppResult<LibraryStats> {
    service.get_library_stats().await
}

/// RPC Command to get all colors extracted for a specific asset.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `asset_id` - The ID of the asset to get colors for.
///
/// # Returns
///
/// * `Ok(Vec<AssetColor>)` if the colors were found successfully.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_asset_colors(
    service: State<'_, AssetQueryService>,
    asset_id: String,
) -> AppResult<Vec<AssetColor>> {
    service.get_asset_colors(&asset_id).await
}

/// RPC Command to get advanced EXIF/technical metadata for an asset.
///
/// # Arguments
///
/// * `service` - The asset query service.
/// * `registry` - The format registry.
/// * `asset_id` - The ID of the asset to get metadata for.
///
/// # Returns
///
/// * `Ok(serde_json::Value)` if the metadata was found successfully.
/// * `Err(AppError)` if the query fails.
#[tauri::command]
pub async fn get_asset_exif(
    app_handle: tauri::AppHandle,
    service: State<'_, AssetQueryService>,
    registry: State<'_, Arc<FormatRegistry>>,
    asset_id: Option<String>,
    path: Option<String>,
) -> AppResult<serde_json::Value> {
    let final_path = if let Some(id) = asset_id {
        let asset: Asset = service.get_asset(&id).await?.ok_or_else(|| {
            crate::core::error::AppError::NotFound(format!("Asset {} not found", id))
        })?;
        if let Ok(app_data_directory) = app_handle.path().app_local_data_dir() {
            crate::core::trash::resolve_physical_path(&asset, &app_data_directory)
        } else {
            asset.path
        }
    } else if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        return Err(crate::core::error::AppError::Generic(
            "Either assetId or path must be provided".to_string(),
        ));
    };

    let provider = registry.resolve(&final_path, &[]).ok_or_else(|| {
        crate::core::error::AppError::UnsupportedFormat(format!(
            "No provider found for path {:?}",
            final_path
        ))
    })?;

    let metadata_cap = provider.metadata().ok_or_else(|| {
        crate::core::error::AppError::Generic(format!(
            "Provider for {:?} does not support metadata extraction",
            final_path
        ))
    })?;

    metadata_cap.extract_technical(&final_path).await
}

/// RPC Command to get statistics about the transcoding and thumbnail cache.
#[tauri::command]
pub async fn get_library_cache_stats(handle: tauri::AppHandle) -> AppResult<serde_json::Value> {
    let app_data = handle
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::core::error::AppError::Generic(e.to_string()))?;

    let thumb_dir = app_data.join("thumbnails");
    let hls_dir = app_data.join("hls");

    async fn get_dir_stats(dir: &std::path::Path) -> (u64, u64) {
        let mut count = 0;
        let mut size = 0;
        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        count += 1;
                        size += metadata.len();
                    } else if metadata.is_dir() {
                        let (c, s) = Box::pin(get_dir_stats(&entry.path())).await;
                        count += c;
                        size += s;
                    }
                }
            }
        }
        (count, size)
    }

    let (thumb_count, thumb_size) = get_dir_stats(&thumb_dir).await;
    let (hls_count, hls_size) = get_dir_stats(&hls_dir).await;

    Ok(serde_json::json!({
        "thumbnails": {
            "count": thumb_count,
            "size": thumb_size,
        },
        "hls": {
            "count": hls_count,
            "size": hls_size,
        },
        "total": {
            "count": thumb_count + hls_count,
            "size": thumb_size + hls_size,
        }
    }))
}

/// RPC Command to get all supported file formats.
#[tauri::command]
pub fn get_library_supported_formats(
    registry: State<'_, Arc<FormatRegistry>>,
) -> Vec<SupportedFormat> {
    registry.get_supported_formats()
}

/// RPC Command to get audio waveform data for a file.
///
/// If `asset_id` is provided the command will look up the asset record and,
/// when the asset is in the Mundam trash (`deleted_at IS NOT NULL`), it
/// resolves the physical path to the internal `app_data/trash/` directory
/// instead of using the original (now missing) library path.
#[tauri::command]
pub async fn get_audio_waveform_data(
    app_handle: tauri::AppHandle,
    service: State<'_, AssetQueryService>,
    asset_id: Option<String>,
    path: String,
) -> AppResult<Vec<f32>> {
    let resolved_path = if let Some(ref id) = asset_id {
        if let Ok(Some(asset)) = service.get_asset(id).await {
            if let Ok(app_data_directory) = app_handle.path().app_local_data_dir() {
                crate::core::trash::resolve_physical_path(&asset, &app_data_directory)
            } else {
                asset.path.clone()
            }
        } else {
            std::path::PathBuf::from(&path)
        }
    } else {
        std::path::PathBuf::from(&path)
    };

    crate::feature::media::waveform::extract_audio_waveform(&resolved_path, &app_handle).await
}
