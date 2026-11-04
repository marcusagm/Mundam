//! Queue telemetry uses the executor's source identity, dependency and missing-result rules.

use crate::core::{
    error::AppResult, processing::control::ProcessingQueueCounts, processing::ProcessingKind,
};
use crate::infra::database::processing::selection::{
    dependency, missing_result, ACTIVE_ASSET, SOURCE_STAMP,
};
use sqlx::SqlitePool;

/// Separates durable outcomes from operations already owned by a worker.
///
/// # Errors
/// Propagates identifier encoding, query or row-decoding failures.
pub(crate) async fn queue_counts(
    pool: &SqlitePool,
    kind: ProcessingKind,
    active_assets: &[String],
) -> AppResult<ProcessingQueueCounts> {
    let statement = format!(
        "SELECT outcome, COUNT(*) FROM (
            SELECT CASE
                WHEN task.source_stamp IS NOT NULL AND task.source_stamp != ({SOURCE_STAMP}) THEN 'pending'
                WHEN task.lease_id IS NOT NULL AND task.lease_expires_at > unixepoch() THEN NULL
                WHEN task.state IN ('failed', 'skipped') THEN task.state
                WHEN task.state = 'retry_wait' AND task.next_attempt_at > unixepoch() THEN 'retry_wait'
                WHEN task.state IN ('pending', 'retry_wait') OR {missing} THEN 'pending'
                ELSE NULL END AS outcome
            FROM assets a
            LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = ?1
            LEFT JOIN duplicate_fingerprints df ON df.asset_id = a.id
            WHERE {ACTIVE_ASSET} {dependency}
                AND a.id NOT IN (SELECT value FROM json_each(?2))
        ) WHERE outcome IS NOT NULL GROUP BY outcome",
        missing = missing_result(kind), dependency = dependency(kind)
    );
    let identities = serde_json::to_string(active_assets)
        .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
    let rows: Vec<(String, i64)> = sqlx::query_as(&statement)
        .bind(kind.as_str())
        .bind(identities)
        .fetch_all(pool)
        .await?;
    let mut counts = ProcessingQueueCounts::default();
    for (outcome, count) in rows {
        let count = count as u64;
        match outcome.as_str() {
            "pending" => counts.pending = count,
            "retry_wait" => counts.retry_wait = count,
            "failed" => counts.failed = count,
            "skipped" => counts.skipped = count,
            _ => {}
        }
    }
    Ok(counts)
}
