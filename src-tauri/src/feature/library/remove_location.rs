//! Coordinates stopping observation and removing a library tree through the Ledger.

use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{LedgerCommand, RemoveFolderPayload};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::repository::AssetQueryHandler;
use crate::processing::watcher::WatcherService;
use tokio_util::sync::CancellationToken;

/// Stops observation before removal so queued events cannot repopulate the tree.
/// The Ledger owns database deletion and post-commit generated-artifact cleanup.
///
/// # Errors
/// Returns `NotFound` for an unknown folder, or propagates query, watcher, removal
/// and cache errors. Observation is restored if a failed transaction kept the folder.
pub async fn execute(
    folder_id: &str,
    ledger: &dyn TransactionalAssetLedger,
    queries: &dyn AssetQueryHandler,
    watcher: &WatcherService,
) -> AppResult<()> {
    let folder = queries
        .get_folder_by_id(folder_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Folder ID not found: {folder_id}")))?;
    watcher.unwatch(folder.path.clone()).await?;

    let result = ledger
        .execute(LedgerCommand::RemoveFolder(RemoveFolderPayload {
            folder_id: folder_id.to_owned(),
        }))
        .await;
    if result.is_err() && queries.get_folder_by_id(folder_id).await?.is_some() {
        watcher.watch(folder.path, CancellationToken::new()).await?;
    }
    result.map(|_| ())
}
