use crate::core::models::asset::AssetState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Payload for creating a new asset in the system.
///
/// This structure captures all mandatory information required by the Ledger
/// to formally register an asset in the database and filesystem registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetPayload {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Initially detected file size in bytes.
    pub file_size: u64,
    /// Initially detected format (MimeType or Extension).
    pub format_type: String,
    /// The high-level family (IMAGE, VIDEO, etc).
    pub family: String,
    /// Initial state for the lifecycle machine (usually Discovered or Indexed).
    pub state_init: AssetState,
    /// Optional parent folder ID.
    pub folder_id: Option<String>,
    /// Optional creation time (from filesystem).
    pub created_at: Option<DateTime<Utc>>,
    /// Optional modification time (from filesystem).
    pub modified_at: Option<DateTime<Utc>>,
}

/// Payload for updating tags associated with an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagsPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
    /// List of tag IDs to be added.
    pub tags_to_add: Vec<String>,
    /// List of tag IDs to be removed.
    pub tags_to_remove: Vec<String>,
}

/// Payload for updating asset core identity (e.g., after a rename).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetPayload {
    pub asset_id: Option<String>,
    pub old_path: Option<PathBuf>,
    pub new_path: PathBuf,
}

/// Payload for creating a new folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderPayload {
    pub parent_id: Option<String>,
    pub name: String,
    pub path: PathBuf,
}

/// Payload for removing a folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveFolderPayload {
    /// Folder identity to remove.
    pub folder_id: String,
    /// Observation fence; absent for explicit unlink/deletion intentions.
    #[serde(default)]
    pub expected_path: Option<PathBuf>,
}

/// Payload for renaming/moving a folder and its contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameFolderPayload {
    pub folder_id: String,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

/// Payload for updating an asset's color palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetColorsPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
    /// The complete list of extracted colors.
    pub colors: Vec<crate::core::models::asset::AssetColor>,
}

/// Payload for updating an asset's rating.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetRatingPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
    /// The new rating value (0-5).
    pub rating: i32,
}

/// Payload for updating an asset's personal notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetNotesPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
    /// The new notes content.
    pub notes: String,
}

/// Payload for toggling an asset's favorite status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleFavoritePayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
}

/// Payload for moving an asset to the trash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToTrashPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
}

/// Payload for restoring an asset from the trash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreFromTrashPayload {
    /// The unique identifier of the target asset.
    pub asset_id: String,
}

/// Payload for updating an asset's technical metadata (dimensions, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTechnicalMetadataPayload {
    pub asset_id: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_secs: Option<f64>,
    pub technical_payload: Option<serde_json::Value>,
    pub semantic_payload: Option<serde_json::Value>,
}

/// Payload for creating a new taxonomy tag.
///
/// The Ledger will generate a UUID for the tag and persist it in the tags table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagPayload {
    /// Display name of the tag (must be unique).
    pub name: String,
    /// Optional parent tag ID for hierarchical organization.
    pub parent_id: Option<String>,
    /// Optional hex color code for the tag.
    pub color: Option<String>,
}

/// Payload for updating an existing taxonomy tag's properties.
///
/// Only non-None fields will be applied in the UPDATE statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagPayload {
    /// Clear the custom color without enabling inheritance.
    #[serde(default)]
    pub reset_color: bool,
    /// Whether this tag follows its parent color, overriding a custom color.
    pub inherits_color: Option<bool>,
    /// The unique identifier of the tag to update.
    pub id: String,
    /// New display name (optional).
    pub name: Option<String>,
    /// New hex color (optional).
    pub color: Option<String>,
    /// New parent tag ID (optional; use Some("") or None logic to unparent).
    pub parent_id: Option<String>,
    /// New sorting order index (optional).
    pub order_index: Option<i64>,
}

/// Payload for batch tag-asset association operations.
///
/// Used by AddTagsToAssetsBatch, RemoveTagsFromAssetsBatch,
/// and ReplaceTagsForAssetsBatch commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTagsPayload {
    /// The asset IDs to operate on.
    pub asset_ids: Vec<String>,
    /// The tag IDs to add/remove/replace.
    pub tag_ids: Vec<String>,
}

/// Payload for creating a smart folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSmartFolderPayload {
    pub name: String,
    pub query_json: String,
}

/// Payload for updating a smart folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSmartFolderPayload {
    pub id: String,
    pub name: String,
    pub query_json: String,
}

/// Payload for deleting a smart folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSmartFolderPayload {
    pub id: String,
}

/// Centralized Enum representing all mutation intentions (Commands) for the Asset Ledger.
///
/// Under CQRS, this represents the "Write" intent. The Ledger is responsible for
/// validating these commands against the current state before applying them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LedgerCommand {
    /// Coordinate filesystem edits with a library transaction.
    LibraryOperation(super::library_operation::LibraryOperation),
    /// Register a new asset in the system.
    CreateAsset(CreateAssetPayload),
    /// Register multiple assets in a single atomic transaction.
    BatchCreate(Vec<CreateAssetPayload>),
    /// Atomic update of asset tags.
    UpdateTags(UpdateTagsPayload),
    /// Update asset metadata (e.g., after a move/rename).
    UpdateAsset(UpdateAssetPayload),
    /// Mark an asset as stale (needs re-probing).
    MarkAsStale { asset_id: String },
    /// Formally delete an asset from the system.
    DeleteAsset {
        asset_id: Option<String>,
        path: Option<PathBuf>,
        /// If true, also attempts to delete the physical file.
        physical_delete: bool,
    },
    /// Create a new logical folder.
    CreateFolder(CreateFolderPayload),
    /// Removes a logical folder and all its contents.
    RemoveFolder(RemoveFolderPayload),
    /// Renames a folder and recursively updates all child paths.
    RenameFolder(RenameFolderPayload),
    /// Assign an asset to a folder.
    SetAssetFolder {
        asset_id: String,
        folder_id: Option<String>,
    },
    /// Update an asset's thumbnail path and state.
    UpdateThumbnail {
        asset_id: String,
        thumbnail_path: String,
    },
    /// Update an asset's color palette.
    UpdateAssetColors(UpdateAssetColorsPayload),
    /// Update an asset's rating.
    UpdateAssetRating(UpdateAssetRatingPayload),
    /// Update an asset's notes.
    UpdateAssetNotes(UpdateAssetNotesPayload),
    /// Toggle an asset's favorite status.
    ToggleFavorite(ToggleFavoritePayload),
    /// Move an asset to trash.
    MoveToTrash(MoveToTrashPayload),
    /// Restore an asset from trash.
    RestoreFromTrash(RestoreFromTrashPayload),
    /// Re-trigger color extraction for an asset.
    ReextractColors { asset_id: String },
    /// Regenerate thumbnails for an asset.
    RegenerateThumbnail { asset_id: String },
    /// Update technical metadata (dimensions, etc).
    UpdateTechnicalMetadata(UpdateTechnicalMetadataPayload),
    /// Update an asset's format type (e.g. during repair/migration).
    UpdateFormat { asset_id: String, format: String },

    /// Create a new taxonomy tag with name, optional color and parent.
    CreateTag(CreateTagPayload),
    /// Atomically merges or replaces a taxonomy from an ordered outline.
    ImportTags(ImportTagsPayload),
    /// Update an existing tag's properties (name, color, parent_id, order_index).
    UpdateTag(UpdateTagPayload),
    /// Delete a tag and remove all its asset associations.
    DeleteTag { id: String },
    /// Associate multiple tags with multiple assets in a single transaction.
    AddTagsToAssetsBatch(BatchTagsPayload),
    /// Remove specific tag associations from multiple assets.
    RemoveTagsFromAssetsBatch(BatchTagsPayload),
    /// Replace all tag associations for multiple assets with a new set.
    ReplaceTagsForAssetsBatch(BatchTagsPayload),

    /// Create a new smart folder.
    CreateSmartFolder(CreateSmartFolderPayload),
    /// Update an existing smart folder.
    UpdateSmartFolder(UpdateSmartFolderPayload),
    /// Delete a smart folder.
    DeleteSmartFolder(DeleteSmartFolderPayload),
    /// Execute a collection of commands in a single atomic transaction.
    Batch(Vec<LedgerCommand>),
}

/// One tag in a preorder outline, referencing an earlier entry as its parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTagEntry {
    /// Unique, nonempty display name.
    pub name: String,
    /// Index of the parent in the same outline; absent for roots.
    pub parent_index: Option<usize>,
    /// Explicit hexadecimal color; absent when inherited.
    pub color: Option<String>,
}

/// A complete taxonomy import performed in a single ledger transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTagsPayload {
    /// Entries in markdown preorder, with sibling order preserved.
    pub entries: Vec<ImportTagEntry>,
    /// Deletes the existing taxonomy and asset associations when true.
    pub replace_existing: bool,
}
