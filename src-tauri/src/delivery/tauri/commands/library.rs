//! Thin IPC adapter for library filesystem operations.
use crate::core::{
    error::AppResult,
    ledger::{library_operation::LibraryOperation, port::TransactionalAssetLedger},
    repository::AssetQueryHandler,
};
use std::sync::Arc;
use tauri::State;

/// Applies a managed library operation.
///
/// # Errors
/// Propagates validation, filesystem and persistence failures from the handler.
#[tauri::command]
pub async fn library_operation(
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    queries: State<'_, Arc<dyn AssetQueryHandler>>,
    watcher: State<'_, Arc<crate::processing::watcher::WatcherService>>,
    operation: LibraryOperation,
) -> AppResult<crate::feature::library::operations::LibraryOperationOutcome> {
    crate::feature::library::operations::execute(
        ledger.inner().as_ref(),
        queries.inner().as_ref(),
        watcher.inner().as_ref(),
        operation,
    )
    .await
}

/// Resolves the recovery directory on an explicit request from settings.
///
/// # Errors
/// Propagates storage availability errors.
#[tauri::command]
pub async fn folder_recovery_directory(
    directories: State<'_, crate::bootstrap::AppDirectories>,
) -> AppResult<std::path::PathBuf> {
    crate::feature::library::operations::recovery_directory(&directories.app_data).await
}
