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
pub(crate) async fn adopt_orphaned_children<'connection>(
    executor: impl Executor<'connection, Database = Sqlite>,
    parent_id: &str,
    parent_path: &str,
) -> AppResult<()> {
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let normalized_parent = parent_path.replace(separator, "/");
    let mut parent_prefix = format!("{}/", normalized_parent.trim_end_matches('/'));
    let comparison = if cfg!(windows) {
        if parent_prefix.as_bytes().get(1) == Some(&b':') {
            parent_prefix.replace_range(..1, &parent_prefix[..1].to_ascii_lowercase());
        }
        "CASE WHEN SUBSTR(normalized_path,2,1) = ':' THEN lower(SUBSTR(normalized_path,1,1)) || SUBSTR(normalized_path,2) ELSE normalized_path END"
    } else {
        "normalized_path"
    };
    sqlx::query(&format!("WITH candidates AS (SELECT id, REPLACE(path, ?, '/') AS normalized_path FROM folders WHERE parent_id IS NULL) UPDATE folders SET parent_id = ? WHERE id IN (SELECT id FROM candidates WHERE id != ? AND SUBSTR({comparison},1,LENGTH(?)) = ? AND LENGTH(normalized_path) > LENGTH(?) AND INSTR(SUBSTR(normalized_path,LENGTH(?) + 1),'/') = 0)"))
        .bind(separator).bind(parent_id).bind(parent_id).bind(&parent_prefix).bind(&parent_prefix).bind(&parent_prefix).bind(&parent_prefix)
        .execute(executor).await?;

    Ok(())
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
