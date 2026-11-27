//! Catalog port for regenerable plugin results tied to published thumbnail bytes.

use crate::core::AppResult;
use async_trait::async_trait;
use serde_json::{Map, Value};

/// Maximum UTF-8 JSON size of a single derived result.
pub const MAX_DERIVED_RESULT_BYTES: usize = 64 * 1024;

/// Opaque identity of one published native thumbnail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginPreviewRevision(pub [u8; 32]);

/// Source identity and expected byte digest for a future mediated read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginPreviewSource {
    pub revision: PluginPreviewRevision,
    pub content_digest: [u8; 32],
}

/// Host-only catalog projection used to open the native cache entry.
pub struct PluginPreviewArtifact {
    pub source: PluginPreviewSource,
    pub file_name: String,
    pub family: String,
}

/// Cache key supplied by the host after resolving a verified package and result declaration.
pub struct PluginDerivedResultKey {
    pub asset_id: String,
    pub plugin_id: String,
    pub publisher_fingerprint: [u8; 32],
    pub result_id: String,
    pub package_digest: [u8; 32],
    pub schema_version: u32,
}

/// Application callers own package admission and grants. The Ledger fences source validity.
#[async_trait]
pub trait PluginDerivedDataLedger: Send + Sync {
    /// Returns the identity of the catalog that owns these assets and publications.
    ///
    /// # Errors
    /// Returns a storage error if the catalog identity is unavailable.
    async fn plugin_catalog_id(&self) -> AppResult<String>;

    /// Resolves the current published thumbnail identity. Missing or unverified thumbnails are unavailable.
    ///
    /// # Errors
    /// Returns a storage error if the current source cannot be read or has an invalid recorded digest.
    async fn plugin_preview_source(&self, asset_id: &str)
        -> AppResult<Option<PluginPreviewSource>>;

    /// Resolves the current source and cache name in a single catalog read.
    ///
    /// # Errors
    /// Propagates catalog read failures and invalid digests.
    async fn plugin_preview_artifact(
        &self,
        asset_id: &str,
    ) -> AppResult<Option<PluginPreviewArtifact>>;

    /// Reads a cache hit for the exact package, schema and current source revision.
    ///
    /// # Errors
    /// Returns a storage error if the source or cached result cannot be read or decoded.
    async fn read_plugin_derived_result(
        &self,
        key: &PluginDerivedResultKey,
        source: PluginPreviewRevision,
    ) -> AppResult<Option<Map<String, Value>>>;

    /// Replaces the cache entry if the submitted revision is still current.
    /// Returns false when the source became unavailable or changed.
    ///
    /// # Errors
    /// Rejects invalid schemas or payloads and propagates storage errors without replacing the result.
    async fn write_plugin_derived_result(
        &self,
        key: &PluginDerivedResultKey,
        source: PluginPreviewRevision,
        value: Map<String, Value>,
    ) -> AppResult<bool>;
}
