//! Library Synchronization Orchestrator
//!
//! Strictly handles flows responsible for unifying the Database state with the
//! mutable state of the native File System, orchestrating both Batch Indexers
//! and FSEvents sensors (Watchers).

use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Lays the logical foundations of the library and activates passive sensors for root directories.
///
/// Autonomously triggers differential verification scans (Boot Scan) on previously
/// engaged *roots* to repair inconsistent states from prior unexpected crashes.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
pub async fn init(app: &AppHandle) {
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
    let lifecycle = app
        .state::<Arc<crate::lifecycle::LifecycleRegistry>>()
        .inner()
        .clone();

    // Read concurrency limit from settings
    let concurrency_limit = if let Some(settings_service) =
        app.try_state::<crate::feature::settings::SettingsService>()
    {
        match settings_service
            .get_setting("indexer_concurrency_limit")
            .await
        {
            Ok(Some(value)) => value.as_u64().unwrap_or(200) as usize,
            _ => 200,
        }
    } else {
        200
    };

    // Initialize Indexer
    let path_guard = app
        .state::<Arc<crate::core::path_guard::LedgerPathGuard>>()
        .inner()
        .clone();
    let indexer = Arc::new(
        crate::feature::library::indexer::LibraryIndexer::new(
            asset_query_handler.clone(),
            asset_ledger.clone(),
            event_bus.clone(),
            format_registry.clone(),
        )
        .with_concurrency_limit(concurrency_limit)
        .with_path_guard(path_guard),
    );
    app.manage(indexer.clone());

    let indexer_token = lifecycle.child_token();
    let indexer_handle = indexer
        .clone()
        .start_event_listener(event_bus.clone(), indexer_token.clone());
    lifecycle.register("indexer_event_listener".to_string(), indexer_token, indexer_handle);

    // Initialize Watcher Service
    let watcher = Arc::new(crate::processing::watcher::WatcherService::new(
        event_bus.clone(),
    ));
    app.manage(watcher.clone());

    // Start Watchers + Boot Scan for Existing Roots
    if let Ok(roots) = asset_query_handler.list_folders(None).await {
        tracing::info!("Starting watchers for {} roots", roots.len());

        for folder in &roots {
            let root_path = std::path::PathBuf::from(&folder.path);

            let watcher_token = lifecycle.child_token();
            if let Err(e) = watcher
                .watch(root_path.clone(), watcher_token.clone())
                .await
            {
                tracing::error!("Failed to start watcher for {}: {}", root_path.display(), e);
            } else {
                lifecycle.register(
                    format!("watcher:{}", root_path.display()),
                    watcher_token,
                    tauri::async_runtime::spawn(async {}),
                );
            }
        }

        if !roots.is_empty() {
            let indexer_for_boot = indexer.clone();
            let roots_for_boot: Vec<_> = roots
                .iter()
                .map(|folder| (std::path::PathBuf::from(&folder.path), folder.id.clone()))
                .collect();

            let boot_scan_token = lifecycle.child_token();
            let token_clone = boot_scan_token.clone();
            let boot_scan_handle = tauri::async_runtime::spawn(async move {
                for (root_path, folder_id) in roots_for_boot {
                    if token_clone.is_cancelled() {
                        tracing::info!("Boot scan cancelled");
                        break;
                    }
                    tracing::info!("Boot scan starting for: {}", root_path.display());
                    if let Err(e) = indexer_for_boot
                        .scan_directory(root_path.clone(), Some(folder_id))
                        .await
                    {
                        tracing::error!("Boot scan failed for {}: {}", root_path.display(), e);
                    }
                }
            });
            lifecycle.register("boot_scan".to_string(), boot_scan_token, boot_scan_handle);
        }
    }
}
