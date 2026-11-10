//! Tauri adapter for application shutdown and its independent presentation.

use crate::feature::shutdown::{ShutdownService, ShutdownSnapshot};
use crate::lifecycle::LifecycleRegistry;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

fn publish(application: &AppHandle, snapshot: &ShutdownSnapshot) {
    if let Err(error) = application.emit_to("shutdown", "shutdown:changed", snapshot) {
        tracing::warn!(%error, "Unable to publish shutdown progress");
    }
}

fn create_window(application: &AppHandle) {
    let result = WebviewWindowBuilder::new(
        application,
        "shutdown",
        WebviewUrl::App("shutdown.html".into()),
    )
    .title("Mundam — Closing")
    .inner_size(480.0, 360.0)
    .center()
    .resizable(false)
    .decorations(false)
    .focused(false)
    .build();
    if let Err(error) = result {
        tracing::error!(%error, "Unable to create shutdown window");
    }
}

/// Claims and starts shutdown once; subsequent requests join the active operation.
pub fn request(application: &AppHandle) {
    let service = application.state::<ShutdownService>();
    match service.request() {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(%error, "Unable to initiate shutdown");
            return;
        }
    }
    tracing::info!("Application shutdown requested");
    let lifecycle = application
        .state::<Arc<LifecycleRegistry>>()
        .inner()
        .clone();
    lifecycle.root_token().cancel();

    let window_handle = application.clone();
    tauri::async_runtime::spawn(async move { create_window(&window_handle) });

    let handle = application.clone();
    tauri::async_runtime::spawn(async move {
        let service = handle.state::<ShutdownService>();
        match service.draining() {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain shutdown progress"),
        }
        let boot_clean = match handle.try_state::<crate::bootstrap::startup::StartupTask>() {
            Some(startup) => startup.wait().await,
            None => true,
        };
        let clean = lifecycle.shutdown_all().await && boot_clean;
        tracing::info!(clean, "Application shutdown completed");
        match service.finish(!clean) {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain shutdown result"),
        }
        if let Err(error) = service.wait_for_presentation().await {
            tracing::error!(%error, "Unable to wait for shutdown presentation");
        }
        if let Err(error) = service.authorize_exit() {
            tracing::error!(%error, "Unable to authorize application exit");
            return;
        }
        handle.exit(0);
    });
}
