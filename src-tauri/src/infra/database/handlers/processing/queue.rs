//! Queue intent and lease admission, retaining each existing transaction boundary.

use crate::core::{processing::*, AppResult};
use crate::infra::database::processing::selection::*;
use sqlx::SqlitePool;

use sqlx::{Sqlite, Transaction};

/// Records explicit stage intent within the caller's asset transaction.
///
/// # Errors
/// Propagates writes so the caller can roll back its asset mutation.
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

/// Claims only eligible work under the same transaction that installs its lease.
///
/// # Errors
/// Propagates admission query, write or commit failures.
pub(crate) async fn begin(
    pool: &SqlitePool,
    kind: ProcessingKind,
    asset_id: &str,
) -> AppResult<Option<ProcessingTicket>> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
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
