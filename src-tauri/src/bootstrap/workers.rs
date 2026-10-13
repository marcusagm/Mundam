//! Continuous Asynchronous Processing Orchestrator
//!
//! Allocates the fleets of reactive workers that handle heavy CPU-bound processing loads,
//! isolating these computations from Tauri IPC routes to prevent frontend bottlenecks.

use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Instantiates and activates queue consumers focused on feature extraction during idle time.
///
/// Initializes dedicated workers for iterative Thumbnail extraction (native resolution or via
/// bundled binaries) and the sub-worker for chromatic extraction.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub fn init(app: &AppHandle) {
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
    let asset_ledger = app
        .state::<Arc<dyn crate::core::ledger::port::TransactionalAssetLedger>>()
        .inner()
        .clone();
    let asset_query_handler = app
        .state::<Arc<dyn crate::core::repository::AssetQueryHandler>>()
        .inner()
        .clone();

    let priority_state = std::sync::Arc::new(
        crate::core::workflows::thumbnails::priority::ThumbnailPriorityState::default(),
    );
    app.manage(priority_state.clone());

    // Start Thumbnail Worker (Hybrid Queue)
    let thumbnail_token = lifecycle.child_token();
    let thumbnail_worker = crate::processing::workers::thumbnail_worker::ThumbnailWorker::new(
        format_registry.clone(),
        asset_ledger.clone(),
        asset_query_handler.clone(),
        priority_state,
        dirs.thumbnails_dir.clone(),
    );
    let thumbnail_handle = thumbnail_worker.start(thumbnail_token.clone(), app.clone());
    lifecycle.register(
        "thumbnail_worker".to_string(),
        thumbnail_token,
        thumbnail_handle,
    );

    // Start Color Worker (Reactive to Thumbnails)
    let color_worker_token = lifecycle.child_token();
    let color_worker = crate::processing::workers::color_worker::ColorWorker::new(
        asset_ledger.clone(),
        event_bus.clone(),
        format_registry.clone(),
        dirs.thumbnails_dir.to_path_buf(),
    );
    let color_handle = color_worker.start(color_worker_token.clone());
    lifecycle.register("color_worker".to_string(), color_worker_token, color_handle);

    // Start Auto-Empty Trash Worker (Periodic Cleanup)
    let trash_worker_token = lifecycle.child_token();
    let settings_adapter = std::sync::Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(dirs.settings_path.clone()),
    );
    let trash_settings_service =
        crate::feature::settings::SettingsService::new(settings_adapter);
    let trash_worker = crate::feature::trash::auto_empty_worker::AutoEmptyTrashWorker::new(
        asset_ledger.clone(),
        app.state::<Arc<crate::infra::database::manager::DbManager>>()
            .inner()
            .clone(),
        trash_settings_service,
        dirs.app_data.to_path_buf(),
    );
    let trash_handle = trash_worker.start(trash_worker_token.clone());
    lifecycle.register(
        "auto_empty_trash_worker".to_string(),
        trash_worker_token,
        trash_handle,
    );

    // Start Duplicate Worker
    let duplicates_repo = app
        .state::<Arc<dyn crate::core::repository::DuplicatesRepository>>()
        .inner()
        .clone();

    let settings_service = app
        .state::<crate::feature::settings::SettingsService>()
        .inner()
        .clone();

    let duplicate_worker_token = lifecycle.child_token();
    let duplicate_worker = crate::feature::duplicates::events::DuplicateWorker::new(
        duplicates_repo.clone(),
        event_bus.clone(),
        settings_service,
    );
    let duplicate_handle = std::sync::Arc::new(duplicate_worker).start(duplicate_worker_token.clone());
    lifecycle.register(
        "duplicate_worker".to_string(),
        duplicate_worker_token,
        duplicate_handle,
    );
}
