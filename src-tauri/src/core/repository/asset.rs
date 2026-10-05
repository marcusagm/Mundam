use crate::core::error::AppResult;
use crate::core::models::{
    Asset, AssetFilter, AssetSummaryDto, Folder, LibraryStats, PageParams, SmartFolder, Tag,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Port for read-only asset operations.
#[async_trait]
pub trait AssetQueryHandler: Send + Sync {
    /// Retrieves a list of all assets in the library.
    /// Obsolete: use list_paginated for better performance.
    ///
    /// # Arguments
    ///
    /// * `self` - The asset query handler.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Asset>)` if the assets were found successfully.
    /// * `Err(AppError)` if the assets could not be found.
    async fn find_all(&self) -> AppResult<Vec<Asset>>;

    /// Retrieves a single asset by ID, including its full metadata join.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Asset>)` if the asset was found successfully.
    /// * `Err(AppError)` if the asset could not be found.
    async fn get_by_id(&self, id: &str) -> AppResult<Option<Asset>>;

    /// Retrieves multiple assets by their unique identifiers in a single batch query.
    ///
    /// # Arguments
    ///
    /// * `asset_identifiers` - Slice of asset unique identifiers to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Asset>)` containing all matching assets.
    /// * `Err(AppError)` if the query fails.
    async fn get_by_ids(&self, asset_identifiers: &[String]) -> AppResult<Vec<Asset>>;

    /// Retrieves all tags associated with multiple assets in a single batch query.
    ///
    /// # Arguments
    ///
    /// * `asset_identifiers` - Slice of asset unique identifiers whose tags to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<Tag>>)` mapping each asset ID to its list of tags.
    /// * `Err(AppError)` if the query fails.
    async fn get_tags_for_assets(
        &self,
        asset_identifiers: &[String],
    ) -> AppResult<HashMap<String, Vec<Tag>>>;

    /// Returns a paginated list of assets focused on performance.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter to apply to the assets.
    /// * `page` - The pagination parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
    /// * `Err(AppError)` if the assets could not be found.
    async fn list_paginated(
        &self,
        filter: AssetFilter,
        page: PageParams,
    ) -> AppResult<Vec<AssetSummaryDto>>;

    /// Lists folders under a parent.
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent folder.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Folder>)` if the folders were found successfully.
    /// * `Err(AppError)` if the folders could not be found.
    async fn list_folders(&self, parent_id: Option<String>) -> AppResult<Vec<Folder>>;

    /// Finds unlinked root folders that should be children of a newly added parent folder
    /// and updates their parent_id relationship.
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the newly added parent folder.
    /// * `parent_path` - The absolute path of the parent folder.
    async fn adopt_orphaned_children(&self, parent_id: &str, parent_path: &str) -> AppResult<()>;

    /// Gets a folder by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the folder to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Folder>)` if the folder was found successfully.
    /// * `Err(AppError)` if the folder could not be found.
    async fn get_folder_by_id(&self, id: &str) -> AppResult<Option<Folder>>;

    /// Lists all folders (entire hierarchy).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Folder>)` if the folders were found successfully.
    /// * `Err(AppError)` if the folders could not be found.
    async fn list_all_subfolders(&self) -> AppResult<Vec<Folder>>;

    /// Returns the asset counts for all folders (recursive).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(String, i64)>)` map of folder id to count.
    /// * `Err(AppError)` if the query fails.
    async fn get_subfolder_asset_counts(&self) -> AppResult<Vec<(String, i64)>>;

    /// Returns the asset counts for root locations.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(String, i64)>)` map of folder id to count.
    /// * `Err(AppError)` if the query fails.
    async fn get_location_root_counts(&self) -> AppResult<Vec<(String, i64)>>;

    /// Returns thumbnail paths for all assets in a folder and its subfolders.
    ///
    /// # Arguments
    ///
    /// * `folder_id` - The ID of the target folder.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` thumbnail paths.
    /// * `Err(AppError)` if the query fails.
    async fn get_folder_thumbnails(&self, folder_id: &str) -> AppResult<Vec<String>>;

    /// Lists all unique tags.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Tag>)` if the tags were found successfully.
    /// * `Err(AppError)` if the tags could not be found.
    async fn list_tags(&self) -> AppResult<Vec<Tag>>;

    /// Performs an advanced search using complex criteria.
    ///
    /// # Arguments
    ///
    /// * `criteria` - The advanced search criteria.
    /// * `page` - The pagination parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
    /// * `Err(AppError)` if the search fails.
    async fn search_assets(
        &self,
        criteria: crate::core::models::SearchCriteria,
        page: PageParams,
    ) -> AppResult<Vec<AssetSummaryDto>>;

    /// Retrieves a list of asset IDs that are missing thumbnails.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of IDs to retrieve.
    async fn get_assets_needing_thumbnails(&self, limit: u32) -> AppResult<Vec<String>>;

    /// Retrieves a single asset by its unique ID.
    async fn get_asset_by_id(&self, id: &str) -> AppResult<Asset>;

    /// Returns a map of path -> (file_size, updated_at) for all assets under a root path.
    /// Used for differential scanning.
    async fn get_all_files_comparison_data(
        &self,
        root_path: &str,
    ) -> AppResult<HashMap<String, (i64, DateTime<Utc>)>>;

    /// Retrieves all tags associated with a specific asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - The unique identifier of the asset.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Tag>)` if the tags were found successfully.
    /// * `Err(AppError)` if the query fails.
    async fn get_tags_for_asset(&self, asset_id: &str) -> AppResult<Vec<Tag>>;

    /// Retrieves all saved smart folders.
    async fn list_smart_folders(&self) -> AppResult<Vec<SmartFolder>>;

    /// Gets the total count of assets matching the criteria.
    async fn get_asset_count(&self, filter: AssetFilter) -> AppResult<i64>;

    /// Retrieves comprehensive statistics about the library.
    async fn get_library_stats(&self) -> AppResult<LibraryStats>;

    /// Gets the total count of assets matching the specified search criteria.
    async fn get_search_count(
        &self,
        criteria: crate::core::models::SearchCriteria,
    ) -> AppResult<i64>;

    /// Retrieves all colors extracted for a specific asset.
    async fn get_asset_colors(
        &self,
        asset_id: &str,
    ) -> AppResult<Vec<crate::core::models::AssetColor>>;

    /// Finds a folder ID by its physical path.
    async fn find_folder_by_path(&self, path: &str) -> AppResult<Option<String>>;

    /// Finds a single asset by its physical path.
    async fn find_asset_by_path(&self, path: &str) -> AppResult<Option<Asset>>;

    /// Finds assets by their file size and state.
    /// Useful for recovering moved files that were treated as Delete + Create.
    async fn find_assets_by_size(
        &self,
        size_bytes: u64,
        state: Option<crate::core::models::AssetState>,
    ) -> AppResult<Vec<Asset>>;

    /// Retrieves assets that need repair (e.g. missing format or thumbnail)
    async fn get_assets_needing_repair(&self) -> AppResult<Vec<Asset>>;
}
