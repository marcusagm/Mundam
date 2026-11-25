//! Desktop-only activation of the external import API.
use crate::{
    bootstrap::AppDirectories,
    core::AppResult,
    delivery::{
        external_api::desktop::{DesktopImportService, ImportEndpoint},
        tauri::admission::IpcRequest,
    },
    infra::database::manager::DbManager,
    lifecycle::LifecycleRegistry,
};
use std::sync::Arc;
use tauri::State;

/// Reports platform support and public connection details.
#[derive(serde::Serialize)]
pub struct ExternalImportStatus {
    pub available: bool,
    pub endpoint: Option<ImportEndpoint>,
}

/// Returns availability and the public endpoint of the active import service.
#[tauri::command]
pub async fn get_external_import_status(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
) -> AppResult<ExternalImportStatus> {
    Ok(ExternalImportStatus {
        available: cfg!(windows),
        endpoint: service.endpoint().await,
    })
}

/// Starts the local listener after an explicit desktop request.
///
/// # Errors
/// Returns identity, initialization or platform availability failures.
#[tauri::command]
pub async fn start_external_import(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    lifecycle: State<'_, Arc<LifecycleRegistry>>,
    database: State<'_, Arc<DbManager>>,
    directories: State<'_, AppDirectories>,
) -> AppResult<ImportEndpoint> {
    service
        .start(
            lifecycle.inner(),
            database.pool().clone(),
            &directories.app_data,
        )
        .await
}

/// Stops the local listener and waits for its connections to drain.
#[tauri::command]
pub async fn stop_external_import(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    lifecycle: State<'_, Arc<LifecycleRegistry>>,
) -> AppResult<()> {
    service.stop(lifecycle.inner()).await;
    Ok(())
}
