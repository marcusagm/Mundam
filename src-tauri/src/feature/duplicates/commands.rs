use crate::core::error::AppResult;
use crate::core::events::bus::AppEventBus;
use crate::core::events::payloads::DomainEvent;
use crate::core::formats::FormatRegistry;
use crate::core::ledger::command::{
    LedgerCommand, ToggleFavoritePayload, UpdateAssetNotesPayload, UpdateAssetRatingPayload,
    UpdateTagsPayload, UpdateTechnicalMetadataPayload,
};
use crate::core::models::{DuplicateResolution, DuplicateResolutionAction, MetadataMergePayload};
use crate::core::repository::AssetQueryHandler;
use crate::core::repository::DuplicatesRepository;
use crate::core::TransactionalAssetLedger;
use std::sync::Arc;

use std::sync::Mutex;
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Represents the current runtime status of a manual duplicate scan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateScanStatus {
    /// Identity fencing progress from previous executions.
    pub run_id: Option<String>,
    pub is_running: bool,
    pub phase: String,
    pub processed: usize,
    pub total: usize,
}

impl Default for DuplicateScanStatus {
    fn default() -> Self {
        Self {
            run_id: None,
            is_running: false,
            phase: "idle".to_string(),
            processed: 0,
            total: 0,
        }
    }
}

/// RAII guard ensuring `is_running` is always reset to `false` when the scan scope exits.
struct ScanRunningGuard {
    status: Arc<RwLock<DuplicateScanStatus>>,
    run_id: String,
    event_bus: Arc<dyn AppEventBus>,
    token: CancellationToken,
    completed: bool,
}

impl Drop for ScanRunningGuard {
    fn drop(&mut self) {
        let mut owns_status = false;
        if let Ok(mut status_guard) = self.status.write() {
            if status_guard.run_id.as_deref() == Some(self.run_id.as_str()) {
                owns_status = true;
                status_guard.is_running = false;
                status_guard.phase = "idle".to_string();
                status_guard.run_id = None;
            }
        }
        if owns_status && !self.completed {
            let _ = self.event_bus.publish(DomainEvent::DuplicateScanStopped {
                run_id: self.run_id.clone(),
                canceled: self.token.is_cancelled(),
            });
        }
    }
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod processing_tests;

/// Command handler for duplicate operations.
#[derive(Clone)]
pub struct DuplicateCommandService {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    ledger: Arc<dyn TransactionalAssetLedger>,
    event_bus: Arc<dyn AppEventBus>,
    asset_query_handler: Arc<dyn AssetQueryHandler>,
    fingerprint_processor: super::fingerprint_processor::FingerprintProcessor,
    scan_cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    scan_status: Arc<RwLock<DuplicateScanStatus>>,
    processing_coordinator: Arc<super::coordinator::DuplicateProcessingCoordinator>,
    processing_control: Arc<crate::core::processing::control::ProcessingControl>,
}

impl crate::core::processing::control::ManualScanActivity for DuplicateCommandService {
    fn snapshot(&self) -> crate::core::processing::control::ManualScanProgress {
        let status = self
            .scan_status
            .read()
            .unwrap_or_else(|error| error.into_inner());
        crate::core::processing::control::ManualScanProgress {
            is_running: status.is_running,
            phase: status.phase.clone(),
            processed: status.processed,
            total: status.total,
        }
    }
}

impl DuplicateCommandService {
    /// Creates a new DuplicateCommandService.
    ///
    /// # Arguments
    /// * `duplicates_repo` - The duplicate repository port.
    /// * `ledger` - The asset ledger for atomic asset mutations.
    /// * `event_bus` - The application event bus.
    /// * `asset_query_handler` - The asset query handler for reading current asset state.
    /// * `format_registry` - The format registry for resolving exotic format providers.
    /// * `processing` - Shared durable eligibility and retry outcomes for fingerprints.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        ledger: Arc<dyn TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
        asset_query_handler: Arc<dyn AssetQueryHandler>,
        format_registry: Arc<FormatRegistry>,
        processing: Arc<dyn crate::core::processing::ProcessingRepository>,
    ) -> Self {
        let fingerprint_processor = super::fingerprint_processor::FingerprintProcessor::new(
            ledger.clone(),
            processing,
            asset_query_handler.clone(),
            format_registry,
        );
        Self {
            duplicates_repo,
            ledger,
            event_bus,
            asset_query_handler,
            fingerprint_processor,
            scan_cancel_token: Arc::new(Mutex::new(None)),
            scan_status: Arc::new(RwLock::new(DuplicateScanStatus::default())),
            processing_coordinator: Arc::new(
                super::coordinator::DuplicateProcessingCoordinator::default(),
            ),
            processing_control: Arc::new(
                crate::core::processing::control::ProcessingControl::default(),
            ),
        }
    }

    /// Shares scan admission with automatic hashing and matching.
    pub fn with_processing_coordinator(
        mut self,
        coordinator: Arc<super::coordinator::DuplicateProcessingCoordinator>,
    ) -> Self {
        self.fingerprint_processor = self
            .fingerprint_processor
            .with_budget(coordinator.budget.clone());
        self.processing_coordinator = coordinator;
        self
    }

    /// Shares session pause intent with automatic duplicate processing.
    pub fn with_processing_control(
        mut self,
        control: Arc<crate::core::processing::control::ProcessingControl>,
    ) -> Self {
        self.processing_control = control;
        self
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
        tracing::info!(
            "DuplicateCommandService: resolving duplicate group {} with action {:?}, kept_asset_ids={:?}",
            group_id,
            action,
            kept_asset_ids
        );

        let selected_asset_id = kept_asset_ids.as_ref().and_then(|ids| ids.first().cloned());
        let mut trashed_asset_identifiers: Vec<String> = Vec::new();

        if matches!(action, DuplicateResolutionAction::CustomSelection) {
            if let Some(kept_ids) = &kept_asset_ids {
                let candidates = self.duplicates_repo.get_group_candidates(group_id).await?;
                for candidate in candidates {
                    if !kept_ids.contains(&candidate.asset_id) {
                        let _ = self
                            .ledger
                            .execute(LedgerCommand::MoveToTrash(
                                crate::core::ledger::command::MoveToTrashPayload {
                                    asset_id: candidate.asset_id.clone(),
                                },
                            ))
                            .await;
                        trashed_asset_identifiers.push(candidate.asset_id.clone());
                    }
                }
            }
        }

        let resolution = DuplicateResolution {
            id: uuid::Uuid::new_v4().to_string(),
            group_id: group_id.to_string(),
            action: action.clone(),
            selected_asset_id,
            payload: kept_asset_ids.as_ref().map(|ids| {
                serde_json::json!({
                    "kept_ids": ids,
                    "trashed_ids": trashed_asset_identifiers
                })
                .to_string()
            }),
            resolved_by: Some("user".to_string()),
            resolved_at: chrono::Utc::now(),
        };

        self.duplicates_repo.save_resolution(resolution).await?;

        let final_status = match action {
            DuplicateResolutionAction::IgnoreGroup => "ignored",
            _ => "resolved",
        };

        self.duplicates_repo
            .update_group_status(group_id, final_status)
            .await?;

        let _ = self.event_bus.publish(DomainEvent::DuplicateGroupResolved {
            group_id: group_id.to_string(),
            action: action.to_string(),
        });

        tracing::info!(
            "DuplicateCommandService: successfully resolved duplicate group {} as '{}' (trashed {} asset(s))",
            group_id,
            final_status,
            trashed_asset_identifiers.len()
        );

        Ok(())
    }

    /// Reverts a duplicate group resolution, restoring any trashed assets and resetting status to open.
    ///
    /// Inspects the most recent resolution record for the specified duplicate group. If the group
    /// was resolved by sending assets to the trash, each trashed asset is restored back to the library
    /// via `LedgerCommand::RestoreFromTrash`. The resolution record is then removed and the group
    /// status is returned to `open`.
    ///
    /// # Arguments
    /// * `group_id` - The unique identifier of the duplicate group to revert.
    ///
    /// # Errors
    /// Returns `AppError::Database` if reading or mutating the database fails.
    pub async fn undo_duplicate_resolution(&self, group_id: &str) -> AppResult<()> {
        tracing::info!(
            "DuplicateCommandService: undoing duplicate resolution for group {}",
            group_id
        );
        let optional_latest_resolution =
            self.duplicates_repo.get_latest_resolution(group_id).await?;

        if let Some(latest_resolution) = optional_latest_resolution {
            let mut asset_identifiers_to_restore: Vec<String> = Vec::new();

            if let Some(payload_string) = &latest_resolution.payload {
                if let Ok(parsed_payload) =
                    serde_json::from_str::<serde_json::Value>(payload_string)
                {
                    if let Some(trashed_identifiers_array) = parsed_payload
                        .get("trashed_ids")
                        .and_then(|array_value| array_value.as_array())
                    {
                        for item in trashed_identifiers_array {
                            if let Some(asset_identifier_string) = item.as_str() {
                                asset_identifiers_to_restore
                                    .push(asset_identifier_string.to_string());
                            }
                        }
                    } else if let Some(kept_identifiers_array) = parsed_payload
                        .get("kept_ids")
                        .and_then(|array_value| array_value.as_array())
                    {
                        let kept_identifiers_set: std::collections::HashSet<String> =
                            kept_identifiers_array
                                .iter()
                                .filter_map(|item| {
                                    item.as_str().map(|identifier| identifier.to_string())
                                })
                                .collect();

                        let group_candidates =
                            self.duplicates_repo.get_group_candidates(group_id).await?;
                        for candidate in group_candidates {
                            if !kept_identifiers_set.contains(&candidate.asset_id) {
                                asset_identifiers_to_restore.push(candidate.asset_id);
                            }
                        }
                    }
                }
            }

            for asset_identifier in &asset_identifiers_to_restore {
                let restore_result = self
                    .ledger
                    .execute(LedgerCommand::RestoreFromTrash(
                        crate::core::ledger::command::RestoreFromTrashPayload {
                            asset_id: asset_identifier.clone(),
                        },
                    ))
                    .await;

                if let Err(ledger_error) = restore_result {
                    tracing::warn!(
                        "undo_duplicate_resolution: failed to restore asset {} from trash: {:?}",
                        asset_identifier,
                        ledger_error
                    );
                }
            }

            self.duplicates_repo
                .delete_resolution(&latest_resolution.id)
                .await?;
            tracing::info!(
                "DuplicateCommandService: restored {} asset(s) from trash for group {}",
                asset_identifiers_to_restore.len(),
                group_id
            );
        }

        self.duplicates_repo
            .update_group_status(group_id, "open")
            .await?;

        let _ = self
            .event_bus
            .publish(DomainEvent::DuplicateResolutionUndone {
                group_id: group_id.to_string(),
            });

        tracing::info!(
            "DuplicateCommandService: successfully reverted group {} to 'open'",
            group_id
        );

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
        tracing::info!(
            "DuplicateCommandService: applying metadata merge for {} kept asset(s): {:?}",
            kept_asset_ids.len(),
            kept_asset_ids
        );

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
                self.ledger
                    .execute(LedgerCommand::Batch(ledger_commands))
                    .await?;
            }

            // Toggle favorite only if the desired state differs from current state.
            // Handled separately because LedgerCommand::ToggleFavorite is a toggle, not a setter.
            if let Some(desired_favorite) = payload.is_favorite {
                let current_asset = self.asset_query_handler.get_by_id(kept_id).await?;
                if let Some(asset) = current_asset {
                    if asset.is_favorite != desired_favorite {
                        self.ledger
                            .execute(LedgerCommand::ToggleFavorite(ToggleFavoritePayload {
                                asset_id: kept_id.clone(),
                            }))
                            .await?;
                    }
                }
            }
        }

        tracing::info!(
            "DuplicateCommandService: successfully applied metadata merge for {} kept asset(s)",
            kept_asset_ids.len()
        );

        Ok(())
    }

    /// Triggers a scan to find new duplicates.
    ///
    /// Reconciles latest-value hashing progress through the shared processing snapshot.
    /// No per-asset progress events or unbounded progress queue are created.
    ///
    /// # Errors
    /// Returns `AppError::DatabaseError` if the scan fails.
    pub async fn start_duplicate_scan(&self) -> AppResult<()> {
        self.run_duplicate_scan(false).await
    }

    async fn run_duplicate_scan(&self, rebuild: bool) -> AppResult<()> {
        self.ensure_processing_resumed()?;
        if self
            .scan_status
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .is_running
        {
            return Ok(());
        }
        if self.duplicates_repo.matching_status().await?.manual_pending {
            return Err(crate::core::AppError::ValidationFailed(
                "A manual duplicate comparison is already pending".into(),
            ));
        }
        tracing::info!("DuplicateCommandService: start_duplicate_scan requested");

        let cancellation_token = CancellationToken::new();
        let run_id = uuid::Uuid::new_v4().to_string();
        {
            let mut cancellation_guard = self
                .scan_cancel_token
                .lock()
                .unwrap_or_else(|poison_error| poison_error.into_inner());
            let mut status_guard = self
                .scan_status
                .write()
                .unwrap_or_else(|poison_error| poison_error.into_inner());
            if status_guard.is_running {
                tracing::warn!("DuplicateCommandService: scan already in progress, ignoring duplicate start request");
                return Ok(());
            }
            *cancellation_guard = Some(cancellation_token.clone());
            status_guard.run_id = Some(run_id.clone());
            status_guard.is_running = true;
            status_guard.phase = "preparing".to_string();
            status_guard.processed = 0;
            status_guard.total = 0;
        }

        let mut scan_running_guard = ScanRunningGuard {
            status: self.scan_status.clone(),
            run_id: run_id.clone(),
            event_bus: self.event_bus.clone(),
            token: cancellation_token.clone(),
            completed: false,
        };

        let _admission = tokio::select! {
            _ = cancellation_token.cancelled() => return Ok(()),
            admission = self.processing_coordinator.acquire() => admission,
        };
        let activity = self
            .processing_control
            .try_admit(
                crate::core::processing::control::WorkerKind::Duplicates,
                "manual_scan",
                None,
            )
            .ok_or_else(|| {
                crate::core::AppError::ValidationFailed(
                    "Resume duplicate processing before starting a scan".into(),
                )
            })?;

        if rebuild {
            self.duplicates_repo.reset_duplicate_index().await?;
        }

        tracing::info!("DuplicateCommandService: [1/4] backfilling missing fingerprints");
        self.duplicates_repo.backfill_missing_fingerprints().await?;

        let (progress_sender, progress_receiver) = tokio::sync::watch::channel((0, 0));
        let progress_handle = tokio::spawn(super::scan_progress::reconcile(
            progress_receiver,
            self.scan_status.clone(),
            run_id.clone(),
        ));

        tracing::info!("DuplicateCommandService: [2/4] rehashing pending fingerprints");
        let rehash_result = self
            .fingerprint_processor
            .rehash_pending(&cancellation_token, Some(progress_sender))
            .await;
        let _ = progress_handle.await;
        let rehashed_count = rehash_result?;

        tracing::info!(
            "DuplicateCommandService: [2/4] rehashed {} pending fingerprints before scan",
            rehashed_count
        );

        if cancellation_token.is_cancelled() {
            tracing::info!("DuplicateCommandService: scan cancelled after rehashing");
            return Ok(());
        }

        self.duplicates_repo.request_matching(&run_id).await?;
        if cancellation_token.is_cancelled() {
            self.duplicates_repo.cancel_matching(Some(&run_id)).await?;
            return Ok(());
        }

        {
            if let Ok(mut status_guard) = self.scan_status.write() {
                status_guard.phase = "exact_matching".to_string();
            }
        }
        tracing::info!("DuplicateCommandService: [3/4] running exact match scan");
        activity.set_phase("exact_matching");
        let exact_groups_count = super::matching_processor::complete(
            self.duplicates_repo.clone(),
            self.processing_coordinator.clone(),
            crate::core::duplicates::matching::MatchingKind::Exact,
            &cancellation_token,
        )
        .await?;

        if cancellation_token.is_cancelled() {
            tracing::info!("DuplicateCommandService: scan cancelled after exact matching");
            self.duplicates_repo.cancel_matching(Some(&run_id)).await?;
            return Ok(());
        }

        {
            if let Ok(mut status_guard) = self.scan_status.write() {
                status_guard.phase = "visual_matching".to_string();
            }
        }
        tracing::info!("DuplicateCommandService: [4/4] running visual match scan");
        activity.set_phase("visual_matching");
        let visual_groups_count = super::matching_processor::complete(
            self.duplicates_repo.clone(),
            self.processing_coordinator.clone(),
            crate::core::duplicates::matching::MatchingKind::Visual,
            &cancellation_token,
        )
        .await?;

        if cancellation_token.is_cancelled() {
            self.duplicates_repo.cancel_matching(Some(&run_id)).await?;
            return Ok(());
        }

        scan_running_guard.completed = true;
        let _ = self.event_bus.publish(DomainEvent::DuplicateScanFinished {
            groups_created: exact_groups_count + visual_groups_count,
            publication_sequence: self
                .duplicates_repo
                .matching_status()
                .await
                .unwrap_or_default()
                .publication_sequence,
        });

        tracing::info!("DuplicateCommandService: full duplicate scan completed successfully");

        let mut cancellation_guard = self
            .scan_cancel_token
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner());
        if let Some(current_token) = cancellation_guard.as_ref() {
            if current_token.is_cancelled() {
                *cancellation_guard = None;
            }
        }

        Ok(())
    }

    /// Returns the current runtime status of the duplicate scan.
    pub fn get_scan_status(&self) -> DuplicateScanStatus {
        self.scan_status
            .read()
            .map(|status_guard| status_guard.clone())
            .unwrap_or_default()
    }

    /// Requests cooperative exit and persists cancellation of recovered comparisons.
    ///
    /// # Errors
    /// Returns a storage error if durable cancellation cannot be acknowledged.
    pub async fn cancel_duplicate_scan(&self) -> AppResult<()> {
        self.processing_coordinator.cancel_manual_matching();
        let mut local_preparation = false;
        {
            let mut cancellation_guard = self
                .scan_cancel_token
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(token) = cancellation_guard.take() {
                token.cancel();
            }
            if let Ok(mut status) = self.scan_status.write() {
                if status.is_running {
                    local_preparation = matches!(status.phase.as_str(), "preparing" | "hashing");
                    status.phase = "cancel_requested".into();
                }
            }
        }
        if local_preparation {
            return Ok(());
        }
        self.duplicates_repo.cancel_matching(None).await
    }

    /// Updates a rule set in the database.
    pub async fn update_rule_set(
        &self,
        rule_set: crate::core::models::DuplicateRuleSet,
    ) -> AppResult<()> {
        tracing::info!(
            "DuplicateCommandService: updating duplicate rule set id='{}', name='{}'",
            rule_set.id,
            rule_set.name
        );
        self.duplicates_repo.save_rule_set(rule_set).await
    }

    /// Erases all existing duplicate index tables (groups, candidates, resolutions, fingerprints)
    /// and triggers a fresh full duplicate scan across the entire library.
    ///
    /// # Errors
    /// Returns `AppError::Database` if resetting or scanning fails.
    pub async fn rebuild_duplicate_index(&self) -> AppResult<()> {
        self.run_duplicate_scan(true).await
    }

    fn ensure_processing_resumed(&self) -> AppResult<()> {
        if self
            .processing_control
            .is_paused(crate::core::processing::control::WorkerKind::Duplicates)
        {
            return Err(crate::core::AppError::ValidationFailed(
                "Resume duplicate processing before starting a scan".into(),
            ));
        }
        Ok(())
    }
}
