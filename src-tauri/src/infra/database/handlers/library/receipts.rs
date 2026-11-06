//! Transactional receipts and idempotent post-commit acknowledgements.
use crate::core::{
    error::{AppError, AppResult},
    library::FileIdentity,
};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashSet;

/// Saves the commit proof alongside catalog mutations.
///
/// # Errors
/// Returns serialization, collision or persistence failures before disk execution.
pub(crate) async fn prepare(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_id: &str,
    entity_id: &str,
    kind: &str,
    confirmed_path: &std::path::Path,
    cleanup_assets: &HashSet<String>,
) -> AppResult<()> {
    sqlx::query("INSERT INTO library_operation_receipts(operation_id,entity_id,operation_kind,cleanup_assets_json,cleanup_pending,confirmed_path) VALUES (?,?,?,?,?,?)")
        .bind(operation_id).bind(entity_id).bind(kind)
        .bind(serde_json::to_string(cleanup_assets).map_err(|error| AppError::Internal(error.to_string()))?)
        .bind(!cleanup_assets.is_empty()).bind(confirmed_path.to_string_lossy().as_ref()).execute(&mut **transaction).await?;
    Ok(())
}

/// Retains creation ownership for safe empty-draft cancellation.
///
/// # Errors
/// Returns serialization or persistence failures that require disk compensation.
pub(crate) async fn creation(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_id: &str,
    identity: &Option<FileIdentity>,
) -> AppResult<()> {
    let identity =
        serde_json::to_string(identity).map_err(|error| AppError::Internal(error.to_string()))?;
    sqlx::query(
        "UPDATE library_operation_receipts SET created_identity_json = ? WHERE operation_id = ?",
    )
    .bind(identity)
    .bind(operation_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Acknowledges completed cleanup; retries remain safe after a lost acknowledgement.
///
/// # Errors
/// Returns persistence errors while leaving the cleanup receipt retryable.
pub(crate) async fn cleaned(pool: &SqlitePool, operation_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE library_operation_receipts SET cleanup_pending = 0, cleanup_assets_json = '[]' WHERE operation_id = ?").bind(operation_id).execute(pool).await?;
    Ok(())
}
