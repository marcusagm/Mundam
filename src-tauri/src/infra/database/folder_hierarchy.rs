//! Shared folder adoption SQL keeps hierarchy mutations consistent across adapters.

use crate::core::error::AppResult;
use sqlx::{Executor, Sqlite};

/// Attaches existing roots only when their physical parent has been indexed.
///
/// Windows accepts both separators; Unix keeps backslashes as literal filename
/// characters. Prefix equality avoids treating `%` and `_` in names as SQL
/// wildcards. Restricting adoption to direct children lets depth-ordered scans
/// create intermediate folders without flattening previously monitored trees.
/// Stored paths remain unchanged for filesystem operations.
///
/// # Errors
/// Returns a database error if the hierarchy update cannot be persisted.
pub(super) async fn adopt_orphaned_children<'connection>(
    executor: impl Executor<'connection, Database = Sqlite>,
    parent_id: &str,
    parent_path: &str,
) -> AppResult<()> {
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let normalized_parent = parent_path.replace(separator, "/");
    let parent_prefix = format!("{}/", normalized_parent.trim_end_matches('/'));

    sqlx::query(
        r#"
        UPDATE folders
        SET parent_id = ?
        WHERE parent_id IS NULL
          AND id != ?
          AND SUBSTR(REPLACE(path, ?, '/'), 1, LENGTH(?)) = ? COLLATE NOCASE
          AND LENGTH(REPLACE(path, ?, '/')) > LENGTH(?)
          AND INSTR(SUBSTR(REPLACE(path, ?, '/'), LENGTH(?) + 1), '/') = 0
        "#,
    )
    .bind(parent_id)
    .bind(parent_id)
    .bind(separator)
    .bind(&parent_prefix)
    .bind(&parent_prefix)
    .bind(separator)
    .bind(&parent_prefix)
    .bind(separator)
    .bind(&parent_prefix)
    .execute(executor)
    .await?;

    Ok(())
}

#[cfg(test)]
#[path = "folder_hierarchy_tests.rs"]
mod tests;
