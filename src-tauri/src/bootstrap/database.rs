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
/// Propagates database connection and migration failures through `AppError`.
pub async fn init(app: &AppHandle) -> crate::core::error::AppResult<()> {
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
            return Err(err);
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
    let processing_repository: Arc<dyn crate::core::processing::ProcessingRepository> = Arc::new(
        crate::infra::database::processing::SqliteProcessingRepository::new(
            db_manager.pool().clone(),
        ),
    );
    app.manage(processing_repository.clone());
    let processing_tools = Arc::new(
        crate::feature::processing::tools::ProcessingToolService::new(
            processing_repository.clone(),
            Arc::new(crate::processing::transcoding::binaries::BinaryToolProbe),
        ),
    );
    app.manage(processing_tools.clone());
    let processing_control = app
        .state::<Arc<crate::core::processing::control::ProcessingControl>>()
        .inner()
        .clone();
    let settings = app
        .state::<crate::feature::settings::SettingsService>()
        .get_settings()
        .await?;
    processing_control
        .budget
        .configure(settings.processing_concurrency, settings.thumbnail_threads);

    let path_guard = Arc::new(crate::core::path_guard::LedgerPathGuard::new());
    app.manage(path_guard.clone());

    let asset_ledger_impl = Arc::new(
        crate::infra::database::ledger::SqliteAssetLedger::new(
            db_manager.pool().clone(),
            event_bus.clone(),
            dirs.app_data.clone(),
            path_guard,
        )
        .with_settings_repository(
            app.state::<Arc<dyn crate::core::settings::SettingsRepository>>()
                .inner()
                .clone(),
        ),
    );

    crate::infra::database::library_operations::journal::recover(db_manager.pool(), &dirs.app_data)
        .await?;
    crate::infra::database::handlers::shared::normalize_database_paths(db_manager.pool()).await?;
    crate::infra::database::handlers::library::identities::seed_missing(db_manager.pool()).await?;
    let cleanup_pool = db_manager.pool().clone();
    let cleanup_directory = dirs.app_data.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(error) = crate::infra::database::library_operations::retry_cleanup(
                &cleanup_pool,
                &cleanup_directory,
            )
            .await
            {
                tracing::warn!(%error, "Library cleanup remains pending");
            }
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
    let duplicates_repo: Arc<dyn crate::core::repository::DuplicatesRepository> = Arc::new(
        crate::infra::database::duplicates::SqliteDuplicatesRepository::new(
            db_manager.pool().clone(),
        ),
    );
    app.manage(duplicates_repo.clone());

    let duplicates_query_service =
        crate::feature::duplicates::queries::DuplicateQueryService::new(duplicates_repo.clone());
    app.manage(duplicates_query_service);

    let duplicate_coordinator = Arc::new(
        crate::feature::duplicates::coordinator::DuplicateProcessingCoordinator::default()
            .with_budget(processing_control.budget.clone()),
    );
    app.manage(duplicate_coordinator.clone());
    let duplicates_command_service =
        crate::feature::duplicates::commands::DuplicateCommandService::new(
            duplicates_repo.clone(),
            asset_ledger.clone(),
            event_bus.clone(),
            asset_query_handler.clone(),
            format_registry.clone(),
            processing_repository.clone(),
        )
        .with_processing_coordinator(duplicate_coordinator)
        .with_processing_control(processing_control.clone());
    app.manage(Arc::new(
        crate::feature::processing::control::ProcessingControlService::new(
            duplicates_repo.clone(),
            processing_control,
            processing_repository,
            app.state::<Arc<dyn crate::core::settings::SettingsRepository>>()
                .inner()
                .clone(),
            Arc::new(duplicates_command_service.clone()),
        )
        .with_tools(processing_tools),
    ));
    app.manage(duplicates_command_service);

    Ok(())
}
