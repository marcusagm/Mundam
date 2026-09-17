use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    DuplicateCandidate, DuplicateFingerprint, DuplicateGroup, DuplicateResolution, DuplicateRuleSet,
};
use crate::core::repository::DuplicatesRepository;
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

/// SQLite implementation of the DuplicatesRepository port.
pub struct SqliteDuplicatesRepository {
    pool: Pool<Sqlite>,
}

impl SqliteDuplicatesRepository {
    /// Creates a new instance of the SqliteDuplicatesRepository.
    ///
    /// # Arguments
    /// * `pool` - The SQLite connection pool.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DuplicatesRepository for SqliteDuplicatesRepository {
    async fn save_fingerprint(&self, fingerprint: DuplicateFingerprint) -> AppResult<()> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save fingerprint: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn get_fingerprint(&self, asset_id: &str) -> AppResult<Option<DuplicateFingerprint>> {
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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get fingerprint for asset {}: {:?}", asset_id, e);
            AppError::Database(e)
        })?;

        Ok(record)
    }

    async fn get_rule_sets(&self) -> AppResult<Vec<DuplicateRuleSet>> {
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch rule sets: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(records)
    }

    async fn save_group(&self, group: DuplicateGroup) -> AppResult<()> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save duplicate group: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn save_candidate(&self, candidate: DuplicateCandidate) -> AppResult<()> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save candidate: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn get_groups_by_status(&self, status: &str) -> AppResult<Vec<DuplicateGroup>> {
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch groups by status {}: {:?}", status, e);
            AppError::Database(e)
        })?;

        let mut groups = Vec::new();
        for record in records {
            use std::str::FromStr;
            let group_type = crate::core::models::DuplicateGroupType::from_str(&record.group_type)
                .map_err(|_| AppError::Internal(format!("Invalid group type in DB: {}", record.group_type)))?;
            let parsed_status = crate::core::models::DuplicateGroupStatus::from_str(&record.status)
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

    async fn get_group_candidates(&self, group_id: &str) -> AppResult<Vec<DuplicateCandidate>> {
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch candidates for group {}: {:?}", group_id, e);
            AppError::Database(e)
        })?;

        Ok(records)
    }

    async fn save_resolution(&self, resolution: DuplicateResolution) -> AppResult<()> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save resolution: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn update_group_status(&self, group_id: &str, status: &str) -> AppResult<()> {
        sqlx::query!(
            r#"
            UPDATE duplicate_groups
            SET status = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
            status,
            group_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update group status {}: {:?}", group_id, e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn run_exact_match_scan(&self, token: Option<tokio_util::sync::CancellationToken>) -> AppResult<()> {
        // Ensure exact-match rule set exists to avoid Foreign Key errors
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO duplicate_rule_sets (
                id, name, description, consider_exact_match, consider_visual_match, consider_crop_match, ignore_resolution_difference, ignore_recompression, allow_rotation, allow_mirroring, min_score, created_at, updated_at
            ) VALUES (
                'exact-match', 'Exact Match', 'Finds identical files using hash', 1, 0, 0, 0, 0, 0, 0, 1.0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to seed exact-match rule set: {:?}", e);
            AppError::Database(e)
        })?;

        // Backfill missing fingerprints for existing assets.
        // Assets added before the duplicate module get a placeholder hash
        // that will be replaced with a real Blake3 hash on next access/rescan.
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to backfill missing fingerprints: {:?}", e);
            AppError::Database(e)
        })?;

        let rows = sqlx::query!(
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch exact match groups: {:?}", e);
            AppError::Database(e)
        })?;

        for row in rows {
            if let Some(tok) = &token {
                if tok.is_cancelled() {
                    tracing::info!("run_exact_match_scan cancelled");
                    break;
                }
            }

            let hash = row.content_hash.unwrap();

            let existing_group = sqlx::query!(
                r#"
                SELECT dg.id
                FROM duplicate_groups dg
                JOIN duplicate_candidates dc ON dg.id = dc.group_id
                JOIN duplicate_fingerprints df ON dc.asset_id = df.asset_id
                WHERE df.content_hash = ?
                LIMIT 1
                "#,
                hash
            ).fetch_optional(&self.pool).await.map_err(AppError::Database)?;

            let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

            let group_id = if let Some(group) = existing_group {
                // Reopen group if it was resolved but a new file was added
                let id = group.id.unwrap_or_default();
                sqlx::query!(
                    r#"UPDATE duplicate_groups SET status = 'open' WHERE id = ? AND status = 'resolved'"#,
                    id
                ).execute(&mut *tx).await.map_err(AppError::Database)?;
                id
            } else {
                let new_id = uuid::Uuid::new_v4().to_string();
                sqlx::query!(
                    r#"
                    INSERT INTO duplicate_groups (id, rule_set_id, group_type, confidence, status, candidate_count, created_at, updated_at)
                    VALUES (?, 'exact-match', 'exact', 1.0, 'open', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                    "#,
                    new_id
                )
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
                new_id
            };

            sqlx::query!(
                r#"
                INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons, is_selected)
                SELECT ?, df.asset_id, 1.0, '{}', 0
                FROM duplicate_fingerprints df
                JOIN assets a ON df.asset_id = a.id
                WHERE df.content_hash = ? AND a.state != 'trashed'
                ON CONFLICT(group_id, asset_id) DO NOTHING
                "#,
                group_id, hash
            )
            .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            tx.commit().await.map_err(AppError::Database)?;
        }

        Ok(())
    }

    async fn run_visual_match_scan(&self, token: Option<tokio_util::sync::CancellationToken>) -> AppResult<()> {
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO duplicate_rule_sets (
                id, name, description, consider_exact_match, consider_visual_match, consider_crop_match, ignore_resolution_difference, ignore_recompression, allow_rotation, allow_mirroring, min_score, created_at, updated_at
            ) VALUES (
                'visual-match', 'Visual Match', 'Finds similar images using perceptual hash', 0, 1, 0, 1, 1, 0, 0, 0.9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to seed visual-match rule set: {:?}", e);
            AppError::Database(e)
        })?;

        let rows = sqlx::query!(
            r#"
            SELECT df.perceptual_hash
            FROM duplicate_fingerprints df
            JOIN assets a ON df.asset_id = a.id
            WHERE df.perceptual_hash IS NOT NULL 
              AND a.state != 'trashed'
            GROUP BY df.perceptual_hash
            HAVING COUNT(df.asset_id) > 1
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch visual match groups: {:?}", e);
            AppError::Database(e)
        })?;

        for row in rows {
            if let Some(tok) = &token {
                if tok.is_cancelled() {
                    tracing::info!("run_visual_match_scan cancelled");
                    break;
                }
            }

            let hash = row.perceptual_hash.unwrap();

            let existing_group = sqlx::query!(
                r#"
                SELECT dg.id
                FROM duplicate_groups dg
                JOIN duplicate_candidates dc ON dg.id = dc.group_id
                JOIN duplicate_fingerprints df ON dc.asset_id = df.asset_id
                WHERE df.perceptual_hash = ? AND dg.rule_set_id = 'visual-match'
                LIMIT 1
                "#,
                hash
            ).fetch_optional(&self.pool).await.map_err(AppError::Database)?;

            let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

            let group_id = if let Some(group) = existing_group {
                let id = group.id.unwrap_or_default();
                sqlx::query!(
                    r#"UPDATE duplicate_groups SET status = 'open' WHERE id = ? AND status = 'resolved'"#,
                    id
                ).execute(&mut *tx).await.map_err(AppError::Database)?;
                id
            } else {
                let new_id = uuid::Uuid::new_v4().to_string();
                sqlx::query!(
                    r#"
                    INSERT INTO duplicate_groups (id, rule_set_id, group_type, confidence, status, candidate_count, created_at, updated_at)
                    VALUES (?, 'visual-match', 'visual', 0.95, 'open', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                    "#,
                    new_id
                )
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
                new_id
            };

            sqlx::query!(
                r#"
                INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons, is_selected)
                SELECT ?, df.asset_id, 0.95, '{}', 0
                FROM duplicate_fingerprints df
                JOIN assets a ON df.asset_id = a.id
                WHERE df.perceptual_hash = ? AND a.state != 'trashed'
                ON CONFLICT(group_id, asset_id) DO NOTHING
                "#,
                group_id, hash
            )
            .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            tx.commit().await.map_err(AppError::Database)?;
        }

        Ok(())
    }
    async fn rehash_pending_fingerprints(&self, token: Option<tokio_util::sync::CancellationToken>) -> AppResult<usize> {
        // Find all fingerprints that still have pending or legacy placeholder hashes
        let pending_assets = sqlx::query!(
            r#"
            SELECT df.asset_id as "asset_id!", a.path as "path!"
            FROM duplicate_fingerprints df
            JOIN assets a ON df.asset_id = a.id
            WHERE df.content_hash LIKE 'pending_%' 
               OR df.content_hash LIKE 'hash_%'
               OR df.fingerprint_version < 2
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query pending fingerprints: {:?}", e);
            AppError::Database(e)
        })?;

        let total_pending = pending_assets.len();
        if total_pending == 0 {
            tracing::info!("rehash_pending_fingerprints: no pending fingerprints found");
            return Ok(0);
        }

        tracing::info!("rehash_pending_fingerprints: found {} fingerprints to rehash", total_pending);

        let mut rehashed_count: usize = 0;

        for record in pending_assets {
            if let Some(tok) = &token {
                if tok.is_cancelled() {
                    tracing::info!("rehash_pending_fingerprints cancelled");
                    break;
                }
            }
            
            let asset_id = record.asset_id.clone();
            let path = record.path.clone();

            // Compute Blake3 hash in a blocking task
            let hash_result = tokio::task::spawn_blocking(move || {
                use std::io::Read;
                let mut file = match std::fs::File::open(&path) {
                    Ok(file) => file,
                    Err(error) => return Err(format!("Cannot open {}: {}", path, error)),
                };
                let metadata = match file.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => return Err(format!("Cannot read metadata for {}: {}", path, error)),
                };

                let mut hasher = blake3::Hasher::new();
                let mut buffer = [0u8; 8192];
                loop {
                    let bytes_read = match file.read(&mut buffer) {
                        Ok(bytes) => bytes,
                        Err(error) => return Err(format!("Read error for {}: {}", path, error)),
                    };
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }

                Ok((hasher.finalize().to_hex().to_string(), metadata.len() as i64))
            })
            .await;

            match hash_result {
                Ok(Ok((content_hash, file_size))) => {
                    if let Err(error) = sqlx::query!(
                        r#"
                        UPDATE duplicate_fingerprints 
                        SET content_hash = ?, file_size = ?, fingerprint_version = 2, updated_at = CURRENT_TIMESTAMP
                        WHERE asset_id = ?
                        "#,
                        content_hash,
                        file_size,
                        asset_id
                    )
                    .execute(&self.pool)
                    .await
                    {
                        tracing::error!("Failed to update fingerprint for {}: {:?}", asset_id, error);
                    } else {
                        rehashed_count += 1;
                    }
                }
                Ok(Err(error)) => {
                    tracing::warn!("rehash_pending_fingerprints: skipping {}: {}", asset_id, error);
                }
                Err(error) => {
                    tracing::error!("rehash_pending_fingerprints: blocking task failed for {}: {}", asset_id, error);
                }
            }
        }

        tracing::info!("rehash_pending_fingerprints: successfully rehashed {}/{}", rehashed_count, total_pending);
        Ok(rehashed_count)
    }

    async fn delete_fingerprint(&self, asset_id: &str) -> AppResult<()> {
        sqlx::query!(
            r#"DELETE FROM duplicate_fingerprints WHERE asset_id = ?"#,
            asset_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete fingerprint for asset {}: {:?}", asset_id, e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    async fn remove_candidate_from_groups(&self, asset_id: &str) -> AppResult<()> {
        // Find all groups this asset belongs to
        let affected_group_ids: Vec<String> = sqlx::query_scalar!(
            r#"SELECT group_id as "group_id!" FROM duplicate_candidates WHERE asset_id = ?"#,
            asset_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find groups for asset {}: {:?}", asset_id, e);
            AppError::Database(e)
        })?;

        // Remove the candidate from all groups
        sqlx::query!(
            r#"DELETE FROM duplicate_candidates WHERE asset_id = ?"#,
            asset_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove candidate {} from groups: {:?}", asset_id, e);
            AppError::Database(e)
        })?;

        // For each affected group, update the count and auto-resolve if < 2 candidates remain
        for group_id in affected_group_ids {
            let remaining_count: i32 = sqlx::query_scalar!(
                r#"SELECT COUNT(*) as "count: i32" FROM duplicate_candidates WHERE group_id = ?"#,
                group_id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?;

            if remaining_count < 2 {
                // Auto-resolve: a group with 0 or 1 candidates is no longer a duplicate
                sqlx::query!(
                    r#"UPDATE duplicate_groups SET status = 'resolved', candidate_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
                    remaining_count,
                    group_id
                )
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;

                tracing::info!("DuplicateRepo: auto-resolved group {} (only {} candidate(s) remain)", group_id, remaining_count);
            } else {
                // Just update the count
                sqlx::query!(
                    r#"UPDATE duplicate_groups SET candidate_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
                    remaining_count,
                    group_id
                )
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
            }
        }

        Ok(())
    }
}

