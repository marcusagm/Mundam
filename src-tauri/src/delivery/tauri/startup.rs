//! Thin desktop adapter for startup queries and main-window acknowledgements.

use crate::bootstrap::startup::publish;
use crate::core::error::{AppError, AppResult};
use crate::feature::startup::{StartupService, StartupSnapshot, StartupStage};
use tauri::{AppHandle, Manager, State, WebviewWindow};

/// Reads the retained boot snapshot without requiring initialized library services.
///
/// # Errors
/// Returns an internal error if startup state is unavailable.
#[tauri::command]
pub fn get_startup_state(service: State<'_, StartupService>) -> AppResult<StartupSnapshot> {
    service.snapshot()
}

/// Adapts main-window milestones to the application startup service.
///
/// # Errors
/// Rejects callers other than the main window and invalid readiness transitions.
#[tauri::command]
pub fn report_startup_stage(
    window: WebviewWindow,
    application: AppHandle,
    service: State<'_, StartupService>,
    stage: StartupStage,
) -> AppResult<()> {
    require_main(&window)?;
    let snapshot = service.report_frontend(stage)?;
    publish(&application, &snapshot);
    Ok(())
}

/// Shows the prepared main window before disposing the startup presentation.
///
/// # Errors
/// Propagates readiness validation and native window failures.
#[tauri::command]
pub fn complete_startup(
    window: WebviewWindow,
    application: AppHandle,
    service: State<'_, StartupService>,
) -> AppResult<()> {
    require_main(&window)?;
    service.ensure_presentable()?;
    window.show()?;
    if let Err(error) = window.set_focus() {
        tracing::warn!(%error, "Unable to focus the prepared main window");
    }
    let snapshot = service.advance(StartupStage::Ready)?;
    publish(&application, &snapshot);
    if let Some(splash) = application.get_webview_window("splashscreen") {
        if let Err(error) = splash.close() {
            tracing::warn!(%error, "Unable to close the completed splash window");
        }
    }
    Ok(())
}

/// Retains a safe frontend boot failure for the still-visible splash.
///
/// # Errors
/// Rejects callers other than the main window or an unavailable state lock.
#[tauri::command]
pub fn fail_frontend_startup(
    window: WebviewWindow,
    application: AppHandle,
    service: State<'_, StartupService>,
) -> AppResult<()> {
    require_main(&window)?;
    let snapshot = service.fail("FRONTEND_STARTUP_FAILED")?;
    publish(&application, &snapshot);
    Ok(())
}

fn require_main(window: &WebviewWindow) -> AppResult<()> {
    if window.label() != "main" {
        return Err(AppError::Forbidden(
            "Only the main window can acknowledge startup".into(),
        ));
    }
    Ok(())
}
