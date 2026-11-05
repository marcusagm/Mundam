//! Tag management command handlers.
//!
//! Encapsulates SQL mutations for tag CRUD operations and batch tag
//! association/dissociation across multiple assets.
use super::shared;
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{
    BatchTagsPayload, CreateTagPayload, UpdateTagPayload, UpdateTagsPayload,
};
use crate::core::models::asset::{Asset, AssetState};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

/// Applies incremental tag additions and removals for a single asset.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset ID, tags to add, and tags to remove.
///
/// # Errors
///
/// Returns `AppError` if the asset record is not found or the database operation fails.
pub async fn handle_update_tags(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateTagsPayload,
) -> AppResult<Asset> {
    let now = Utc::now();

    // 1. Add Tags by ID
    for tag_id in &payload.tags_to_add {
        sqlx::query!(
            r#"
            INSERT INTO asset_tags (asset_id, tag_id)
            VALUES (?, ?)
            ON CONFLICT DO NOTHING
            "#,
            payload.asset_id,
            tag_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // 2. Remove Tags by ID
    for tag_id in &payload.tags_to_remove {
        sqlx::query!(
            r#"
            DELETE FROM asset_tags
            WHERE asset_id = ? AND tag_id = ?
            "#,
            payload.asset_id,
            tag_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // 3. Update Asset timestamp
    sqlx::query!(
        "UPDATE assets SET updated_at = ? WHERE id = ?",
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    // 4. Audit Log
    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "UPDATE_TAGS",
        &payload.asset_id,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Creates a new taxonomy tag with optional hierarchy and color.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the tag name, optional parent ID, and optional color.
///
/// # Errors
///
/// Returns `AppError` if the tag name conflicts with an existing one or the insert fails.
pub async fn handle_create_tag(
    tx: &mut Transaction<'_, Sqlite>,
    payload: CreateTagPayload,
) -> AppResult<Asset> {
    let tag_id = Uuid::new_v4().to_string();

    let normalized_parent_id = payload.parent_id.as_ref().and_then(|id| {
        if id.is_empty() || id == "0" {
            None
        } else {
            Some(id.clone())
        }
    });

    if let Some(parent_id) = &normalized_parent_id {
        super::tag_hierarchy::validate_parent(tx, &tag_id, parent_id).await?;
    }
    sqlx::query("INSERT INTO tags (id, name, color, parent_id, order_index, inherits_color) VALUES (?, ?, ?, ?, (SELECT COALESCE(MAX(order_index), -100) + 100 FROM tags WHERE parent_id IS ?), ?)")
        .bind(&tag_id).bind(&payload.name).bind(&payload.color)
        .bind(&normalized_parent_id).bind(&normalized_parent_id).bind(payload.color.is_none())
        .execute(&mut **tx).await?;
    super::tag_hierarchy::refresh_colors(tx).await?;

    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "CREATE_TAG",
        &tag_id,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: tag_id,
        name: payload.name.clone(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "tag".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Updates the mutable properties of an existing tag using a dynamic query.
///
/// Only the fields present in the payload will be modified, preserving unset values.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the tag ID and any optional fields to update.
///
/// # Errors
///
/// Returns `AppError` if the tag is not found or the database update fails.
pub async fn handle_update_tag(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateTagPayload,
) -> AppResult<Asset> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?)")
        .bind(&payload.id)
        .fetch_one(&mut **tx)
        .await?;
    if !exists {
        return Err(AppError::NotFound(payload.id.clone()));
    }
    if let Some(parent_id) = &payload.parent_id {
        super::tag_hierarchy::validate_parent(tx, &payload.id, parent_id).await?;
    }
    let inheritance = payload
        .inherits_color
        .or_else(|| payload.color.as_ref().map(|_| false));
    let mut set_clauses = Vec::new();

    if inheritance.is_some() {
        set_clauses.push("inherits_color = ?");
    }
    if payload.name.is_some() {
        set_clauses.push("name = ?");
    }
    if payload.color.is_some() {
        set_clauses.push("color = ?");
    }
    if payload.parent_id.is_some() {
        set_clauses.push("parent_id = ?");
    }
    if payload.order_index.is_some() {
        set_clauses.push("order_index = ?");
    }

    if !set_clauses.is_empty() {
        let update_sql = format!("UPDATE tags SET {} WHERE id = ?", set_clauses.join(", "));
        let mut query = sqlx::query(&update_sql);

        if let Some(inherits_color) = inheritance {
            query = query.bind(inherits_color);
        }
        if let Some(ref tag_name) = payload.name {
            query = query.bind(tag_name);
        }
        if let Some(ref tag_color) = payload.color {
            query = query.bind(tag_color);
        }
        if let Some(ref parent_tag_id) = payload.parent_id {
            if parent_tag_id.is_empty() || parent_tag_id == "0" {
                query = query.bind(None::<String>);
            } else {
                query = query.bind(parent_tag_id);
            }
        }
        if let Some(sort_order) = payload.order_index {
            query = query.bind(sort_order);
        }

        query = query.bind(&payload.id);
        query.execute(&mut **tx).await?;
    }
    if payload.color.is_some() || payload.parent_id.is_some() || inheritance.is_some() {
        super::tag_hierarchy::refresh_colors(tx).await?;
    }

    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "UPDATE_TAG",
        &payload.id,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: payload.id.clone(),
        name: "updated_tag".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "tag".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Deletes a tag and removes all its asset associations.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `id` - The unique identifier of the tag to delete.
///
/// # Errors
///
/// Returns `AppError` if the database operation fails.
pub async fn handle_delete_tag(tx: &mut Transaction<'_, Sqlite>, id: String) -> AppResult<Asset> {
    // 1. Remove all asset associations first
    sqlx::query!("DELETE FROM asset_tags WHERE tag_id = ?", id)
        .execute(&mut **tx)
        .await?;

    // 2. Delete the tag itself
    sqlx::query!("DELETE FROM tags WHERE id = ?", id)
        .execute(&mut **tx)
        .await?;

    super::tag_hierarchy::refresh_colors(tx).await?;

    shared::log_operation(
        tx,
        "DELETE_TAG",
        &id,
        serde_json::json!({ "tag_id": id }),
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: id.clone(),
        name: "deleted_tag".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Offline,
        format_type: "tag".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Associates a set of tags with a set of assets in a single atomic batch.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the list of asset IDs and tag IDs to associate.
///
/// # Errors
///
/// Returns `AppError` if any of the insert operations fail.
pub async fn handle_add_tags_to_assets_batch(
    tx: &mut Transaction<'_, Sqlite>,
    payload: BatchTagsPayload,
) -> AppResult<Asset> {
    if !payload.asset_ids.is_empty() && !payload.tag_ids.is_empty() {
        for current_asset_id in &payload.asset_ids {
            for current_tag_id in &payload.tag_ids {
                sqlx::query!(
                    "INSERT INTO asset_tags (asset_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
                    current_asset_id,
                    current_tag_id
                )
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "ADD_TAGS_BATCH",
        "batch",
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: "batch_add_tags".to_string(),
        name: "batch_operation".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "batch".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Removes a set of tags from a set of assets in a single atomic batch.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the list of asset IDs and tag IDs to disassociate.
///
/// # Errors
///
/// Returns `AppError` if any of the delete operations fail.
pub async fn handle_remove_tags_from_assets_batch(
    tx: &mut Transaction<'_, Sqlite>,
    payload: BatchTagsPayload,
) -> AppResult<Asset> {
    if !payload.asset_ids.is_empty() && !payload.tag_ids.is_empty() {
        for current_asset_id in &payload.asset_ids {
            for current_tag_id in &payload.tag_ids {
                sqlx::query!(
                    "DELETE FROM asset_tags WHERE asset_id = ? AND tag_id = ?",
                    current_asset_id,
                    current_tag_id
                )
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "REMOVE_TAGS_BATCH",
        "batch",
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: "batch_remove_tags".to_string(),
        name: "batch_operation".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "batch".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Replaces all current tags of each asset with a new set in a single atomic batch.
///
/// # Arguments
///
/// * `tx` - The active database transaction.
/// * `payload` - Contains the asset IDs and the new tag IDs that will replace all existing ones.
///
/// # Errors
///
/// Returns `AppError` if any database operation fails.
pub async fn handle_replace_tags_for_assets_batch(
    tx: &mut Transaction<'_, Sqlite>,
    payload: BatchTagsPayload,
) -> AppResult<Asset> {
    if !payload.asset_ids.is_empty() {
        for current_asset_id in &payload.asset_ids {
            // Remove all existing tags for this asset
            sqlx::query!(
                "DELETE FROM asset_tags WHERE asset_id = ?",
                current_asset_id
            )
            .execute(&mut **tx)
            .await?;

            // Add the new set of tags
            for current_tag_id in &payload.tag_ids {
                sqlx::query!(
                    "INSERT INTO asset_tags (asset_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
                    current_asset_id,
                    current_tag_id
                )
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    let operation_payload = serde_json::to_value(&payload).map_err(|serialization_error| {
        AppError::Internal(format!(
            "Failed to serialize payload: {}",
            serialization_error
        ))
    })?;

    shared::log_operation(
        tx,
        "REPLACE_TAGS_BATCH",
        "batch",
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: "batch_replace_tags".to_string(),
        name: "batch_operation".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "batch".to_string(),
        family: "TAG".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: None,
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: None,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}
