//! Host-only management of data retained by the active catalog.

use serde::{Deserialize, Serialize};

/// Identifies one plugin namespace and its exact signing identity.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredPluginOwner {
    pub plugin_id: String,
    pub publisher_fingerprint: String,
}

/// Counts presented to the user before data is removed.
#[derive(Default, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPluginDataCounts {
    pub annotations: u64,
    pub recovery_annotations: u64,
    pub cached_results: u64,
}

/// One identity in the active catalog inventory, even after removing a package.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPluginData {
    pub owner: StoredPluginOwner,
    pub counts: StoredPluginDataCounts,
}

/// Snapshot of the exact records the host is about to delete.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPluginDataReview {
    pub counts: StoredPluginDataCounts,
    pub revision: String,
}
