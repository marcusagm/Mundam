//! SQLite implementation of the `TransactionalAssetLedger` port.
//!
//! Acts as the transactional router and domain-event emitter for all write
//! operations. It begins a SQLite transaction, delegates execution to the
//! appropriate specialized handler in `handlers/`, commits the transaction,
//! runs any post-commit Saga (Outbox) steps, and finally publishes domain
//! events on the `AppEventBus`.
//!
//! **Architectural Invariant**: This module must contain ZERO lines of raw SQL
//! or business logic. All mutations are delegated to `handlers/*_handler.rs`
//! and all shared infrastructure utilities live in `handlers/shared.rs`.
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::error::{AppError, AppResult};
use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::ledger::command::LedgerCommand;
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::models::asset::Asset;

/// SQLite implementation of the Asset Ledger.
///
/// This adapter acts as a **pure transactional router**: it opens a transaction,
/// dispatches the command to the appropriate handler module, commits, executes
/// any Saga post-commit steps, and publishes domain events. It intentionally
/// contains no SQL queries or business rules itself.
pub struct SqliteAssetLedger {
    pool: SqlitePool,
    event_bus: Arc<dyn AppEventBus>,
    /// Root directory for app-local data. Used by post-commit Saga steps
    /// to resolve trash file paths without leaking infrastructure concerns
    /// into domain payloads.
    app_data_directory: PathBuf,
}

impl SqliteAssetLedger {
    /// Creates a new instance of the SqliteAssetLedger.
    ///
    /// # Arguments
    ///
    /// * `pool` - The database connection pool.
    /// * `event_bus` - The event bus for publishing domain events.
    /// * `app_data_directory` - The root directory for app-local data,
    ///   used to resolve trash paths in Saga post-commit steps.
    pub fn new(pool: SqlitePool, event_bus: Arc<dyn AppEventBus>, app_data_directory: PathBuf) -> Self {
        Self { pool, event_bus, app_data_directory }
    }
}

/// Implementation of the TransactionalAssetLedger trait for SqliteAssetLedger.
///
/// This trait is used to execute commands that modify the state of the asset ledger.
///
/// # Arguments
///
/// * `command` - The command to execute.
///
/// # Returns
///
/// A `Result` containing the updated asset or an error.
#[async_trait]
impl TransactionalAssetLedger for SqliteAssetLedger {
    /// Executes a command that modifies the state of the asset ledger.
    async fn execute(&self, command: LedgerCommand) -> AppResult<Asset> {
        let mut tx = self.pool.begin().await?;

        // 1. Resolve and expand commands (Handle Batch expansion but NOT BatchCreate)
        let commands_to_process = match &command {
            LedgerCommand::Batch(cmds) => cmds.clone(),
            // DO NOT MAP BatchCreate into CreateAsset!
            // It must be passed to execute_single as a single BatchCreate
            // so we don't trigger per-file lock-heavy checks.
            _ => vec![command.clone()],
        };

        // 2. Execute commands and collect results
        let mut results = Vec::new();
        for command_item in commands_to_process {
            let asset = self.execute_single(&mut tx, command_item.clone()).await?;
            results.push((asset, command_item));
        }

        tx.commit().await?;

        // 2.5 Post-commit Saga Execution (Filesystem Operations)
        for (asset, command_item) in &results {
            match command_item {
                LedgerCommand::DeleteAsset {
                    physical_delete: true,
                    path: Some(path_reference),
                    ..
                } => {
                    self.execute_saga_physical_delete(asset, path_reference).await;
                }
                LedgerCommand::MoveToTrash(_) => {
                    self.execute_saga_move_to_trash(asset).await;
                }
                LedgerCommand::RestoreFromTrash(_) => {
                    self.execute_saga_restore_from_trash(asset).await;
                }
                _ => {}
            }
        }

        // 3. Publish Domain Events only AFTER commit
        // For BatchCreate: emit a single summary to avoid flooding the broadcast channel.
        // Individual AssetCreated events would overflow the 2048-capacity buffer.
        match &command {
            LedgerCommand::BatchCreate(payloads) => {
                let batch_count = payloads.len();
                if batch_count > 0 {
                    let _ = self.event_bus.publish(DomainEvent::ScanProgress {
                        total: batch_count,
                        processed: batch_count,
                        current_file: format!("Batch committed: {} assets", batch_count),
                    });
                }
            }
            _ => {
                for (asset, command_item) in &results {
                    self.emit_event_for_command(asset, command_item)?;
                }
            }
        }

        // Return the last asset in the batch (or the only asset if not a batch)
        let last_asset = results.into_iter().last().map(|(a, _)| a).ok_or_else(|| {
            AppError::Internal("Transaction succeeded but no results were returned".to_string())
        })?;

        Ok(last_asset)
    }
}

impl SqliteAssetLedger {
    /// Internal helper to emit domain events after a successful commit.
    fn emit_event_for_command(&self, asset: &Asset, command: &LedgerCommand) -> AppResult<()> {
        match command {
            LedgerCommand::CreateAsset(_) | LedgerCommand::BatchCreate(_) => {
                self.event_bus.publish(DomainEvent::AssetCreated {
                    asset_id: asset.id.clone(),
                    path: asset.path.to_string_lossy().to_string(),
                    format: asset.format_type.clone(),
                })?;
            }
            LedgerCommand::UpdateTags(p) => {
                self.event_bus.publish(DomainEvent::AssetTagsUpdated {
                    asset_id: asset.id.clone(),
                    active_tags: p.tags_to_add.clone(),
                })?;
            }
            LedgerCommand::UpdateAsset(_) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: asset.id.clone(),
                })?;
            }
            LedgerCommand::DeleteAsset { .. } => {
                self.event_bus.publish(DomainEvent::AssetDeleted {
                    asset_id: asset.id.clone(),
                    folder_id: asset.folder_id.clone(),
                })?;
            }
            LedgerCommand::CreateFolder(_) => {
                self.event_bus.publish(DomainEvent::FolderCreated {
                    folder_id: asset.id.clone(),
                    parent_id: asset.folder_id.clone(),
                    name: asset.name.clone(),
                    path: asset.path.to_string_lossy().to_string(),
                })?;
            }
            LedgerCommand::SetAssetFolder { .. } => {
                self.event_bus.publish(DomainEvent::AssetFolderChanged {
                    asset_id: asset.id.clone(),
                    folder_id: asset.folder_id.clone(),
                })?;
            }
            LedgerCommand::RenameFolder(payload) => {
                self.event_bus.publish(DomainEvent::FolderMetadataUpdated {
                    folder_id: payload.folder_id.clone(),
                })?;
            }
            LedgerCommand::UpdateThumbnail { thumbnail_path, .. } => {
                self.event_bus.publish(DomainEvent::ThumbnailGenerated {
                    asset_id: asset.id.clone(),
                    path: thumbnail_path.clone(),
                    format: asset.format_type.clone(),
                })?;
            }
            LedgerCommand::UpdateAssetColors(_) => {
                self.event_bus.publish(DomainEvent::ExtractionCompleted {
                    asset_id: asset.id.clone(),
                    capability: "COLORS".to_string(),
                })?;
            }
            LedgerCommand::CreateTag(_) => {
                self.event_bus.publish(DomainEvent::TagCreated {
                    id: asset.id.clone(),
                    name: asset.name.clone(),
                })?;
            }
            LedgerCommand::UpdateTag(tag_payload) => {
                self.event_bus.publish(DomainEvent::TagUpdated {
                    id: tag_payload.id.clone(),
                })?;
            }
            LedgerCommand::DeleteTag { id } => {
                self.event_bus
                    .publish(DomainEvent::TagDeleted { id: id.clone() })?;
            }
            LedgerCommand::CreateSmartFolder(_) => {
                self.event_bus.publish(DomainEvent::SmartFolderCreated {
                    id: asset.id.clone(),
                    name: asset.name.clone(),
                })?;
            }
            LedgerCommand::UpdateSmartFolder(sf_payload) => {
                self.event_bus.publish(DomainEvent::SmartFolderUpdated {
                    id: sf_payload.id.clone(),
                })?;
            }
            LedgerCommand::DeleteSmartFolder(sf_payload) => {
                self.event_bus.publish(DomainEvent::SmartFolderDeleted {
                    id: sf_payload.id.clone(),
                })?;
            }
            LedgerCommand::UpdateAssetRating(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::UpdateAssetNotes(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::ToggleFavorite(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::MoveToTrash(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::RestoreFromTrash(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::UpdateFormat { asset_id, .. } => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: asset_id.clone(),
                })?;
            }
            LedgerCommand::UpdateTechnicalMetadata(p) => {
                self.event_bus.publish(DomainEvent::AssetMetadataUpdated {
                    asset_id: p.asset_id.clone(),
                })?;
            }
            LedgerCommand::ReextractColors { asset_id } => {
                self.event_bus.publish(DomainEvent::ReextractAssetColors {
                    asset_id: asset_id.clone(),
                })?;
            }
            LedgerCommand::RegenerateThumbnail { asset_id } => {
                self.event_bus.publish(DomainEvent::ThumbnailInvalidated {
                    asset_id: asset_id.clone(),
                })?;
            }
            LedgerCommand::AddTagsToAssetsBatch(p)
            | LedgerCommand::RemoveTagsFromAssetsBatch(p)
            | LedgerCommand::ReplaceTagsForAssetsBatch(p) => {
                for asset_id in &p.asset_ids {
                    self.event_bus.publish(DomainEvent::AssetTagsUpdated {
                        asset_id: asset_id.clone(),
                        active_tags: p.tag_ids.clone(),
                    })?;
                }
            }
            _ => {
                // Other commands without specific domain events
            }
        }
        Ok(())
    }

    /// Pure dispatch table: routes each `LedgerCommand` variant to the
    /// appropriate handler module. Contains no SQL or business logic.
    async fn execute_single(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        command: LedgerCommand,
    ) -> AppResult<Asset> {
        use crate::infra::database::handlers::{
            asset_handler, folder_handler, metadata_handler, smart_folder_handler, tags_handler,
            thumbnail_handler,
        };

        match command {
            LedgerCommand::CreateAsset(payload) => {
                asset_handler::handle_create(tx, payload).await
            }
            LedgerCommand::BatchCreate(payloads) => {
                asset_handler::handle_batch_create(tx, payloads).await
            }
            LedgerCommand::UpdateAsset(payload) => {
                asset_handler::handle_update_asset(tx, payload).await
            }
            LedgerCommand::MarkAsStale { asset_id } => {
                asset_handler::handle_mark_as_stale(tx, &asset_id).await
            }
            LedgerCommand::DeleteAsset {
                asset_id,
                path,
                physical_delete,
            } => {
                asset_handler::handle_delete_asset(tx, asset_id, path, physical_delete).await
            }
            LedgerCommand::SetAssetFolder {
                asset_id,
                folder_id,
            } => {
                asset_handler::handle_set_asset_folder(tx, &asset_id, folder_id.as_deref()).await
            }
            LedgerCommand::UpdateTags(payload) => {
                tags_handler::handle_update_tags(tx, payload).await
            }
            LedgerCommand::CreateTag(payload) => {
                tags_handler::handle_create_tag(tx, payload).await
            }
            LedgerCommand::UpdateTag(payload) => {
                tags_handler::handle_update_tag(tx, payload).await
            }
            LedgerCommand::DeleteTag { id } => {
                tags_handler::handle_delete_tag(tx, id).await
            }
            LedgerCommand::AddTagsToAssetsBatch(payload) => {
                tags_handler::handle_add_tags_to_assets_batch(tx, payload).await
            }
            LedgerCommand::RemoveTagsFromAssetsBatch(payload) => {
                tags_handler::handle_remove_tags_from_assets_batch(tx, payload).await
            }
            LedgerCommand::ReplaceTagsForAssetsBatch(payload) => {
                tags_handler::handle_replace_tags_for_assets_batch(tx, payload).await
            }
            LedgerCommand::CreateFolder(payload) => {
                folder_handler::handle_create_folder(tx, payload).await
            }
            LedgerCommand::RemoveFolder(payload) => {
                folder_handler::handle_remove_folder(tx, payload).await
            }
            LedgerCommand::RenameFolder(payload) => {
                folder_handler::handle_rename_folder(tx, payload).await
            }
            LedgerCommand::UpdateThumbnail {
                asset_id,
                thumbnail_path,
            } => {
                thumbnail_handler::handle_update_thumbnail(tx, &asset_id, &thumbnail_path).await
            }
            LedgerCommand::RegenerateThumbnail { asset_id } => {
                thumbnail_handler::handle_regenerate_thumbnail(tx, &asset_id).await
            }
            LedgerCommand::UpdateAssetColors(payload) => {
                metadata_handler::handle_update_asset_colors(tx, payload).await
            }
            LedgerCommand::UpdateAssetRating(payload) => {
                metadata_handler::handle_update_rating(tx, payload).await
            }
            LedgerCommand::UpdateAssetNotes(payload) => {
                metadata_handler::handle_update_notes(tx, payload).await
            }
            LedgerCommand::ToggleFavorite(payload) => {
                metadata_handler::handle_toggle_favorite(tx, payload).await
            }
            LedgerCommand::MoveToTrash(payload) => {
                asset_handler::handle_move_to_trash(tx, payload).await
            }
            LedgerCommand::RestoreFromTrash(payload) => {
                asset_handler::handle_restore_from_trash(tx, payload).await
            }
            LedgerCommand::UpdateFormat { asset_id, format } => {
                metadata_handler::handle_update_format(tx, &asset_id, &format).await
            }
            LedgerCommand::UpdateTechnicalMetadata(payload) => {
                metadata_handler::handle_update_technical_metadata(tx, payload).await
            }
            LedgerCommand::ReextractColors { asset_id } => {
                metadata_handler::handle_reextract_colors(tx, &asset_id).await
            }
            LedgerCommand::CreateSmartFolder(payload) => {
                smart_folder_handler::handle_create_smart_folder(tx, payload).await
            }
            LedgerCommand::UpdateSmartFolder(payload) => {
                smart_folder_handler::handle_update_smart_folder(tx, payload).await
            }
            LedgerCommand::DeleteSmartFolder(payload) => {
                smart_folder_handler::handle_delete_smart_folder(tx, payload).await
            }
            LedgerCommand::Batch(_) => Err(AppError::Internal(
                "Nested Batch commands are not supported".to_string(),
            )),
        }
    }

    /// Saga post-commit step: physically deletes a file from the filesystem.
    ///
    /// Mirrors the same PENDING→COMPLETED/FAILED lifecycle used by all Saga
    /// operations. If the file is already missing, the operation is considered
    /// successful (idempotent delete).
    async fn execute_saga_physical_delete(&self, asset: &Asset, path_reference: &Path) {
        let filesystem_result = tokio::fs::remove_file(path_reference).await;
        match filesystem_result {
            Ok(_) => {
                tracing::info!(
                    "Ledger: Physical delete SUCCESS for {}",
                    path_reference.display()
                );
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "DELETE_ASSET",
                    "COMPLETED",
                    None,
                )
                .await;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(
                    "Ledger: Physical file already missing for {}",
                    path_reference.display()
                );
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "DELETE_ASSET",
                    "COMPLETED",
                    None,
                )
                .await;
            }
            Err(error) => {
                tracing::warn!(
                    "Ledger: Physical delete FAILED for {}: {}",
                    path_reference.display(),
                    error
                );
                let error_message = error.to_string();
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "DELETE_ASSET",
                    "FAILED",
                    Some(&error_message),
                )
                .await;
            }
        }
    }

    /// Saga post-commit step: physically moves a file into the trash directory.
    ///
    /// After the DB transaction has set `deleted_at`, this method moves the
    /// physical file into `{app_data}/trash/` using the centralized path logic
    /// from `core::trash`. This eliminates the previously duplicated FS move
    /// code from both `delivery/mutations.rs` and `delivery/duplicates.rs`.
    ///
    /// The trash directory is created on-demand if it does not exist.
    async fn execute_saga_move_to_trash(&self, asset: &Asset) {
        let Some(deleted_at) = asset.deleted_at else {
            tracing::warn!(
                "Ledger: MoveToTrash Saga skipped for {} — deleted_at is None after commit",
                asset.id
            );
            return;
        };

        let trash_directory = crate::core::trash::trash_directory(&self.app_data_directory);
        if !trash_directory.exists() {
            if let Err(directory_creation_error) = std::fs::create_dir_all(&trash_directory) {
                tracing::error!(
                    "Ledger: Failed to create trash directory {}: {}",
                    trash_directory.display(),
                    directory_creation_error
                );
                let error_message = directory_creation_error.to_string();
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "MOVE_TO_TRASH",
                    "FAILED",
                    Some(&error_message),
                )
                .await;
                return;
            }
        }

        let Some(trash_path) = crate::core::trash::build_trash_path(
            &self.app_data_directory,
            &asset.id,
            &asset.path,
            &deleted_at,
        ) else {
            tracing::warn!(
                "Ledger: MoveToTrash Saga skipped for {} — could not build trash path from {:?}",
                asset.id,
                asset.path
            );
            return;
        };

        if !asset.path.exists() {
            tracing::info!(
                "Ledger: MoveToTrash — source file already missing for {}",
                asset.id
            );
            let _ = crate::infra::database::handlers::shared::update_operation_status(
                &self.pool,
                &asset.id,
                "MOVE_TO_TRASH",
                "COMPLETED",
                None,
            )
            .await;
            return;
        }

        match tokio::fs::rename(&asset.path, &trash_path).await {
            Ok(_) => {
                tracing::info!(
                    "Ledger: MoveToTrash SUCCESS for {} → {}",
                    asset.id,
                    trash_path.display()
                );
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "MOVE_TO_TRASH",
                    "COMPLETED",
                    None,
                )
                .await;
            }
            Err(error) => {
                tracing::warn!(
                    "Ledger: MoveToTrash FAILED for {}: {}",
                    asset.id,
                    error
                );
                let error_message = error.to_string();
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "MOVE_TO_TRASH",
                    "FAILED",
                    Some(&error_message),
                )
                .await;
            }
        }
    }

    /// Saga post-commit step: physically restores a file from the trash directory
    /// back to its original path.
    ///
    /// After the DB transaction has cleared `deleted_at`, this method moves the
    /// file from `{app_data}/trash/` back to the original asset path. Uses the
    /// centralized `core::trash::resolve_physical_path` which handles both the
    /// timestamped format and the legacy naming convention.
    async fn execute_saga_restore_from_trash(&self, asset: &Asset) {
        let original_path = asset.path.clone();

        // After RestoreFromTrash handler clears deleted_at, the asset no longer
        // has deleted_at set. We need to look up the most recent MOVE_TO_TRASH
        // audit log to find where the file was placed. However, the handler
        // already returned the asset with deleted_at = None. We use
        // resolve_physical_path with a re-constructed "trashed" state to find
        // the file in the trash directory.
        //
        // Since deleted_at has been cleared, we scan the trash directory for the
        // file matching this asset's ID pattern instead.
        let trash_directory = crate::core::trash::trash_directory(&self.app_data_directory);
        let asset_id_prefix = format!("{}_", asset.id);

        let trash_source_path = match std::fs::read_dir(&trash_directory) {
            Ok(entries) => {
                let mut found_path: Option<PathBuf> = None;
                for entry_result in entries {
                    if let Ok(entry) = entry_result {
                        if entry.file_name().to_string_lossy().starts_with(&asset_id_prefix) {
                            found_path = Some(entry.path());
                            break;
                        }
                    }
                }
                found_path
            }
            Err(_) => None,
        };

        let Some(source_path) = trash_source_path else {
            tracing::info!(
                "Ledger: RestoreFromTrash — file not found in trash for {} (may have been already restored)",
                asset.id
            );
            let _ = crate::infra::database::handlers::shared::update_operation_status(
                &self.pool,
                &asset.id,
                "RESTORE_FROM_TRASH",
                "COMPLETED",
                None,
            )
            .await;
            return;
        };

        if source_path == original_path {
            tracing::info!(
                "Ledger: RestoreFromTrash — source and target are the same for {}",
                asset.id
            );
            let _ = crate::infra::database::handlers::shared::update_operation_status(
                &self.pool,
                &asset.id,
                "RESTORE_FROM_TRASH",
                "COMPLETED",
                None,
            )
            .await;
            return;
        }

        match tokio::fs::rename(&source_path, &original_path).await {
            Ok(_) => {
                tracing::info!(
                    "Ledger: RestoreFromTrash SUCCESS for {} → {}",
                    asset.id,
                    original_path.display()
                );
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "RESTORE_FROM_TRASH",
                    "COMPLETED",
                    None,
                )
                .await;
            }
            Err(error) => {
                tracing::warn!(
                    "Ledger: RestoreFromTrash FAILED for {}: {}",
                    asset.id,
                    error
                );
                let error_message = error.to_string();
                let _ = crate::infra::database::handlers::shared::update_operation_status(
                    &self.pool,
                    &asset.id,
                    "RESTORE_FROM_TRASH",
                    "FAILED",
                    Some(&error_message),
                )
                .await;
            }
        }
    }
}
