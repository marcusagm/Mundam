//! Asset metadata command handlers.
//!
//! Encapsulates SQL mutations for all metadata fields: colors, rating, notes,
//! technical metadata (envelope upsert), and format type corrections.
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{
    UpdateAssetColorsPayload, UpdateAssetNotesPayload, UpdateAssetRatingPayload,
    UpdateTechnicalMetadataPayload,
};
use crate::core::models::asset::Asset;
use super::shared;
use chrono::Utc;
use sqlx::{Sqlite, Transaction};

/// Updates an asset's dominant color palette, replacing the existing color set.
///
/// This is a replace-all strategy: all existing colors for the asset are first
/// deleted and the new set from the payload is inserted.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID and the full list of new color entries.
///
/// # Errors
///
/// Returns `AppError` if the asset record is not found or any color insert fails.
pub async fn handle_update_asset_colors(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateAssetColorsPayload,
) -> AppResult<Asset> {
    let now = Utc::now();
    let asset_id_reference = &payload.asset_id;
    let eligible: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM assets WHERE id = ? AND deleted_at IS NULL AND LOWER(state) NOT IN ('trashed', 'offline'))",
    ).bind(asset_id_reference).fetch_one(&mut **tx).await?;
    if !eligible {
        return Err(AppError::NotFound("Asset is no longer available for color processing".into()));
    }

    // 1. Delete existing colors for this asset
    sqlx::query!(
        "DELETE FROM asset_colors WHERE asset_id = ?",
        asset_id_reference
    )
    .execute(&mut **tx)
    .await?;

    // 2. Insert new colors
    for color in &payload.colors {
        sqlx::query!(
            r#"
            INSERT INTO asset_colors (asset_id, hex_color, lab_lightness, lab_green_red, lab_blue_yellow, percentage, rank)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            asset_id_reference,
            color.hex_color,
            color.lab_lightness,
            color.lab_green_red,
            color.lab_blue_yellow,
            color.percentage,
            color.rank
        )
        .execute(&mut **tx)
        .await?;
    }

    // 3. Update dominant_color in assets table if we have colors
    if let Some(dominant) = payload.colors.first() {
        sqlx::query("UPDATE assets SET dominant_color = ?, updated_at = ? WHERE id = ?")
            .bind(serde_json::json!(dominant.hex_color))
            .bind(now)
            .bind(asset_id_reference)
            .execute(&mut **tx)
            .await?;
    }

    // 4. Audit Log
    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "UPDATE_ASSET_COLORS",
        asset_id_reference,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Updates an asset's star rating.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID and the new rating value.
///
/// # Errors
///
/// Returns `AppError` if the database update fails.
pub async fn handle_update_rating(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateAssetRatingPayload,
) -> AppResult<Asset> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE assets SET rating = ?, updated_at = ? WHERE id = ?",
        payload.rating,
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "UPDATE_ASSET_RATING",
        &payload.asset_id,
        serde_json::json!({ "rating": payload.rating }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Updates the freeform text notes for an asset.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID and the new notes string.
///
/// # Errors
///
/// Returns `AppError` if the database update fails.
pub async fn handle_update_notes(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateAssetNotesPayload,
) -> AppResult<Asset> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE assets SET notes = ?, updated_at = ? WHERE id = ?",
        payload.notes,
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "UPDATE_ASSET_NOTES",
        &payload.asset_id,
        serde_json::json!({ "notes": payload.notes }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Toggles the favorite status for an asset.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID.
///
/// # Errors
///
/// Returns `AppError` if the database update fails.
pub async fn handle_toggle_favorite(
    tx: &mut Transaction<'_, Sqlite>,
    payload: crate::core::ledger::command::ToggleFavoritePayload,
) -> AppResult<Asset> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE assets SET is_favorite = NOT is_favorite, updated_at = ? WHERE id = ?",
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "TOGGLE_FAVORITE",
        &payload.asset_id,
        serde_json::json!({}),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Corrects the stored format type of an asset.
///
/// Used when the initial indexer guesses an incorrect format and a deeper
/// provider later confirms the canonical type.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset.
/// * `format` - The corrected format type string (e.g., "JPEG", "PNG").
///
/// # Errors
///
/// Returns `AppError` if the database update fails.
pub async fn handle_update_format(
    tx: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
    format: &str,
) -> AppResult<Asset> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE assets SET format_type = ?, updated_at = ? WHERE id = ?",
        format,
        now,
        asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "UPDATE_FORMAT",
        asset_id,
        serde_json::json!({ "format": format }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, asset_id).await
}

/// Upserts technical metadata (dimensions, duration, JSON payloads) into the envelope table.
///
/// Uses an `ON CONFLICT(asset_id) DO UPDATE` strategy to ensure idempotency:
/// re-running extraction for the same asset will safely overwrite the previous values.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID and all optional technical metadata fields.
///
/// # Errors
///
/// Returns `AppError` if the envelope insert/update or any database operation fails.
pub async fn handle_update_technical_metadata(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateTechnicalMetadataPayload,
) -> AppResult<Asset> {
    let now = Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO asset_metadata_envelope (
            asset_id, width, height, duration_secs, 
            technical_payload, semantic_payload, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(asset_id) DO UPDATE SET
            width = excluded.width,
            height = excluded.height,
            duration_secs = excluded.duration_secs,
            technical_payload = excluded.technical_payload,
            semantic_payload = excluded.semantic_payload,
            updated_at = excluded.updated_at
        "#,
        payload.asset_id,
        payload.width,
        payload.height,
        payload.duration_secs,
        payload.technical_payload,
        payload.semantic_payload,
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    // Update assets.updated_at to reflect metadata change
    sqlx::query!(
        "UPDATE assets SET updated_at = ? WHERE id = ?",
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "UPDATE_TECHNICAL_METADATA",
        &payload.asset_id,
        serde_json::to_value(&payload).unwrap_or_default(),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Records re-extraction intent while preserving the last successful palette.
///
/// The worker recovers this durable request even if the `ReextractAssetColors`
/// wake-up event emitted after commit is missed.
/// This handler is deliberately stateless (no access to EventBus) to maintain
/// the pure-function contract of all handlers.
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset.
///
/// # Errors
///
/// Returns `AppError::NotFound` if the asset does not exist.
pub async fn handle_reextract_colors(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
) -> AppResult<Asset> {
    crate::infra::processing::enqueue(transaction, crate::core::processing::ProcessingKind::Colors, asset_id).await?;

    shared::log_operation(
        transaction,
        "REEXTRACT_COLORS",
        asset_id,
        serde_json::json!({}),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(transaction, asset_id).await
}
