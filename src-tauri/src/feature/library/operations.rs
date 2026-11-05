//! Application entry point for managed library writes.
use crate::core::{
    error::{AppError, AppResult},
    ledger::{
        command::LedgerCommand, library_operation::LibraryOperation, port::TransactionalAssetLedger,
    },
    models::asset::Asset,
    repository::AssetQueryHandler,
};
use crate::processing::watcher::WatcherService;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

/// A committed edit may carry a monitoring warning without claiming the edit failed.
#[derive(Serialize)]
pub struct LibraryOperationOutcome {
    /// Preserve the asset-shaped IPC response used by existing callers.
    #[serde(flatten)]
    pub asset: Asset,
    /// Actionable warning if native monitoring could not be reattached.
    pub warning: Option<String>,
}

/// Runs a user intention through the transactional ledger.
///
/// # Errors
/// Returns validation, filesystem or database errors without hiding failures.
pub async fn execute(
    ledger: &dyn TransactionalAssetLedger,
    queries: &dyn AssetQueryHandler,
    watcher: &WatcherService,
    operation: LibraryOperation,
) -> AppResult<LibraryOperationOutcome> {
    let relocated_folder = match &operation {
        LibraryOperation::MoveFolder { folder_id, .. }
        | LibraryOperation::RenameFolder { folder_id, .. } => Some(
            queries
                .get_folder_by_id(folder_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Folder not found".into()))?,
        ),
        _ => None,
    };
    let asset = ledger
        .execute(LedgerCommand::LibraryOperation(operation))
        .await?;
    let mut warning = None;
    if let Some(previous) = relocated_folder {
        if previous.path != asset.path {
            if let Err(error) =
                refresh_monitoring(queries, watcher, &previous.id, previous.path).await
            {
                tracing::error!(%error, "Folder edit committed but watcher reconfiguration failed");
                warning = Some("Folder updated, but monitoring could not restart. Restart Mundam to resume monitoring.".into());
            }
        }
    }
    Ok(LibraryOperationOutcome { asset, warning })
}

async fn refresh_monitoring(
    queries: &dyn AssetQueryHandler,
    watcher: &WatcherService,
    identifier: &str,
    old_path: std::path::PathBuf,
) -> AppResult<()> {
    watcher.unwatch(old_path).await?;
    let folder = queries
        .get_folder_by_id(identifier)
        .await?
        .ok_or_else(|| AppError::NotFound("Moved folder not found".into()))?;
    if folder.parent_id.is_none() {
        watcher.watch(folder.path, CancellationToken::new()).await?;
    }
    Ok(())
}

/// Resolves the advanced settings recovery directory.
///
/// # Errors
/// Returns filesystem errors if recovery storage cannot be created.
pub async fn recovery_directory(app_data: &std::path::Path) -> AppResult<std::path::PathBuf> {
    crate::infra::filesystem::folder_recovery::directory(app_data).await
}
