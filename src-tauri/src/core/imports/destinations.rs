//! Catalog-local import destination policy shared by desktop and external delivery.
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;

/// The desktop preference and identity of the open catalog.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreference {
    pub catalog_id: String,
    pub folder_id: Option<String>,
}

/// Authorization resolved by the host from an authenticated client's grant.
pub struct DestinationGrant {
    pub catalog_id: String,
    pub root_ids: Vec<String>,
}

/// Catalog queries and administrative updates for destination selection.
#[async_trait]
pub trait ImportDestinations: Send + Sync {
    /// Reads the saved preference, including destinations currently offline.
    async fn preference(&self) -> AppResult<ImportPreference>;

    /// Persists or clears a folder selection in the open catalog.
    async fn set_default(
        &self,
        catalog_id: &str,
        folder_id: Option<&str>,
    ) -> AppResult<ImportPreference>;

    /// Resolves a destination against the grant and current catalog topology in one read.
    /// An omitted request looks up the catalog's default.
    async fn resolve(
        &self,
        grant: &DestinationGrant,
        requested: Option<&str>,
    ) -> AppResult<Option<String>>;
}

/// Chooses an authorized folder before admitting an import. Publication must check
/// the resulting ID against current authorization, topology and filesystem identity again.
///
/// # Errors
/// Returns a generic not-found error when the choice is outside the grant or unavailable.
pub async fn require_destination(
    destinations: &dyn ImportDestinations,
    grant: &DestinationGrant,
    requested: Option<&str>,
) -> AppResult<String> {
    destinations
        .resolve(grant, requested)
        .await?
        .ok_or_else(|| AppError::NotFound("Import destination unavailable".into()))
}
