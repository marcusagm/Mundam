use crate::core::error::AppResult;
use crate::core::models::{
    Asset, AssetColor, AssetFilter, AssetSummaryDto, Folder, LibraryStats, PageParams, SmartFolder,
    Tag,
};
use crate::core::repository::AssetQueryHandler;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

/// SQLite implementation of the AssetQueryHandler port.
pub struct SqliteAssetQueries {
    /// The database connection pool.
    pool: SqlitePool,
    /// The central "Cartório" for format definitions.
    registry: Arc<crate::core::formats::registry::FormatRegistry>,
}

impl SqliteAssetQueries {
    /// Creates a new instance with the given connection pool.
    pub fn new(
        pool: SqlitePool,
        registry: Arc<crate::core::formats::registry::FormatRegistry>,
    ) -> Self {
        Self { pool, registry }
    }
}

#[async_trait]
impl AssetQueryHandler for SqliteAssetQueries {
    async fn find_all(&self) -> AppResult<Vec<Asset>> {
        crate::infra::database::query_handlers::asset_queries::find_all(&self.pool, &self.registry).await
    }

    async fn get_by_id(&self, id: &str) -> AppResult<Option<Asset>> {
        crate::infra::database::query_handlers::asset_queries::get_by_id(&self.pool, &self.registry, id).await
    }

    async fn get_by_ids(&self, asset_identifiers: &[String]) -> AppResult<Vec<Asset>> {
        crate::infra::database::query_handlers::asset_queries::get_by_ids(
            &self.pool,
            &self.registry,
            asset_identifiers,
        )
        .await
    }

    async fn get_tags_for_assets(
        &self,
        asset_identifiers: &[String],
    ) -> AppResult<HashMap<String, Vec<Tag>>> {
        crate::infra::database::query_handlers::tags_queries::get_tags_for_assets(
            &self.pool,
            &self.registry,
            asset_identifiers,
        )
        .await
    }

    async fn list_paginated(
        &self,
        filter: AssetFilter,
        page: PageParams,
    ) -> AppResult<Vec<AssetSummaryDto>> {
        crate::infra::database::query_handlers::asset_queries::list_paginated(&self.pool, &self.registry, filter, page).await
    }

    async fn list_folders(&self, parent_id: Option<String>) -> AppResult<Vec<Folder>> {
        crate::infra::database::query_handlers::folder_queries::list_folders(&self.pool, &self.registry, parent_id).await
    }

    async fn adopt_orphaned_children(&self, parent_id: &str, parent_path: &str) -> AppResult<()> {
        crate::infra::database::query_handlers::folder_queries::adopt_orphaned_children(&self.pool, &self.registry, parent_id, parent_path).await
    }

    async fn get_folder_by_id(&self, id: &str) -> AppResult<Option<Folder>> {
        crate::infra::database::query_handlers::folder_queries::get_folder_by_id(&self.pool, &self.registry, id).await
    }

    async fn list_all_subfolders(&self) -> AppResult<Vec<Folder>> {
        crate::infra::database::query_handlers::folder_queries::list_all_subfolders(&self.pool, &self.registry).await
    }

    async fn get_subfolder_asset_counts(&self) -> AppResult<Vec<(String, i64)>> {
        crate::infra::database::query_handlers::folder_queries::get_subfolder_asset_counts(&self.pool, &self.registry).await
    }

    async fn get_location_root_counts(&self) -> AppResult<Vec<(String, i64)>> {
        crate::infra::database::query_handlers::folder_queries::get_location_root_counts(&self.pool, &self.registry).await
    }

    async fn get_folder_thumbnails(&self, folder_id: &str) -> AppResult<Vec<String>> {
        crate::infra::database::query_handlers::metadata_queries::get_folder_thumbnails(&self.pool, &self.registry, folder_id).await
    }

    async fn list_tags(&self) -> AppResult<Vec<Tag>> {
        crate::infra::database::query_handlers::tags_queries::list_tags(&self.pool, &self.registry).await
    }

    async fn search_assets(
        &self,
        criteria: crate::core::models::SearchCriteria,
        page: PageParams,
    ) -> AppResult<Vec<AssetSummaryDto>> {
        crate::infra::database::query_handlers::search_queries::search_assets(&self.pool, &self.registry, criteria, page).await
    }

    async fn get_assets_needing_thumbnails(&self, limit: u32) -> AppResult<Vec<String>> {
        crate::infra::database::query_handlers::asset_queries::get_assets_needing_thumbnails(&self.pool, &self.registry, limit).await
    }

    async fn get_asset_by_id(&self, id: &str) -> AppResult<Asset> {
        crate::infra::database::query_handlers::asset_queries::get_asset_by_id(&self.pool, &self.registry, id).await
    }

    async fn get_all_files_comparison_data(
        &self,
        root_path: &str,
    ) -> AppResult<HashMap<String, (i64, DateTime<Utc>)>> {
        crate::infra::database::query_handlers::asset_queries::get_all_files_comparison_data(&self.pool, &self.registry, root_path).await
    }

    async fn get_tags_for_asset(&self, asset_id: &str) -> AppResult<Vec<Tag>> {
        crate::infra::database::query_handlers::tags_queries::get_tags_for_asset(&self.pool, &self.registry, asset_id).await
    }

    async fn list_smart_folders(&self) -> AppResult<Vec<SmartFolder>> {
        crate::infra::database::query_handlers::search_queries::list_smart_folders(&self.pool, &self.registry).await
    }

    async fn get_asset_count(&self, filter: AssetFilter) -> AppResult<i64> {
        crate::infra::database::query_handlers::asset_queries::get_asset_count(&self.pool, &self.registry, filter).await
    }

    async fn get_library_stats(&self) -> AppResult<LibraryStats> {
        crate::infra::database::query_handlers::stats_queries::get_library_stats(&self.pool, &self.registry).await
    }

    async fn get_search_count(
        &self,
        criteria: crate::core::models::SearchCriteria,
    ) -> AppResult<i64> {
        crate::infra::database::query_handlers::search_queries::get_search_count(&self.pool, &self.registry, criteria).await
    }

    async fn get_asset_colors(
        &self,
        asset_id: &str,
    ) -> AppResult<Vec<AssetColor>> {
        crate::infra::database::query_handlers::metadata_queries::get_asset_colors(&self.pool, &self.registry, asset_id).await
    }

    async fn find_folder_by_path(&self, path: &str) -> AppResult<Option<String>> {
        crate::infra::database::query_handlers::folder_queries::find_folder_by_path(&self.pool, &self.registry, path).await
    }

    async fn find_asset_by_path(&self, path: &str) -> AppResult<Option<Asset>> {
        crate::infra::database::query_handlers::asset_queries::find_asset_by_path(&self.pool, &self.registry, path).await
    }

    async fn find_assets_by_size(
        &self,
        size_bytes: u64,
        state: Option<crate::core::models::AssetState>,
    ) -> AppResult<Vec<Asset>> {
        crate::infra::database::query_handlers::asset_queries::find_assets_by_size(&self.pool, &self.registry, size_bytes, state).await
    }

    async fn get_assets_needing_repair(&self) -> AppResult<Vec<Asset>> {
        crate::infra::database::query_handlers::asset_queries::get_assets_needing_repair(&self.pool, &self.registry).await
    }
}
