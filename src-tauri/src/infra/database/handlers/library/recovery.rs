//! Recovery manifests participate in the same transaction and disk compensation as deletion.
use super::plan::DiskChanges;
use super::snapshot;
use crate::core::{
    error::{AppError, AppResult},
    models::asset::Asset,
};
use sqlx::{Sqlite, Transaction};
use std::path::{Path, PathBuf};

/// Commits the restore point atomically with the catalog deletion.
///
/// # Errors
/// Returns persistence or snapshot serialization errors without moving originals.
pub(crate) async fn capture(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: &str,
    source: &Path,
    archive: &Path,
    byte_count: i64,
    budget: i64,
) -> AppResult<()> {
    let unmeasured: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM folder_recovery WHERE byte_count < 0")
            .fetch_one(&mut **transaction)
            .await?;
    if unmeasured != 0 {
        return Err(AppError::ValidationFailed(
            "Recovery storage inventory is pending. Retry after inventory completes.".into(),
        ));
    }
    let used: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(byte_count),0) FROM folder_recovery")
        .fetch_one(&mut **transaction)
        .await?;
    if used.saturating_add(byte_count) > budget {
        return Err(AppError::ValidationFailed("Folder recovery storage budget exceeded. Restore archived folders or increase the budget in settings.".into()));
    }
    let snapshot = snapshot::capture(transaction, identifier).await?;
    sqlx::query("INSERT INTO folder_recovery (folder_id, original_path, archive_path, snapshot_json, byte_count) VALUES (?, ?, ?, ?, ?)")
        .bind(identifier).bind(source.to_string_lossy().as_ref()).bind(archive.to_string_lossy().as_ref()).bind(snapshot).bind(byte_count)
        .execute(&mut **transaction).await?;
    Ok(())
}

/// Plans an exclusive move back to the original location with catalog compensation.
///
/// # Errors
/// Rejects occupied destinations, missing parents, invalid archives and metadata conflicts.
pub(crate) async fn restore(
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
    super::validation::available(transaction, &original).await?;
    if !tokio::fs::symlink_metadata(&archive).await?.is_dir() {
        return Err(AppError::ValidationFailed(
            "Recovery copy is unavailable or is not a directory".into(),
        ));
    }
    let parent_path = original
        .parent()
        .ok_or_else(|| AppError::ValidationFailed("Original parent is unavailable".into()))?;
    let parent_id = crate::infra::database::handlers::shared::resolve_folder_id_robust(
        transaction,
        parent_path,
    )
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
    sqlx::query("UPDATE library_operation_receipts SET cleanup_pending = 0, cleanup_assets_json = '[]' WHERE entity_id = ? AND operation_kind = 'deleteFolder'").bind(identifier).execute(&mut **transaction).await?;
    changes.restored_assets = snapshot::restore(transaction, &snapshot).await?;
    super::identities::restore_from_archive(transaction, identifier, &original, &archive).await?;
    sqlx::query("DELETE FROM folder_recovery WHERE folder_id = ?")
        .bind(identifier)
        .execute(&mut **transaction)
        .await?;
    changes.moves.push((archive, original.clone()));
    sqlx::query("UPDATE folders SET parent_id = ? WHERE id = ?")
        .bind(&parent_id)
        .bind(identifier)
        .execute(&mut **transaction)
        .await?;
    crate::infra::database::handlers::shared::log_operation(
        transaction,
        "RESTORE_FOLDER",
        identifier,
        serde_json::json!({ "path": original, "parentId": parent_id }),
        "COMPLETED",
        None,
    )
    .await?;
    let name = original
        .file_name()
        .ok_or_else(|| AppError::ValidationFailed("Invalid recovery name".into()))?
        .to_string_lossy()
        .into_owned();
    Ok(super::projection::folder(
        identifier.into(),
        name,
        original,
        Some(parent_id),
    ))
}

/// Backfills legacy sizes outside a write transaction and never deletes retained archives.
///
/// # Errors
/// Returns inventory or persistence failures; incomplete rows remain retryable.
pub(crate) async fn refresh_sizes(pool: &sqlx::SqlitePool) -> AppResult<()> {
    for (identifier, path) in
        crate::infra::database::query_handlers::library_recovery::unmeasured(pool).await?
    {
        let path = Path::new(&path);
        let bytes = if tokio::fs::try_exists(path).await? {
            crate::infra::filesystem::folder_recovery::measure(path).await?
        } else {
            0
        };
        sqlx::query(
            "UPDATE folder_recovery SET byte_count = ? WHERE folder_id = ? AND byte_count < 0",
        )
        .bind(bytes)
        .bind(identifier)
        .execute(pool)
        .await?;
    }
    Ok(())
}
