//! Stateless read-only query functions for the duplicates subsystem.
//!
//! Following the project-wide pattern established in `asset_queries.rs`,
//! `folder_queries.rs` etc., these are free functions that receive a pool
//! reference and return domain models. The facade in `duplicates.rs`
//! delegates here for all read operations.

use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    DuplicateCandidate, DuplicateFingerprint, DuplicateGroup,
    DuplicateGroupStatus, DuplicateGroupType, DuplicateRuleSet,
};
use sqlx::SqlitePool;

/// Retrieves a duplicate fingerprint by asset ID.
///
/// # Arguments
/// * `pool` - The database connection pool.
/// * `asset_id` - The unique identifier of the asset.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn get_fingerprint(
    pool: &SqlitePool,
    asset_id: &str,
) -> AppResult<Option<DuplicateFingerprint>> {
    let record = sqlx::query_as!(
        DuplicateFingerprint,
        r#"
        SELECT 
            asset_id as "asset_id!", content_hash, perceptual_hash, block_hash, thumb_hash,
            width as "width: i32", height as "height: i32", file_size as "file_size: i64", 
            mime_type, format_family, color_profile, orientation as "orientation: i32", 
            fingerprint_version as "fingerprint_version: i32", 
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM duplicate_fingerprints
        WHERE asset_id = ?
        "#,
        asset_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to get fingerprint for asset {}: {:?}", asset_id, database_error);
        AppError::Database(database_error)
    })?;

    Ok(record)
}

/// Retrieves all active duplicate rule sets.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn get_rule_sets(pool: &SqlitePool) -> AppResult<Vec<DuplicateRuleSet>> {
    let records = sqlx::query_as!(
        DuplicateRuleSet,
        r#"
        SELECT 
            id as "id!", name as "name!", description, 
            consider_exact_match as "consider_exact_match: bool", 
            consider_visual_match as "consider_visual_match: bool", 
            consider_crop_match as "consider_crop_match: bool", 
            ignore_resolution_difference as "ignore_resolution_difference: bool", 
            ignore_recompression as "ignore_recompression: bool", 
            allow_rotation as "allow_rotation: bool", 
            allow_mirroring as "allow_mirroring: bool", 
            min_score as "min_score: f64", 
            created_at as "created_at: chrono::DateTime<chrono::Utc>", 
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM duplicate_rule_sets
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to fetch rule sets: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(records)
}

/// Retrieves duplicate groups filtered by their status.
///
/// Parses the raw `group_type` and `status` strings from the database
/// into their corresponding domain enums.
///
/// # Arguments
/// * `pool` - The database connection pool.
/// * `status` - The status string to filter by (e.g., "open", "resolved").
///
/// # Errors
/// Returns `AppError::Database` if the query fails, or `AppError::Internal`
/// if a stored enum value cannot be parsed.
pub async fn get_groups_by_status(
    pool: &SqlitePool,
    status: &str,
) -> AppResult<Vec<DuplicateGroup>> {
    let records = sqlx::query!(
        r#"
        SELECT 
            id as "id!", rule_set_id as "rule_set_id!", group_type as "group_type!", canonical_asset_id, confidence as "confidence: f64",
            status as "status!", candidate_count as "candidate_count: i32", 
            created_at as "created_at: chrono::DateTime<chrono::Utc>", 
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM duplicate_groups
        WHERE status = ?
        "#,
        status
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to fetch groups by status {}: {:?}", status, database_error);
        AppError::Database(database_error)
    })?;

    let mut groups = Vec::new();
    for record in records {
        use std::str::FromStr;
        let group_type = DuplicateGroupType::from_str(&record.group_type)
            .map_err(|_| AppError::Internal(format!("Invalid group type in DB: {}", record.group_type)))?;
        let parsed_status = DuplicateGroupStatus::from_str(&record.status)
            .map_err(|_| AppError::Internal(format!("Invalid status in DB: {}", record.status)))?;

        groups.push(DuplicateGroup {
            id: record.id,
            rule_set_id: record.rule_set_id,
            group_type,
            canonical_asset_id: record.canonical_asset_id,
            confidence: record.confidence,
            status: parsed_status,
            candidate_count: record.candidate_count,
            created_at: record.created_at,
            updated_at: record.updated_at,
        });
    }

    Ok(groups)
}

/// Retrieves all candidates belonging to a specific duplicate group.
///
/// # Arguments
/// * `pool` - The database connection pool.
/// * `group_id` - The unique identifier of the duplicate group.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn get_group_candidates(
    pool: &SqlitePool,
    group_id: &str,
) -> AppResult<Vec<DuplicateCandidate>> {
    let records = sqlx::query_as!(
        DuplicateCandidate,
        r#"
        SELECT 
            group_id as "group_id!", asset_id as "asset_id!", score as "score: f64", reasons as "reasons!", is_selected as "is_selected: bool"
        FROM duplicate_candidates
        WHERE group_id = ?
        "#,
        group_id
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to fetch candidates for group {}: {:?}", group_id, database_error);
        AppError::Database(database_error)
    })?;

    Ok(records)
}
