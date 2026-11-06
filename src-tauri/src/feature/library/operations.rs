//! Serializes committed intentions and reconciles native observation through Core ports.
use crate::core::{
    error::AppResult,
    ledger::{library_operation::LibraryOperation, port::TransactionalAssetLedger},
    library::{LibraryOperationResult, RecoveryStorage, RootObservation},
    repository::AssetQueryHandler,
};

/// Serializes user intentions with desired-root reconciliation.
pub(super) static OPERATION_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Commits an intention and returns monitoring failures as retryable warnings.
///
/// # Errors
/// Returns validation and write failures before a confirmed commit.
#[tracing::instrument(skip_all, fields(kind = operation.kind()))]
pub async fn execute(
    ledger: &dyn TransactionalAssetLedger,
    queries: &dyn AssetQueryHandler,
    observer: &dyn RootObservation,
    operation: LibraryOperation,
) -> AppResult<LibraryOperationResult> {
    let _operation = OPERATION_GATE.lock().await;
    let mut result = ledger.execute_library(operation).await?;
    if let Err(error) = reconcile_unlocked(queries, observer).await {
        tracing::warn!(operation_id = %result.operation_id, %error, "Committed edit; observation retry required");
        let warning = "Library updated. Monitoring will retry automatically.";
        result.warning = Some(match result.warning.take() {
            Some(previous) => format!("{previous} {warning}"),
            None => warning.into(),
        });
    }
    Ok(result)
}

/// Rebuilds the desired observation set from durable catalog state.
///
/// # Errors
/// Propagates catalog and native observation failures for automatic retry.
pub async fn reconcile(
    queries: &dyn AssetQueryHandler,
    observer: &dyn RootObservation,
) -> AppResult<()> {
    let _operation = OPERATION_GATE.lock().await;
    reconcile_unlocked(queries, observer).await
}

/// Reconciles roots while the caller holds the operation gate.
///
/// # Errors
/// Returns catalog and native observation failures for retry.
pub(super) async fn reconcile_unlocked(
    queries: &dyn AssetQueryHandler,
    observer: &dyn RootObservation,
) -> AppResult<()> {
    let roots = queries
        .list_folders(None)
        .await?
        .into_iter()
        .filter(|folder| folder.parent_id.is_none())
        .map(|folder| folder.path)
        .collect();
    observer.reconcile(roots).await
}

/// Resolves advanced recovery storage through its platform adapter.
///
/// # Errors
/// Propagates unavailable recovery storage errors.
pub async fn recovery_directory(storage: &dyn RecoveryStorage) -> AppResult<std::path::PathBuf> {
    storage.directory().await
}

/// Lists persistent Undo entries with a bounded query.
///
/// # Errors
/// Returns catalog read failures.
pub async fn recovery_page(
    queries: &dyn AssetQueryHandler,
    offset: u32,
) -> AppResult<Vec<crate::core::library::FolderRecoveryEntry>> {
    queries.folder_recovery_page(offset).await
}
