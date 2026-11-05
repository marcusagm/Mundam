//! Capability observations retain ordering with extraction outcomes through revision fencing.

use crate::core::{processing::tools::ExtractionToolCheck, AppResult};
use sqlx::SqlitePool;

/// Applies a complete probe batch without overriding failures observed after probing began.
/// # Errors
/// Rolls back the batch on storage failures.
pub(crate) async fn record(pool: &SqlitePool, checks: Vec<ExtractionToolCheck>) -> AppResult<()> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    for check in checks {
        sqlx::query("UPDATE processing_tool_health SET state = ?, revision = revision + 1, checked_at = unixepoch()
            WHERE tool = ? AND revision = ?")
            .bind(if check.available { "available" } else { "unavailable" })
            .bind(check.tool.as_str()).bind(check.expected_revision)
            .execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}
