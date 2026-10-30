//! Desktop boot composition and event adaptation for retained startup state.

use crate::core::error::{AppError, AppResult};
use crate::feature::startup::{StartupService, StartupSnapshot, StartupStage};
use tauri::{AppHandle, Emitter, Manager};

/// Registers observable state before launching a supervised, non-blocking boot.
pub fn start(application: &AppHandle) {
    application.manage(StartupService::default());
    let handle = application.clone();
    tauri::async_runtime::spawn(async move {
        let startup_handle = handle.clone();
        let initialization =
            tauri::async_runtime::spawn(async move { initialize(&startup_handle).await });
        let failure = match initialization.await {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        tracing::error!(%failure, "Application startup failed");
        match handle
            .state::<StartupService>()
            .fail("BACKEND_STARTUP_FAILED")
        {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain startup failure"),
        }
    });
}

/// Publishes a retained snapshot; a missed event is recoverable through querying.
pub fn publish(application: &AppHandle, snapshot: &StartupSnapshot) {
    if let Err(error) = application.emit("startup:changed", snapshot) {
        tracing::warn!(%error, "Unable to publish startup progress");
    }
}

/// Keeps auxiliary-window disposal separate from application shutdown.
/// A splash closed by the user during boot must still terminate the hidden main.
pub fn should_shutdown_on_close(application: &AppHandle, label: &str) -> bool {
    match label {
        "main" => true,
        "splashscreen" => application
            .try_state::<StartupService>()
            .and_then(|service| service.snapshot().ok())
            .map(|snapshot| snapshot.stage != StartupStage::Ready || snapshot.failed)
            .unwrap_or(true),
        _ => false,
    }
}

fn advance(application: &AppHandle, stage: StartupStage) -> AppResult<()> {
    tracing::info!(?stage, "Application startup milestone");
    let snapshot = application.state::<StartupService>().advance(stage)?;
    publish(application, &snapshot);
    Ok(())
}

/// Initializes dependencies after setup returns so the splash stays responsive.
///
/// # Errors
/// Propagates filesystem, database and startup-state failures to the boot supervisor.
async fn initialize(application: &AppHandle) -> AppResult<()> {
    let directory_handle = application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        super::system::init_directories(&directory_handle)
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;
    super::system::init_settings(application);
    super::system::init_events(application);
    super::system::init_formats(application);
    advance(application, StartupStage::Database)?;
    super::database::init(application).await?;
    advance(application, StartupStage::Media)?;
    super::streaming::init(application).await;
    super::workers::init(application);
    advance(application, StartupStage::Library)?;
    super::library::init(application).await;
    advance(application, StartupStage::BackendReady)
}
