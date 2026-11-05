//! Coordinates SQLite writes and compensating filesystem changes for user edits.
use crate::core::{
    error::{AppError, AppResult},
    events::{AppEventBus, DomainEvent},
    ledger::library_operation::LibraryOperation,
    models::asset::Asset,
    path_guard::LedgerPathGuard,
};
use sqlx::SqlitePool;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
mod planning;
mod recovery;
mod snapshot;

/// A transaction's physical changes, reversed if a later step fails.
#[derive(Default)]
struct DiskChanges {
    moves: Vec<(PathBuf, PathBuf)>,
    create: Option<PathBuf>,
    removed_asset_ids: std::collections::HashSet<String>,
    restored_assets: Vec<(String, String, String)>,
}

/// Commits managed changes and emits refresh hints only after persistence.
///
/// # Errors
/// Rejects invalid paths, collisions and unavailable volumes. Failed writes
/// compensate completed disk changes and retain originals on move failures.
pub(super) async fn execute(
    pool: &SqlitePool,
    events: &dyn AppEventBus,
    guard: &Arc<LedgerPathGuard>,
    app_data: &Path,
    operation: LibraryOperation,
) -> AppResult<Asset> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut changes = DiskChanges::default();
    let asset = planning::prepare(&mut transaction, app_data, operation, &mut changes).await?;
    let mut completed = Vec::new();
    for (source, destination) in &changes.moves {
        guard.mark(source);
        guard.mark(destination);
        if let Err(error) = move_without_replace(source, destination).await {
            compensate(&completed).await?;
            return Err(error);
        }
        completed.push((source.clone(), destination.clone()));
    }
    if let Some(path) = &changes.create {
        guard.mark(path);
        tokio::fs::create_dir(path).await?;
    }
    if let Err(error) = transaction.commit().await {
        compensate(&completed).await?;
        if let Some(path) = &changes.create {
            tokio::fs::remove_dir(path).await?;
        }
        return Err(error.into());
    }
    if let Err(error) = crate::infra::filesystem::asset_cache::remove_asset_artifacts(
        app_data,
        &changes.removed_asset_ids,
    )
    .await
    {
        tracing::warn!(%error, "Folder deleted; generated cache cleanup needs retry");
    }
    for (asset_id, path, format) in changes.restored_assets {
        if let Err(error) = events.publish(DomainEvent::AssetCreated {
            asset_id,
            path,
            format,
        }) {
            tracing::warn!(%error, "Restored asset committed but processing wake-up was lost");
        }
    }
    // A dropped UI hint must not report a committed disk operation as failed.
    if let Err(error) = events.publish(DomainEvent::FolderMetadataUpdated {
        folder_id: asset.folder_id.clone().unwrap_or_else(|| asset.id.clone()),
    }) {
        tracing::warn!(%error, "Library operation committed but refresh hint was lost");
    }
    Ok(asset)
}

async fn compensate(completed: &[(PathBuf, PathBuf)]) -> AppResult<()> {
    for (source, destination) in completed.iter().rev() {
        move_without_replace(destination, source)
            .await
            .map_err(|error| {
                AppError::Internal(format!(
                    "Rollback failed: {error}. File remains at {} (original: {})",
                    destination.display(),
                    source.display()
                ))
            })?;
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
mod tests;
