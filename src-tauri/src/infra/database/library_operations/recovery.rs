//! Recovery manifests participate in the same transaction and disk compensation as deletion.
use super::{snapshot, DiskChanges};
use crate::core::{
    error::{AppError, AppResult},
    ledger::command::CreateFolderPayload,
    models::asset::Asset,
};
use sqlx::{Sqlite, Transaction};
use std::path::{Path, PathBuf};

/// Commits the restore point atomically with the catalog deletion.
///
/// # Errors
/// Returns persistence or snapshot serialization errors without moving originals.
pub(super) async fn capture(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: &str,
    source: &Path,
    archive: &Path,
) -> AppResult<()> {
    let snapshot = snapshot::capture(transaction, identifier).await?;
    sqlx::query("INSERT INTO folder_recovery (folder_id, original_path, archive_path, snapshot_json) VALUES (?, ?, ?, ?)")
        .bind(identifier).bind(source.to_string_lossy().as_ref()).bind(archive.to_string_lossy().as_ref()).bind(snapshot)
        .execute(&mut **transaction).await?;
    Ok(())
}

/// Plans an exclusive move back to the original location with catalog compensation.
///
/// # Errors
/// Rejects occupied destinations, missing parents, invalid archives and metadata conflicts.
pub(super) async fn restore(
    transaction: &mut Transaction<'_, Sqlite>,
    app_data: &Path,
    identifier: &str,
    changes: &mut DiskChanges,
) -> AppResult<Asset> {
    let (original, archive, snapshot): (String, String, String) = sqlx::query_as("SELECT original_path, archive_path, snapshot_json FROM folder_recovery WHERE folder_id = ?")
        .bind(identifier).fetch_optional(&mut **transaction).await?.ok_or_else(|| AppError::NotFound("Folder recovery is no longer available".into()))?;
    let original = PathBuf::from(original);
    let archive = PathBuf::from(archive);
    if archive.parent() != Some(app_data.join("deleted-folders").as_path()) {
        return Err(AppError::ValidationFailed(
            "Invalid recovery directory".into(),
        ));
    }
    super::planning::available(transaction, &original).await?;
    if !tokio::fs::symlink_metadata(&archive).await?.is_dir() {
        return Err(AppError::ValidationFailed(
            "Recovery copy is unavailable or is not a directory".into(),
        ));
    }
    let parent_path = original
        .parent()
        .ok_or_else(|| AppError::ValidationFailed("Original parent is unavailable".into()))?;
    let parent_id: String =
        sqlx::query_scalar("SELECT id FROM folders WHERE path = ? COLLATE NOCASE")
            .bind(parent_path.to_string_lossy().as_ref())
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| {
                AppError::ValidationFailed(
                    "Restore the original parent folder before undoing this deletion".into(),
                )
            })?;
    if !tokio::fs::symlink_metadata(parent_path).await?.is_dir() {
        return Err(AppError::ValidationFailed(
            "Original parent is unavailable".into(),
        ));
    }
    changes.restored_assets = snapshot::restore(transaction, &snapshot).await?;
    sqlx::query("DELETE FROM folder_recovery WHERE folder_id = ?")
        .bind(identifier)
        .execute(&mut **transaction)
        .await?;
    changes.moves.push((archive, original.clone()));
    crate::infra::database::handlers::folder_handler::handle_create_folder(
        transaction,
        CreateFolderPayload {
            parent_id: Some(parent_id),
            name: original
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: original,
        },
    )
    .await
}
