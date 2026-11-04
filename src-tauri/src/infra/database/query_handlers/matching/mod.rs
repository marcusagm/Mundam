//! Read-only status and transactional input projections for matching.

use crate::core::{duplicates::matching::*, AppResult};
use sqlx::{Row, SqlitePool};
pub(crate) mod inputs;

/// Projects durable comparison intent independently of transient worker events.
///
/// # Errors
/// Propagates comparison-status query and row-decoding failures.
pub(crate) async fn matching_status(pool: &SqlitePool) -> AppResult<MatchingStatus> {
    let rows = sqlx::query(
        "SELECT kind, state, manual_run_id, cancel_requested,
        requested_revision <= completed_revision AS completed,
        lease_id IS NOT NULL AND lease_expires_at > unixepoch() AS leased,
        lease_manual_run_id, next_attempt_at > unixepoch() AS deferred
        FROM duplicate_matching_jobs",
    )
    .fetch_all(pool)
    .await?;
    let mut status = MatchingStatus::default();
    status.publication_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT publication_sequence FROM duplicate_matching_revision WHERE id = 1",
    )
    .fetch_one(pool)
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
                "retry_wait" if row.try_get::<Option<bool>, _>("deferred")?.unwrap_or(false) => {
                    status.retry_wait += 1
                }
                _ => status.pending += 1,
            }
        }
    }
    Ok(status)
}
