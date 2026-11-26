//! Catalog-scoped storage port for asset data owned by a verified plugin.

use crate::core::AppResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// An Inspector request bound to one asset and the displayed package revision.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSectionTarget {
    pub asset_id: String,
    pub plugin_id: String,
    pub section_id: String,
    pub package_revision: String,
}

/// An edit attempted by a host-rendered boolean control.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginBooleanEdit {
    pub target: PluginSectionTarget,
    pub field_id: String,
    pub expected_revision: String,
    pub value: bool,
}

/// The current editable or read-only state of a rendered field.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectorFieldState {
    pub field_id: String,
    pub state: PluginInspectorValue,
}

/// JSON uses decimal revision strings to preserve the full SQLite integer range.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PluginInspectorValue {
    Available {
        value: bool,
        revision: String,
        writable: bool,
    },
    SchemaMismatch,
}

/// The host either committed the edit or instructs the renderer to refresh.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PluginInspectorEditResult {
    Saved { value: bool, revision: String },
    Conflict,
    SchemaMismatch,
}

/// Identifies data within one catalog and one verified signing identity.
/// Application callers resolve the publisher key from the trusted package.
pub struct PluginAssetFieldKey {
    pub asset_id: String,
    pub plugin_id: String,
    pub publisher_fingerprint: [u8; 32],
    pub field_id: String,
}

/// The first field type supported by the declarative plugin SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginBooleanField {
    pub schema_version: u32,
    pub revision: i64,
    pub value: bool,
}

/// Compares the caller's revision to the stored revision before any mutation.
/// An expected revision of zero denotes an absent field.
pub struct PluginBooleanWrite {
    pub schema_version: u32,
    pub expected_revision: i64,
    pub value: bool,
}

/// A conditional write always distinguishes a revision conflict from a schema mismatch.
#[derive(Debug, PartialEq, Eq)]
pub enum PluginBooleanWriteOutcome {
    Stored(PluginBooleanField),
    Conflict { current_revision: i64 },
    SchemaMismatch { stored_version: u32 },
}

/// Data operations on the active catalog. The application service owns grant checks.
#[async_trait]
pub trait PluginAssetDataLedger: Send + Sync {
    /// Reads an optional boolean field on an active asset.
    async fn read_plugin_boolean(
        &self,
        key: &PluginAssetFieldKey,
    ) -> AppResult<Option<PluginBooleanField>>;

    /// Writes a boolean field on an active asset using optimistic concurrency.
    async fn write_plugin_boolean(
        &self,
        key: &PluginAssetFieldKey,
        write: PluginBooleanWrite,
    ) -> AppResult<PluginBooleanWriteOutcome>;
}
