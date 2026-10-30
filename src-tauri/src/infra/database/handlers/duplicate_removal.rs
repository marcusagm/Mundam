//! Reconciles duplicate groups before deletion effects can be lost to FK cascades.

use crate::core::error::{AppError, AppResult};
use sqlx::{Sqlite, Transaction};

/// Captures group identities while an asset's candidate links still exist.
///
/// # Errors
/// Returns a database error if affected groups cannot be read.
pub(crate) async fn collect_asset_groups(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM duplicate_groups
         WHERE canonical_asset_id = ?1
            OR id IN (SELECT group_id FROM duplicate_candidates WHERE asset_id = ?1)",
    )
    .bind(asset_id)
    .fetch_all(&mut **transaction)
    .await?)
}

/// Repairs surviving groups and deletes affected groups that no longer represent duplicates.
///
/// Counts, canonical membership and deletion are committed with asset removal.
/// Unrelated groups and their resolution history are left intact.
///
/// # Errors
/// Returns a database error if dependent groups cannot be reconciled.
pub(crate) async fn reconcile_groups(
    transaction: &mut Transaction<'_, Sqlite>,
    group_ids: &[String],
) -> AppResult<()> {
    if group_ids.is_empty() {
        return Ok(());
    }
    let serialized_ids = serde_json::to_string(group_ids).map_err(|error| {
        AppError::Internal(format!("Failed to encode duplicate group IDs: {error}"))
    })?;
    sqlx::query(
        "UPDATE duplicate_groups SET
            candidate_count = (SELECT COUNT(*) FROM duplicate_candidates WHERE group_id = duplicate_groups.id),
            canonical_asset_id = CASE
                WHEN EXISTS (SELECT 1 FROM duplicate_candidates
                             WHERE group_id = duplicate_groups.id AND asset_id = duplicate_groups.canonical_asset_id)
                THEN canonical_asset_id
                ELSE (SELECT asset_id FROM duplicate_candidates WHERE group_id = duplicate_groups.id
                      ORDER BY score DESC, asset_id LIMIT 1)
            END,
            updated_at = CURRENT_TIMESTAMP
         WHERE id IN (SELECT value FROM json_each(?))",
    )
    .bind(&serialized_ids)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "DELETE FROM duplicate_groups
         WHERE id IN (SELECT value FROM json_each(?)) AND candidate_count < 2",
    )
    .bind(&serialized_ids)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
