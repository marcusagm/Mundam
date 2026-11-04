//! SQLite ownership and immutable input snapshots for coalesced duplicate comparisons.

use super::SqliteDuplicatesRepository;
use crate::core::{matching::*, AppError, AppResult};
use async_trait::async_trait;
use sqlx::Row;

#[path = "matching_inputs.rs"]
mod inputs;
#[path = "matching_publication.rs"]
mod publication;

#[async_trait]
impl MatchingRepository for SqliteDuplicatesRepository {
    async fn matching_status(&self) -> AppResult<MatchingStatus> {
        let rows = sqlx::query(
            "SELECT kind, state, manual_run_id, cancel_requested,
            requested_revision <= completed_revision AS completed,
            lease_id IS NOT NULL AND lease_expires_at > unixepoch() AS leased,
            lease_manual_run_id, next_attempt_at > unixepoch() AS deferred
            FROM duplicate_matching_jobs",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut status = MatchingStatus::default();
        status.publication_sequence = sqlx::query_scalar::<_, i64>(
            "SELECT publication_sequence FROM duplicate_matching_revision WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await? as u64;
        for row in rows {
            let leased: bool = row.try_get("leased")?;
            let manual: Option<String> = row.try_get("manual_run_id")?;
            let leased_manual: Option<String> = row.try_get("lease_manual_run_id")?;
            let kind: String = row.try_get("kind")?;
            if row.try_get::<bool, _>("completed")? && !leased {
                status.completed.push(kind);
                continue;
            }
            if manual.is_some() || (leased && leased_manual.is_some()) {
                status.manual_pending = true;
                status.manual_cancel_requested |= row.try_get::<bool, _>("cancel_requested")?;
                if leased {
                    status.manual_phase = Some(row.try_get("kind")?);
                }
            }
            if !leased {
                match row.try_get::<String, _>("state")?.as_str() {
                    "failed" => {
                        status.failed += 1;
                        status.failed_phases.push(kind);
                    }
                    "retry_wait"
                        if row.try_get::<Option<bool>, _>("deferred")?.unwrap_or(false) =>
                    {
                        status.retry_wait += 1
                    }
                    _ => status.pending += 1,
                }
            }
        }
        Ok(status)
    }

    async fn request_matching(&self, run_id: &str) -> AppResult<()> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
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

    async fn claim_matching(
        &self,
        kind: MatchingKind,
        automatic_enabled: bool,
    ) -> AppResult<Option<MatchingInput>> {
        super::super::query_handlers::duplicates_queries::get_active_rule_set(&self.pool).await?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
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
        let policy_json = serde_json::to_string(&policy)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        sqlx::query("UPDATE duplicate_matching_jobs SET lease_id = ?, lease_expires_at = unixepoch() + 120,
            lease_manual_run_id = manual_run_id, cancel_requested = 0, policy_json = ? WHERE kind = ?")
            .bind(&ticket.lease_id).bind(policy_json).bind(kind.as_str()).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(MatchingInput {
            ticket,
            policy,
            fingerprints,
        }))
    }

    async fn renew_matching(&self, ticket: &MatchingTicket) -> AppResult<bool> {
        Ok(sqlx::query("UPDATE duplicate_matching_jobs SET lease_expires_at = unixepoch() + 120
            WHERE kind = ? AND lease_id = ? AND requested_revision = ? AND cancel_requested = 0
            AND lease_expires_at > unixepoch()
            AND requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1)")
            .bind(ticket.kind.as_str()).bind(&ticket.lease_id).bind(ticket.revision)
            .execute(&self.pool).await?.rows_affected() == 1)
    }

    async fn publish_matching(
        &self,
        input: &MatchingInput,
        groups: Vec<MatchingGroup>,
    ) -> AppResult<bool> {
        publication::publish(&self.pool, input, groups).await
    }

    async fn release_matching(
        &self,
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
            .execute(&self.pool).await?;
        Ok(())
    }

    async fn cancel_matching(&self, run_id: Option<&str>) -> AppResult<()> {
        sqlx::query("UPDATE duplicate_matching_jobs SET manual_run_id = NULL,
            cancel_requested = 1,
            state = CASE WHEN automatic_requested = 1 THEN 'pending' ELSE 'succeeded' END,
            completed_revision = CASE WHEN automatic_requested = 0 THEN requested_revision ELSE completed_revision END
            WHERE (?1 IS NULL OR manual_run_id = ?1) AND manual_run_id IS NOT NULL")
            .bind(run_id).execute(&self.pool).await?;
        Ok(())
    }
}
