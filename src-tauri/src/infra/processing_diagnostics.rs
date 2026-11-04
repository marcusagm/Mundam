//! Aggregated read model and explicit retry mutations on the application's shared SQLite pool.

use super::{dependency, ACTIVE_ASSET, SOURCE_STAMP};
use crate::core::{processing::ProcessingKind, processing_diagnostics::*, AppResult};
use sqlx::{Row, SqlitePool};

pub(super) async fn diagnostics(
    pool: &SqlitePool,
    kind: ProcessingKind,
) -> AppResult<Vec<ProcessingDiagnostic>> {
    let allowed = serde_json::to_string(&ProcessingIssue::CODES)
        .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
    let statement = format!("SELECT CASE WHEN task.issue_code IN (SELECT value FROM json_each(?2)) THEN task.issue_code
            WHEN task.state = 'skipped' THEN 'unknown_unsupported' ELSE 'unknown_failure' END AS issue,
            task.state, COUNT(*) AS count
        FROM processing_tasks task JOIN assets a ON a.id = task.asset_id
        WHERE task.kind = ?1 AND task.state IN ('failed', 'skipped', 'retry_wait')
          AND {ACTIVE_ASSET} {} AND task.source_stamp = ({SOURCE_STAMP})
          AND (task.lease_id IS NULL OR task.lease_expires_at <= unixepoch())
        GROUP BY issue, task.state ORDER BY task.state, issue", dependency(kind));
    sqlx::query(&statement)
        .bind(kind.as_str())
        .bind(allowed)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ProcessingDiagnostic {
                issue: row.try_get("issue")?,
                state: row.try_get("state")?,
                count: row.try_get::<i64, _>("count")? as u64,
            })
        })
        .collect()
}

pub(super) async fn retry(
    pool: &SqlitePool,
    kind: ProcessingKind,
    scope: ProcessingRetryScope,
) -> AppResult<ProcessingRetryCounts> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let state = match scope {
        ProcessingRetryScope::Failed => "failed",
        ProcessingRetryScope::Unsupported => "skipped",
    };
    let statement = format!("UPDATE processing_tasks SET state = 'pending', generation = generation + 1,
        attempts = 0, next_attempt_at = NULL, last_error = NULL, issue_code = NULL,
        lease_id = NULL, lease_expires_at = NULL, updated_at = CURRENT_TIMESTAMP
        WHERE kind = ?1 AND asset_id IN (
            SELECT a.id FROM assets a JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = ?1
            WHERE {ACTIVE_ASSET} {} AND task.state = ?2 AND task.source_stamp = ({SOURCE_STAMP})
            AND (task.lease_id IS NULL OR task.lease_expires_at <= unixepoch()))", dependency(kind));
    let assets = sqlx::query(&statement)
        .bind(kind.as_str())
        .bind(state)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    let comparisons = if kind == ProcessingKind::Fingerprint
        && scope == ProcessingRetryScope::Failed
    {
        sqlx::query("UPDATE duplicate_matching_jobs SET state = 'pending', attempts = 0, next_attempt_at = NULL,
            last_error = NULL, automatic_requested = 1, cancel_requested = 0,
            lease_id = NULL, lease_expires_at = NULL, lease_manual_run_id = NULL
            WHERE state = 'failed' AND (lease_id IS NULL OR lease_expires_at <= unixepoch())")
            .execute(&mut *transaction).await?.rows_affected()
    } else {
        0
    };
    transaction.commit().await?;
    Ok(ProcessingRetryCounts {
        assets,
        comparisons,
    })
}
