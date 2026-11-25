//! Thin IPC adapter for library filesystem operations.
use crate::core::{
    error::AppResult,
    ledger::{library_operation::LibraryOperation, port::TransactionalAssetLedger},
    repository::AssetQueryHandler,
};
use crate::delivery::tauri::admission::IpcRequest;
use std::sync::Arc;
use tauri::State;

use crate::{
    core::imports::destinations::{ImportDestinations, ImportPreference},
    infra::database::{import_destinations::SqliteImportDestinations, manager::DbManager},
};

/// Reads the default destination for the desktop settings panel.
///
/// # Errors
/// Propagates catalog read failures.
#[tauri::command]
pub async fn get_import_preference(
    _ipc: IpcRequest,
    database: State<'_, Arc<DbManager>>,
) -> AppResult<ImportPreference> {
    SqliteImportDestinations(database.pool()).preference().await
}

/// Persists the selection made in the desktop settings panel.
///
/// # Errors
/// Propagates catalog failures and rejects selections from a different catalog or removed folders.
#[tauri::command]
pub async fn set_import_preference(
    _ipc: IpcRequest,
    database: State<'_, Arc<DbManager>>,
    catalog_id: String,
    folder_id: Option<String>,
) -> AppResult<ImportPreference> {
    SqliteImportDestinations(database.pool())
        .set_default(&catalog_id, folder_id.as_deref())
        .await
}

/// Applies a managed library operation.
///
/// # Errors
/// Propagates validation, filesystem and persistence failures from the handler.
#[tauri::command]
pub async fn library_operation(
    _ipc: IpcRequest,
    ledger: State<'_, Arc<dyn TransactionalAssetLedger>>,
    queries: State<'_, Arc<dyn AssetQueryHandler>>,
    watcher: State<'_, Arc<dyn crate::core::library::RootObservation>>,
    operation: LibraryOperation,
) -> AppResult<crate::core::library::LibraryOperationResult> {
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
    _ipc: IpcRequest,
    storage: State<'_, Arc<dyn crate::core::library::RecoveryStorage>>,
) -> AppResult<std::path::PathBuf> {
    crate::feature::library::operations::recovery_directory(storage.inner().as_ref()).await
}

/// Lists persistent folder Undo entries from settings.
///
/// # Errors
/// Propagates catalog read failures.
#[tauri::command]
pub async fn folder_recovery_history(
    _ipc: IpcRequest,
    queries: State<'_, Arc<dyn AssetQueryHandler>>,
    offset: u32,
) -> AppResult<Vec<crate::core::library::FolderRecoveryEntry>> {
    crate::feature::library::operations::recovery_page(queries.inner().as_ref(), offset).await
}
