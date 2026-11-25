//! Catalog reads and writes for one publication transaction.
use super::content::InspectedFile;
use crate::{
    core::{
        error::{AppError, AppResult},
        imports::publication::{PublishPreparedImport, PublishedImport},
        library::FileIdentity,
        models::asset::AssetState,
    },
    infra::{
        database::handlers::{library::validation, shared},
        filesystem::identity,
    },
};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::path::{Path, PathBuf};

pub(super) struct ImportDestination {
    pub path: PathBuf,
    pub identity: FileIdentity,
}

pub(super) async fn destination(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<ImportDestination> {
    let (path, _) = validation::folder(transaction, folder_id).await?;
    let identity = identity::read(&path).await?;
    let matches: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM library_folder_identities
         WHERE folder_id = ? AND device = ? AND object = ? AND birth IS ?)",
    )
    .bind(folder_id)
    .bind(identity.device.to_string())
    .bind(identity.object.to_string())
    .bind(identity.birth.map(|birth| birth.to_string()))
    .fetch_one(&mut **transaction)
    .await?;
    if !matches {
        return Err(AppError::ValidationFailed(
            "Import destination has no matching catalog identity".into(),
        ));
    }
    Ok(ImportDestination { path, identity })
}

pub(super) async fn replay(
    connection: &mut SqliteConnection,
    batch_id: &str,
    item_id: &str,
    fingerprint: &[u8; 32],
) -> AppResult<Option<PublishedImport>> {
    let stored: Option<(Vec<u8>, String, String)> = sqlx::query_as(
        "SELECT fingerprint, asset_id, file_name FROM import_publication_receipts
         WHERE batch_id = ? AND item_id = ?",
    )
    .bind(batch_id)
    .bind(item_id)
    .fetch_optional(connection)
    .await?;
    match stored {
        Some((stored, asset_id, file_name)) if stored.as_slice() == fingerprint => {
            Ok(Some(PublishedImport {
                asset_id,
                file_name,
            }))
        }
        Some(_) => Err(AppError::ValidationFailed(
            "Published import item differs from the previous request".into(),
        )),
        None => Ok(None),
    }
}

pub(super) async fn allocate(
    transaction: &mut Transaction<'_, Sqlite>,
    directory: &Path,
    requested_name: &str,
) -> AppResult<PathBuf> {
    let (stem, extension) = requested_name
        .rsplit_once('.')
        .ok_or_else(|| AppError::ValidationFailed("Missing import file extension".into()))?;
    for attempt in 0..=100 {
        let destination = if attempt == 0 {
            directory.join(requested_name)
        } else {
            directory.join(format!("{stem} ({attempt}).{extension}"))
        };
        match validation::available(transaction, &destination).await {
            Ok(()) => return Ok(destination),
            Err(AppError::ValidationFailed(_)) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(AppError::ValidationFailed(
        "Import destination has no available name".into(),
    ))
}

pub(super) async fn record(
    transaction: &mut Transaction<'_, Sqlite>,
    command: &PublishPreparedImport,
    batch_id: &str,
    fingerprint: &[u8; 32],
    destination: &Path,
    inspected: &InspectedFile,
) -> AppResult<PublishedImport> {
    let asset_id = uuid::Uuid::new_v4().to_string();
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Internal("Allocated import path has no UTF-8 filename".into()))?
        .to_owned();
    let now = chrono::Utc::now();
    let modified_at: chrono::DateTime<chrono::Utc> = inspected.modified.into();
    sqlx::query(
        "INSERT INTO assets (id, name, path, state, format_type, family, file_size,
         created_at, modified_at, added_at, updated_at, folder_id, source_url)
         VALUES (?, ?, ?, ?, 'png', 'IMAGE', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&asset_id)
    .bind(&file_name)
    .bind(destination.to_string_lossy().as_ref())
    .bind(AssetState::Indexed.to_string())
    .bind(inspected.length as i64)
    .bind(now)
    .bind(modified_at)
    .bind(now)
    .bind(now)
    .bind(&command.folder_id)
    .bind(&command.item.source_url)
    .execute(&mut **transaction)
    .await?;
    sqlx::query("INSERT INTO asset_metadata_envelope (asset_id, width, height) VALUES (?, ?, ?)")
        .bind(&asset_id)
        .bind(inspected.width)
        .bind(inspected.height)
        .execute(&mut **transaction)
        .await?;
    for tag_id in &command.item.tag_ids {
        let result = sqlx::query(
            "INSERT INTO asset_tags (asset_id, tag_id) SELECT ?, id FROM tags WHERE id = ?",
        )
        .bind(&asset_id)
        .bind(tag_id)
        .execute(&mut **transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AppError::ValidationFailed(
                "An import tag is no longer available".into(),
            ));
        }
    }
    sqlx::query(
        "INSERT INTO import_publication_receipts (batch_id, item_id, fingerprint, asset_id, file_name, attempt_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(batch_id).bind(&command.item.item_id).bind(fingerprint.as_slice())
    .bind(&asset_id).bind(&file_name)
    .bind(command.attempt_id.map(|attempt_id| attempt_id.to_string()))
    .execute(&mut **transaction).await?;
    shared::log_operation(
        transaction,
        "IMPORT_ASSET",
        &asset_id,
        serde_json::json!({ "batchId": batch_id, "itemId": command.item.item_id }),
        "COMPLETED",
        None,
    )
    .await?;
    Ok(PublishedImport {
        asset_id,
        file_name,
    })
}

pub(super) async fn record_identity(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
    identity: &FileIdentity,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO library_file_identities (asset_id, device, object, birth) VALUES (?, ?, ?, ?)",
    )
    .bind(asset_id)
    .bind(identity.device.to_string())
    .bind(identity.object.to_string())
    .bind(identity.birth.map(|birth| birth.to_string()))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
