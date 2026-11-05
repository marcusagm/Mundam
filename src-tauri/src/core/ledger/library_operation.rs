//! User filesystem intentions, distinct from watcher observations.
use serde::{Deserialize, Serialize};

/// Validated edits to indexed files and folders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LibraryOperation {
    /// Create an on-disk child folder.
    CreateFolder { parent_id: String, name: String },
    /// Rename a folder while preserving its identity.
    RenameFolder { folder_id: String, name: String },
    /// Move a subtree into an indexed folder.
    MoveFolder {
        folder_id: String,
        destination_id: String,
    },
    /// Quarantine a subtree before removing its library records.
    DeleteFolder { folder_id: String },
    /// Restore files and catalog metadata from a committed folder deletion.
    RestoreFolder { folder_id: String },
    /// Persist folder presentation preferences.
    FolderColor {
        folder_id: String,
        color: Option<String>,
        inherits_color: bool,
    },
    /// Rename an active asset, preserving its extension.
    RenameAsset { asset_id: String, name: String },
    /// Move a selection, rejecting collisions before changing disk state.
    MoveAssets {
        asset_ids: Vec<String>,
        destination_id: String,
    },
}
