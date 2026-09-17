//! Persistence Infrastructure Initialization Orchestrator
//!
//! Centralizes the SQLite local database connection and the configuration of all
//! Infrastructure-side CQRS abstractions. Injects transactional Ledgers and
//! highly optimized Query Handlers into the application ecosystem.

use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Initializes the system's asynchronous data abstractions and domain Query services.
///
/// # Arguments
/// * `app` - Reference to the Tauri AppHandle.
///
/// # Errors
/// Returns `Err(String)` propagating critical SQLx connection pool assembly failures.
pub async fn init(app: &AppHandle) -> Result<(), String> {
    let dirs = app.state::<crate::bootstrap::AppDirectories>();
    let format_registry = app
        .state::<Arc<crate::core::formats::FormatRegistry>>()
        .inner()
        .clone();
    let event_bus = app
        .state::<Arc<dyn crate::core::events::AppEventBus>>()
        .inner()
        .clone();

    // Initialize Database Infrastructure
    let db_manager = match crate::infra::database::manager::DbManager::new(&dirs.db_path).await {
        Ok(manager) => Arc::new(manager),
        Err(err) => {
            tracing::error!("Failed to initialize database manager: {}", err);
            return Err(err.to_string());
        }
    };
    app.manage(db_manager.clone());

    // Initialize Query Handler early
    let asset_query_handler: Arc<dyn crate::core::repository::AssetQueryHandler> =
        Arc::new(crate::infra::database::queries::SqliteAssetQueries::new(
            db_manager.pool().clone(),
            format_registry.clone(),
        ));
    app.manage(asset_query_handler.clone());

    let asset_ledger_impl = Arc::new(crate::infra::database::ledger::SqliteAssetLedger::new(
        db_manager.pool().clone(),
        event_bus.clone(),
    ));

    // Run database path normalization cleanup (one-time logic)
    let normalization_pool = db_manager.pool().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::infra::database::handlers::shared::normalize_database_paths(&normalization_pool).await {
            tracing::error!("Failed to normalize database paths: {}", error);
        }
    });

    // Run Saga Recovery
    let recovery_service =
        crate::infra::database::saga_recovery::SagaRecoveryService::new(db_manager.pool().clone());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = recovery_service.run_recovery().await {
            tracing::error!("Failed to run saga recovery: {}", e);
        }
    });

    let asset_ledger: Arc<dyn crate::core::ledger::port::TransactionalAssetLedger> =
        asset_ledger_impl;
    app.manage(asset_ledger.clone());

    // Initialize High-level query/search services
    let asset_query_service =
        crate::feature::assets::queries::AssetQueryService::new(asset_query_handler.clone());
    app.manage(asset_query_service);

    let search_query_handler =
        crate::feature::search::SearchQueryHandler::new(asset_query_handler.clone());
    app.manage(search_query_handler);

    // Initialize Duplicates Services
    let duplicates_repo: Arc<dyn crate::core::repository::DuplicatesRepository> = 
        Arc::new(crate::infra::sqlite::SqliteDuplicatesRepository::new(db_manager.pool().clone()));
    app.manage(duplicates_repo.clone());

    let duplicates_query_service =
        crate::feature::duplicates::queries::DuplicateQueryService::new(duplicates_repo.clone());
    app.manage(duplicates_query_service);

    let duplicates_command_service =
        crate::feature::duplicates::commands::DuplicateCommandService::new(
            duplicates_repo.clone(),
            asset_ledger.clone(),
            event_bus.clone(),
            asset_query_handler.clone(),
        );
    app.manage(duplicates_command_service);

    Ok(())
}
