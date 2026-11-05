//! Validates user intentions and prepares transactional read-model mutations.
use super::DiskChanges;
use crate::core::{
    error::{AppError, AppResult},
    ledger::{
        command::{
            CreateFolderPayload, RemoveFolderPayload, RenameFolderPayload, UpdateAssetPayload,
        },
        library_operation::LibraryOperation,
    },
    models::asset::Asset,
};
use crate::infra::database::handlers::{asset_handler, folder_handler, shared};
use sqlx::{Sqlite, Transaction};
use std::path::{Path, PathBuf};

fn invalid(message: &str) -> AppError {
    AppError::ValidationFailed(message.into())
}

fn validate_name(name: &str) -> AppResult<()> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
        || [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
    {
        return Err(invalid(
            "Use a valid file or folder name without path separators",
        ));
    }
    Ok(())
}

async fn folder(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: &str,
) -> AppResult<(PathBuf, Option<String>)> {
    let (path, parent): (String, Option<String>) =
        sqlx::query_as("SELECT path, parent_id FROM folders WHERE id = ?")
            .bind(identifier)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("Folder not found".into()))?;
    let path = PathBuf::from(path);
    let metadata = tokio::fs::symlink_metadata(&path).await?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("Folder is unavailable or is a symbolic link"));
    }
    Ok((path, parent))
}

pub(super) async fn available(
    transaction: &mut Transaction<'_, Sqlite>,
    path: &Path,
) -> AppResult<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => {
            return Err(invalid(
                "A file or folder already exists at the destination",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let occupied: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM assets WHERE path = ? COLLATE NOCASE) + (SELECT COUNT(*) FROM folders WHERE path = ? COLLATE NOCASE)")
        .bind(path.to_string_lossy().as_ref()).bind(path.to_string_lossy().as_ref()).fetch_one(&mut **transaction).await?;
    if occupied > 0 {
        return Err(invalid("Destination is already registered in the library"));
    }
    Ok(())
}

async fn rename_asset(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: &str,
    destination: PathBuf,
    changes: &mut DiskChanges,
) -> AppResult<Asset> {
    let asset = shared::fetch_asset_by_id(transaction, identifier).await?;
    if asset.deleted_at.is_some() {
        return Err(invalid("Restore the asset before moving or renaming it"));
    }
    if destination == asset.path {
        return Ok(asset);
    }
    available(transaction, &destination).await?;
    if changes.moves.iter().any(|(_, other)| {
        other.to_string_lossy().to_lowercase() == destination.to_string_lossy().to_lowercase()
    }) {
        return Err(invalid("Selected assets have conflicting names"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Missing destination folder"))?
        .to_string_lossy()
        .to_string();
    let folder_id: String =
        sqlx::query_scalar("SELECT id FROM folders WHERE path = ? COLLATE NOCASE")
            .bind(parent)
            .fetch_one(&mut **transaction)
            .await?;
    let result = asset_handler::handle_update_asset(
        transaction,
        UpdateAssetPayload {
            asset_id: Some(identifier.into()),
            old_path: Some(asset.path.clone()),
            new_path: destination.clone(),
        },
    )
    .await?;
    asset_handler::handle_set_asset_folder(transaction, identifier, Some(&folder_id)).await?;
    changes.moves.push((asset.path, destination));
    Ok(result)
}

pub(super) async fn prepare(
    transaction: &mut Transaction<'_, Sqlite>,
    app_data: &Path,
    operation: LibraryOperation,
    changes: &mut DiskChanges,
) -> AppResult<Asset> {
    match operation {
        LibraryOperation::CreateFolder { parent_id, name } => {
            validate_name(&name)?;
            let (parent, _) = folder(transaction, &parent_id).await?;
            let path = parent.join(&name);
            available(transaction, &path).await?;
            changes.create = Some(path.clone());
            folder_handler::handle_create_folder(
                transaction,
                CreateFolderPayload {
                    parent_id: Some(parent_id),
                    name,
                    path,
                },
            )
            .await
        }
        LibraryOperation::RenameAsset { asset_id, name } => {
            validate_name(&name)?;
            let asset = shared::fetch_asset_by_id(transaction, &asset_id).await?;
            let mut name = name;
            if let Some(extension) = asset.path.extension().and_then(|value| value.to_str()) {
                let suffix = format!(".{extension}");
                if !name.to_lowercase().ends_with(&suffix.to_lowercase()) {
                    name.push_str(&suffix);
                }
            }
            let destination = asset.path.with_file_name(name);
            rename_asset(transaction, &asset_id, destination, changes).await
        }
        LibraryOperation::MoveAssets {
            asset_ids,
            destination_id,
        } => {
            let (destination, _) = folder(transaction, &destination_id).await?;
            let mut result = None;
            let mut unique = std::collections::HashSet::new();
            for identifier in asset_ids {
                if !unique.insert(identifier.clone()) {
                    continue;
                }
                let asset = shared::fetch_asset_by_id(transaction, &identifier).await?;
                let name = asset
                    .path
                    .file_name()
                    .ok_or_else(|| invalid("Invalid asset path"))?;
                result = Some(
                    rename_asset(transaction, &identifier, destination.join(name), changes).await?,
                );
            }
            result.ok_or_else(|| invalid("Select at least one asset"))
        }
        LibraryOperation::FolderColor {
            folder_id,
            color,
            inherits_color,
        } => {
            if let Some(value) = &color {
                if value.len() != 7
                    || !value.starts_with('#')
                    || !value[1..]
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
                {
                    return Err(invalid("Use a hexadecimal color such as #5588cc"));
                }
            }
            let (path, parent_id) = folder(transaction, &folder_id).await?;
            sqlx::query("UPDATE folders SET color = ?, inherits_color = ? WHERE id = ?")
                .bind(color)
                .bind(inherits_color)
                .bind(&folder_id)
                .execute(&mut **transaction)
                .await?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            folder_handler::handle_create_folder(
                transaction,
                CreateFolderPayload {
                    parent_id,
                    name,
                    path,
                },
            )
            .await
        }
        LibraryOperation::RenameFolder { folder_id, name } => {
            validate_name(&name)?;
            let (source, _) = folder(transaction, &folder_id).await?;
            let destination = source.with_file_name(name);
            move_folder(transaction, folder_id, source, destination, changes).await
        }
        LibraryOperation::MoveFolder {
            folder_id,
            destination_id,
        } => {
            let (source, _) = folder(transaction, &folder_id).await?;
            let (destination, _) = folder(transaction, &destination_id).await?;
            let canonical_source = tokio::fs::canonicalize(&source).await?;
            if tokio::fs::canonicalize(&destination)
                .await?
                .starts_with(&canonical_source)
            {
                return Err(invalid(
                    "A folder cannot be moved into itself or a descendant",
                ));
            }
            let name = source
                .file_name()
                .ok_or_else(|| invalid("Invalid folder path"))?;
            let target = destination.join(name);
            move_folder(transaction, folder_id, source, target, changes).await
        }
        LibraryOperation::RestoreFolder { folder_id } => {
            super::recovery::restore(transaction, app_data, &folder_id, changes).await
        }
        LibraryOperation::DeleteFolder { folder_id } => {
            let (source, parent) = folder(transaction, &folder_id).await?;
            if parent.is_none() {
                return Err(invalid("Unlink monitored roots instead of deleting them"));
            }
            let archive = app_data.join("deleted-folders");
            tokio::fs::create_dir_all(&archive).await?;
            let destination = archive.join(format!(
                "{}-{}",
                uuid::Uuid::new_v4(),
                source.file_name().unwrap_or_default().to_string_lossy()
            ));
            super::recovery::capture(transaction, &folder_id, &source, &destination).await?;
            changes.removed_asset_ids.extend(
                crate::infra::database::handlers::folder_removal::collect_assets(
                    transaction,
                    &folder_id,
                )
                .await?
                .into_iter()
                .map(|asset| asset.id),
            );
            changes.moves.push((source, destination.clone()));
            let mut result = folder_handler::handle_remove_folder(
                transaction,
                RemoveFolderPayload { folder_id },
            )
            .await?;
            result.path = destination;
            Ok(result)
        }
    }
}

async fn move_folder(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: String,
    source: PathBuf,
    destination: PathBuf,
    changes: &mut DiskChanges,
) -> AppResult<Asset> {
    if source != destination {
        available(transaction, &destination).await?;
        changes.moves.push((source.clone(), destination.clone()));
    }
    folder_handler::handle_rename_folder(
        transaction,
        RenameFolderPayload {
            folder_id: identifier,
            old_path: source,
            new_path: destination,
        },
    )
    .await
}
