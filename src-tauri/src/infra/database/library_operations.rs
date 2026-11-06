//! Coordinates SQLite writes and compensating filesystem changes for user edits.
use super::{
    handlers::library::{planning, receipts},
    query_handlers::library_receipts,
};
use crate::core::{
    error::{AppError, AppResult},
    events::{AppEventBus, DomainEvent},
    ledger::library_operation::LibraryOperation,
    models::asset::Asset,
    path_guard::LedgerPathGuard,
};
use sqlx::SqlitePool;
use std::{path::Path, sync::Arc};
pub(crate) mod journal;

pub(crate) static LIBRARY_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use super::handlers::library::plan::DiskChanges;

/// Returns a dedicated IPC receipt while retaining the legacy internal ledger projection.
///
/// # Errors
/// Returns validation, preparation, disk and commit failures with repair diagnostics.
pub(super) async fn execute_record(
    pool: &SqlitePool,
    events: &dyn AppEventBus,
    guard: &Arc<LedgerPathGuard>,
    app_data: &Path,
    operation: LibraryOperation,
    recovery_budget: i64,
) -> AppResult<(Asset, crate::core::library::LibraryOperationResult)> {
    let _operation = LIBRARY_GATE.lock().await;
    journal::recover(pool, app_data).await?;
    if matches!(
        operation,
        LibraryOperation::RenameFolder { .. } | LibraryOperation::MoveFolder { .. }
    ) {
        super::handlers::shared::normalize_database_paths(pool).await?;
    }
    let mut changes = DiskChanges {
        archive_budget: recovery_budget,
        ..Default::default()
    };
    if let LibraryOperation::DeleteFolder { folder_id } = &operation {
        super::handlers::library::recovery::refresh_sizes(pool).await?;
        let (source, parent) =
            super::query_handlers::library_recovery::deletion_source(pool, folder_id).await?;
        if parent.is_some() {
            changes.archive_bytes =
                Some(crate::infra::filesystem::folder_recovery::measure(Path::new(&source)).await?);
        }
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let entity_type = match &operation {
        LibraryOperation::RenameAsset { .. } | LibraryOperation::MoveAssets { .. } => {
            crate::core::library::LibraryEntity::Asset
        }
        _ => crate::core::library::LibraryEntity::Folder,
    };
    let operation_kind = operation.kind();
    let asset = planning::prepare(&mut transaction, app_data, operation, &mut changes).await?;
    let mut journal = journal::Journal::prepare(app_data, &changes).await?;
    tracing::info!(operation_id = %journal.operation_id, "Library operation prepared");
    receipts::prepare(
        &mut transaction,
        &journal.operation_id,
        &asset.id,
        operation_kind,
        &asset.path,
        &changes.removed_asset_ids,
    )
    .await?;
    for (index, (source, destination)) in changes.moves.iter().enumerate() {
        if let Err(error) = journal.verify_source(index).await {
            if index == 0 {
                if let Err(cleanup) = journal.remove(app_data).await {
                    tracing::warn!(%cleanup, "Unstarted journal retained for inspection");
                }
                return Err(error);
            }
            return Err(rollback_error(&journal, app_data, error).await);
        }
        guard.mark(source);
        guard.mark(destination);
        if let Err(error) = move_without_replace(source, destination).await {
            return Err(rollback_error(&journal, app_data, error).await);
        }
    }
    if let Some(path) = &changes.create {
        // An empty new folder has no watcher noise; marking it would hide files copied in.
        if let Err(error) = tokio::fs::create_dir(path).await {
            return Err(rollback_error(&journal, app_data, error.into()).await);
        }
        if let Err(error) = journal.record_creation(app_data).await {
            return Err(rollback_error(&journal, app_data, error).await);
        }
    }
    if let Some(path) = &changes.remove_empty {
        if let Err(error) = tokio::fs::remove_dir(path).await {
            return Err(rollback_error(&journal, app_data, error.into()).await);
        }
    }
    if changes.create.is_some() {
        if let Err(error) = receipts::creation(
            &mut transaction,
            &journal.operation_id,
            &journal.creation_identity,
        )
        .await
        {
            return Err(rollback_error(&journal, app_data, error).await);
        }
    }
    if let Some(identity) = &journal.creation_identity {
        if let Err(error) =
            super::handlers::library::identities::folder(&mut transaction, &asset.id, identity)
                .await
        {
            return Err(rollback_error(&journal, app_data, error).await);
        }
    }
    commit_or_repair(transaction, pool, &journal, app_data).await?;
    let mut warnings = Vec::new();
    if let Err(error) = cleanup_unlocked(pool, app_data).await {
        tracing::warn!(%error, "Folder deleted; generated cache cleanup needs retry");
        warnings.push("Generated cache cleanup will retry automatically.");
    }
    if let Err(error) = journal.remove(app_data).await {
        tracing::warn!(%error, operation_id = %journal.operation_id, "Committed journal retained for startup finalization");
        warnings.push("Operation confirmed; journal finalization remains pending.");
    }
    let effects = changes.confirmed_effects();
    let mut processing_hint_failed = false;
    for (asset_id, path, format) in changes.restored_assets {
        if let Err(error) = events.publish(DomainEvent::AssetCreated {
            asset_id,
            path,
            format,
        }) {
            tracing::warn!(%error, "Restored asset committed but processing wake-up was lost");
            processing_hint_failed = true;
        }
    }
    if processing_hint_failed {
        warnings.push("Files restored; processing will resume through reconciliation.");
    }
    let hint = match operation_kind {
        "deleteFolder" | "cancelFolderDraft" => DomainEvent::LibraryFoldersRemoved {
            folder_ids: vec![asset.id.clone()],
            removed_asset_count: changes.removed_asset_ids.len(),
        },
        "createFolder" => DomainEvent::FolderCreated {
            folder_id: asset.id.clone(),
            parent_id: asset.folder_id.clone(),
            name: asset.name.clone(),
            path: asset.path.to_string_lossy().into_owned(),
        },
        "renameAsset" => DomainEvent::AssetMetadataUpdated {
            asset_id: asset.id.clone(),
        },
        "moveAssets" => match &asset.folder_id {
            Some(identifier) => DomainEvent::FolderMetadataUpdated {
                folder_id: identifier.clone(),
            },
            None => DomainEvent::AssetMetadataUpdated {
                asset_id: asset.id.clone(),
            },
        },
        _ => DomainEvent::FolderMetadataUpdated {
            folder_id: asset.id.clone(),
        },
    };
    if let Err(error) = events.publish(hint) {
        tracing::warn!(%error, operation_id = %journal.operation_id, "Committed library refresh hint was lost");
        warnings.push("Library updated; the refresh notification could not be delivered.");
    }
    let previous_path = effects
        .iter()
        .find(|effect| effect.destination.as_ref() == Some(&asset.path))
        .and_then(|effect| effect.source.clone());
    let result = crate::core::library::LibraryOperationResult {
        operation_id: journal.operation_id,
        id: asset.id.clone(),
        entity_type,
        path: asset.path.clone(),
        previous_path,
        effects,
        warning: (!warnings.is_empty()).then(|| warnings.join(" ")),
    };
    Ok((asset, result))
}

async fn rollback_error(
    journal: &journal::Journal,
    app_data: &Path,
    original: AppError,
) -> AppError {
    match journal.rollback().await {
        Ok(()) => {
            if let Err(error) = journal.remove(app_data).await {
                tracing::warn!(%error, "Completed rollback journal retained");
            }
            original
        }
        Err(repair) => AppError::Internal(format!(
            "{original}; operation {} requires repair: {repair}",
            journal.operation_id
        )),
    }
}

/// Retries idempotent post-commit cache cleanup from durable receipts.
///
/// # Errors
/// Returns storage errors and leaves pending receipts available for retry.
pub(crate) async fn retry_cleanup(pool: &SqlitePool, app_data: &Path) -> AppResult<()> {
    let _operation = LIBRARY_GATE.lock().await;
    cleanup_unlocked(pool, app_data).await
}

async fn cleanup_unlocked(pool: &SqlitePool, app_data: &Path) -> AppResult<()> {
    super::handlers::library::recovery::refresh_sizes(pool).await?;
    let pending = library_receipts::pending_cleanup(pool).await?;
    for (operation_id, payload) in pending {
        let identifiers = serde_json::from_str(&payload)
            .map_err(|error| AppError::Internal(format!("Invalid cleanup receipt: {error}")))?;
        crate::infra::filesystem::asset_cache::remove_asset_artifacts(app_data, &identifiers)
            .await?;
        receipts::cleaned(pool, &operation_id).await?;
    }
    Ok(())
}

/// Native exclusive moves preserve both source and destination on collisions.
async fn move_without_replace(source: &Path, destination: &Path) -> AppResult<()> {
    let metadata = tokio::fs::symlink_metadata(source).await?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::ValidationFailed(
            "Symbolic links cannot be moved".into(),
        ));
    }
    crate::infra::filesystem::exclusive_move::move_path(source, destination).await?;
    Ok(())
}

#[cfg(test)]
mod crash_tests;
#[cfg(test)]
mod tests;

async fn commit_or_repair(
    transaction: sqlx::Transaction<'_, sqlx::Sqlite>,
    pool: &SqlitePool,
    journal: &journal::Journal,
    app_data: &Path,
) -> AppResult<()> {
    if let Err(error) = transaction.commit().await {
        let committed = library_receipts::committed(pool, &journal.operation_id)
            .await
            .map_err(|lookup| {
                AppError::Internal(format!(
                    "Commit outcome uncertain for {}: {error}; {lookup}. Journal retained",
                    journal.operation_id
                ))
            })?;
        if !committed {
            return Err(rollback_error(journal, app_data, error.into()).await);
        }
        tracing::warn!(operation_id = %journal.operation_id, %error, "Receipt confirms commit despite acknowledgement failure");
    }
    Ok(())
}
