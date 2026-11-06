//! Smart Folder command handlers.
//!
//! Encapsulates SQL mutations for saved search (SmartFolder) CRUD operations.
//! Smart folders store a JSON query expression and are not backed by a real
//! filesystem directory.
use super::shared;
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{
    CreateSmartFolderPayload, DeleteSmartFolderPayload, UpdateSmartFolderPayload,
};
use crate::core::models::asset::{Asset, AssetState};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub async fn handle_create_smart_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: CreateSmartFolderPayload,
) -> AppResult<Asset> {
    let sf_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query!(
        r#"INSERT INTO smart_folders (id, name, query_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"#,
        sf_id,
        payload.name,
        payload.query_json,
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    let op_payload = serde_json::to_value(&payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?;

    shared::log_operation(
        tx,
        "CREATE_SMART_FOLDER",
        &sf_id,
        op_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: sf_id,
        name: payload.name.clone(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "smart_folder".to_string(),
        family: "SMART_FOLDER".to_string(),
        file_size: 0,
        created_at: Some(now),
        modified_at: Some(now),
        added_at: Some(now),
        updated_at: Some(now),
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

pub async fn handle_update_smart_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: UpdateSmartFolderPayload,
) -> AppResult<Asset> {
    let now = Utc::now();

    sqlx::query!(
        "UPDATE smart_folders SET name = ?, query_json = ?, updated_at = ? WHERE id = ?",
        payload.name,
        payload.query_json,
        now,
        payload.id
    )
    .execute(&mut **tx)
    .await?;

    let op_payload = serde_json::to_value(&payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?;

    shared::log_operation(
        tx,
        "UPDATE_SMART_FOLDER",
        &payload.id,
        op_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: payload.id.clone(),
        name: "updated_smart_folder".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Idle,
        format_type: "smart_folder".to_string(),
        family: "SMART_FOLDER".to_string(),
        file_size: 0,
        created_at: None,
        modified_at: None,
        added_at: None,
        updated_at: Some(now),
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

pub async fn handle_delete_smart_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: DeleteSmartFolderPayload,
) -> AppResult<Asset> {
    sqlx::query!("DELETE FROM smart_folders WHERE id = ?", payload.id)
        .execute(&mut **tx)
        .await?;

    let op_payload = serde_json::to_value(&payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?;

    shared::log_operation(
        tx,
        "DELETE_SMART_FOLDER",
        &payload.id,
        op_payload,
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: payload.id.clone(),
        name: "deleted_smart_folder".to_string(),
        path: std::path::PathBuf::new(),
        state: AssetState::Offline,
        format_type: "smart_folder".to_string(),
        family: "SMART_FOLDER".to_string(),
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
