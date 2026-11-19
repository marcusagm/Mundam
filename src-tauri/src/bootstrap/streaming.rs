//! Dynamic Media Initialization Orchestrator
//!
//! Manages the setup of the internal HTTP server (Axum), the instantiation of the
//! on-demand HLS transcode manager, and health-checks for native video conversion dependencies.

use crate::delivery::streaming::server::start_server;
use crate::feature::transcoding::cache::TranscodeCache;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Bootstraps the streaming infrastructure and validates the sanity of vital external binaries.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub async fn init(app: &AppHandle) {
    let dirs = app.state::<crate::bootstrap::AppDirectories>();
    let lifecycle = app
        .state::<Arc<crate::lifecycle::LifecycleRegistry>>()
        .inner()
        .clone();
    let format_registry = app
        .state::<Arc<crate::core::formats::FormatRegistry>>()
        .inner()
        .clone();
    let event_bus = app
        .state::<Arc<dyn crate::core::events::AppEventBus>>()
        .inner()
        .clone();

    // Initialize HLS On-the-Fly Streaming Manager
    let hls_token = lifecycle.child_token();
    let hls_manager = crate::feature::transcoding::hls_manager::HlsManager::new(
        &dirs.app_data,
        hls_token.clone(),
    );

    let manager = hls_manager.clone();
    let hls_lifecycle = lifecycle.clone();
    if !lifecycle.spawn_unique("hls_cleanup_worker".into(), async move {
        if let Err(error) = manager.run(90).await {
            tracing::error!(%error, "IPC HLS owner failed");
            hls_lifecycle.report_failure("IPC HLS sessions");
        }
    }) {
        hls_token.cancel();
    }
    app.manage(hls_manager.clone());

    // Generate a session token for streaming server authentication
    let session_token = uuid::Uuid::new_v4().to_string();
    app.manage(
        crate::delivery::tauri::commands::queries::StreamingSessionToken(session_token.clone()),
    );

    // Initialize Transcode Cache
    let transcode_cache = Arc::new(TranscodeCache::new(&dirs.app_data, format_registry.clone()));
    app.manage(transcode_cache);

    // Start Streaming Server (Axum)
    let server_token = lifecycle.child_token();
    let server_handle = start_server(app.clone(), 9876, server_token.clone()).await;
    lifecycle.register("streaming_server".to_string(), server_token, server_handle);

    // FFmpeg Health Check
    let availability = tauri::async_runtime::spawn_blocking(
        crate::processing::transcoding::check_transcoding_availability,
    )
    .await;
    if !matches!(availability, Ok(true)) {
        tracing::warn!("FFmpeg not found. Video transcoding unavailable.");
        let _ = event_bus.publish(crate::core::events::DomainEvent::SystemHealthIssue {
            component: "ffmpeg".to_string(),
            message: "FFmpeg not found. Video transcoding unavailable.".to_string(),
        });
    }
}
