//! Base Systems Initialization Orchestrator
//!
//! Prepares the foundational layers of the application prior to loading heavy
//! infrastructure dependencies. This includes path resolution, user settings,
//! event buses, and in-memory registries.

use crate::core::events::AppEventBus;
use crate::infra::events::TokioEventBus;
use crate::lifecycle::LifecycleRegistry;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Resolves and ensures the creation of all fundamental directories for local persistence.
///
/// This method acts synchronously/blockingly, as failure to obtain or create the
/// local application directory completely invalidates the boot process.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
///
/// # Returns
/// Populated `AppDirectories` instance, which is also injected into the application State.
pub fn init_directories(app: &AppHandle) -> crate::bootstrap::AppDirectories {
    let app_data = app
        .path()
        .app_local_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&app_data).ok();

    let db_path = app_data.join("mundam.db");
    let settings_path = app_data.join("settings.json");
    let thumbnails_dir = app_data.join("thumbnails");
    std::fs::create_dir_all(&thumbnails_dir).ok();
    let previews_dir = app_data.join("previews");
    std::fs::create_dir_all(&previews_dir).ok();

    let dirs = crate::bootstrap::AppDirectories {
        app_data,
        db_path,
        settings_path,
        thumbnails_dir,
        previews_dir,
    };

    app.manage(dirs.clone());
    dirs
}

/// Initializes the settings service and manages it in the application state.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub fn init_settings(app: &AppHandle) {
    let dirs = app.state::<crate::bootstrap::AppDirectories>();
    let settings_adapter = Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(dirs.settings_path.clone()),
    );
    let settings_service = crate::feature::settings::SettingsService::new(settings_adapter);
    app.manage(settings_service);
}

/// Initializes the event bus and manages it in the application state.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
///
/// # Returns
/// The initialized `Arc<dyn AppEventBus>` instance.
pub fn init_events(app: &AppHandle) {
    let event_bus = Arc::new(TokioEventBus::new());
    app.manage(event_bus.clone() as Arc<dyn AppEventBus>);

    // Bridge Event Bus to Frontend
    let mut rx = event_bus.subscribe();
    let app_handle = app.clone();
    let lifecycle = app.state::<Arc<LifecycleRegistry>>().inner().clone();
    let bridge_token = lifecycle.child_token();
    let token_clone = bridge_token.clone();

    let bridge_handle = tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                recv_result = rx.recv() => {
                    match recv_result {
                        Ok(event) => {
                            use crate::core::events::DomainEvent;
                            // Standard Domain Event Bridge
                            if let Err(e) = app_handle.emit("mundam://domain-event", &event) {
                                tracing::error!("Failed to emit domain event to frontend: {}", e);
                            }

                            match event {
                                // ── Targeted Asset Deletion (instant UI removal) ──────
                                DomainEvent::AssetDeleted {
                                    asset_id,
                                    folder_id,
                                } => {
                                    let _ = app_handle.emit(
                                        "library:batch-change",
                                        serde_json::json!({
                                            "added": [],
                                            "removed": [{
                                                "id": asset_id,
                                                "folder_id": folder_id.unwrap_or_default(),
                                                "tag_ids": []
                                            }],
                                            "updated": [],
                                            "needs_refresh": false
                                        }),
                                    );
                                }

                                // ── Events requiring a full library refresh ──────────
                                DomainEvent::AssetCreated { .. }
                                | DomainEvent::AssetFolderChanged { .. }
                                | DomainEvent::FolderMetadataUpdated { .. }
                                | DomainEvent::FolderRemoved { .. }
                                | DomainEvent::FsDirectoryDeleted { .. }
                                | DomainEvent::FsPathRenamed { .. } => {
                                    let _ = app_handle.emit(
                                        "library:batch-change",
                                        serde_json::json!({
                                            "added": [],
                                            "removed": [],
                                            "updated": [],
                                            "needs_refresh": true
                                        }),
                                    );
                                }
                                DomainEvent::ScanProgress {
                                    total,
                                    processed,
                                    current_file,
                                } => {
                                    let _ = app_handle.emit(
                                        "indexer:progress",
                                        serde_json::json!({
                                            "total": total,
                                            "processed": processed,
                                            "current_file": current_file,
                                        }),
                                    );
                                }
                                DomainEvent::ScanCompleted { .. } => {
                                    let _ = app_handle.emit("indexer:complete", 0);
                                }
                                DomainEvent::ThumbnailGenerated { asset_id, path, .. } => {
                                    let _ = app_handle.emit(
                                        "thumbnail:ready",
                                        serde_json::json!({
                                            "id": asset_id,
                                            "path": path,
                                        }),
                                    );
                                }
                                DomainEvent::AssetMetadataUpdated { asset_id } => {
                                    let _ = app_handle.emit(
                                        "metadata:ready",
                                        serde_json::json!({
                                            "id": asset_id,
                                        }),
                                    );
                                }
                                DomainEvent::ExtractionCompleted {
                                    asset_id,
                                    capability,
                                } => {
                                    if capability == "COLORS" {
                                        let _ = app_handle.emit("extraction:completed", asset_id);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Event bridge lagged by {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event bridge channel closed");
                            break;
                        }
                    }
                }
                _ = token_clone.cancelled() => {
                    tracing::info!("Domain Event Bridge cancelled. Exiting.");
                    break;
                }
            }
        }
    });

    lifecycle.register("event_bridge_frontend".to_string(), bridge_token, bridge_handle);
}

/// Initializes the lifecycle registry and manages it in the application state.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub fn init_lifecycle(app: &AppHandle) {
    let lifecycle = std::sync::Arc::new(LifecycleRegistry::new());
    app.manage(lifecycle);
}

/// Initializes the format registry and manages it in the application state.
///
/// Assembles the `FormatRegistry` by collecting all providers from each media
/// category and registering them. The generic fallback provider is always
/// registered last to ensure it only matches when no specific provider could
/// identify the file.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub fn init_formats(app: &AppHandle) {
    use crate::core::formats::FormatRegistry;
    use crate::processing::media::providers;

    let mut format_registry = FormatRegistry::new();
    format_registry.register_batch(providers::collect_all_providers());
    format_registry.register_fallback(providers::fallback_provider());

    app.manage(std::sync::Arc::new(format_registry));
}
