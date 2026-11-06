//! Validates user intentions and prepares transactional read-model mutations.
use super::plan::DiskChanges;
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

use super::validation::{available, folder, invalid};
use crate::core::library::policy::validate_name;

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
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Missing destination folder"))?;
    let folder_id = shared::resolve_folder_id_robust(transaction, parent)
        .await?
        .ok_or_else(|| invalid("Destination parent is not indexed"))?;
    let canonical_parent: String = sqlx::query_scalar("SELECT path FROM folders WHERE id = ?")
        .bind(&folder_id)
        .fetch_one(&mut **transaction)
        .await?;
    let destination = Path::new(&canonical_parent).join(
        destination
            .file_name()
            .ok_or_else(|| invalid("Invalid destination name"))?,
    );
    if destination == asset.path {
        if asset.folder_id.as_deref() != Some(&folder_id) {
            asset_handler::handle_set_asset_folder(transaction, identifier, Some(&folder_id))
                .await?;
            return shared::fetch_asset_by_id(transaction, identifier).await;
        }
        return Ok(asset);
    }
    let same_object = crate::infra::filesystem::identity::same(&asset.path, &destination).await?;
    if !same_object {
        available(transaction, &destination).await?;
    }
    if changes.moves.iter().any(|(_, other)| other == &destination) {
        return Err(invalid("Selected assets have conflicting names"));
    }
    let result = asset_handler::handle_update_asset(
        transaction,
        UpdateAssetPayload {
            asset_id: Some(identifier.into()),
            old_path: Some(asset.path.clone()),
            new_path: destination.clone(),
        },
    )
    .await?;
    stage_move(changes, asset.path, destination, same_object);
    Ok(result)
}

/// Prepares catalog mutations and the recoverable native-effect plan.
///
/// # Errors
/// Returns invalid intentions and storage failures before any planned movement.
pub(crate) async fn prepare(
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
        LibraryOperation::CancelFolderDraft {
            folder_id,
            creation_operation_id,
        } => super::draft::cancel(transaction, folder_id, creation_operation_id, changes).await,
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
            if asset_ids.is_empty()
                || asset_ids.len() > crate::core::library::policy::MAX_MOVE_SELECTION
            {
                return Err(invalid(
                    "Select between 1 and 5000 assets; move a whole folder for larger subtrees",
                ));
            }
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
        } => super::color::update(transaction, folder_id, color, inherits_color).await,
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
            let creates_cycle: bool = sqlx::query_scalar("WITH RECURSIVE subtree(id) AS (SELECT id FROM folders WHERE id = ? UNION SELECT folders.id FROM folders JOIN subtree ON folders.parent_id = subtree.id) SELECT EXISTS(SELECT 1 FROM subtree WHERE id = ?)").bind(&folder_id).bind(&destination_id).fetch_one(&mut **transaction).await?;
            if creates_cycle {
                return Err(invalid(
                    "A folder cannot be moved into its catalog descendant",
                ));
            }
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
            super::recovery::capture(
                transaction,
                &folder_id,
                &source,
                &destination,
                changes
                    .archive_bytes
                    .ok_or_else(|| AppError::Internal("Missing recovery inventory".into()))?,
                changes.archive_budget,
            )
            .await?;
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
                RemoveFolderPayload {
                    folder_id,
                    expected_path: None,
                },
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
        let same_object = crate::infra::filesystem::identity::same(&source, &destination).await?;
        if !same_object {
            available(transaction, &destination).await?;
        }
        stage_move(changes, source.clone(), destination.clone(), same_object);
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

fn stage_move(changes: &mut DiskChanges, source: PathBuf, destination: PathBuf, same_object: bool) {
    if same_object {
        let temporary = source.with_file_name(format!(".mundam-{}", uuid::Uuid::new_v4()));
        changes.moves.push((source, temporary.clone()));
        changes.moves.push((temporary, destination));
    } else {
        changes.moves.push((source, destination));
    }
}
