//! Publishes complete comparisons without replacing reviewed history or partially clearing groups.

use crate::core::{matching::*, AppError, AppResult};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use std::collections::BTreeSet;

struct ExistingGroup {
    id: String,
    members: Vec<String>,
    canonical: Option<String>,
}

pub(super) async fn publish(
    pool: &SqlitePool,
    input: &MatchingInput,
    groups: Vec<MatchingGroup>,
) -> AppResult<bool> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if !current(&mut transaction, &input.ticket).await? {
        return Ok(false);
    }
    let group_types = match input.ticket.kind {
        MatchingKind::Exact => "'exact'",
        MatchingKind::Visual => "'visual', 'derived', 'near'",
    };
    let rows = sqlx::query(&format!("SELECT id, status, canonical_asset_id FROM duplicate_groups WHERE group_type IN ({group_types}) ORDER BY id"))
        .fetch_all(&mut *transaction).await?;
    let mut protected_assets = BTreeSet::new();
    let mut open_groups = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let members: Vec<String> = sqlx::query_scalar(
            "SELECT asset_id FROM duplicate_candidates WHERE group_id = ? ORDER BY asset_id",
        )
        .bind(&id)
        .fetch_all(&mut *transaction)
        .await?;
        if row.try_get::<String, _>("status")? != "open" {
            protected_assets.extend(members);
        } else {
            open_groups.push(ExistingGroup {
                id,
                members,
                canonical: row.try_get("canonical_asset_id")?,
            });
        }
    }
    let eligible: BTreeSet<&str> = input
        .fingerprints
        .iter()
        .map(|fingerprint| fingerprint.asset_id.as_str())
        .collect();
    let mut retained = BTreeSet::new();
    let mut signatures = BTreeSet::new();
    for mut group in groups {
        group
            .candidates
            .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
        let members: Vec<String> = group
            .candidates
            .iter()
            .map(|candidate| candidate.asset_id.clone())
            .collect();
        let valid_type = match input.ticket.kind {
            MatchingKind::Exact => {
                group.group_type == crate::core::models::DuplicateGroupType::Exact
            }
            MatchingKind::Visual => {
                group.group_type != crate::core::models::DuplicateGroupType::Exact
            }
        };
        if !valid_type
            || members.len() < 2
            || members.windows(2).any(|pair| pair[0] == pair[1])
            || !group.confidence.is_finite()
            || !(0.0..=1.0).contains(&group.confidence)
            || group.candidates.iter().any(|candidate| {
                !eligible.contains(candidate.asset_id.as_str())
                    || !candidate.score.is_finite()
                    || !(0.0..=1.0).contains(&candidate.score)
            })
            || !signatures.insert(members.clone())
        {
            return Err(AppError::ValidationFailed(
                "Invalid duplicate comparison result".into(),
            ));
        }
        if members
            .iter()
            .any(|member| protected_assets.contains(member))
        {
            continue;
        }
        let previous = open_groups
            .iter()
            .find(|existing| existing.members == members && !retained.contains(&existing.id));
        let id = previous
            .map(|existing| existing.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let canonical = previous
            .and_then(|existing| existing.canonical.clone())
            .filter(|canonical| members.contains(canonical))
            .unwrap_or_else(|| members[0].clone());
        retained.insert(id.clone());
        sqlx::query("INSERT INTO duplicate_groups (id, rule_set_id, group_type, canonical_asset_id, confidence, status, candidate_count, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'open', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET rule_set_id = excluded.rule_set_id, group_type = excluded.group_type,
                canonical_asset_id = excluded.canonical_asset_id, confidence = excluded.confidence,
                candidate_count = excluded.candidate_count, updated_at = CURRENT_TIMESTAMP")
            .bind(&id).bind(&input.policy.id).bind(group.group_type.to_string()).bind(canonical)
            .bind(group.confidence).bind(members.len() as i64).execute(&mut *transaction).await?;
        for candidate in group.candidates {
            let reasons = serde_json::to_string(&candidate.reasons)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            sqlx::query("INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons, is_selected) VALUES (?, ?, ?, ?, 0)
                ON CONFLICT(group_id, asset_id) DO UPDATE SET score = excluded.score, reasons = excluded.reasons")
                .bind(&id).bind(candidate.asset_id).bind(candidate.score).bind(reasons).execute(&mut *transaction).await?;
        }
    }
    for previous in open_groups {
        if !retained.contains(&previous.id) {
            sqlx::query("DELETE FROM duplicate_groups WHERE id = ? AND status = 'open'")
                .bind(previous.id)
                .execute(&mut *transaction)
                .await?;
        }
    }
    if !current(&mut transaction, &input.ticket).await? {
        return Ok(false);
    }
    sqlx::query("UPDATE duplicate_matching_jobs SET completed_revision = requested_revision, state = 'succeeded',
        automatic_requested = 0, manual_run_id = NULL, lease_manual_run_id = NULL, cancel_requested = 0,
        lease_id = NULL, lease_expires_at = NULL, attempts = 0, next_attempt_at = NULL, last_error = NULL WHERE kind = ?")
        .bind(input.ticket.kind.as_str()).execute(&mut *transaction).await?;
    sqlx::query("UPDATE duplicate_matching_revision SET publication_sequence = publication_sequence + 1 WHERE id = 1")
        .execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

async fn current(
    transaction: &mut Transaction<'_, Sqlite>,
    ticket: &MatchingTicket,
) -> AppResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM duplicate_matching_jobs WHERE kind = ? AND lease_id = ?
        AND lease_expires_at > unixepoch() AND cancel_requested = 0 AND requested_revision = ?
        AND requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1))",
    )
    .bind(ticket.kind.as_str())
    .bind(&ticket.lease_id)
    .bind(ticket.revision)
    .fetch_one(&mut **transaction)
    .await?)
}
