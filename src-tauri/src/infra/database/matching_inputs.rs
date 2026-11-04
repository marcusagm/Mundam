//! Reads matching inputs inside the same write transaction that admits their reservation.

use crate::core::{
    models::{DuplicateFingerprint, DuplicateRuleSet},
    AppResult,
};
use sqlx::{Row, Sqlite, Transaction};

pub(super) async fn policy(
    transaction: &mut Transaction<'_, Sqlite>,
) -> AppResult<DuplicateRuleSet> {
    super::super::super::duplicate_policy::read(&mut **transaction)
        .await?
        .ok_or_else(|| {
            crate::core::AppError::Internal("No duplicate comparison policy is available".into())
        })
}

pub(super) async fn fingerprints(
    transaction: &mut Transaction<'_, Sqlite>,
) -> AppResult<Vec<DuplicateFingerprint>> {
    let rows = sqlx::query("SELECT df.* FROM duplicate_fingerprints df JOIN assets a ON a.id = df.asset_id
        LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = 'fingerprint'
        WHERE a.deleted_at IS NULL AND LOWER(a.state) NOT IN ('trashed', 'offline')
          AND df.content_hash IS NOT NULL AND df.content_hash NOT LIKE 'pending_%' AND df.content_hash NOT LIKE 'hash_%'
          AND df.fingerprint_version >= 5
          AND (task.asset_id IS NULL OR task.source_stamp = (a.path || '|' || a.file_size || '|' || COALESCE(a.modified_at, '')))
        ORDER BY df.asset_id").fetch_all(&mut **transaction).await?;
    rows.into_iter()
        .map(|row| {
            Ok(DuplicateFingerprint {
                asset_id: row.try_get("asset_id")?,
                content_hash: row.try_get("content_hash")?,
                perceptual_hash: row.try_get("perceptual_hash")?,
                block_hash: row.try_get("block_hash")?,
                thumb_hash: row.try_get("thumb_hash")?,
                width: row.try_get("width")?,
                height: row.try_get("height")?,
                file_size: row.try_get("file_size")?,
                mime_type: row.try_get("mime_type")?,
                format_family: row.try_get("format_family")?,
                color_profile: row.try_get("color_profile")?,
                orientation: row.try_get("orientation")?,
                fingerprint_version: row.try_get("fingerprint_version")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}
