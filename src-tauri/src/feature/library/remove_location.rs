//! Unlinks catalog ownership and reconciles observation through Core ports.
use crate::core::{
    error::{AppError, AppResult},
    ledger::{
        command::{LedgerCommand, RemoveFolderPayload},
        port::TransactionalAssetLedger,
    },
    library::RootObservation,
    repository::AssetQueryHandler,
};

/// Unlinks a monitored tree while preserving physical originals.
///
/// Queued observations cannot recreate unowned ancestors. Native observation is
/// reconciled after commit and retried from durable catalog state by the bootstrap task.
///
/// # Errors
/// Returns missing folders and uncommitted catalog failures. Confirmed deletion
/// survives post-commit refresh/cleanup failures without reporting the write as failed.
pub async fn execute(
    folder_id: &str,
    ledger: &dyn TransactionalAssetLedger,
    queries: &dyn AssetQueryHandler,
    observer: &dyn RootObservation,
) -> AppResult<()> {
    let _operation = super::operations::OPERATION_GATE.lock().await;
    queries
        .get_folder_by_id(folder_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Folder ID not found: {folder_id}")))?;
    if let Err(error) = ledger
        .execute(LedgerCommand::RemoveFolder(RemoveFolderPayload {
            folder_id: folder_id.into(),
            expected_path: None,
        }))
        .await
    {
        if queries.get_folder_by_id(folder_id).await?.is_some() {
            return Err(error);
        }
        tracing::warn!(%error, folder_id, "Folder unlink committed; post-commit effect needs reconciliation");
    }
    if let Err(error) = super::operations::reconcile_unlocked(queries, observer).await {
        tracing::warn!(%error, folder_id, "Unlinked folder observation will retry");
    }
    Ok(())
}
