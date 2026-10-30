//! SQLite-backed pending work, generation checks and bounded processing retries.

use crate::core::error::AppResult;
use crate::core::processing::{
    ProcessingKind, ProcessingOutcome, ProcessingRepository, ProcessingTicket,
    ThumbnailProcessingState, ThumbnailProcessingStatus,
};
use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

const SOURCE_STAMP: &str = "a.path || '|' || a.file_size || '|' || COALESCE(a.modified_at, '')";
const ACTIVE_ASSET: &str = "a.deleted_at IS NULL AND LOWER(a.state) NOT IN ('trashed', 'offline')";

/// Persists stage outcomes separately from the asset's aggregate lifecycle.
pub struct SqliteProcessingRepository {
    pool: SqlitePool,
}

impl SqliteProcessingRepository {
    /// Uses the existing application pool so asset removal cascades to pending work.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn pending_selection(kind: ProcessingKind) -> String {
    let missing_result = match kind {
        ProcessingKind::Thumbnail => "(a.thumbnail_path IS NULL OR a.thumbnail_path = '')",
        ProcessingKind::Colors => "NOT EXISTS (SELECT 1 FROM asset_colors WHERE asset_id = a.id)",
        ProcessingKind::Fingerprint => "(df.asset_id IS NULL OR df.content_hash LIKE 'pending_%' OR df.content_hash LIKE 'hash_%' OR df.fingerprint_version < 5)",
    };
    let dependency = if kind == ProcessingKind::Colors {
        "AND a.thumbnail_path IS NOT NULL AND a.thumbnail_path != ''"
    } else {
        ""
    };
    format!(
        "FROM assets a
         LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = ?1
         LEFT JOIN duplicate_fingerprints df ON df.asset_id = a.id
         WHERE {ACTIVE_ASSET} {dependency}
         AND ({missing_result} OR task.state IN ('pending', 'retry_wait') OR task.source_stamp != ({SOURCE_STAMP}))
         AND (task.asset_id IS NULL OR task.source_stamp != ({SOURCE_STAMP})
              OR task.state = 'pending'
              OR (task.state = 'succeeded' AND {missing_result})
              OR (task.state = 'retry_wait' AND task.next_attempt_at <= unixepoch()))"
    )
}

/// Records explicit intent in the same transaction as the requesting Ledger command.
///
/// # Errors
/// Returns a database error if the request cannot be persisted.
pub(crate) async fn enqueue(
    transaction: &mut Transaction<'_, Sqlite>,
    kind: ProcessingKind,
    asset_id: &str,
) -> AppResult<()> {
    let statement = format!(
        "INSERT INTO processing_tasks (asset_id, kind, state, source_stamp)
         SELECT a.id, ?1, 'pending', {SOURCE_STAMP} FROM assets a WHERE a.id = ?2
         ON CONFLICT(asset_id, kind) DO UPDATE SET state = 'pending',
            generation = processing_tasks.generation + 1, source_stamp = excluded.source_stamp,
            attempts = 0, next_attempt_at = NULL, last_error = NULL, updated_at = CURRENT_TIMESTAMP"
    );
    sqlx::query(&statement)
        .bind(kind.as_str())
        .bind(asset_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

#[async_trait]
impl ProcessingRepository for SqliteProcessingRepository {
    async fn thumbnail_statuses(
        &self,
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
            .fetch_all(&self.pool)
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

    async fn pending(&self, kind: ProcessingKind, limit: u32) -> AppResult<Vec<String>> {
        let statement = format!(
            "SELECT a.id {} ORDER BY CASE WHEN task.state = 'retry_wait' THEN 0 ELSE 1 END,
                COALESCE(task.next_attempt_at, 0), a.added_at, a.id LIMIT ?2",
            pending_selection(kind)
        );
        Ok(sqlx::query_scalar(&statement)
            .bind(kind.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?)
    }

    async fn begin(
        &self,
        kind: ProcessingKind,
        asset_id: &str,
    ) -> AppResult<Option<ProcessingTicket>> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let statement = format!(
            "SELECT a.id, {SOURCE_STAMP} {} AND a.id = ?2 LIMIT 1",
            pending_selection(kind)
        );
        let selected: Option<(String, String)> = sqlx::query_as(&statement)
            .bind(kind.as_str())
            .bind(asset_id)
            .fetch_optional(&mut *transaction)
            .await?;
        let Some((asset_id, source_stamp)) = selected else {
            return Ok(None);
        };
        let generation: i64 = sqlx::query_scalar(
            "INSERT INTO processing_tasks (asset_id, kind, state, source_stamp)
             VALUES (?, ?, 'pending', ?)
             ON CONFLICT(asset_id, kind) DO UPDATE SET state = 'pending',
                generation = processing_tasks.generation + (processing_tasks.source_stamp != excluded.source_stamp),
                attempts = CASE WHEN processing_tasks.source_stamp != excluded.source_stamp THEN 0 ELSE processing_tasks.attempts END,
                source_stamp = excluded.source_stamp, updated_at = CURRENT_TIMESTAMP
             RETURNING generation",
        ).bind(&asset_id).bind(kind.as_str()).bind(&source_stamp)
            .fetch_one(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(ProcessingTicket {
            asset_id,
            kind,
            generation,
            source_stamp,
        }))
    }

    async fn is_current(&self, ticket: &ProcessingTicket) -> AppResult<bool> {
        let statement = format!(
            "SELECT EXISTS (SELECT 1 FROM assets a JOIN processing_tasks task ON task.asset_id = a.id
             WHERE a.id = ? AND task.kind = ? AND task.generation = ?
               AND task.source_stamp = ? AND ({SOURCE_STAMP}) = ? AND {ACTIVE_ASSET})"
        );
        Ok(sqlx::query_scalar(&statement)
            .bind(&ticket.asset_id)
            .bind(ticket.kind.as_str())
            .bind(ticket.generation)
            .bind(&ticket.source_stamp)
            .bind(&ticket.source_stamp)
            .fetch_one(&self.pool)
            .await?)
    }

    async fn finish(&self, ticket: &ProcessingTicket, outcome: ProcessingOutcome) -> AppResult<()> {
        let (state, error) = match outcome {
            ProcessingOutcome::Completed => ("succeeded", None),
            ProcessingOutcome::Skipped(reason) => ("skipped", Some(reason)),
            ProcessingOutcome::Failed(error) => ("retry_wait", Some(error)),
        };
        let statement = format!(
            "UPDATE processing_tasks SET
                state = CASE WHEN ?1 = 'retry_wait' AND attempts + 1 >= 3 THEN 'failed' ELSE ?1 END,
                attempts = attempts + (?1 = 'retry_wait'),
                next_attempt_at = CASE WHEN ?1 = 'retry_wait' THEN unixepoch() + CASE attempts WHEN 0 THEN 5 WHEN 1 THEN 30 ELSE 120 END ELSE NULL END,
                last_error = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE asset_id = ?3 AND kind = ?4 AND generation = ?5 AND source_stamp = ?6
               AND EXISTS (SELECT 1 FROM assets a WHERE a.id = ?3 AND ({SOURCE_STAMP}) = ?6 AND {ACTIVE_ASSET})"
        );
        sqlx::query(&statement)
            .bind(state)
            .bind(error.map(|message| message.chars().take(1000).collect::<String>()))
            .bind(&ticket.asset_id)
            .bind(ticket.kind.as_str())
            .bind(ticket.generation)
            .bind(&ticket.source_stamp)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "processing_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "thumbnail_status_tests.rs"]
mod thumbnail_status_tests;
