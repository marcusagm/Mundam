//! Restricted IPC for the independent shutdown presentation.

use crate::core::error::{AppError, AppResult};
use crate::feature::shutdown::{ShutdownPhase, ShutdownService, ShutdownSnapshot};
use tauri::{AppHandle, Manager, State, WebviewWindow};

fn require_shutdown(window: &WebviewWindow) -> AppResult<()> {
    if window.label() != "shutdown" {
        return Err(AppError::Forbidden(
            "Only the shutdown window can access shutdown state".into(),
        ));
    }
    Ok(())
}

/// Returns retained shutdown progress to the shutdown window.
///
/// # Errors
/// Rejects other windows and unavailable state.
#[tauri::command]
pub fn get_shutdown_state(
    window: WebviewWindow,
    service: State<'_, ShutdownService>,
) -> AppResult<ShutdownSnapshot> {
    require_shutdown(&window)?;
    service.snapshot()
}

/// Acknowledges a rendered shutdown presentation.
///
/// # Errors
/// Rejects other windows, acknowledgements outside active shutdown, and unavailable state.
#[tauri::command]
pub async fn acknowledge_shutdown_presentation(
    window: WebviewWindow,
    application: AppHandle,
    service: State<'_, ShutdownService>,
) -> AppResult<()> {
    require_shutdown(&window)?;
    let timely = match service.presented() {
        Ok(()) => true,
        Err(AppError::ValidationFailed(_))
            if service.snapshot()?.phase != ShutdownPhase::Idle
                && !service.exit_authorized()? =>
        {
            // A rendered window can replace the main window even when it arrived
            // too late to extend the bounded minimum presentation interval.
            false
        }
        Err(error) => return Err(error),
    };
    tracing::info!(
        timely,
        "Shutdown presentation acknowledged; hiding previous windows"
    );
    for label in ["main", "splashscreen"] {
        if let Some(previous) = application.get_webview_window(label) {
            if let Err(error) = previous.hide() {
                tracing::warn!(window = label, %error, "Unable to hide previous window during shutdown");
            }
        }
    }
    Ok(())
}
