//! Bounded persistent recovery projections for the settings panel.
use crate::core::{error::AppResult, library::FolderRecoveryEntry};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
struct RecoveryRow {
    entry_id: String,
    folder_id: String,
    original_path: String,
    created_at: String,
    byte_count: i64,
    restorable: bool,
}

/// Reads at most fifty archived deletions, newest first.
///
/// # Errors
/// Returns database query failures without mutating recovery state.
pub(crate) async fn page(pool: &SqlitePool, offset: u32) -> AppResult<Vec<FolderRecoveryEntry>> {
    let rows: Vec<RecoveryRow> = sqlx::query_as("SELECT entry_id,folder_id,original_path,created_at,byte_count,restorable FROM (SELECT folder_id AS entry_id,folder_id,original_path,created_at,byte_count,1 AS restorable FROM folder_recovery UNION ALL SELECT operation_id AS entry_id,entity_id AS folder_id,confirmed_path AS original_path,created_at,0 AS byte_count,0 AS restorable FROM library_operation_receipts WHERE operation_kind = 'restoreFolder') ORDER BY created_at DESC,entry_id DESC LIMIT 50 OFFSET ?").bind(offset).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| FolderRecoveryEntry {
            entry_id: row.entry_id,
            folder_id: row.folder_id,
            original_path: row.original_path,
            created_at: row.created_at,
            byte_count: row.byte_count,
            restorable: row.restorable,
        })
        .collect())
}

/// Reads deletion preflight data without holding the catalog writer during inventory.
///
/// # Errors
/// Returns missing folders and database failures.
pub(crate) async fn deletion_source(
    pool: &SqlitePool,
    identifier: &str,
) -> AppResult<(String, Option<String>)> {
    sqlx::query_as("SELECT path,parent_id FROM folders WHERE id = ?")
        .bind(identifier)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::core::error::AppError::NotFound("Folder not found".into()))
}

/// Lists legacy archives whose size predates storage-budget accounting.
///
/// # Errors
/// Returns catalog read failures.
pub(crate) async fn unmeasured(pool: &SqlitePool) -> AppResult<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT folder_id,archive_path FROM folder_recovery WHERE byte_count < 0 LIMIT 32",
    )
    .fetch_all(pool)
    .await?)
}
