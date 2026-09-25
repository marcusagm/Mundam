use crate::core::error::AppResult;
use crate::core::repository::DuplicatesRepository;
use crate::core::TransactionalAssetLedger;
use crate::core::models::{DuplicateResolution, DuplicateResolutionAction, MetadataMergePayload};
use crate::core::ledger::command::{
    LedgerCommand, UpdateAssetRatingPayload, UpdateAssetNotesPayload,
    UpdateTagsPayload, UpdateTechnicalMetadataPayload, ToggleFavoritePayload,
};
use crate::core::events::payloads::DomainEvent;
use crate::core::events::bus::AppEventBus;
use crate::core::repository::AssetQueryHandler;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use std::sync::Mutex;

/// Command handler for duplicate operations.
pub struct DuplicateCommandService {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    ledger: Arc<dyn TransactionalAssetLedger>,
    event_bus: Arc<dyn AppEventBus>,
    asset_query_handler: Arc<dyn AssetQueryHandler>,
    scan_cancel_token: Mutex<Option<CancellationToken>>,
}

impl DuplicateCommandService {
    /// Creates a new DuplicateCommandService.
    ///
    /// # Arguments
    /// * `duplicates_repo` - The duplicate repository port.
    /// * `ledger` - The asset ledger for atomic asset mutations.
    /// * `event_bus` - The application event bus.
    /// * `asset_query_handler` - The asset query handler for reading current asset state.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        ledger: Arc<dyn TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
        asset_query_handler: Arc<dyn AssetQueryHandler>,
    ) -> Self {
        Self {
            duplicates_repo,
            ledger,
            event_bus,
            asset_query_handler,
            scan_cancel_token: Mutex::new(None),
        }
    }

    /// Resolves a duplicate group with a given action.
    ///
    /// The actual asset mutations (e.g. deleting assets) are delegated to the Asset Ledger
    /// to ensure transactional integrity and proper event publishing for the broader system.
    ///
    /// # Arguments
    /// * `group_id` - The unique ID of the duplicate group.
    /// * `action` - The resolution action chosen by the user.
    /// * `kept_asset_ids` - Optional list of asset IDs to keep.
    ///
    /// # Errors
    /// Returns `AppError::Database` if the resolution fails to be saved.
    pub async fn resolve_group(
        &self,
        group_id: &str,
        action: DuplicateResolutionAction,
        kept_asset_ids: Option<Vec<String>>,
    ) -> AppResult<()> {
        let selected_asset_id = kept_asset_ids.as_ref().and_then(|ids| ids.first().cloned());

        let resolution = DuplicateResolution {
            id: uuid::Uuid::new_v4().to_string(),
            group_id: group_id.to_string(),
            action: action.clone(),
            selected_asset_id,
            payload: kept_asset_ids.as_ref().map(|ids| serde_json::json!({ "kept_ids": ids }).to_string()),
            resolved_by: Some("user".to_string()),
            resolved_at: chrono::Utc::now(),
        };

        self.duplicates_repo.save_resolution(resolution).await?;

        let final_status = match action {
            DuplicateResolutionAction::IgnoreGroup => "ignored",
            _ => "resolved",
        };

        self.duplicates_repo.update_group_status(group_id, final_status).await?;

        if matches!(action, DuplicateResolutionAction::CustomSelection) {
            if let Some(kept_ids) = &kept_asset_ids {
                let candidates = self.duplicates_repo.get_group_candidates(group_id).await?;
                for candidate in candidates {
                    if !kept_ids.contains(&candidate.asset_id) {
                        let _ = self.ledger.execute(LedgerCommand::MoveToTrash(
                            crate::core::ledger::command::MoveToTrashPayload {
                                asset_id: candidate.asset_id.clone(),
                            }
                        )).await;
                    }
                }
            }
        }

        let _ = self.event_bus.publish(DomainEvent::DuplicateGroupResolved {
            group_id: group_id.to_string(),
            action: action.to_string(),
        });

        Ok(())
    }

    /// Applies a user-confirmed metadata merge to one or more kept assets.
    ///
    /// This is an explicit, user-initiated action separate from `resolve_group`.
    /// Each field in the payload that is `Some(...)` is applied to all kept assets
    /// via the Asset Ledger. Fields that are `None` are left untouched.
    ///
    /// `is_favorite` requires reading the current asset state to decide whether a
    /// toggle is needed, since the Ledger exposes a toggle — not a setter.
    ///
    /// # Arguments
    /// * `kept_asset_ids` - The IDs of the assets that will receive the merged metadata.
    /// * `payload` - The merge decisions confirmed by the user in the UI.
    ///
    /// # Errors
    /// Returns `AppError::Database` if any ledger command fails.
    pub async fn apply_metadata_merge(
        &self,
        kept_asset_ids: &[String],
        payload: MetadataMergePayload,
    ) -> AppResult<()> {
        for kept_id in kept_asset_ids {
            let mut ledger_commands: Vec<LedgerCommand> = Vec::new();

            if let Some(rating) = payload.rating {
                ledger_commands.push(LedgerCommand::UpdateAssetRating(UpdateAssetRatingPayload {
                    asset_id: kept_id.clone(),
                    rating,
                }));
            }

            if let Some(notes) = &payload.notes {
                ledger_commands.push(LedgerCommand::UpdateAssetNotes(UpdateAssetNotesPayload {
                    asset_id: kept_id.clone(),
                    notes: notes.as_str().to_owned(),
                }));
            }

            if !payload.tags_to_add.is_empty() {
                ledger_commands.push(LedgerCommand::UpdateTags(UpdateTagsPayload {
                    asset_id: kept_id.clone(),
                    tags_to_add: payload.tags_to_add.clone(),
                    tags_to_remove: vec![],
                }));
            }

            if let Some(technical_payload) = &payload.technical_payload_override {
                ledger_commands.push(LedgerCommand::UpdateTechnicalMetadata(
                    UpdateTechnicalMetadataPayload {
                        asset_id: kept_id.clone(),
                        width: None,
                        height: None,
                        duration_secs: None,
                        technical_payload: Some(technical_payload.clone() as serde_json::Value),
                        semantic_payload: None,
                    },
                ));
            }

            if !ledger_commands.is_empty() {
                self.ledger.execute(LedgerCommand::Batch(ledger_commands)).await?;
            }

            // Toggle favorite only if the desired state differs from current state.
            // Handled separately because LedgerCommand::ToggleFavorite is a toggle, not a setter.
            if let Some(desired_favorite) = payload.is_favorite {
                let current_asset = self.asset_query_handler.get_by_id(kept_id).await?;
                if let Some(asset) = current_asset {
                    if asset.is_favorite != desired_favorite {
                        self.ledger.execute(LedgerCommand::ToggleFavorite(ToggleFavoritePayload {
                            asset_id: kept_id.clone(),
                        })).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Triggers a scan to find new duplicates.
    ///
    /// Emits `DuplicateScanProgressed` with granular per-asset progress during
    /// the rehash phase via an internal mpsc channel. The event includes the
    /// `total` field so the frontend can render a real percentage progress bar.
    ///
    /// # Errors
    /// Returns `AppError::DatabaseError` if the scan fails.
    pub async fn start_duplicate_scan(&self) -> AppResult<()> {
        let token = CancellationToken::new();
        {
            let mut guard = self.scan_cancel_token.lock().unwrap_or_else(|poison| poison.into_inner());
            if let Some(old_token) = guard.replace(token.clone()) {
                old_token.cancel();
            }
        }

        let (progress_sender, mut progress_receiver) =
            tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();

        let event_bus_clone = self.event_bus.clone();
        tokio::spawn(async move {
            while let Some((processed, total)) = progress_receiver.recv().await {
                let _ = event_bus_clone.publish(DomainEvent::DuplicateScanProgressed {
                    processed,
                    matched: 0,
                    groups_created: 0,
                    total,
                });
            }
        });

        self.duplicates_repo.backfill_missing_fingerprints().await?;

        let rehashed = self
            .duplicates_repo
            .rehash_pending_fingerprints(Some(token.clone()), Some(progress_sender))
            .await?;

        tracing::info!(
            "DuplicateCommandService: rehashed {} pending fingerprints before scan",
            rehashed
        );

        self.duplicates_repo.run_exact_match_scan(Some(token.clone())).await?;
        self.duplicates_repo.run_visual_match_scan(Some(token.clone())).await?;

        let _ = self.event_bus.publish(DomainEvent::DuplicateScanFinished {
            groups_created: 0,
        });

        let mut guard = self.scan_cancel_token.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(current) = guard.as_ref() {
            if current.is_cancelled() {
                *guard = None;
            }
        }

        Ok(())
    }

    pub fn cancel_duplicate_scan(&self) {
        let mut guard = self.scan_cancel_token.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(token) = guard.take() {
            token.cancel();
            tracing::info!("DuplicateCommandService: cancelled ongoing scan");
        }
    }

    /// Updates a rule set in the database.
    pub async fn update_rule_set(&self, rule_set: crate::core::models::DuplicateRuleSet) -> AppResult<()> {
        self.duplicates_repo.save_rule_set(rule_set).await
    }
}
