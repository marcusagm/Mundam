//! Folder command handlers.
//!
//! Encapsulates SQL mutations for folder creation, cascade removal, and
//! recursive path renaming across both the `folders` and `assets` tables.
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{CreateFolderPayload, RemoveFolderPayload, RenameFolderPayload};
use crate::core::models::asset::{Asset, AssetState};
use super::shared;
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub async fn handle_create_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: CreateFolderPayload,
) -> AppResult<Asset> {
    let folder_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let path_str = payload.path.to_string_lossy().to_string();

    // UPSERT pattern for folders to handle existing paths and update hierarchy
    let result = sqlx::query!(
        r#"
        INSERT INTO folders (id, parent_id, name, path, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET 
            parent_id = COALESCE(excluded.parent_id, folders.parent_id),
            updated_at = excluded.updated_at
        RETURNING id as "id!"
        "#,
        folder_id,
        payload.parent_id,
        payload.name,
        path_str,
        now,
        now
    )
    .fetch_one(&mut **tx)
    .await?;

    let actual_id = result.id;

    super::super::folder_hierarchy::adopt_orphaned_children(&mut **tx, &actual_id, &path_str)
        .await?;

    let op_payload = serde_json::to_value(&payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?;

    shared::log_operation(
        tx,
        "CREATE_FOLDER",
        &actual_id,
        op_payload,
        "COMPLETED",
        None,
    )
    .await?;

    // Return a dummy asset with the folder info
    Ok(Asset {
        id: actual_id,
        name: payload.name.clone(),
        path: payload.path.clone(),
        state: AssetState::Idle,
        format_type: "folder".to_string(),
        family: "FOLDER".to_string(),
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
        folder_id: payload.parent_id.clone(),
        thumbnail_path: None,
        rating: None,
        notes: None,
    is_favorite: false,
    deleted_at: None,
    })
}

pub async fn handle_remove_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: RemoveFolderPayload,
) -> AppResult<Asset> {
    let folder_id_ref = &payload.folder_id;

    // 1. Get folder path to include in the event
    let folder_path = sqlx::query!(
        r#"SELECT path as "path!" FROM folders WHERE id = ?"#,
        folder_id_ref
    )
    .fetch_optional(&mut **tx)
    .await?
    .map(|r| r.path)
    .ok_or_else(|| AppError::NotFound(format!("Folder ID not found: {}", folder_id_ref)))?;

    super::folder_removal::delete_assets(tx, folder_id_ref).await?;
    sqlx::query!("DELETE FROM folders WHERE id = ?", folder_id_ref)
        .execute(&mut **tx)
        .await?;

    // 3. Audit Log
    let op_payload = serde_json::to_value(&payload)
        .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?;

    shared::log_operation(
        tx,
        "REMOVE_FOLDER",
        folder_id_ref,
        op_payload,
        "COMPLETED",
        None,
    )
    .await?;

    // Return a dummy/tombstone Asset
    Ok(Asset {
        id: payload.folder_id.clone(),
        name: "deleted_folder".to_string(),
        path: std::path::PathBuf::from(folder_path),
        state: AssetState::Offline,
        format_type: "folder".to_string(),
        family: "FOLDER".to_string(),
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

pub async fn handle_rename_folder(
    tx: &mut Transaction<'_, Sqlite>,
    payload: RenameFolderPayload,
) -> AppResult<Asset> {
    let old_path_str = payload.old_path.to_string_lossy().to_string();
    let new_path_str = payload.new_path.to_string_lossy().to_string();
    let now = Utc::now();

    // 1. Update the target folder name and path
    let folder_name = payload
        .new_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    sqlx::query!(
        "UPDATE folders SET name = ?, path = ?, updated_at = ? WHERE id = ?",
        folder_name,
        new_path_str,
        now,
        payload.folder_id
    )
    .execute(&mut **tx)
    .await?;

    // 2. Recursively update all subfolder paths
    sqlx::query!(
        r#"
        UPDATE folders 
        SET path = ? || SUBSTR(path, LENGTH(?) + 1),
            updated_at = ?
        WHERE path LIKE ? || '/%'
        "#,
        new_path_str,
        old_path_str,
        now,
        old_path_str
    )
    .execute(&mut **tx)
    .await?;

    // 3. Recursively update all asset paths
    sqlx::query!(
        r#"
        UPDATE assets 
        SET path = ? || SUBSTR(path, LENGTH(?) + 1),
            updated_at = ?
        WHERE path LIKE ? || '%'
        "#,
        new_path_str,
        old_path_str,
        now,
        old_path_str
    )
    .execute(&mut **tx)
    .await?;

    // 4. Audit Log
    shared::log_operation(
        tx,
        "RENAME_FOLDER",
        &payload.folder_id,
        serde_json::to_value(&payload).unwrap_or_default(),
        "COMPLETED",
        None,
    )
    .await?;

    Ok(Asset {
        id: payload.folder_id,
        name: folder_name,
        path: payload.new_path,
        state: AssetState::Idle,
        format_type: "folder".to_string(),
        family: "FOLDER".to_string(),
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
