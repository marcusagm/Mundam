//! Stateless write/mutation functions for the duplicates subsystem.
//!
//! Following the project-wide pattern established in `asset_handler.rs`,
//! `tags_handler.rs` etc., these are free functions that receive a pool
//! reference and execute mutations. The facade in `duplicates.rs`
//! delegates here for all write operations.
//!
//! This module covers: CRUD mutations,
//! fingerprint persistence, and group lifecycle management.

use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    DuplicateCandidate, DuplicateFingerprint, DuplicateGroup, DuplicateResolution, DuplicateRuleSet,
};
use sqlx::SqlitePool;

/// Saves or updates a duplicate fingerprint for an asset.
///
/// Uses `ON CONFLICT` upsert to handle both initial creation and subsequent
/// updates from rehash cycles.
///
/// # Errors
/// Returns `AppError::Database` if the insert/update fails.
pub async fn save_fingerprint(
    pool: &SqlitePool,
    fingerprint: DuplicateFingerprint,
) -> AppResult<()> {
    save_fingerprint_with_executor(pool, fingerprint).await
}

/// Persists a fingerprint within the ownership-checked result transaction.
///
/// # Errors
/// Propagates database constraint and write failures to the enclosing transaction.
pub(crate) async fn save_fingerprint_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    fingerprint: DuplicateFingerprint,
) -> AppResult<()> {
    save_fingerprint_with_executor(&mut **transaction, fingerprint).await
}

async fn save_fingerprint_with_executor<'connection>(
    executor: impl sqlx::Executor<'connection, Database = sqlx::Sqlite>,
    fingerprint: DuplicateFingerprint,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO duplicate_fingerprints (
            asset_id, content_hash, perceptual_hash, block_hash, thumb_hash,
            width, height, file_size, mime_type, format_family, color_profile,
            orientation, fingerprint_version, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(asset_id) DO UPDATE SET
            content_hash = excluded.content_hash,
            perceptual_hash = excluded.perceptual_hash,
            block_hash = excluded.block_hash,
            thumb_hash = excluded.thumb_hash,
            width = excluded.width,
            height = excluded.height,
            file_size = excluded.file_size,
            mime_type = excluded.mime_type,
            format_family = excluded.format_family,
            color_profile = excluded.color_profile,
            orientation = excluded.orientation,
            fingerprint_version = excluded.fingerprint_version,
            updated_at = excluded.updated_at
        "#,
        fingerprint.asset_id,
        fingerprint.content_hash,
        fingerprint.perceptual_hash,
        fingerprint.block_hash,
        fingerprint.thumb_hash,
        fingerprint.width,
        fingerprint.height,
        fingerprint.file_size,
        fingerprint.mime_type,
        fingerprint.format_family,
        fingerprint.color_profile,
        fingerprint.orientation,
        fingerprint.fingerprint_version,
        fingerprint.updated_at,
    )
    .execute(executor)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to save fingerprint: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Saves or updates a duplicate rule set configuration.
///
/// # Errors
/// Returns `AppError::Database` if the insert/update fails.
pub async fn save_rule_set(pool: &SqlitePool, rule_set: DuplicateRuleSet) -> AppResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO duplicate_rule_sets (
            id, name, description, 
            consider_exact_match, consider_visual_match, consider_crop_match, 
            ignore_resolution_difference, ignore_recompression, 
            allow_rotation, allow_mirroring, min_score, 
            min_visual_score, min_crop_score,
            created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            consider_exact_match = excluded.consider_exact_match,
            consider_visual_match = excluded.consider_visual_match,
            consider_crop_match = excluded.consider_crop_match,
            ignore_resolution_difference = excluded.ignore_resolution_difference,
            ignore_recompression = excluded.ignore_recompression,
            allow_rotation = excluded.allow_rotation,
            allow_mirroring = excluded.allow_mirroring,
            min_score = excluded.min_score,
            min_visual_score = excluded.min_visual_score,
            min_crop_score = excluded.min_crop_score,
            updated_at = excluded.updated_at
        "#,
        rule_set.id,
        rule_set.name,
        rule_set.description,
        rule_set.consider_exact_match,
        rule_set.consider_visual_match,
        rule_set.consider_crop_match,
        rule_set.ignore_resolution_difference,
        rule_set.ignore_recompression,
        rule_set.allow_rotation,
        rule_set.allow_mirroring,
        rule_set.min_score,
        rule_set.min_visual_score,
        rule_set.min_crop_score,
        rule_set.created_at,
        rule_set.updated_at
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to save rule set: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Saves a new duplicate group or updates an existing one.
///
/// # Errors
/// Returns `AppError::Database` if the insert/update fails.
pub async fn save_group(pool: &SqlitePool, group: DuplicateGroup) -> AppResult<()> {
    let group_type_str = group.group_type.to_string();
    let status_str = group.status.to_string();

    sqlx::query!(
        r#"
        INSERT INTO duplicate_groups (
            id, rule_set_id, group_type, canonical_asset_id, confidence,
            status, candidate_count, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            rule_set_id = excluded.rule_set_id,
            group_type = excluded.group_type,
            canonical_asset_id = excluded.canonical_asset_id,
            confidence = excluded.confidence,
            status = excluded.status,
            candidate_count = excluded.candidate_count,
            updated_at = excluded.updated_at
        "#,
        group.id,
        group.rule_set_id,
        group_type_str,
        group.canonical_asset_id,
        group.confidence,
        status_str,
        group.candidate_count,
        group.created_at,
        group.updated_at,
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to save duplicate group: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Adds a candidate to a duplicate group.
///
/// # Errors
/// Returns `AppError::Database` if the insert/update fails.
pub async fn save_candidate(
    pool: &SqlitePool,
    candidate: DuplicateCandidate,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO duplicate_candidates (
            group_id, asset_id, score, reasons, is_selected
        )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(group_id, asset_id) DO UPDATE SET
            score = excluded.score,
            reasons = excluded.reasons,
            is_selected = excluded.is_selected
        "#,
        candidate.group_id,
        candidate.asset_id,
        candidate.score,
        candidate.reasons,
        candidate.is_selected,
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to save candidate: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Saves a resolution decision for a duplicate group.
///
/// # Errors
/// Returns `AppError::Database` if the insert fails.
pub async fn save_resolution(
    pool: &SqlitePool,
    resolution: DuplicateResolution,
) -> AppResult<()> {
    let action_str = resolution.action.to_string();
    sqlx::query!(
        r#"
        INSERT INTO duplicate_resolutions (
            id, group_id, action, selected_asset_id, payload, resolved_by, resolved_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        resolution.id,
        resolution.group_id,
        action_str,
        resolution.selected_asset_id,
        resolution.payload,
        resolution.resolved_by,
        resolution.resolved_at,
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to save resolution: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Deletes a resolution decision from the database by its ID.
///
/// Invoked by the undo mechanism when reverting a resolved duplicate group
/// back to the open status, ensuring audit consistency and clean state.
///
/// # Arguments
/// * `pool` - The SQLite database connection pool.
/// * `resolution_id` - The unique identifier of the resolution to delete.
///
/// # Errors
/// Returns `AppError::Database` if the delete statement fails.
pub async fn delete_resolution(
    pool: &SqlitePool,
    resolution_id: &str,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        DELETE FROM duplicate_resolutions
        WHERE id = ?
        "#,
        resolution_id
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!(
            "Failed to delete resolution {}: {:?}",
            resolution_id,
            database_error
        );
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Updates the status of a duplicate group.
///
/// # Errors
/// Returns `AppError::Database` if the update fails.
pub async fn update_group_status(
    pool: &SqlitePool,
    group_id: &str,
    status: &str,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        UPDATE duplicate_groups
        SET status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
        status,
        group_id
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to update group status {}: {:?}", group_id, database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Backfills missing fingerprint rows for assets not yet tracked.
///
/// Inserts placeholder records with `pending_` content hashes so the
/// rehash cycle can pick them up later.
///
/// # Errors
/// Returns `AppError::Database` if the query or insert fails.
pub async fn backfill_missing_fingerprints(pool: &SqlitePool) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO duplicate_fingerprints (
            asset_id, content_hash, file_size, format_family, fingerprint_version, updated_at
        )
        SELECT 
            a.id, 
            'pending_' || a.id, 
            a.file_size, 
            a.family, 
            0, 
            CURRENT_TIMESTAMP
        FROM assets a
        WHERE a.deleted_at IS NULL AND LOWER(a.state) NOT IN ('trashed', 'offline')
          AND NOT EXISTS (
            SELECT 1 FROM duplicate_fingerprints df WHERE df.asset_id = a.id
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to backfill missing fingerprints: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Deletes the fingerprint record for a given asset.
///
/// # Errors
/// Returns `AppError::Database` if the delete fails.
pub async fn delete_fingerprint(pool: &SqlitePool, asset_id: &str) -> AppResult<()> {
    sqlx::query!(
        r#"DELETE FROM duplicate_fingerprints WHERE asset_id = ?"#,
        asset_id
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to delete fingerprint for asset {}: {:?}", asset_id, database_error);
        AppError::Database(database_error)
    })?;

    Ok(())
}

/// Removes a candidate from all groups it belongs to.
///
/// If removing the candidate leaves a group with fewer than 2 candidates,
/// the group is automatically resolved since it is no longer a duplicate.
///
/// # Errors
/// Returns `AppError::Database` if the operation fails.
pub async fn remove_candidate_from_groups(pool: &SqlitePool, asset_id: &str) -> AppResult<()> {
    let affected_group_ids: Vec<String> = sqlx::query_scalar!(
        r#"SELECT group_id as "group_id!" FROM duplicate_candidates WHERE asset_id = ?"#,
        asset_id
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to find groups for asset {}: {:?}", asset_id, database_error);
        AppError::Database(database_error)
    })?;

    sqlx::query!(
        r#"DELETE FROM duplicate_candidates WHERE asset_id = ?"#,
        asset_id
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to remove candidate {} from groups: {:?}", asset_id, database_error);
        AppError::Database(database_error)
    })?;

    for group_id in affected_group_ids {
        let remaining_count: i32 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count: i32" FROM duplicate_candidates WHERE group_id = ?"#,
            group_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        if remaining_count < 2 {
            sqlx::query!(
                r#"UPDATE duplicate_groups SET status = 'resolved', candidate_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
                remaining_count,
                group_id
            )
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

            tracing::info!("DuplicateHandler: auto-resolved group {} (only {} candidate(s) remain)", group_id, remaining_count);
        } else {
            sqlx::query!(
                r#"UPDATE duplicate_groups SET candidate_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
                remaining_count,
                group_id
            )
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
    }

    Ok(())
}

/// Erases all existing duplicate candidates, groups, resolutions, and fingerprints,
/// allowing a clean index rebuild from scratch.
///
/// # Arguments
/// * `pool` - The SQLite database connection pool.
///
/// # Errors
/// Returns `AppError::Database` if any delete statement fails.
pub async fn reset_duplicate_index(pool: &SqlitePool) -> AppResult<()> {
    let mut database_transaction = pool.begin().await.map_err(AppError::Database)?;

    sqlx::query("DELETE FROM processing_tasks WHERE kind = 'fingerprint'")
        .execute(&mut *database_transaction)
        .await?;

    sqlx::query!("DELETE FROM duplicate_resolutions")
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

    sqlx::query!("DELETE FROM duplicate_candidates")
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

    sqlx::query!("DELETE FROM duplicate_groups")
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

    sqlx::query!("DELETE FROM duplicate_fingerprints")
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

    database_transaction.commit().await.map_err(AppError::Database)?;

    tracing::info!("reset_duplicate_index: successfully wiped all duplicate tables");
    Ok(())
}

/// Retrieves assets that do not yet have a duplicate fingerprint computed.
///
/// # Arguments
/// * `pool` - The SQLite database connection pool.
/// * `limit` - The maximum number of assets to return.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn get_assets_needing_fingerprints(
    pool: &SqlitePool,
    limit: u32,
) -> AppResult<Vec<crate::core::models::PendingFingerprintAsset>> {
    let limit_integer = limit as i64;
    let database_rows = sqlx::query!(
        r#"
        SELECT a.id as "id!", a.path as "path!", a.format_type as "format_type!"
        FROM assets a
        LEFT JOIN duplicate_fingerprints df ON a.id = df.asset_id
        WHERE (
            df.asset_id IS NULL 
            OR df.content_hash LIKE 'pending_%' 
            OR df.content_hash LIKE 'hash_%' 
            OR df.fingerprint_version < 5
        )
        AND a.state != 'trashed' 
        AND a.deleted_at IS NULL
        LIMIT ?
        "#,
        limit_integer
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let pending_assets: Vec<crate::core::models::PendingFingerprintAsset> = database_rows
        .into_iter()
        .map(|database_row| crate::core::models::PendingFingerprintAsset {
            asset_id: database_row.id,
            path: database_row.path,
            format_type: database_row.format_type,
        })
        .collect();

    if !pending_assets.is_empty() {
        tracing::debug!(
            "get_assets_needing_fingerprints: fetched {} pending asset(s) needing fingerprints (limit={})",
            pending_assets.len(),
            limit
        );
    }

    Ok(pending_assets)
}

