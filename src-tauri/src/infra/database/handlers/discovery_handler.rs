//! Transactional persistence and audit logging for asset discovery metadata.
use super::shared;
use crate::core::error::AppResult;
use crate::core::models::asset::Asset;
use chrono::Utc;
use sqlx::{Sqlite, Transaction};

/// Persists discovery metadata within the ledger transaction and audit trail.
///
/// # Errors
/// Returns validation errors for unsafe URLs, NotFound for missing assets, or database errors.
pub async fn handle_update_discovery(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: crate::core::ledger::command::UpdateAssetDiscoveryPayload,
) -> AppResult<Asset> {
    let mut asset = shared::fetch_asset_by_id(transaction, &payload.asset_id).await?;
    if let Some(keywords) = payload.keywords {
        asset.keywords = crate::core::ledger::discovery_metadata::normalize_keywords(&keywords);
    }
    if let Some(source_url) = payload.source_url {
        asset.source_url =
            crate::core::ledger::discovery_metadata::normalize_source_url(&source_url)?;
    }
    let keywords = serde_json::to_string(&asset.keywords)
        .map_err(|error| crate::core::error::AppError::Internal(error.to_string()))?;
    sqlx::query("UPDATE assets SET keywords = ?, source_url = ?, updated_at = ? WHERE id = ?")
        .bind(keywords)
        .bind(&asset.source_url)
        .bind(Utc::now())
        .bind(&asset.id)
        .execute(&mut **transaction)
        .await?;
    shared::log_operation(
        transaction,
        "UPDATE_ASSET_DISCOVERY",
        &asset.id,
        serde_json::json!({"keywords": asset.keywords, "source_url": asset.source_url}),
        "COMPLETED",
        None,
    )
    .await?;
    shared::fetch_asset_by_id(transaction, &asset.id).await
}
