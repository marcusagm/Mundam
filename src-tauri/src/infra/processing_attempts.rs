//! Ownership predicates shared by lease renewal, bookkeeping and transactional result commits.

use super::{ACTIVE_ASSET, SOURCE_STAMP};
use crate::core::{
    processing::{ProcessingOutcome, ProcessingTicket},
    AppResult,
};
use sqlx::{Sqlite, SqlitePool, Transaction};

fn ownership() -> String {
    format!(
        "asset_id = ?1 AND kind = ?2 AND generation = ?3 AND source_stamp = ?4
        AND lease_id = ?5 AND lease_expires_at > unixepoch()
        AND EXISTS (SELECT 1 FROM assets a WHERE a.id = ?1
            AND ({SOURCE_STAMP}) = ?4 AND {ACTIVE_ASSET})"
    )
}

/// Checks ownership inside the caller's write transaction, before any derived mutation.
///
/// # Errors
/// Propagates database read failures.
pub(crate) async fn is_current(
    transaction: &mut Transaction<'_, Sqlite>,
    ticket: &ProcessingTicket,
) -> AppResult<bool> {
    let statement = format!(
        "SELECT EXISTS (SELECT 1 FROM processing_tasks WHERE {})",
        ownership()
    );
    Ok(sqlx::query_scalar(&statement)
        .bind(&ticket.asset_id)
        .bind(ticket.kind.as_str())
        .bind(ticket.generation)
        .bind(&ticket.source_stamp)
        .bind(&ticket.lease_id)
        .fetch_one(&mut **transaction)
        .await?)
}

/// Finishes an owned attempt in the same transaction as its derived result.
///
/// # Errors
/// Propagates database writes; callers roll back the result when this fails.
pub(crate) async fn finish(
    transaction: &mut Transaction<'_, Sqlite>,
    ticket: &ProcessingTicket,
    outcome: ProcessingOutcome,
) -> AppResult<bool> {
    let (state, error) = match outcome {
        ProcessingOutcome::Completed => ("succeeded", None),
        ProcessingOutcome::Skipped(reason) => ("skipped", Some(reason)),
        ProcessingOutcome::Failed(error) => ("retry_wait", Some(error)),
    };
    let statement = format!("UPDATE processing_tasks SET
        state = CASE WHEN ?6 = 'retry_wait' AND attempts + 1 >= 3 THEN 'failed' ELSE ?6 END,
        attempts = attempts + (?6 = 'retry_wait'),
        next_attempt_at = CASE WHEN ?6 = 'retry_wait' THEN unixepoch() + CASE attempts WHEN 0 THEN 5 WHEN 1 THEN 30 ELSE 120 END ELSE NULL END,
        last_error = ?7, lease_id = NULL, lease_expires_at = NULL, updated_at = CURRENT_TIMESTAMP
        WHERE {}", ownership());
    Ok(sqlx::query(&statement)
        .bind(&ticket.asset_id)
        .bind(ticket.kind.as_str())
        .bind(ticket.generation)
        .bind(&ticket.source_stamp)
        .bind(&ticket.lease_id)
        .bind(state)
        .bind(error.map(|message| message.chars().take(1000).collect::<String>()))
        .execute(&mut **transaction)
        .await?
        .rows_affected()
        == 1)
}

pub(super) async fn renew(pool: &SqlitePool, ticket: &ProcessingTicket) -> AppResult<bool> {
    let statement = format!(
        "UPDATE processing_tasks SET lease_expires_at = unixepoch() + 120
        WHERE {}",
        ownership()
    );
    Ok(sqlx::query(&statement)
        .bind(&ticket.asset_id)
        .bind(ticket.kind.as_str())
        .bind(ticket.generation)
        .bind(&ticket.source_stamp)
        .bind(&ticket.lease_id)
        .execute(pool)
        .await?
        .rows_affected()
        == 1)
}

pub(super) async fn release(pool: &SqlitePool, ticket: &ProcessingTicket) -> AppResult<()> {
    sqlx::query(
        "UPDATE processing_tasks SET lease_id = NULL, lease_expires_at = NULL
        WHERE asset_id = ? AND kind = ? AND generation = ? AND lease_id = ?",
    )
    .bind(&ticket.asset_id)
    .bind(ticket.kind.as_str())
    .bind(ticket.generation)
    .bind(&ticket.lease_id)
    .execute(pool)
    .await?;
    Ok(())
}
