//! Thumbnail command handlers.
//!
//! Encapsulates SQL mutations for persisting and invalidating asset thumbnail paths,
//! decoupled from the actual image generation workers.
use crate::core::error::{AppError, AppResult};
use crate::core::processing::ProcessingKind;
use crate::core::models::asset::{Asset, AssetState};
use super::shared;
use chrono::Utc;
use sqlx::{Sqlite, Transaction};

/// Persists a generated thumbnail path for an asset and transitions its state to `Thumbnailed`.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset.
/// * `thumbnail_path` - The filesystem path to the generated thumbnail.
///
/// # Errors
///
/// Returns `AppError` if the asset record does not exist or the database update fails.
pub async fn handle_update_thumbnail(
    tx: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
    thumbnail_path: &str,
) -> AppResult<Asset> {
    if thumbnail_path.is_empty() {
        return Err(AppError::ValidationFailed("A successful thumbnail requires a generated file".into()));
    }
    let now = Utc::now();
    let state_thumb = AssetState::Thumbnailed.to_string();

    // 1. Update Asset
    let updated = sqlx::query(
        "UPDATE assets SET thumbnail_path = ?, state = ?, updated_at = ?
         WHERE id = ? AND deleted_at IS NULL AND LOWER(state) NOT IN ('trashed', 'offline')",
    )
    .bind(thumbnail_path).bind(state_thumb).bind(now).bind(asset_id)
    .execute(&mut **tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Asset is no longer available for thumbnail processing".into()));
    }
    crate::infra::processing::enqueue(tx, ProcessingKind::Colors, asset_id).await?;

    // 2. Audit Log
    shared::log_operation(
        tx,
        "UPDATE_THUMBNAIL",
        asset_id,
        serde_json::json!({ "path": thumbnail_path }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, asset_id).await
}

/// Clears an asset's cached thumbnail path, signalling that regeneration is required.
///
/// A new pending generation is stored in this transaction and recovered by polling,
/// independently of viewport priority or delivery of `ThumbnailInvalidated`.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset.
///
/// # Errors
///
/// Returns `AppError` if the asset record does not exist or the database update fails.
pub async fn handle_regenerate_thumbnail(
    tx: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
) -> AppResult<Asset> {
    let now = Utc::now();

    // 1. Clear thumbnail_path in assets table
    sqlx::query(
        "UPDATE assets SET thumbnail_path = NULL, state = 'Indexed', updated_at = ?
         WHERE id = ? AND deleted_at IS NULL AND LOWER(state) NOT IN ('trashed', 'offline')",
    )
    .bind(now).bind(asset_id)
    .execute(&mut **tx)
    .await?;
    crate::infra::processing::enqueue(tx, ProcessingKind::Thumbnail, asset_id).await?;

    // 2. Audit Log
    shared::log_operation(
        tx,
        "REGENERATE_THUMBNAIL",
        asset_id,
        serde_json::json!({ "asset_id": asset_id }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, asset_id).await
}
