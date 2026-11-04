//! Atomic explicit retries preserve current outputs and ownership fences.

use crate::core::{processing::diagnostics::*, processing::ProcessingKind, AppResult};
use crate::infra::database::processing::selection::{dependency, ACTIVE_ASSET, SOURCE_STAMP};
use sqlx::SqlitePool;

/// Resets selected terminal outcomes without clearing published outputs or live leases.
///
/// # Errors
/// Propagates storage failures; asset and comparison resets roll back together.
pub(crate) async fn retry(
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
