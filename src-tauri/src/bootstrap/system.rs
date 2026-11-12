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
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct BridgeShutdownEvents {
    total: u64,
    scan_progress: u64,
    scan_completed: u64,
    scan_failed: u64,
    thumbnail: u64,
    metadata: u64,
    lagged: u64,
}

impl BridgeShutdownEvents {
    fn record(&mut self, event: &crate::core::events::DomainEvent) {
        use crate::core::events::DomainEvent;
        self.total += 1;
        match event {
            DomainEvent::ScanProgress { .. } => self.scan_progress += 1,
            DomainEvent::ScanCompleted { .. } => self.scan_completed += 1,
            DomainEvent::ScanFailed { .. } => self.scan_failed += 1,
            DomainEvent::ThumbnailGenerated { .. } => self.thumbnail += 1,
            DomainEvent::AssetMetadataUpdated { .. } => self.metadata += 1,
            _ => {}
        }
    }

    fn log(&self) {
        tracing::info!(
            total = self.total,
            scan_progress = self.scan_progress,
            scan_completed = self.scan_completed,
            scan_failed = self.scan_failed,
            thumbnail = self.thumbnail,
            metadata = self.metadata,
            lagged = self.lagged,
            "Domain Event Bridge shutdown receive summary; frontend delivery is not verified"
        );
    }
}

async fn next_bridge_event(
    rx: &mut broadcast::Receiver<crate::core::events::DomainEvent>,
    token: &CancellationToken,
) -> Option<Result<crate::core::events::DomainEvent, broadcast::error::RecvError>> {
    if token.is_cancelled() {
        return next_ready_bridge_event(rx);
    }
    tokio::select! {
        result = rx.recv() => Some(result),
        _ = token.cancelled() => next_ready_bridge_event(rx),
    }
}

fn next_ready_bridge_event(
    rx: &mut broadcast::Receiver<crate::core::events::DomainEvent>,
) -> Option<Result<crate::core::events::DomainEvent, broadcast::error::RecvError>> {
    match rx.try_recv() {
        Ok(event) => Some(Ok(event)),
        Err(broadcast::error::TryRecvError::Lagged(count)) => {
            Some(Err(broadcast::error::RecvError::Lagged(count)))
        }
        Err(broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed) => None,
    }
}

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
///
/// # Errors
/// Propagates unavailable application paths and directory creation failures.
pub fn init_directories(
    app: &AppHandle,
) -> crate::core::error::AppResult<crate::bootstrap::AppDirectories> {
    let app_data = app.path().app_local_data_dir()?;
    std::fs::create_dir_all(&app_data)?;

    let db_path = app_data.join("mundam.db");
    let settings_path = app_data.join("settings.json");
    let thumbnails_dir = app_data.join("thumbnails");
    std::fs::create_dir_all(&thumbnails_dir)?;
    let previews_dir = app_data.join("previews");
    std::fs::create_dir_all(&previews_dir)?;

    let dirs = crate::bootstrap::AppDirectories {
        app_data,
        db_path,
        settings_path,
        thumbnails_dir,
        previews_dir,
    };

    app.manage(dirs.clone());
    Ok(dirs)
}

/// Initializes the settings service and manages it in the application state.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub fn init_settings(app: &AppHandle) {
    let dirs = app.state::<crate::bootstrap::AppDirectories>();
    let settings_adapter: Arc<dyn crate::core::settings::SettingsRepository> = Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(dirs.settings_path.clone()),
    );
    let control = Arc::new(crate::core::processing::control::ProcessingControl::default());
    let settings_service = crate::feature::settings::SettingsService::new(settings_adapter.clone())
        .with_control(control.clone());
    app.manage(control);
    app.manage(settings_adapter);
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
    let bridge_token = CancellationToken::new();
    let token_clone = bridge_token.clone();
    let shutdown_token = lifecycle.root_token();

    let bridge_handle = tauri::async_runtime::spawn(async move {
        let mut shutdown_events = BridgeShutdownEvents::default();
        loop {
            match next_bridge_event(&mut rx, &token_clone).await {
                Some(recv_result) => {
                    match recv_result {
                        Ok(event) => {
                            use crate::core::events::DomainEvent;
                            if shutdown_token.is_cancelled() {
                                shutdown_events.record(&event);
                            }
                            // Standard Domain Event Bridge
                            if let Err(e) = app_handle.emit("mundam://domain-event", &event) {
                                tracing::error!("Failed to emit domain event to frontend: {}", e);
                            }

                            match event {
                                DomainEvent::LibraryFoldersRemoved { .. } => {
                                    let _ = app_handle.emit(
                                        "library:batch-change",
                                        serde_json::json!({
                                            "operation": "folder_removal",
                                            "needs_refresh": true
                                        }),
                                    );
                                }
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
                                DomainEvent::ScanFailed { library_id } => {
                                    if let Err(error) = app_handle.emit(
                                        "indexer:failed",
                                        serde_json::json!({ "library_id": library_id }),
                                    ) {
                                        tracing::error!(%error, "Failed to emit indexer failure to frontend");
                                    }
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
                                } if capability == "COLORS" => {
                                    let _ = app_handle.emit("extraction:completed", asset_id);
                                }
                                _ => {}
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            if shutdown_token.is_cancelled() {
                                shutdown_events.lagged += n;
                            }
                            tracing::warn!("Event bridge lagged by {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event bridge channel closed");
                            break;
                        }
                    }
                }
                None => {
                    tracing::info!("Domain Event Bridge drained and stopped.");
                    break;
                }
            }
        }
        if shutdown_token.is_cancelled() {
            shutdown_events.log();
        }
    });

    lifecycle.register(
        "event_bridge_frontend".to_string(),
        bridge_token,
        bridge_handle,
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::DomainEvent;

    #[tokio::test]
    async fn bridge_drains_queued_events_after_cancellation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = broadcast::channel(4);
        sender.send(DomainEvent::ScanStarted {
            library_id: "one".into(),
        })?;
        sender.send(DomainEvent::ScanCompleted {
            library_id: "one".into(),
        })?;
        let token = CancellationToken::new();
        token.cancel();
        assert!(matches!(
            next_bridge_event(&mut receiver, &token).await,
            Some(Ok(DomainEvent::ScanStarted { .. }))
        ));
        assert!(matches!(
            next_bridge_event(&mut receiver, &token).await,
            Some(Ok(DomainEvent::ScanCompleted { .. }))
        ));
        assert!(next_bridge_event(&mut receiver, &token).await.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn bridge_reports_lag_after_cancellation() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = broadcast::channel(1);
        for id in ["one", "two"] {
            sender.send(DomainEvent::ScanStarted {
                library_id: id.into(),
            })?;
        }
        let token = CancellationToken::new();
        token.cancel();
        assert!(matches!(
            next_bridge_event(&mut receiver, &token).await,
            Some(Err(broadcast::error::RecvError::Lagged(1)))
        ));
        assert!(matches!(
            next_bridge_event(&mut receiver, &token).await,
            Some(Ok(DomainEvent::ScanStarted { .. }))
        ));
        assert!(next_bridge_event(&mut receiver, &token).await.is_none());
        Ok(())
    }

    #[test]
    fn shutdown_event_counts_are_categorized() {
        let mut counts = BridgeShutdownEvents::default();
        counts.record(&DomainEvent::ScanCompleted {
            library_id: "one".into(),
        });
        counts.record(&DomainEvent::AssetMetadataUpdated {
            asset_id: "asset".into(),
        });
        counts.record(&DomainEvent::ScanStarted {
            library_id: "one".into(),
        });
        assert_eq!(counts.total, 3);
        assert_eq!(counts.scan_completed, 1);
        assert_eq!(counts.metadata, 1);
        assert_eq!(counts.scan_progress, 0);
    }
}
