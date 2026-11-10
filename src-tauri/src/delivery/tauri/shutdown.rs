//! Restricted IPC for the independent shutdown presentation.

use crate::core::error::{AppError, AppResult};
use crate::feature::shutdown::{ShutdownService, ShutdownSnapshot};
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
/// Rejects other windows, late acknowledgements, and unavailable state.
#[tauri::command]
pub async fn acknowledge_shutdown_presentation(
    window: WebviewWindow,
    application: AppHandle,
    service: State<'_, ShutdownService>,
) -> AppResult<()> {
    require_shutdown(&window)?;
    service.presented()?;
    for label in ["main", "splashscreen"] {
        if let Some(previous) = application.get_webview_window(label) {
            if let Err(error) = previous.hide() {
                tracing::warn!(window = label, %error, "Unable to hide previous window during shutdown");
            }
        }
    }
    Ok(())
}
