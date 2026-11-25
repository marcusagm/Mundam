//! Reconstructs a sealed item from a consistent, owner-scoped view of staging.
use super::{check_active, lookup, parse_manifest, parts::PARTS_PER_ITEM, SqliteImportStaging};
use crate::core::{
    imports::{
        staging::{SealedStagedItem, UploadError, UploadOwner, LIMITS, PART_BYTES},
        ImportManifest,
    },
    AppError,
};
use sha2::{Digest, Sha256};
use sqlx::SqliteConnection;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

#[cfg(test)]
mod tests;

impl SqliteImportStaging {
    /// Lists the immutable declarations once every item in the batch has arrived.
    ///
    /// # Errors
    /// Rejects inaccessible, cancelled, expired, incomplete or inconsistent batches.
    pub async fn sealed_manifest(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: &str,
    ) -> Result<ImportManifest, UploadError> {
        let mut snapshot = self.0.begin().await?;
        let manifest = complete_manifest(&mut snapshot, owner, batch_id).await?;
        snapshot.commit().await?;
        Ok(manifest)
    }

    /// Copies one item in order into an unpublished, host-owned output.
    ///
    /// Callers discard partial outputs on failure and sync complete files before publication.
    /// The snapshot proves the received content. Live authorization is checked at publication.
    ///
    /// # Errors
    /// Rejects unavailable declarations and missing, malformed or corrupted parts.
    /// Returns writer errors without changing staged content.
    pub async fn materialize_into(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: &str,
        item_id: &str,
        output: &mut (impl AsyncWrite + Unpin),
    ) -> Result<SealedStagedItem, UploadError> {
        let mut snapshot = self.0.begin().await?;
        let sealed = materialize(&mut snapshot, owner, batch_id, item_id, output).await?;
        snapshot.commit().await?;
        Ok(sealed)
    }
}

pub(super) async fn materialize(
    snapshot: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    batch_id: &str,
    item_id: &str,
    output: &mut (impl AsyncWrite + Unpin),
) -> Result<SealedStagedItem, UploadError> {
    let manifest = complete_manifest(snapshot, owner, batch_id).await?;
    let (position, item) = manifest
        .items
        .into_iter()
        .enumerate()
        .find(|(_, item)| item.item_id == item_id)
        .ok_or(UploadError::Missing)?;
    let folder_id = manifest.folder_id.ok_or(UploadError::StagingCorrupt)?;
    let batch_uuid = Uuid::parse_str(batch_id).map_err(|_| UploadError::StagingCorrupt)?;
    let mut remaining = item.byte_length;
    let mut digest = Sha256::new();
    for part_number in 0..remaining.div_ceil(PART_BYTES as u64) {
        let sequence = position as i64 * PARTS_PER_ITEM + part_number as i64;
        let expected_size = remaining.min(PART_BYTES as u64);
        let (part_digest, content): (String, Vec<u8>) = sqlx::query_as(
            "SELECT digest, content FROM import_parts
                 WHERE batch_id = ? AND sequence = ? AND item_id = ? AND part_number = ?
                    AND length(content) = ? AND length(digest) = 64",
        )
        .bind(batch_id)
        .bind(sequence)
        .bind(&item.item_id)
        .bind(part_number as i64)
        .bind(expected_size as i64)
        .fetch_optional(&mut *snapshot)
        .await?
        .ok_or(UploadError::StagingCorrupt)?;
        if format!("{:x}", Sha256::digest(&content)) != part_digest {
            return Err(UploadError::StagingCorrupt);
        }
        digest.update(&content);
        output.write_all(&content).await.map_err(AppError::from)?;
        remaining -= expected_size;
    }
    output.flush().await.map_err(AppError::from)?;
    Ok(SealedStagedItem {
        batch_id: batch_uuid,
        folder_id,
        item,
        sha256: digest.finalize().into(),
    })
}

pub(super) async fn complete_manifest(
    snapshot: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    batch_id: &str,
) -> Result<ImportManifest, UploadError> {
    let row = lookup(snapshot, owner, batch_id).await?;
    check_active(&row)?;
    let mut manifest = parse_manifest(&row.manifest)?;
    manifest.folder_id = Some(row.folder_id);
    let manifest = manifest
        .normalize(&LIMITS)
        .map_err(|_| UploadError::StagingCorrupt)?;
    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_id, COUNT(*) FROM import_parts WHERE batch_id = ? GROUP BY item_id LIMIT ?",
    )
    .bind(batch_id)
    .bind(LIMITS.max_items as i64 + 1)
    .fetch_all(snapshot)
    .await?;
    for (item_id, count) in &counts {
        let item = manifest
            .items
            .iter()
            .find(|item| &item.item_id == item_id)
            .ok_or(UploadError::StagingCorrupt)?;
        if *count as u64 > item.byte_length.div_ceil(PART_BYTES as u64) {
            return Err(UploadError::StagingCorrupt);
        }
    }
    for item in &manifest.items {
        let confirmed = counts
            .iter()
            .find(|(item_id, _)| item_id == &item.item_id)
            .map_or(0, |(_, count)| *count as u64);
        if confirmed != item.byte_length.div_ceil(PART_BYTES as u64) {
            return Err(UploadError::Incomplete);
        }
    }
    Ok(manifest)
}
