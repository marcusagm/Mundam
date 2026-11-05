//! Read-only fixed-size inventory of observed extraction capabilities.

use crate::core::{processing::tools::*, AppResult};
use sqlx::{Row, SqlitePool};

/// Reads safe facts without spawning processes or altering queue eligibility.
/// # Errors
/// Propagates query and row-decoding failures.
pub(crate) async fn statuses(pool: &SqlitePool) -> AppResult<Vec<ExtractionToolStatus>> {
    let mut statuses = Vec::new();
    for tool in ExtractionTool::ALL {
        let row = sqlx::query(
            "SELECT state, revision, checked_at FROM processing_tool_health WHERE tool = ?",
        )
        .bind(tool.as_str())
        .fetch_one(pool)
        .await?;
        statuses.push(ExtractionToolStatus {
            tool,
            revision: row.try_get("revision")?,
            checked_at: row.try_get("checked_at")?,
            state: match row.try_get::<String, _>("state")?.as_str() {
                "available" => ToolAvailability::Available,
                "unavailable" => ToolAvailability::Unavailable,
                _ => ToolAvailability::Unknown,
            },
        });
    }
    Ok(statuses)
}
