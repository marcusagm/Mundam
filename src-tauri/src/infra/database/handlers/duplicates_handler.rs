//! Stateless write/mutation functions for the duplicates subsystem.
//!
//! Following the project-wide pattern established in `asset_handler.rs`,
//! `tags_handler.rs` etc., these are free functions that receive a pool
//! reference and execute mutations. The facade in `duplicates.rs`
//! delegates here for all write operations.
//!
//! This module covers: CRUD mutations, scan algorithms (exact + visual),
//! fingerprint rehashing, and group lifecycle management.

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
    .execute(pool)
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
            created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    sqlx::query!(
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
        WHERE NOT EXISTS (
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

/// Runs a database-level scan to find exact duplicates and group them.
///
/// Groups assets by identical `content_hash` values. For each hash with
/// more than one matching asset, creates (or reopens) a duplicate group
/// with all matching assets as candidates.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn run_exact_match_scan(
    pool: &SqlitePool,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        INSERT OR IGNORE INTO duplicate_rule_sets (
            id, name, description, consider_exact_match, consider_visual_match, consider_crop_match, ignore_resolution_difference, ignore_recompression, allow_rotation, allow_mirroring, min_score, created_at, updated_at
        ) VALUES (
            'exact-match', 'Exact Match', 'Finds identical files using hash', 1, 0, 0, 0, 0, 0, 0, 1.0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to seed exact-match rule set: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    backfill_missing_fingerprints(pool).await?;

    let duplicate_hash_records = sqlx::query!(
        r#"
        SELECT df.content_hash
        FROM duplicate_fingerprints df
        JOIN assets a ON df.asset_id = a.id
        WHERE df.content_hash IS NOT NULL 
          AND df.content_hash NOT LIKE 'pending_%'
          AND a.state != 'trashed'
        GROUP BY df.content_hash
        HAVING COUNT(df.asset_id) > 1
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to fetch exact match groups: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    for hash_record in duplicate_hash_records {
        if let Some(token) = &cancellation_token {
            if token.is_cancelled() {
                tracing::info!("run_exact_match_scan cancelled");
                break;
            }
        }

        let content_hash_value = hash_record.content_hash.unwrap();

        let existing_group = sqlx::query!(
            r#"
            SELECT dg.id
            FROM duplicate_groups dg
            JOIN duplicate_candidates dc ON dg.id = dc.group_id
            JOIN duplicate_fingerprints df ON dc.asset_id = df.asset_id
            WHERE df.content_hash = ?
            LIMIT 1
            "#,
            content_hash_value
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        let mut database_transaction = pool.begin().await.map_err(AppError::Database)?;

        let group_id = if let Some(group) = existing_group {
            let existing_group_id = group.id.unwrap_or_default();
            sqlx::query!(
                r#"UPDATE duplicate_groups SET status = 'open' WHERE id = ? AND status = 'resolved'"#,
                existing_group_id
            )
            .execute(&mut *database_transaction)
            .await
            .map_err(AppError::Database)?;
            existing_group_id
        } else {
            let new_group_id = uuid::Uuid::new_v4().to_string();
            sqlx::query!(
                r#"
                INSERT INTO duplicate_groups (id, rule_set_id, group_type, confidence, status, candidate_count, created_at, updated_at)
                VALUES (?, 'exact-match', 'exact', 1.0, 'open', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
                new_group_id
            )
            .execute(&mut *database_transaction)
            .await
            .map_err(AppError::Database)?;
            new_group_id
        };

        sqlx::query!(
            r#"
            INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons, is_selected)
            SELECT ?, df.asset_id, 1.0, '["exact_content_hash"]', 0
            FROM duplicate_fingerprints df
            JOIN assets a ON df.asset_id = a.id
            WHERE df.content_hash = ? AND a.state != 'trashed'
            ON CONFLICT(group_id, asset_id) DO NOTHING
            "#,
            group_id, content_hash_value
        )
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

        sqlx::query!(
            r#"
            UPDATE duplicate_groups
            SET candidate_count = (SELECT COUNT(*) FROM duplicate_candidates WHERE group_id = ?),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
            group_id, group_id
        )
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

        database_transaction.commit().await.map_err(AppError::Database)?;
    }

    Ok(())
}

/// Runs a database-level scan to find visually similar duplicates and group them.
///
/// Compares perceptual hashes (dHash), spatial block hashes, and multiscale
/// hashes to detect near-duplicates. Uses the most recently updated rule set
/// to determine thresholds and which comparison strategies to apply.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
pub async fn run_visual_match_scan(
    pool: &SqlitePool,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> AppResult<()> {
    sqlx::query!(
        r#"
        INSERT OR IGNORE INTO duplicate_rule_sets (
            id, name, description, consider_exact_match, consider_visual_match, consider_crop_match, ignore_resolution_difference, ignore_recompression, allow_rotation, allow_mirroring, min_score, created_at, updated_at
        ) VALUES (
            'visual-match', 'Visual Match', 'Finds similar images using perceptual hash', 0, 1, 0, 1, 1, 0, 0, 0.9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to seed visual-match rule set: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    let rule_set_record = sqlx::query!(
        r#"
        SELECT id, min_score, consider_visual_match, consider_crop_match
        FROM duplicate_rule_sets
        ORDER BY updated_at DESC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    let (rule_set_id, minimum_score, consider_visual_match, consider_crop_match) = match rule_set_record {
        Some(record) => (
            record.id,
            record.min_score,
            record.consider_visual_match != 0,
            record.consider_crop_match != 0,
        ),
        None => return Ok(()),
    };

    if !consider_visual_match && !consider_crop_match {
        tracing::info!("run_visual_match_scan: visual and crop matching both disabled in active rule set");
        return Ok(());
    }

    struct VisualCandidateData {
        asset_id: String,
        content_hash: Option<String>,
        perceptual_hash_value: u64,
        spatial_block_hash: Option<String>,
        multiscale_hash: Option<String>,
        image_width: Option<i32>,
        image_height: Option<i32>,
        file_size_bytes: Option<i64>,
    }

    let database_rows = sqlx::query!(
        r#"
        SELECT df.asset_id as "asset_id!",
               df.content_hash,
               df.perceptual_hash,
               df.block_hash,
               df.thumb_hash,
               df.width,
               df.height,
               df.file_size
        FROM duplicate_fingerprints df
        JOIN assets a ON df.asset_id = a.id
        WHERE df.perceptual_hash IS NOT NULL 
          AND a.state != 'trashed'
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut candidate_assets: Vec<VisualCandidateData> = Vec::new();
    for row in database_rows {
        if let Some(hash_string) = row.perceptual_hash {
            if let Ok(hash_integer) = u64::from_str_radix(&hash_string, 16) {
                candidate_assets.push(VisualCandidateData {
                    asset_id: row.asset_id,
                    content_hash: row.content_hash,
                    perceptual_hash_value: hash_integer,
                    spatial_block_hash: row.block_hash,
                    multiscale_hash: row.thumb_hash,
                    image_width: row.width.map(|dimension_value| dimension_value as i32),
                    image_height: row.height.map(|dimension_value| dimension_value as i32),
                    file_size_bytes: row.file_size,
                });
            }
        }
    }

    struct DiscoveredCandidateInfo {
        asset_id: String,
        score: f64,
        reasons: Vec<String>,
    }

    struct DiscoveredGroupInfo {
        group_type: String,
        confidence: f64,
        candidates: Vec<DiscoveredCandidateInfo>,
    }

    let total_assets_count = candidate_assets.len();
    let mut grouped_asset_indices = vec![false; total_assets_count];
    let mut discovered_groups: Vec<DiscoveredGroupInfo> = Vec::new();

    for primary_index in 0..total_assets_count {
        if grouped_asset_indices[primary_index] {
            continue;
        }

        let primary_asset = &candidate_assets[primary_index];
        let mut current_group_candidates: Vec<DiscoveredCandidateInfo> = Vec::new();
        let mut detected_group_type = "visual".to_string();
        let mut group_scores: Vec<f64> = Vec::new();

        current_group_candidates.push(DiscoveredCandidateInfo {
            asset_id: primary_asset.asset_id.clone(),
            score: 1.0,
            reasons: vec!["canonical_anchor".to_string()],
        });

        for comparison_index in (primary_index + 1)..total_assets_count {
            if grouped_asset_indices[comparison_index] {
                continue;
            }

            let comparison_asset = &candidate_assets[comparison_index];

            if let (Some(hash_one), Some(hash_two)) = (
                &primary_asset.content_hash,
                &comparison_asset.content_hash,
            ) {
                if hash_one == hash_two {
                    continue;
                }
            }

            let match_result = crate::feature::duplicates::matcher::evaluate_visual_and_crop_match(
                primary_asset.perceptual_hash_value,
                comparison_asset.perceptual_hash_value,
                primary_asset.spatial_block_hash.as_deref(),
                comparison_asset.spatial_block_hash.as_deref(),
                primary_asset.multiscale_hash.as_deref(),
                comparison_asset.multiscale_hash.as_deref(),
                (primary_asset.image_width, primary_asset.image_height),
                (comparison_asset.image_width, comparison_asset.image_height),
                primary_asset.file_size_bytes,
                comparison_asset.file_size_bytes,
                minimum_score,
                consider_crop_match,
            );

            if let Some(evaluation) = match_result {
                if evaluation.match_type == crate::core::models::DuplicateGroupType::Derived {
                    detected_group_type = "derived".to_string();
                }
                group_scores.push(evaluation.score);
                current_group_candidates.push(DiscoveredCandidateInfo {
                    asset_id: comparison_asset.asset_id.clone(),
                    score: evaluation.score,
                    reasons: evaluation.reasons,
                });
                grouped_asset_indices[comparison_index] = true;
            }
        }

        if current_group_candidates.len() > 1 {
            grouped_asset_indices[primary_index] = true;
            let average_confidence = if !group_scores.is_empty() {
                group_scores.iter().sum::<f64>() / (group_scores.len() as f64)
            } else {
                minimum_score
            };

            discovered_groups.push(DiscoveredGroupInfo {
                group_type: detected_group_type,
                confidence: average_confidence,
                candidates: current_group_candidates,
            });
        }
    }

    let mut database_transaction = pool.begin().await.map_err(AppError::Database)?;

    sqlx::query!(
        r#"DELETE FROM duplicate_groups WHERE status = 'open' AND (group_type = 'visual' OR group_type = 'derived' OR group_type = 'near')"#
    )
    .execute(&mut *database_transaction)
    .await
    .map_err(AppError::Database)?;

    for discovered_group in discovered_groups {
        if let Some(token) = &cancellation_token {
            if token.is_cancelled() {
                tracing::info!("run_visual_match_scan cancelled");
                break;
            }
        }

        let new_group_id = uuid::Uuid::new_v4().to_string();
        let candidate_count = discovered_group.candidates.len() as i64;

        sqlx::query!(
            r#"
            INSERT INTO duplicate_groups (id, rule_set_id, group_type, confidence, status, candidate_count, created_at, updated_at)
            VALUES (?, ?, ?, ?, 'open', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
            new_group_id,
            rule_set_id,
            discovered_group.group_type,
            discovered_group.confidence,
            candidate_count
        )
        .execute(&mut *database_transaction)
        .await
        .map_err(AppError::Database)?;

        for candidate_record in discovered_group.candidates {
            let serialized_reasons = serde_json::to_string(&candidate_record.reasons)
                .unwrap_or_else(|_| "[]".to_string());

            sqlx::query!(
                r#"
                INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons, is_selected)
                VALUES (?, ?, ?, ?, 0)
                "#,
                new_group_id,
                candidate_record.asset_id,
                candidate_record.score,
                serialized_reasons
            )
            .execute(&mut *database_transaction)
            .await
            .map_err(AppError::Database)?;
        }
    }

    database_transaction.commit().await.map_err(AppError::Database)?;

    Ok(())
}

/// Rehashes all fingerprints that have a pending or outdated hash.
///
/// Computes BLAKE3 content hashes and, for image assets, perceptual hashes
/// (dHash-64, spatial block hash, multiscale dHash-256). Reports progress
/// via an optional `UnboundedSender` so the frontend can display a real
/// progress bar.
///
/// # Errors
/// Returns `AppError::Database` if the query or update fails.
pub async fn rehash_pending_fingerprints(
    pool: &SqlitePool,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
    progress_sender: Option<tokio::sync::mpsc::UnboundedSender<(usize, usize)>>,
) -> AppResult<usize> {
    let pending_assets = sqlx::query!(
        r#"
        SELECT df.asset_id as "asset_id!", a.path as "path!", a.family as "format_family!", a.format_type as "format_type!"
        FROM duplicate_fingerprints df
        JOIN assets a ON df.asset_id = a.id
        WHERE df.content_hash LIKE 'pending_%' 
           OR df.content_hash LIKE 'hash_%'
           OR df.fingerprint_version < 4
           OR (df.perceptual_hash IS NULL AND (a.family LIKE '%image%' OR a.format_type LIKE '%image%' OR a.format_type LIKE '%png%' OR a.format_type LIKE '%jpg%' OR a.format_type LIKE '%jpeg%'))
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|database_error| {
        tracing::error!("Failed to query pending fingerprints: {:?}", database_error);
        AppError::Database(database_error)
    })?;

    let total_pending_count = pending_assets.len();
    if total_pending_count == 0 {
        tracing::info!("rehash_pending_fingerprints: no pending fingerprints found");
        return Ok(0);
    }

    tracing::info!("rehash_pending_fingerprints: found {} fingerprints to rehash", total_pending_count);

    let mut rehashed_count: usize = 0;

    for (processed_index, asset_record) in pending_assets.into_iter().enumerate() {
        if let Some(token) = &cancellation_token {
            if token.is_cancelled() {
                tracing::info!("rehash_pending_fingerprints cancelled");
                break;
            }
        }

        let asset_id = asset_record.asset_id.clone();
        let file_path = asset_record.path.clone();
        let format_family = asset_record.format_family.clone();
        let format_type = asset_record.format_type.clone();

        let hash_computation_result = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut target_file = match std::fs::File::open(&file_path) {
                Ok(opened_file) => opened_file,
                Err(file_error) => return Err(format!("Cannot open {}: {}", file_path, file_error)),
            };
            let file_metadata = match target_file.metadata() {
                Ok(retrieved_metadata) => retrieved_metadata,
                Err(metadata_error) => return Err(format!("Cannot read metadata for {}: {}", file_path, metadata_error)),
            };

            let mut blake_hasher = blake3::Hasher::new();
            let mut read_buffer = [0u8; 8192];
            loop {
                let bytes_read = match target_file.read(&mut read_buffer) {
                    Ok(bytes_count) => bytes_count,
                    Err(read_error) => return Err(format!("Read error for {}: {}", file_path, read_error)),
                };
                if bytes_read == 0 {
                    break;
                }
                blake_hasher.update(&read_buffer[..bytes_read]);
            }

            let format_is_image = crate::feature::duplicates::fingerprints::is_image_media(&format_family, &format_type);

            let (perceptual_hash, block_hash, thumb_hash, image_width, image_height) = {
                if format_is_image {
                    if let Ok(dynamic_image) = image::open(&file_path) {
                        use image::GenericImageView;
                        let (width, height) = dynamic_image.dimensions();
                        let dhash_value = crate::feature::duplicates::fingerprints::compute_perceptual_dhash_64(&dynamic_image);
                        let multiscale_hash = crate::feature::duplicates::fingerprints::compute_multiscale_dhash_256(&dynamic_image);
                        let spatial_block_hash = crate::feature::duplicates::fingerprints::compute_spatial_block_hash(&dynamic_image);

                        (
                            Some(format!("{:016x}", dhash_value)),
                            Some(spatial_block_hash),
                            Some(multiscale_hash),
                            Some(width as i32),
                            Some(height as i32),
                        )
                    } else {
                        (None, None, None, None, None)
                    }
                } else {
                    (None, None, None, None, None)
                }
            };

            Ok((
                blake_hasher.finalize().to_hex().to_string(),
                file_metadata.len() as i64,
                perceptual_hash,
                block_hash,
                thumb_hash,
                image_width,
                image_height,
            ))
        })
        .await;

        match hash_computation_result {
            Ok(Ok((content_hash, file_size, perceptual_hash, block_hash, thumb_hash, image_width, image_height))) => {
                if let Err(database_error) = sqlx::query!(
                    r#"
                    UPDATE duplicate_fingerprints 
                    SET content_hash = ?,
                        file_size = ?,
                        perceptual_hash = ?,
                        block_hash = ?,
                        thumb_hash = ?,
                        width = COALESCE(?, width),
                        height = COALESCE(?, height),
                        fingerprint_version = 4,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE asset_id = ?
                    "#,
                    content_hash,
                    file_size,
                    perceptual_hash,
                    block_hash,
                    thumb_hash,
                    image_width,
                    image_height,
                    asset_id
                )
                .execute(pool)
                .await
                {
                    tracing::error!("Failed to update fingerprint for {}: {:?}", asset_id, database_error);
                } else {
                    rehashed_count += 1;
                }
            }
            Ok(Err(task_error)) => {
                tracing::warn!("rehash_pending_fingerprints: skipping {}: {}", asset_id, task_error);
            }
            Err(join_error) => {
                tracing::error!("rehash_pending_fingerprints: blocking task failed for {}: {}", asset_id, join_error);
            }
        }

        if let Some(sender) = &progress_sender {
            let _ = sender.send((processed_index + 1, total_pending_count));
        }
    }

    tracing::info!("rehash_pending_fingerprints: successfully rehashed {}/{}", rehashed_count, total_pending_count);
    Ok(rehashed_count)
}
