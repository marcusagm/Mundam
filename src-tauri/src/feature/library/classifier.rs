//! Pure file classification for the fan-out producer pipeline.
//!
//! This module contains the stateless classification function used by
//! `scan_directory` to determine whether a filesystem entry needs indexing.
//! It compares on-disk metadata against a preloaded comparison cache and
//! resolves format and folder ownership via shared read-only references.

use crate::core::ledger::command::CreateAssetPayload;
use crate::core::models::asset::AssetState;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use unicode_normalization::UnicodeNormalization;
use walkdir::DirEntry;

/// Normalizes a path to NFC; macOS may report NFD while the database stores NFC.
pub fn nfc_path(path: &std::path::Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().nfc().collect::<String>())
}

/// Intermediate result produced by each fan-out producer task.
pub enum AssetDiscoveryResult {
    /// A new or modified asset was detected and needs to be persisted.
    NewAsset(CreateAssetPayload),
    /// The file was already indexed and unchanged — skip.
    ExistingAsset,
    /// A parsing or I/O error occurred — skip this file gracefully.
    Error(String),
}

/// Pure classification function executed by each producer task.
///
/// Reads the filesystem metadata and compares with the comparison cache
/// to decide if a file needs indexing. This function is entirely stateless
/// and safe to run concurrently across many tasks.
///
/// # Arguments
///
/// * `entry` - The walkdir entry representing a file on disk.
/// * `comparison_cache` - Preloaded map of `path → (size, modified_at)` from the database.
/// * `folder_cache` - Shared read-only map of `directory_path → folder_id`.
/// * `registry` - The format registry for detecting supported file types.
/// * `root_folder_id` - Fallback folder ID when no parent folder is found in the cache.
///
/// # Returns
///
/// An `AssetDiscoveryResult` indicating whether the file is new, unchanged, or errored.
pub async fn classify_file_entry(
    entry: &DirEntry,
    comparison_cache: &HashMap<String, (i64, DateTime<Utc>)>,
    folder_cache: &Arc<RwLock<HashMap<PathBuf, String>>>,
    registry: &crate::core::formats::registry::FormatRegistry,
    root_folder_id: &Option<String>,
) -> AssetDiscoveryResult {
    let entry_path = nfc_path(entry.path());
    let path_str = entry_path.to_string_lossy().to_string();

    let metadata = match entry.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            return AssetDiscoveryResult::Error(format!(
                "Failed to read metadata for {:?}: {}",
                entry_path, error
            ));
        }
    };

    let disk_size = metadata.len() as i64;
    let disk_modified_time: DateTime<Utc> = metadata
        .modified()
        .ok()
        .map(|time| time.into())
        .unwrap_or_else(Utc::now);
    let disk_created_time: Option<DateTime<Utc>> = metadata.created().ok().map(|time| time.into());

    let needs_indexing =
        if let Some((cached_size, cached_modified_time)) = comparison_cache.get(&path_str) {
            disk_size != *cached_size
                || (disk_modified_time - *cached_modified_time)
                    .num_seconds()
                    .abs()
                    >= 1
        } else {
            true
        };

    if !needs_indexing {
        return AssetDiscoveryResult::ExistingAsset;
    }

    let (format_name, family_name) = if let Some(supported_format) = registry.detect(&entry_path) {
        (
            supported_format.name.to_string(),
            supported_format.type_category.to_string(),
        )
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    let asset_folder_id = {
        let cache_read = folder_cache.read().await;
        entry_path
            .parent()
            .and_then(|parent| cache_read.get(parent).cloned())
            .or_else(|| root_folder_id.clone())
    };

    AssetDiscoveryResult::NewAsset(CreateAssetPayload {
        path: entry_path,
        file_size: disk_size as u64,
        format_type: format_name,
        family: family_name,
        state_init: AssetState::Indexed,
        folder_id: asset_folder_id,
        created_at: disk_created_time,
        modified_at: Some(disk_modified_time),
    })
}
