//! Transport-independent catalog projections for the external importer.
use super::destinations::DestinationGrant;
use crate::core::AppResult;
use async_trait::async_trait;
use serde::Serialize;

/// Read permissions supplied by the authentication layer for the current client.
pub struct CatalogReadGrant {
    pub client_id: String,
    pub grant_version: u64,
    pub destinations: DestinationGrant,
    pub can_read_folders: bool,
    pub can_read_tags: bool,
}

/// Pagination parameters shared by the two catalog queries.
#[derive(Default)]
pub struct CatalogPageRequest<'a> {
    pub cursor: Option<&'a str>,
    pub limit: Option<u32>,
}

/// Limited catalog projection and its continuation token.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage<Item> {
    pub items: Vec<Item>,
    pub next_cursor: Option<String>,
}

/// Folder metadata safe to show to an authorized client.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFolder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub has_children: bool,
    pub availability: FolderAvailability,
}

/// Availability reported by catalog queries. Admission verifies availability separately.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderAvailability {
    Unknown,
}

/// Tag metadata with ancestor names followed by the name of the tag itself.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTag {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub order_index: i64,
    pub labels: Vec<String>,
}

/// Read-only import catalog used by delivery adapters.
#[async_trait]
pub trait ImportCatalog: Send + Sync {
    /// Lists granted roots or direct children of a visible parent.
    ///
    /// # Errors
    /// Rejects missing grants, invisible parents, stale cursors and storage errors.
    async fn list_folders(
        &self,
        grant: &CatalogReadGrant,
        parent_id: Option<&str>,
        page: CatalogPageRequest<'_>,
    ) -> AppResult<CatalogPage<ImportFolder>>;

    /// Lists the taxonomy in tag order, optionally filtering by tag name.
    ///
    /// # Errors
    /// Rejects missing grants, invalid filters, stale cursors and storage errors.
    async fn list_tags(
        &self,
        grant: &CatalogReadGrant,
        query: Option<&str>,
        page: CatalogPageRequest<'_>,
    ) -> AppResult<CatalogPage<ImportTag>>;
}
