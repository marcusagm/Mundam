//! Transactional hierarchy safeguards and effective color propagation.

use crate::core::error::{AppError, AppResult};
use sqlx::{Sqlite, Transaction};

/// Validates a parent change without exposing the tree to cycles.
///
/// # Errors
/// Returns validation errors for cycles and missing parents, or database errors.
pub async fn validate_parent(
    transaction: &mut Transaction<'_, Sqlite>,
    tag_id: &str,
    parent_id: &str,
) -> AppResult<()> {
    if parent_id.is_empty() || parent_id == "0" {
        return Ok(());
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?)")
        .bind(parent_id)
        .fetch_one(&mut **transaction)
        .await?;
    if !exists {
        return Err(AppError::ValidationFailed(
            "Parent tag does not exist.".into(),
        ));
    }
    let is_cycle: bool = sqlx::query_scalar(
        "WITH RECURSIVE descendants(id) AS (SELECT ? UNION SELECT tags.id FROM tags JOIN descendants ON tags.parent_id = descendants.id) SELECT EXISTS(SELECT 1 FROM descendants WHERE id = ?)"
    ).bind(tag_id).bind(parent_id).fetch_one(&mut **transaction).await?;
    if is_cycle {
        return Err(AppError::ValidationFailed(
            "A tag cannot be its own ancestor.".into(),
        ));
    }
    Ok(())
}

/// Recomputes effective colors through inherited branches, including custom boundaries.
///
/// # Errors
/// Returns database errors if the effective colors cannot be persisted.
pub async fn refresh_colors(transaction: &mut Transaction<'_, Sqlite>) -> AppResult<()> {
    sqlx::query(
        "WITH RECURSIVE effective(id, color) AS (
            SELECT id, CASE WHEN inherits_color THEN NULL ELSE color END FROM tags WHERE parent_id IS NULL
            UNION ALL
            SELECT child.id, CASE WHEN child.inherits_color THEN effective.color ELSE child.color END
            FROM tags child JOIN effective ON child.parent_id = effective.id
        ) UPDATE tags SET color = (SELECT color FROM effective WHERE effective.id = tags.id)
          WHERE inherits_color = TRUE"
    ).execute(&mut **transaction).await?;
    Ok(())
}
