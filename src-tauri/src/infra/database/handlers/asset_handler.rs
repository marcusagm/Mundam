//! Asset lifecycle command handlers.
//!
//! Handles SQL mutations for asset creation (single and batch),
//! move-detection recovery, logical/physical deletion via the Outbox pattern,
//! path/name updates, state transitions, and folder assignment.
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{CreateAssetPayload, UpdateAssetPayload};
use crate::core::models::asset::{Asset, AssetState};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use tracing::info;
use uuid::Uuid;

use super::shared;

/// Handles the creation of a new asset.
///
/// Executes an UPSERT operation for the asset and resolves move signatures.
///
/// # Arguments
///
/// * `transaction` - The database transaction.
/// * `payload` - The payload containing the asset details.
///
/// # Errors
///
/// Returns `AppError` if validation fails or the database query encounters an error.
pub async fn handle_create(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: CreateAssetPayload,
) -> AppResult<Asset> {
    let asset_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let name = payload
        .path
        .file_name()
        .and_then(|file_name_os| file_name_os.to_str())
        .ok_or_else(|| AppError::ValidationFailed("Invalid file path".to_string()))?;
    let path_string = payload.path.to_string_lossy().to_string();
    let state_string = payload.state_init.to_string();
    let file_size_i64 = payload.file_size as i64;

    let created_at_val = payload.created_at.unwrap_or(now);
    let modified_at_val = payload.modified_at.unwrap_or(now);
    let added_at_val = now;

    if let Some(filesystem_created_at) = payload.created_at {
        let move_candidates: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, folder_id, path FROM assets WHERE file_size = ? AND created_at = ?",
        )
        .bind(file_size_i64)
        .bind(filesystem_created_at)
        .fetch_all(&mut **transaction)
        .await?;

        for (existing_asset_id, _old_folder_id, old_path_string) in &move_candidates {
            if !std::path::Path::new(old_path_string).exists() && old_path_string != &path_string {
                tracing::info!(
                    "Ledger: MOVE DETECTED (V1 recovery). Updating asset {} from '{}' to '{}'",
                    existing_asset_id,
                    old_path_string,
                    path_string
                );

                let folder_id_for_update = payload.folder_id.as_deref();
                sqlx::query!(
                    "UPDATE assets SET path = ?, name = ?, folder_id = ?, modified_at = ?, updated_at = ?, state = ? WHERE id = ?",
                    path_string,
                    name,
                    folder_id_for_update,
                    modified_at_val,
                    now,
                    state_string,
                    existing_asset_id
                )
                .execute(&mut **transaction)
                .await?;

                shared::log_operation(
                    transaction,
                    "CREATE_ASSET_MOVE_RECOVERY",
                    existing_asset_id,
                    serde_json::json!({
                        "old_path": old_path_string,
                        "new_path": path_string,
                        "recovery_type": "signature_match"
                    }),
                    "COMPLETED",
                    None,
                )
                .await?;

                return shared::fetch_asset_by_id(transaction, existing_asset_id).await;
            }
        }
    }

    let path_reference = &path_string;
    let state_reference = &state_string;
    let format_type_reference = &payload.format_type;
    let family_reference = &payload.family;
    let folder_id_reference = payload.folder_id.as_deref();
    let asset_id_final_reference = &asset_id;

    let row = sqlx::query!(
        r#"
        INSERT INTO assets (
            id, name, path, state, format_type, family, file_size,
            created_at, modified_at, added_at, updated_at, folder_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
            updated_at = excluded.updated_at
        RETURNING id as "id!"
        "#,
        asset_id_final_reference,
        name,
        path_reference,
        state_reference,
        format_type_reference,
        family_reference,
        file_size_i64,
        created_at_val,
        modified_at_val,
        added_at_val,
        now,
        folder_id_reference
    )
    .fetch_one(&mut **transaction)
    .await?;

    let asset_id_final = row.id.to_string();

    let operation_payload = serde_json::to_value(&payload)
        .map_err(|error| AppError::Internal(format!("Failed to serialize payload: {}", error)))?;

    shared::log_operation(
        transaction,
        "CREATE_ASSET",
        &asset_id_final,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    let asset = Asset {
        id: asset_id_final,
        name: name.to_string(),
        path: payload.path.clone(),
        state: payload.state_init,
        format_type: payload.format_type.clone(),
        family: payload.family.clone(),
        file_size: payload.file_size,
        created_at: Some(created_at_val),
        modified_at: Some(modified_at_val),
        added_at: Some(added_at_val),
        updated_at: Some(now),
        width: None,
        height: None,
        duration_secs: None,
        technical_payload: None,
        semantic_payload: None,
        dominant_color: None,
        folder_id: payload.folder_id.clone(),
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    };

    Ok(asset)
}

/// Handles the batch creation of multiple assets.
///
/// This avoids excessive locks and emits a unified response.
///
/// # Arguments
///
/// * `transaction` - The database transaction.
/// * `payloads` - A vector of create payloads.
///
/// # Errors
///
/// Returns `AppError` if validation fails or the database query encounters an error.
pub async fn handle_batch_create(
    transaction: &mut Transaction<'_, Sqlite>,
    payloads: Vec<CreateAssetPayload>,
) -> AppResult<Asset> {
    let mut created_assets = Vec::new();

    for payload in payloads {
        let asset_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let name = payload
            .path
            .file_name()
            .and_then(|file_name_os| file_name_os.to_str())
            .ok_or_else(|| AppError::ValidationFailed("Invalid file path".to_string()))?;
        let state_string = payload.state_init.to_string();
        let path_string = payload.path.to_string_lossy().to_string();
        let file_size_i64 = payload.file_size as i64;

        let created_at_val = payload.created_at.unwrap_or(now);
        let modified_at_val = payload.modified_at.unwrap_or(now);
        let added_at_val = now;

        let path_reference = &path_string;
        let state_reference = &state_string;
        let format_type_reference = &payload.format_type;
        let family_reference = &payload.family;
        let folder_id_reference = payload.folder_id.as_deref();
        let asset_id_reference = &asset_id;

        let row = sqlx::query!(
            r#"
            INSERT INTO assets (
                id, name, path, state, format_type, family, file_size,
                created_at, modified_at, added_at, updated_at, folder_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
                updated_at = excluded.updated_at
            RETURNING id as "id!"
            "#,
            asset_id_reference,
            name,
            path_reference,
            state_reference,
            format_type_reference,
            family_reference,
            file_size_i64,
            created_at_val,
            modified_at_val,
            added_at_val,
            now,
            folder_id_reference
        )
        .fetch_one(&mut **transaction)
        .await?;

        let asset_id = row.id;

        let operation_payload = serde_json::to_value(&payload).map_err(|error| {
            AppError::Internal(format!("Failed to serialize payload: {}", error))
        })?;

        shared::log_operation(
            transaction,
            "CREATE_ASSET_BATCH_MEMBER",
            &asset_id,
            operation_payload,
            "COMPLETED",
            None,
        )
        .await?;

        created_assets.push(Asset {
            id: asset_id.clone(),
            name: name.to_string(),
            path: payload.path.clone(),
            state: payload.state_init,
            format_type: payload.format_type.clone(),
            family: payload.family.clone(),
            file_size: payload.file_size,
            created_at: Some(created_at_val),
            modified_at: Some(modified_at_val),
            added_at: Some(added_at_val),
            updated_at: Some(now),
            width: None,
            height: None,
            duration_secs: None,
            technical_payload: None,
            semantic_payload: None,
            dominant_color: None,
            folder_id: payload.folder_id.clone(),
            thumbnail_path: None,
            rating: None,
            notes: None,
            is_favorite: false,
            deleted_at: None,
        });
    }

    created_assets
        .into_iter()
        .next()
        .ok_or_else(|| AppError::ValidationFailed("Empty batch".to_string()))
}

/// Handles the deletion of an asset.
///
/// Supports logical deletion (tombstone) and physical deletion via the Saga/Outbox pattern.
///
/// # Arguments
///
/// * `transaction` - The database transaction.
/// * `asset_id` - The optional ID of the asset.
/// * `path` - The optional file path of the asset.
/// * `physical_delete` - Whether to physically delete the file on disk.
///
/// # Errors
///
/// Returns `AppError` if the asset is not found or database constraints fail.
pub async fn handle_delete_asset(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: Option<String>,
    path: Option<std::path::PathBuf>,
    physical_delete: bool,
) -> AppResult<Asset> {
    let path_string = path
        .as_ref()
        .map(|path_reference| path_reference.to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string());
    tracing::info!(
        "Ledger: DeleteAsset START. asset_id: {:?}, path: {}",
        asset_id,
        path_string
    );

    let resolved_id: String = match (asset_id, path.clone()) {
        (Some(id), _) => {
            tracing::info!("Ledger: DeleteAsset resolved by ID: {}", id);
            id.clone()
        }
        (None, Some(path_reference)) => {
            match shared::resolve_asset_id_robust(transaction, &path_reference).await? {
                Some(id) => {
                    tracing::info!(
                        "Ledger: DeleteAsset resolved path '{}' to ID: {}",
                        path_reference.display(),
                        id
                    );
                    id
                }
                None => {
                    tracing::warn!("Ledger: DeleteAsset IGNORED - path '{}' not found in DB (even after robust fallback)", path_reference.display());
                    return Err(AppError::NotFound(format!(
                        "Asset not found at path: {}",
                        path_reference.display()
                    )));
                }
            }
        }
        _ => {
            tracing::error!("Ledger: DeleteAsset FAILED - missing ID and path");
            return Err(AppError::ValidationFailed(
                "DeleteAsset requires either asset_id or path".to_string(),
            ));
        }
    };

    let pre_delete_row = sqlx::query!(
        r#"SELECT folder_id as "folder_id?", name as "name!", deleted_at as "deleted_at?: chrono::DateTime<chrono::Utc>" FROM assets WHERE id = ?"#,
        resolved_id
    )
    .fetch_optional(&mut **transaction)
    .await?;

    if let Some(row) = &pre_delete_row {
        if row.deleted_at.is_some() && !physical_delete {
            tracing::info!("Ledger: DeleteAsset IGNORED - Asset {} is already in trash", resolved_id);
            return Err(AppError::NotFound(format!("Asset {} is already in trash", resolved_id)));
        }
    }

    let pre_delete_folder_id = pre_delete_row
        .as_ref()
        .and_then(|row| row.folder_id.clone());
    let pre_delete_name = pre_delete_row
        .as_ref()
        .map(|row| row.name.clone())
        .unwrap_or_else(|| "deleted".to_string());

    tracing::info!(
        "Ledger: DeleteAsset executing DELETE for ID {}",
        resolved_id
    );
    sqlx::query!("DELETE FROM assets WHERE id = ?", resolved_id)
        .execute(&mut **transaction)
        .await?;

    let status = if physical_delete {
        "PENDING"
    } else {
        "COMPLETED"
    };
    shared::log_operation(
        transaction,
        "DELETE_ASSET",
        &resolved_id,
        serde_json::json!({"physical": physical_delete, "path": path.map(|path_buf| path_buf.to_string_lossy().to_string())}),
        status,
        None,
    )
    .await?;

    tracing::info!("Ledger: DeleteAsset SUCCESS for ID {}", resolved_id);
    Ok(Asset {
        id: resolved_id,
        name: pre_delete_name,
        path: std::path::PathBuf::new(),
        state: AssetState::Offline,
        format_type: "".to_string(),
        family: "".to_string(),
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
        folder_id: pre_delete_folder_id,
        thumbnail_path: None,
        rating: None,
        notes: None,
        is_favorite: false,
        deleted_at: None,
    })
}

/// Handles updating an asset's path and name after a rename or move operation.
///
/// Performs robust asset ID resolution via both direct path match and the
/// NFC/NFD-aware fallback strategy. Includes a safety DELETE to prevent
/// UNIQUE constraint violations when the new path already exists as a
/// different record (stale ghost from a previous incomplete operation).
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `payload` - Contains the optional asset ID, old path, and new path.
///
/// # Errors
///
/// Returns `AppError::NotFound` if the asset cannot be resolved by ID or path.
/// Returns `AppError::ValidationFailed` if neither `asset_id` nor `old_path` is provided.
pub async fn handle_update_asset(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: UpdateAssetPayload,
) -> AppResult<Asset> {
    let now = Utc::now();
    let old_path_string = payload
        .old_path
        .as_ref()
        .map(|path_reference| path_reference.to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string());
    let new_path_string = payload.new_path.to_string_lossy().to_string();

    tracing::info!(
        "Ledger: UpdateAsset START. old: {}, new: {}",
        old_path_string,
        new_path_string
    );

    let asset_id: String = match (&payload.asset_id, &payload.old_path) {
        (Some(id), _) => {
            tracing::info!("Ledger: UpdateAsset resolved by ID: {}", id);
            id.clone()
        }
        (None, Some(old_path)) => {
            match shared::resolve_asset_id_robust(transaction, old_path).await? {
                Some(id) => {
                    tracing::info!(
                        "Ledger: UpdateAsset resolved old_path '{}' to ID: {}",
                        old_path.display(),
                        id
                    );
                    id
                }
                None => {
                    tracing::warn!("Ledger: UpdateAsset IGNORED - old_path '{}' not found in DB (even after robust fallback)", old_path.display());
                    return Err(AppError::NotFound(format!(
                        "Asset not found at path: {}",
                        old_path.display()
                    )));
                }
            }
        }
        _ => {
            tracing::error!("Ledger: UpdateAsset FAILED - missing both ID and old_path");
            return Err(AppError::ValidationFailed(
                "UpdateAsset requires either asset_id or old_path".to_string(),
            ));
        }
    };

    let new_name = payload
        .new_path
        .file_name()
        .and_then(|name_os_str| name_os_str.to_str())
        .ok_or_else(|| AppError::ValidationFailed("Invalid new file path".to_string()))?;
    let new_path_str = payload.new_path.to_string_lossy().to_string();

    info!(
        "Ledger: UpdateAsset safety DELETE checking for '{}' (collision prevention)",
        new_path_str
    );
    let delete_result = sqlx::query!(
        "DELETE FROM assets WHERE path = ? AND id != ?",
        new_path_str,
        asset_id
    )
    .execute(&mut **transaction)
    .await?;

    if delete_result.rows_affected() > 0 {
        info!(
            "Ledger: UpdateAsset collision DETECTED. Pruned {} record(s) for '{}'",
            delete_result.rows_affected(),
            new_path_str
        );
    }

    info!(
        "Ledger: UpdateAsset executing UPDATE for ID {} to NEW path '{}'",
        asset_id, new_path_str
    );
    sqlx::query!(
        "UPDATE assets SET path = ?, name = ?, updated_at = ? WHERE id = ?",
        new_path_str,
        new_name,
        now,
        asset_id
    )
    .execute(&mut **transaction)
    .await?;

    let operation_payload = serde_json::to_value(&payload)
        .map_err(|error| AppError::Internal(format!("Failed to serialize payload: {}", error)))?;

    shared::log_operation(
        transaction,
        "UPDATE_ASSET",
        &asset_id,
        operation_payload,
        "COMPLETED",
        None,
    )
    .await?;

    info!("Ledger: UpdateAsset SUCCESS for ID {}", asset_id);
    shared::fetch_asset_by_id(transaction, &asset_id).await
}

/// Marks an asset as stale, indicating it needs re-probing by the extraction pipeline.
///
/// This is typically triggered when the filesystem watcher detects that a file
/// has been modified in-place without a rename.
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset to mark.
///
/// # Errors
///
/// Returns `AppError` if the database update fails or the asset is not found.
pub async fn handle_mark_as_stale(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
) -> AppResult<Asset> {
    let now = Utc::now();
    let state_stale = AssetState::Stale.to_string();

    sqlx::query!(
        "UPDATE assets SET state = ?, updated_at = ? WHERE id = ?",
        state_stale,
        now,
        asset_id
    )
    .execute(&mut **transaction)
    .await?;

    shared::log_operation(
        transaction,
        "MARK_STALE",
        asset_id,
        serde_json::json!({}),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(transaction, asset_id).await
}

/// Assigns an asset to a folder (or unassigns it by passing `None`).
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset.
/// * `folder_id` - The target folder ID, or `None` to unassign.
///
/// # Errors
///
/// Returns `AppError` if the database update fails or the asset is not found.
pub async fn handle_set_asset_folder(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
    folder_id: Option<&str>,
) -> AppResult<Asset> {
    let now = Utc::now();

    sqlx::query!(
        "UPDATE assets SET folder_id = ?, updated_at = ? WHERE id = ?",
        folder_id,
        now,
        asset_id
    )
    .execute(&mut **transaction)
    .await?;

    shared::log_operation(
        transaction,
        "SET_ASSET_FOLDER",
        asset_id,
        serde_json::json!({ "folder_id": folder_id }),
        "COMPLETED",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(transaction, asset_id).await
}

/// Moves an asset to the trash by setting deleted_at.
/// 
/// This creates a soft delete record without immediately removing the file.
/// 
/// # Arguments
/// 
/// * `tx` - The active database transaction.
/// * `payload` - The payload containing the target asset_id.
/// 
/// # Errors
/// 
/// Returns `AppError` if the database update fails or the asset is not found.
pub async fn handle_move_to_trash(
    tx: &mut Transaction<'_, Sqlite>,
    payload: crate::core::ledger::command::MoveToTrashPayload,
) -> AppResult<Asset> {
    let now = chrono::Utc::now();
    sqlx::query!(
        "UPDATE assets SET deleted_at = ?, updated_at = ? WHERE id = ?",
        now,
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "MOVE_TO_TRASH",
        &payload.asset_id,
        serde_json::json!({}),
        "PENDING",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}

/// Restores an asset from the trash by clearing deleted_at.
/// 
/// Re-activates a soft deleted item, bringing it back to the library view.
/// 
/// # Arguments
/// 
/// * `tx` - The active database transaction.
/// * `payload` - The payload containing the target asset_id.
/// 
/// # Errors
/// 
/// Returns `AppError` if the database update fails or the asset is not found.
pub async fn handle_restore_from_trash(
    tx: &mut Transaction<'_, Sqlite>,
    payload: crate::core::ledger::command::RestoreFromTrashPayload,
) -> AppResult<Asset> {
    let now = chrono::Utc::now();
    sqlx::query!(
        "UPDATE assets SET deleted_at = NULL, updated_at = ? WHERE id = ?",
        now,
        payload.asset_id
    )
    .execute(&mut **tx)
    .await?;

    shared::log_operation(
        tx,
        "RESTORE_FROM_TRASH",
        &payload.asset_id,
        serde_json::json!({}),
        "PENDING",
        None,
    )
    .await?;

    shared::fetch_asset_by_id(tx, &payload.asset_id).await
}
