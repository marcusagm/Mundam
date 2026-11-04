//! Deterministic active policy selection shared by UI queries and matching snapshots.
use crate::core::{models::DuplicateRuleSet, AppResult};
use sqlx::Row;

/// Reads the active policy with a stable tie-break when saves share a timestamp.
/// # Errors
/// Returns query and decoding errors.
pub(crate) async fn read<'connection>(
    executor: impl sqlx::Executor<'connection, Database = sqlx::Sqlite>,
) -> AppResult<Option<DuplicateRuleSet>> {
    let row =
        sqlx::query("SELECT * FROM duplicate_rule_sets ORDER BY updated_at DESC, id ASC LIMIT 1")
            .fetch_optional(executor)
            .await?;
    let Some(row) = row else { return Ok(None) };
    Ok(Some(DuplicateRuleSet {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        consider_exact_match: row.try_get("consider_exact_match")?,
        consider_visual_match: row.try_get("consider_visual_match")?,
        consider_crop_match: row.try_get("consider_crop_match")?,
        ignore_resolution_difference: row.try_get("ignore_resolution_difference")?,
        ignore_recompression: row.try_get("ignore_recompression")?,
        allow_rotation: row.try_get("allow_rotation")?,
        allow_mirroring: row.try_get("allow_mirroring")?,
        min_score: row.try_get("min_score")?,
        min_visual_score: row.try_get("min_visual_score")?,
        min_crop_score: row.try_get("min_crop_score")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    }))
}
