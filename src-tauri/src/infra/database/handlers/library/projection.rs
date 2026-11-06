//! Isolates the historical Asset-shaped internal Ledger result from dedicated library IPC receipts.
use crate::core::models::asset::{Asset, AssetState};
use std::path::PathBuf;

/// Builds a side-effect-free compatibility projection; public library results use their own DTO.
pub(crate) fn folder(id: String, name: String, path: PathBuf, parent_id: Option<String>) -> Asset {
    Asset {
        id,
        name,
        path,
        folder_id: parent_id,
        state: AssetState::Idle,
        format_type: "folder".into(),
        family: "FOLDER".into(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    }
}
