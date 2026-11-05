//! Finite grouped processing causes, without exposing private extractor logs.

use crate::core::{processing::diagnostics::*, processing::ProcessingKind, AppResult};
use crate::infra::database::processing::selection::{
    dependency, ACTIVE_ASSET, SOURCE_STAMP, TOOL_READY, TOOL_WAIT,
};
use sqlx::{Row, SqlitePool};

/// Bounds diagnostic groups to safe codes for active current sources.
///
/// # Errors
/// Propagates whitelist encoding, query or row-decoding failures.
pub(crate) async fn diagnostics(
    pool: &SqlitePool,
    kind: ProcessingKind,
) -> AppResult<Vec<ProcessingDiagnostic>> {
    let allowed = serde_json::to_string(&ProcessingIssue::CODES)
        .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
    let statement = format!("SELECT CASE WHEN task.issue_code IN (SELECT value FROM json_each(?2)) THEN task.issue_code
            WHEN task.state = 'skipped' THEN 'unknown_unsupported' ELSE 'unknown_failure' END AS issue,
            CASE WHEN {TOOL_WAIT} AND NOT {TOOL_READY} THEN 'blocked' ELSE task.state END AS outcome, COUNT(*) AS count
        FROM processing_tasks task JOIN assets a ON a.id = task.asset_id
        WHERE task.kind = ?1 AND task.state IN ('failed', 'skipped', 'retry_wait')
          AND {ACTIVE_ASSET} {} AND task.source_stamp = ({SOURCE_STAMP})
          AND (task.lease_id IS NULL OR task.lease_expires_at <= unixepoch())
        GROUP BY issue, outcome ORDER BY outcome, issue", dependency(kind));
    sqlx::query(&statement)
        .bind(kind.as_str())
        .bind(allowed)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ProcessingDiagnostic {
                issue: row.try_get("issue")?,
                state: row.try_get("outcome")?,
                count: row.try_get::<i64, _>("count")? as u64,
            })
        })
        .collect()
}
