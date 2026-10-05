use crate::core::models::{Asset, AssetFilter, Folder, PageParams, PaginatedAssetsDto, Tag};
use crate::core::repository::AssetQueryHandler;
use crate::core::AppResult;
use std::sync::Arc;

/// Service handler for asset-related queries.
pub struct AssetQueryService {
    repository: Arc<dyn AssetQueryHandler>,
}

/// Implementation of the AssetQueryService struct.
impl AssetQueryService {
    /// Creates a new instance of AssetQueryService.
    ///
    /// # Arguments
    ///
    /// * `repository` - The asset query repository.
    ///
    /// # Returns
    ///
    /// * `AssetQueryService` - The asset query service.
    pub fn new(repository: Arc<dyn AssetQueryHandler>) -> Self {
        Self { repository }
    }

    /// Lists assets with specific filters and pagination.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter to apply to the assets.
    /// * `page` - The pagination parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(PaginatedAssetsDto)` if the assets were found successfully.
    /// * `Err(AppError)` if the assets could not be found.
    pub async fn list_assets(
        &self,
        filter: AssetFilter,
        page: PageParams,
    ) -> AppResult<PaginatedAssetsDto> {
        let items = self.repository.list_paginated(filter.clone(), page).await?;
        let total_items = self.repository.get_asset_count(filter).await?;

        Ok(PaginatedAssetsDto { items, total_items })
    }

    /// Gets a single asset with full metadata.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Asset>)` if the asset was found successfully.
    /// * `Err(AppResult<Asset>)` if the asset could not be found.
    pub async fn get_asset(&self, id: &str) -> AppResult<Option<Asset>> {
        self.repository.get_by_id(id).await
    }

    /// Gets multiple assets by their unique identifiers in a single batch query.
    ///
    /// # Arguments
    ///
    /// * `asset_identifiers` - Slice of asset unique identifiers to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Asset>)` containing all matching assets found.
    /// * `Err(AppError)` if the query fails.
    pub async fn get_assets_by_ids(&self, asset_identifiers: &[String]) -> AppResult<Vec<Asset>> {
        self.repository.get_by_ids(asset_identifiers).await
    }

    /// Gets all tags associated with multiple assets in a single batch query.
    ///
    /// # Arguments
    ///
    /// * `asset_identifiers` - Slice of asset unique identifiers whose tags to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<Tag>>)` mapping each asset identifier to its associated tags.
    /// * `Err(AppError)` if the query fails.
    pub async fn get_tags_for_assets(
        &self,
        asset_identifiers: &[String],
    ) -> AppResult<std::collections::HashMap<String, Vec<Tag>>> {
        self.repository.get_tags_for_assets(asset_identifiers).await
    }

    /// Lists folders under a parent.
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent folder.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Folder>)` if the folders were found successfully.
    /// * `Err(AppResult<Folder>)` if the folders could not be found.
    pub async fn list_folders(&self, parent_id: Option<String>) -> AppResult<Vec<Folder>> {
        self.repository.list_folders(parent_id).await
    }

    /// Lists all tags.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Tag>)` if the tags were found successfully.
    /// * `Err(AppResult<Tag>)` if the tags could not be found.
    pub async fn list_tags(&self) -> AppResult<Vec<Tag>> {
        self.repository.list_tags().await
    }

    /// Gets all tags associated with a specific asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - The unique identifier of the asset.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Tag>)` if the tags were found successfully.
    /// * `Err(AppError)` if the query fails.
    pub async fn get_tags_for_asset(&self, asset_id: &str) -> AppResult<Vec<Tag>> {
        self.repository.get_tags_for_asset(asset_id).await
    }

    /// Lists all folders (entire hierarchy).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Folder>)` if the folders were found successfully.
    /// * `Err(AppResult<Folder>)` if the folders could not be found.
    pub async fn list_all_subfolders(&self) -> AppResult<Vec<Folder>> {
        self.repository.list_all_subfolders().await
    }

    /// Returns the asset counts for all folders (recursive).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
    /// * `Err(AppResult<Vec<(String, i64)>>)` if the counts could not be found.
    pub async fn get_subfolder_asset_counts(&self) -> AppResult<Vec<(String, i64)>> {
        self.repository.get_subfolder_asset_counts().await
    }

    /// Returns the asset counts for root locations.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
    /// * `Err(AppResult<Vec<(String, i64)>>)` if the counts could not be found.
    pub async fn get_location_root_counts(&self) -> AppResult<Vec<(String, i64)>> {
        self.repository.get_location_root_counts().await
    }

    /// Returns thumbnail paths for all assets in a folder and its subfolders.
    ///
    /// # Arguments
    ///
    /// * `folder_id` - The ID of the folder to retrieve thumbnails for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` if the thumbnails were found successfully.
    /// * `Err(AppResult<Vec<String>>)` if the thumbnails could not be found.
    pub async fn get_folder_thumbnails(&self, folder_id: &str) -> AppResult<Vec<String>> {
        self.repository.get_folder_thumbnails(folder_id).await
    }

    /// Lists all smart folders.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<crate::core::models::SmartFolder>)` if successful.
    /// * `Err(AppError)` if the query fails.
    pub async fn list_smart_folders(&self) -> AppResult<Vec<crate::core::models::SmartFolder>> {
        self.repository.list_smart_folders().await
    }

    /// Gets the total count of assets matching the specified filter.
    ///
    /// # Arguments
    ///
    /// * `filter` - The given filters.
    ///
    /// # Returns
    ///
    /// * `Ok(i64)` asset count.
    /// * `Err(AppError)` on query failure.
    pub async fn get_asset_count(&self, filter: AssetFilter) -> AppResult<i64> {
        self.repository.get_asset_count(filter).await
    }

    /// Gets library statistics (total assets, folders, tags, size).
    ///
    /// # Returns
    ///
    /// * `Ok(crate::core::models::LibraryStats)` with aggregated data.
    /// * `Err(AppError)` on query failure.
    pub async fn get_library_stats(&self) -> AppResult<crate::core::models::LibraryStats> {
        self.repository.get_library_stats().await
    }

    /// Retrieves all colors extracted for a specific asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - The unique identifier of the asset.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AssetColor>)` if successful.
    /// * `Err(AppError)` if the query fails.
    pub async fn get_asset_colors(
        &self,
        asset_id: &str,
    ) -> AppResult<Vec<crate::core::models::AssetColor>> {
        self.repository.get_asset_colors(asset_id).await
    }
}
