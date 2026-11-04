//! Read-only processing projections; eligibility is shared with admission.

use crate::core::{processing::*, AppResult};
use crate::infra::database::processing::selection::*;
use sqlx::SqlitePool;

mod counts;
mod diagnostics;
pub(crate) use counts::queue_counts;
pub(crate) use diagnostics::diagnostics;

/// Projects bounded asset outcomes without exposing extractor logs.
///
/// # Errors
/// Propagates identifier encoding, query or row-decoding failures.
pub(crate) async fn thumbnail_statuses(
    pool: &SqlitePool,
    asset_ids: &[String],
) -> AppResult<Vec<ThumbnailProcessingStatus>> {
    if asset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let identifiers = serde_json::to_string(asset_ids)
        .map_err(|error| crate::core::error::AppError::Internal(error.to_string()))?;
    let statement = format!(
        "SELECT a.id,
            CASE WHEN NOT ({ACTIVE_ASSET}) THEN 'unavailable'
                 WHEN task.source_stamp IS NOT NULL AND task.source_stamp != ({SOURCE_STAMP}) THEN 'pending'
                 WHEN task.state IN ('pending', 'retry_wait', 'failed', 'skipped') THEN task.state
                 WHEN a.thumbnail_path IS NOT NULL AND a.thumbnail_path != '' THEN 'succeeded'
                 ELSE 'pending' END,
            COALESCE(task.generation, 0), a.thumbnail_path
         FROM assets a LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = 'thumbnail'
         WHERE a.id IN (SELECT value FROM json_each(?))"
    );
    let rows: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(&statement)
        .bind(identifiers)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(asset_id, state, generation, thumbnail_path)| ThumbnailProcessingStatus {
                asset_id,
                generation,
                thumbnail_path,
                state: match state.as_str() {
                    "retry_wait" => ThumbnailProcessingState::RetryWait,
                    "failed" => ThumbnailProcessingState::Failed,
                    "skipped" => ThumbnailProcessingState::Skipped,
                    "succeeded" => ThumbnailProcessingState::Succeeded,
                    "unavailable" => ThumbnailProcessingState::Unavailable,
                    _ => ThumbnailProcessingState::Pending,
                },
            },
        )
        .collect())
}

/// Uses the admission predicates so read-side backlog cannot diverge from claims.
///
/// # Errors
/// Propagates pending-queue query failures.
pub(crate) async fn pending(
    pool: &SqlitePool,
    kind: ProcessingKind,
    limit: u32,
) -> AppResult<Vec<String>> {
    let statement = format!(
        "SELECT a.id {} ORDER BY CASE WHEN task.state = 'retry_wait' THEN 0 ELSE 1 END,
            COALESCE(task.next_attempt_at, 0), a.added_at, a.id LIMIT ?2",
        pending_selection(kind)
    );
    Ok(sqlx::query_scalar(&statement)
        .bind(kind.as_str())
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await?)
}

/// Resolves only the published preview pointer, not uncommitted attempt artifacts.
///
/// # Errors
/// Propagates preview-pointer query failures.
pub(crate) async fn preview_path(pool: &SqlitePool, asset_id: &str) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT preview_path FROM processing_tasks WHERE asset_id = ? AND kind = 'thumbnail'",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await?
    .flatten())
}
