//! Auto-Empty Trash Worker
//!
//! A background task that periodically checks for expired items in the
//! Mundam trash and permanently deletes them. The worker reads configuration
//! from the user settings (`trash_auto_empty_enabled`, `trash_auto_empty_days`)
//! and only acts when the feature is enabled.
//!
//! ## Worker Pattern
//!
//! Follows the same lifecycle pattern used by other Mundam workers
//! (e.g., `HlsManager::start_cleanup_worker`):
//! - Spawned via `tokio::spawn` during application boot.
//! - Accepts a `CancellationToken` for graceful shutdown.
//! - Polls at a configurable interval (default: 1 hour).

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::core::error::AppError;
use crate::core::ledger::command::LedgerCommand;
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::feature::settings::SettingsService;

/// Default polling interval in seconds (1 hour).
const POLL_INTERVAL_SECONDS: u64 = 3600;

/// The auto-empty trash worker.
///
/// Periodically checks for trashed assets whose `deleted_at` timestamp
/// exceeds the configured retention period and permanently removes them.
pub struct AutoEmptyTrashWorker {
    /// Reference to the asset ledger for executing physical deletes.
    ledger: Arc<dyn TransactionalAssetLedger>,
    /// Database pool for querying trashed assets.
    pool_manager: Arc<crate::infra::database::manager::DbManager>,
    /// Settings service for reading auto-empty configuration.
    settings_service: SettingsService,
    /// Base application data directory (for trash path resolution).
    app_data_directory: std::path::PathBuf,
}

impl AutoEmptyTrashWorker {
    /// Creates a new auto-empty trash worker.
    ///
    /// # Arguments
    ///
    /// * `ledger` - The asset ledger for executing deletes.
    /// * `pool_manager` - The database connection pool manager.
    /// * `settings_service` - The settings service for reading configuration.
    /// * `app_data_directory` - The base app data directory.
    pub fn new(
        ledger: Arc<dyn TransactionalAssetLedger>,
        pool_manager: Arc<crate::infra::database::manager::DbManager>,
        settings_service: SettingsService,
        app_data_directory: std::path::PathBuf,
    ) -> Self {
        Self {
            ledger,
            pool_manager,
            settings_service,
            app_data_directory,
        }
    }

    /// Starts the background worker loop.
    ///
    /// The worker runs until the cancellation token is triggered.
    /// On each tick it reads the settings, checks if auto-empty is enabled,
    /// and if so, purges expired trash items.
    ///
    /// # Arguments
    ///
    /// * `cancellation_token` - Token to signal graceful shutdown.
    ///
    /// # Returns
    ///
    /// A join handle for the spawned background task.
    pub fn start(
        self,
        cancellation_token: CancellationToken,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let token_clone = cancellation_token.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECONDS));

            loop {
                tokio::select! {
                    _ = token_clone.cancelled() => {
                        tracing::info!("AutoEmptyTrashWorker: Shutdown signal received.");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(error) = self.run_cycle().await {
                            tracing::error!("AutoEmptyTrashWorker: Cycle failed: {}", error);
                        }
                    }
                }
            }
        })
    }

    /// Executes a single auto-empty cycle.
    ///
    /// Reads the user settings to determine if auto-empty is enabled and what
    /// the retention period is. If enabled, queries for all trashed assets
    /// older than the retention period and permanently deletes them.
    async fn run_cycle(&self) -> Result<(), AppError> {
        // Read settings
        let is_enabled = self
            .settings_service
            .get_setting("trash_auto_empty_enabled")
            .await?
            .and_then(|value| value.as_str().map(|string| string == "true"))
            .unwrap_or(false);

        if !is_enabled {
            return Ok(());
        }

        let retention_days = self
            .settings_service
            .get_setting("trash_auto_empty_days")
            .await?
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|string| string.parse().ok()))
            })
            .unwrap_or(30) as i64;

        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_string = cutoff.to_rfc3339();

        let pool = self.pool_manager.pool();

        let expired_assets = sqlx::query!(
            r#"SELECT id as "id!", path as "path!", deleted_at as "deleted_at?: chrono::DateTime<chrono::Utc>"
               FROM assets
               WHERE deleted_at IS NOT NULL AND deleted_at < ?"#,
            cutoff_string
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        if expired_assets.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "AutoEmptyTrashWorker: Found {} expired trash items (older than {} days).",
            expired_assets.len(),
            retention_days
        );

        let mut purged_count = 0;

        for record in expired_assets {
            let original_path = std::path::PathBuf::from(&record.path);

            // Resolve the physical trash path so the Saga deletes the correct file.
            // Tries timestamped format first, then falls back to legacy format.
            let resolved_trash_path = self.resolve_trash_file_path(
                &record.id,
                &original_path,
                record.deleted_at.as_ref(),
            );

            let result = self
                .ledger
                .execute(LedgerCommand::DeleteAsset {
                    asset_id: Some(record.id.clone()),
                    path: Some(resolved_trash_path),
                    physical_delete: true,
                })
                .await;

            if result.is_ok() {
                purged_count += 1;
            } else {
                tracing::warn!(
                    "AutoEmptyTrashWorker: Failed to purge asset {}: {:?}",
                    record.id,
                    result.err()
                );
            }
        }

        tracing::info!(
            "AutoEmptyTrashWorker: Purged {} expired trash items.",
            purged_count
        );

        Ok(())
    }

    /// Resolves the physical path of a file in the trash directory.
    ///
    /// Tries the timestamped format (`{asset_id}_{epoch}_{filename}`) first.
    /// If that file does not exist on disk, falls back to the legacy format
    /// (`{asset_id}_{filename}`). Returns the timestamped path as default
    /// if neither exists (the Saga handles `NotFound` gracefully).
    fn resolve_trash_file_path(
        &self,
        asset_id: &str,
        original_path: &std::path::Path,
        deleted_at: Option<&chrono::DateTime<chrono::Utc>>,
    ) -> std::path::PathBuf {
        if let Some(deleted_at_value) = deleted_at {
            if let Some(trash_path) = crate::core::trash::build_trash_path(
                &self.app_data_directory, asset_id, original_path, deleted_at_value,
            ) {
                if trash_path.exists() {
                    return trash_path;
                }
            }
        }

        if let Some(file_name) = original_path.file_name() {
            let legacy_path = crate::core::trash::trash_directory(&self.app_data_directory)
                .join(format!("{}_{}", asset_id, file_name.to_string_lossy()));
            if legacy_path.exists() {
                return legacy_path;
            }
        }

        if let Some(deleted_at_value) = deleted_at {
            if let Some(trash_path) = crate::core::trash::build_trash_path(
                &self.app_data_directory, asset_id, original_path, deleted_at_value,
            ) {
                return trash_path;
            }
        }

        original_path.to_path_buf()
    }
}
