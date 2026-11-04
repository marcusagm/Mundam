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
    let missing_result = missing_result(kind);
    let dependency = dependency(kind);
    format!(
        "FROM assets a
         LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = ?1
         LEFT JOIN duplicate_fingerprints df ON df.asset_id = a.id
         WHERE {ACTIVE_ASSET} {dependency}
         AND ({missing_result} OR task.state IN ('pending', 'retry_wait') OR task.source_stamp != ({SOURCE_STAMP}))
         AND (task.asset_id IS NULL OR task.source_stamp != ({SOURCE_STAMP})
              OR task.state = 'pending'
              OR (task.state = 'succeeded' AND {missing_result})
              OR (task.state = 'retry_wait' AND task.next_attempt_at <= unixepoch()))
         AND (task.lease_id IS NULL OR task.lease_expires_at <= unixepoch()
              OR task.source_stamp != ({SOURCE_STAMP}))"
    )
}

fn missing_result(kind: ProcessingKind) -> &'static str {
    match kind {
        ProcessingKind::Thumbnail => "(a.thumbnail_path IS NULL OR a.thumbnail_path = '')",
        ProcessingKind::Colors => "NOT EXISTS (SELECT 1 FROM asset_colors WHERE asset_id = a.id)",
        ProcessingKind::Fingerprint => "(df.asset_id IS NULL OR df.content_hash LIKE 'pending_%' OR df.content_hash LIKE 'hash_%' OR df.fingerprint_version < 5)",
    }
}

fn dependency(kind: ProcessingKind) -> &'static str {
    if kind == ProcessingKind::Colors {
        "AND a.thumbnail_path IS NOT NULL AND a.thumbnail_path != ''"
    } else {
        ""
    }
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
            attempts = 0, next_attempt_at = NULL, last_error = NULL, issue_code = NULL,
            lease_id = NULL, lease_expires_at = NULL, updated_at = CURRENT_TIMESTAMP"
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
    async fn diagnostics(
        &self,
        kind: ProcessingKind,
    ) -> AppResult<Vec<crate::core::processing_diagnostics::ProcessingDiagnostic>> {
        diagnostics::diagnostics(&self.pool, kind).await
    }

    async fn retry(
        &self,
        kind: ProcessingKind,
        scope: crate::core::processing_diagnostics::ProcessingRetryScope,
    ) -> AppResult<crate::core::processing_diagnostics::ProcessingRetryCounts> {
        diagnostics::retry(&self.pool, kind, scope).await
    }
    async fn queue_counts(
        &self,
        kind: ProcessingKind,
        active_assets: &[String],
    ) -> AppResult<crate::core::processing_control::ProcessingQueueCounts> {
        counts::queue_counts(&self.pool, kind, active_assets).await
    }
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
        let lease_id = uuid::Uuid::new_v4().to_string();
        let generation: i64 = sqlx::query_scalar(
            "INSERT INTO processing_tasks (asset_id, kind, state, source_stamp, lease_id, lease_expires_at)
             VALUES (?, ?, 'pending', ?, ?, unixepoch() + 120)
             ON CONFLICT(asset_id, kind) DO UPDATE SET state = 'pending',
                generation = processing_tasks.generation + (processing_tasks.source_stamp != excluded.source_stamp),
                attempts = CASE WHEN processing_tasks.source_stamp != excluded.source_stamp THEN 0 ELSE processing_tasks.attempts END,
                source_stamp = excluded.source_stamp, lease_id = excluded.lease_id,
                lease_expires_at = excluded.lease_expires_at, updated_at = CURRENT_TIMESTAMP
             RETURNING generation",
        ).bind(&asset_id).bind(kind.as_str()).bind(&source_stamp).bind(&lease_id)
            .fetch_one(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(ProcessingTicket {
            asset_id,
            kind,
            generation,
            source_stamp,
            lease_id,
        }))
    }

    async fn is_current(&self, ticket: &ProcessingTicket) -> AppResult<bool> {
        let mut transaction = self.pool.begin().await?;
        attempts::is_current(&mut transaction, ticket).await
    }

    async fn renew(&self, ticket: &ProcessingTicket) -> AppResult<bool> {
        attempts::renew(&self.pool, ticket).await
    }

    async fn release(&self, ticket: &ProcessingTicket) -> AppResult<()> {
        attempts::release(&self.pool, ticket).await
    }

    async fn preview_path(&self, asset_id: &str) -> AppResult<Option<String>> {
        Ok(sqlx::query_scalar(
            "SELECT preview_path FROM processing_tasks WHERE asset_id = ? AND kind = 'thumbnail'",
        )
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten())
    }

    async fn finish(&self, ticket: &ProcessingTicket, outcome: ProcessingOutcome) -> AppResult<()> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        attempts::finish(&mut transaction, ticket, outcome).await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[path = "processing_counts.rs"]
mod counts;

#[path = "processing_diagnostics.rs"]
mod diagnostics;

#[cfg(test)]
#[path = "processing_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "thumbnail_status_tests.rs"]
mod thumbnail_status_tests;

#[path = "processing_attempts.rs"]
pub(crate) mod attempts;

#[cfg(test)]
#[path = "processing_lease_tests.rs"]
mod lease_tests;

#[cfg(test)]
#[path = "processing_diagnostics_tests.rs"]
mod diagnostic_tests;
