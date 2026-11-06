//! Folder command handlers.
//!
//! Encapsulates SQL mutations for folder creation, cascade removal, and
//! recursive path renaming across both the `folders` and `assets` tables.
use super::shared;
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{CreateFolderPayload, RemoveFolderPayload, RenameFolderPayload};
use crate::core::models::asset::{Asset, AssetState};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

/// Creates or discovers a folder and adopts catalogued children in the same transaction.
///
/// # Errors
/// Returns invalid paths, persistence and hierarchy validation errors.
pub async fn handle_create_folder(
    transaction: &mut Transaction<'_, Sqlite>,
    mut payload: CreateFolderPayload,
) -> AppResult<Asset> {
    if let Some(parent_id) = &payload.parent_id {
        let parent: String = sqlx::query_scalar("SELECT path FROM folders WHERE id = ?")
            .bind(parent_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("Folder parent no longer exists".into()))?;
        let canonical = std::path::Path::new(&parent).join(
            payload
                .path
                .file_name()
                .ok_or_else(|| AppError::ValidationFailed("Invalid folder path".into()))?,
        );
        if canonical != payload.path
            && !crate::infra::filesystem::identity::same(&payload.path, &canonical).await?
        {
            return Err(AppError::ValidationFailed(
                "Folder observation does not belong to its catalog parent".into(),
            ));
        }
        payload.path = canonical;
    }
    if let Some(identifier) = shared::resolve_folder_id_robust(transaction, &payload.path).await? {
        payload.path = sqlx::query_scalar::<_, String>("SELECT path FROM folders WHERE id = ?")
            .bind(identifier)
            .fetch_one(&mut **transaction)
            .await?
            .into();
    } else {
        let recovered =
            match crate::infra::database::query_handlers::library_identity::folder_in_connection(
                transaction,
                &payload.path,
            )
            .await
            {
                Ok(identifier) => identifier,
                Err(AppError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            };
        if let Some(identifier) = recovered {
            let original: String = sqlx::query_scalar("SELECT path FROM folders WHERE id = ?")
                .bind(&identifier)
                .fetch_one(&mut **transaction)
                .await?;
            return handle_rename_folder(
                transaction,
                RenameFolderPayload {
                    folder_id: identifier,
                    old_path: original.into(),
                    new_path: payload.path,
                },
            )
            .await;
        }
    }
    let folder_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let folder_path = payload.path.to_string_lossy().to_string();

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
        folder_path,
        now,
        now
    )
    .fetch_one(&mut **transaction)
    .await?;

    let actual_id = result.id;
    if let Ok(identity) = crate::infra::filesystem::identity::read(&payload.path).await {
        super::library::identities::folder(transaction, &actual_id, &identity).await?;
    }

    crate::infra::database::handlers::folders::hierarchy::adopt_orphaned_children(
        &mut **transaction,
        &actual_id,
        &folder_path,
    )
    .await?;

    let operation_payload = serde_json::to_value(&payload)
        .map_err(|error| AppError::Internal(format!("Failed to serialize payload: {}", error)))?;

    shared::log_operation(
        transaction,
        "CREATE_FOLDER",
        &actual_id,
        operation_payload,
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

/// Removes folder-owned catalog data while leaving physical policy to the coordinator.
///
/// # Errors
/// Returns missing folder and transactional persistence failures.
pub async fn handle_remove_folder(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: RemoveFolderPayload,
) -> AppResult<Asset> {
    let folder_identifier = &payload.folder_id;

    // 1. Get folder path to include in the event
    let folder_path = sqlx::query!(
        r#"SELECT path as "path!" FROM folders WHERE id = ?"#,
        folder_identifier
    )
    .fetch_optional(&mut **transaction)
    .await?
    .map(|row| row.path)
    .ok_or_else(|| AppError::NotFound(format!("Folder ID not found: {}", folder_identifier)))?;

    if let Some(expected) = &payload.expected_path {
        if expected.to_string_lossy() != folder_path || tokio::fs::try_exists(expected).await? {
            return Err(AppError::NotFound(
                "Folder deletion observation is stale".into(),
            ));
        }
    }
    super::folder_removal::delete_assets(transaction, folder_identifier).await?;
    sqlx::query!("DELETE FROM folders WHERE id = ?", folder_identifier)
        .execute(&mut **transaction)
        .await?;

    // 3. Audit Log
    let operation_payload = serde_json::to_value(&payload)
        .map_err(|error| AppError::Internal(format!("Failed to serialize payload: {}", error)))?;

    shared::log_operation(
        transaction,
        "REMOVE_FOLDER",
        folder_identifier,
        operation_payload,
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

/// Updates a complete catalog subtree while preserving stable identities.
///
/// # Errors
/// Returns inconsistent subtree paths, collisions and database failures.
pub async fn handle_rename_folder(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: RenameFolderPayload,
) -> AppResult<Asset> {
    if tokio::fs::try_exists(&payload.new_path).await?
        && !crate::infra::database::query_handlers::library_identity::matches_destination(
            transaction,
            crate::core::library::LibraryEntity::Folder,
            &payload.folder_id,
            &payload.new_path,
        )
        .await?
    {
        return Err(AppError::ValidationFailed(
            "Observed rename does not match the original folder identity".into(),
        ));
    }
    let now = Utc::now();

    let folder_name = payload
        .new_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::ValidationFailed("Invalid folder name".into()))?
        .to_string();
    let parent_path = payload
        .new_path
        .parent()
        .map(|path| path.to_string_lossy().to_string());
    let parent_id = if let Some(parent) = &parent_path {
        shared::resolve_folder_id_robust(transaction, std::path::Path::new(parent)).await?
    } else {
        None
    };
    let old_root = payload.old_path.to_string_lossy().to_string();
    let old_root = if cfg!(windows) {
        old_root.replace('/', "\\")
    } else {
        old_root
    };
    let comparison_path = if cfg!(windows) {
        "REPLACE(path,'/',char(92))"
    } else {
        "path"
    };
    let old_prefix = format!(
        "{}{}",
        old_root.trim_end_matches(['/', '\\']),
        std::path::MAIN_SEPARATOR
    );
    let new_root = payload.new_path.to_string_lossy().to_string();
    let new_prefix = format!(
        "{}{}",
        new_root.trim_end_matches(['/', '\\']),
        std::path::MAIN_SEPARATOR
    );
    let subtree = "WITH RECURSIVE subtree(id) AS (SELECT id FROM folders WHERE id = ? UNION SELECT folders.id FROM folders JOIN subtree ON folders.parent_id = subtree.id)";
    let invalid: i64 = sqlx::query_scalar(&format!("{subtree} SELECT (SELECT COUNT(*) FROM folders WHERE id IN (SELECT id FROM subtree) AND {comparison_path} != ? AND SUBSTR({comparison_path},1,LENGTH(?)) != ?) + (SELECT COUNT(*) FROM assets WHERE folder_id IN (SELECT id FROM subtree) AND SUBSTR({comparison_path},1,LENGTH(?)) != ?)")).bind(&payload.folder_id).bind(&old_root).bind(&old_prefix).bind(&old_prefix).bind(&old_prefix).bind(&old_prefix).fetch_one(&mut **transaction).await?;
    if invalid != 0 {
        return Err(AppError::ValidationFailed(
            "Catalog subtree paths do not match the source directory".into(),
        ));
    }
    sqlx::query(&format!("{subtree} UPDATE assets SET path = ? || SUBSTR(path,LENGTH(?) + 1), updated_at = ? WHERE folder_id IN (SELECT id FROM subtree)"))
        .bind(&payload.folder_id).bind(&new_prefix).bind(&old_prefix).bind(now).execute(&mut **transaction).await?;
    sqlx::query(&format!("{subtree} UPDATE folders SET path = CASE WHEN id = ? THEN ? ELSE ? || SUBSTR(path,LENGTH(?) + 1) END, updated_at = ? WHERE id IN (SELECT id FROM subtree)"))
        .bind(&payload.folder_id).bind(&payload.folder_id).bind(&new_root).bind(&new_prefix).bind(&old_prefix).bind(now).execute(&mut **transaction).await?;
    sqlx::query("UPDATE folders SET name = ?, parent_id = ? WHERE id = ?")
        .bind(&folder_name)
        .bind(&parent_id)
        .bind(&payload.folder_id)
        .execute(&mut **transaction)
        .await?;

    // 4. Audit Log
    shared::log_operation(
        transaction,
        "RENAME_FOLDER",
        &payload.folder_id,
        serde_json::to_value(&payload).map_err(|error| AppError::Internal(error.to_string()))?,
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
