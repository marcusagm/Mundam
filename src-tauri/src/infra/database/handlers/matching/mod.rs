//! Durable matching admission, retry and cancellation mutations.

use crate::core::{duplicates::matching::*, AppError, AppResult};
use crate::infra::database::query_handlers::matching::inputs;
use sqlx::{Row, SqlitePool};
pub(crate) mod publication;

/// Coalesces manual phase intent without taking an existing live reservation.
///
/// # Errors
/// Rejects a conflicting manual run or propagates storage failures.
pub(crate) async fn request_matching(pool: &SqlitePool, run_id: &str) -> AppResult<()> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM duplicate_matching_jobs
        WHERE (manual_run_id IS NOT NULL AND manual_run_id != ?1 AND state != 'failed')
           OR (lease_manual_run_id IS NOT NULL AND lease_manual_run_id != ?1 AND lease_expires_at > unixepoch()))")
        .bind(run_id).fetch_one(&mut *transaction).await?;
    if occupied {
        return Err(AppError::ValidationFailed(
            "A manual duplicate comparison is already pending".into(),
        ));
    }
    sqlx::query("UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        state = 'pending', manual_run_id = ?, cancel_requested = 0, attempts = 0, next_attempt_at = NULL, last_error = NULL")
        .bind(run_id).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(())
}

/// Captures policy and inputs atomically with a new comparison reservation.
///
/// # Errors
/// Propagates default-policy, snapshot, reservation or commit failures.
pub(crate) async fn claim_matching(
    pool: &SqlitePool,
    kind: MatchingKind,
    automatic_enabled: bool,
) -> AppResult<Option<MatchingInput>> {
    crate::infra::database::handlers::duplicates_handler::ensure_default_rule_set(pool).await?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let row = sqlx::query(
        "SELECT requested_revision, manual_run_id FROM duplicate_matching_jobs
        WHERE kind = ? AND requested_revision > completed_revision
        AND (? OR manual_run_id IS NOT NULL)
        AND (lease_id IS NULL OR lease_expires_at <= unixepoch())
        AND (state = 'pending' OR (state = 'retry_wait' AND next_attempt_at <= unixepoch()))",
    )
    .bind(kind.as_str())
    .bind(automatic_enabled)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let ticket = MatchingTicket {
        kind,
        lease_id: uuid::Uuid::new_v4().to_string(),
        revision: row.try_get("requested_revision")?,
        manual_run_id: row.try_get("manual_run_id")?,
    };
    let policy = inputs::policy(&mut transaction).await?;
    let fingerprints = inputs::fingerprints(&mut transaction).await?;
    let policy_json =
        serde_json::to_string(&policy).map_err(|error| AppError::Internal(error.to_string()))?;
    sqlx::query(
        "UPDATE duplicate_matching_jobs SET lease_id = ?, lease_expires_at = unixepoch() + 120,
        lease_manual_run_id = manual_run_id, cancel_requested = 0, policy_json = ? WHERE kind = ?",
    )
    .bind(&ticket.lease_id)
    .bind(policy_json)
    .bind(kind.as_str())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Some(MatchingInput {
        ticket,
        policy,
        fingerprints,
    }))
}

/// Extends only current ownership rather than reviving obsolete computations.
///
/// # Errors
/// Propagates reservation update failures.
pub(crate) async fn renew_matching(pool: &SqlitePool, ticket: &MatchingTicket) -> AppResult<bool> {
    Ok(sqlx::query(
        "UPDATE duplicate_matching_jobs SET lease_expires_at = unixepoch() + 120
        WHERE kind = ? AND lease_id = ? AND requested_revision = ? AND cancel_requested = 0
        AND lease_expires_at > unixepoch()
        AND requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1)",
    )
    .bind(ticket.kind.as_str())
    .bind(&ticket.lease_id)
    .bind(ticket.revision)
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

/// Retains bounded retry state while releasing only the supplied reservation.
///
/// # Errors
/// Propagates reservation release failures.
pub(crate) async fn release_matching(
    pool: &SqlitePool,
    ticket: &MatchingTicket,
    error: Option<String>,
) -> AppResult<()> {
    let message = error.map(|message| message.chars().take(1000).collect::<String>());
    sqlx::query("UPDATE duplicate_matching_jobs SET
        state = CASE WHEN requested_revision != ?3 OR cancel_requested = 1 THEN state
            WHEN ?4 IS NOT NULL AND attempts + 1 >= 3 THEN 'failed'
            WHEN ?4 IS NOT NULL THEN 'retry_wait' ELSE state END,
        attempts = attempts + (?4 IS NOT NULL AND requested_revision = ?3 AND cancel_requested = 0),
        next_attempt_at = CASE WHEN ?4 IS NOT NULL AND requested_revision = ?3 AND cancel_requested = 0
            THEN unixepoch() + CASE attempts WHEN 0 THEN 5 ELSE 30 END ELSE next_attempt_at END,
        last_error = CASE WHEN requested_revision = ?3 AND cancel_requested = 0 THEN ?4 ELSE last_error END,
        manual_run_id = CASE WHEN ?4 IS NOT NULL AND requested_revision = ?3 AND attempts + 1 >= 3 THEN NULL ELSE manual_run_id END,
        lease_id = NULL, lease_expires_at = NULL, lease_manual_run_id = NULL, cancel_requested = 0
        WHERE kind = ?1 AND lease_id = ?2")
        .bind(ticket.kind.as_str()).bind(&ticket.lease_id).bind(ticket.revision).bind(message)
        .execute(pool).await?;
    Ok(())
}

/// Removes the requested manual intent while preserving independent automatic work.
///
/// # Errors
/// Propagates cancellation update failures.
pub(crate) async fn cancel_matching(pool: &SqlitePool, run_id: Option<&str>) -> AppResult<()> {
    sqlx::query("UPDATE duplicate_matching_jobs SET manual_run_id = NULL,
        cancel_requested = 1,
        state = CASE WHEN automatic_requested = 1 THEN 'pending' ELSE 'succeeded' END,
        completed_revision = CASE WHEN automatic_requested = 0 THEN requested_revision ELSE completed_revision END
        WHERE (?1 IS NULL OR manual_run_id = ?1) AND manual_run_id IS NOT NULL")
        .bind(run_id).execute(pool).await?;
    Ok(())
}
