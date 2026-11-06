//! Contracts and pure policies for recoverable user edits and root observation.
pub mod policy;
use crate::core::error::AppResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stable filesystem object identity; paths and timestamps are not identity proofs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    /// Volume/device identifier supplied by the operating system.
    pub device: u64,
    /// File index/inode within the volume, unchanged by an in-volume rename.
    pub object: u64,
    /// Native creation generation reduces inode/file-index reuse ambiguity.
    #[serde(default)]
    pub birth: Option<u64>,
}

/// Catalog entity affected by an intention, independent of legacy asset DTOs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryEntity {
    /// Indexed directory or recovery entry.
    Folder,
    /// Indexed media file.
    Asset,
}

/// One confirmed path effect, with temporary rename stages coalesced.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPathEffect {
    /// Absent for directory creation.
    pub source: Option<PathBuf>,
    /// Absent for empty-draft removal.
    pub destination: Option<PathBuf>,
}

/// Confirmed outcome of a managed edit, independent of the asset data model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryOperationResult {
    /// Unique receipt used for tracing and idempotent recovery.
    pub operation_id: String,
    /// Stable folder or asset identifier affected by the command.
    pub id: String,
    /// Explicit discriminator instead of a fabricated asset family.
    pub entity_type: LibraryEntity,
    /// Confirmed destination, including the archive for a deletion.
    pub path: PathBuf,
    /// Confirmed source when the operation relocated an entity.
    pub previous_path: Option<PathBuf>,
    /// Every confirmed movement in a selection, without exposing staging paths.
    pub effects: Vec<LibraryPathEffect>,
    /// Operational warning that does not negate a committed write.
    pub warning: Option<String>,
}

/// Persistent recovery entry, available after notifications and process restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRecoveryEntry {
    /// Stable history-entry identity for deterministic pagination.
    pub entry_id: String,
    /// Whether an archive is still available for Undo.
    pub restorable: bool,
    /// Stable identity retained by Undo.
    pub folder_id: String,
    /// Destination required for restoration.
    pub original_path: String,
    /// UTC deletion timestamp.
    pub created_at: String,
    /// Logical archive size measured without following symlinks.
    pub byte_count: i64,
}

/// Reconciles native observation with the complete desired root set.
#[async_trait]
pub trait RootObservation: Send + Sync {
    /// Applies an idempotent desired-state update.
    ///
    /// # Errors
    /// Reports native watch/unwatch failures without forgetting failed registrations.
    async fn reconcile(&self, roots: Vec<PathBuf>) -> AppResult<()>;
}

/// Storage access for the explicit advanced recovery action.
#[async_trait]
pub trait RecoveryStorage: Send + Sync {
    /// Resolves and creates app-owned recovery storage.
    ///
    /// # Errors
    /// Returns permission or storage availability failures.
    async fn directory(&self) -> AppResult<PathBuf>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_receipt_matches_the_shared_typescript_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../src/core/store/library/fixtures/libraryOperationResult.json"
        ));
        let result: super::LibraryOperationResult = serde_json::from_str(fixture)?;
        assert_eq!(
            serde_json::to_value(result)?,
            serde_json::from_str::<serde_json::Value>(fixture)?
        );
        Ok(())
    }
}
