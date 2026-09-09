use crate::core::error::AppResult;
use crate::core::repository::DuplicatesRepository;
use crate::core::TransactionalAssetLedger;
use crate::core::models::{DuplicateResolution, DuplicateResolutionAction};
use crate::core::events::payloads::DomainEvent;
use crate::core::events::bus::AppEventBus;
use std::sync::Arc;

/// Command handler for duplicate operations.
pub struct DuplicateCommandService {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    ledger: Arc<dyn TransactionalAssetLedger>,
    event_bus: Arc<dyn AppEventBus>,
}

impl DuplicateCommandService {
    /// Creates a new DuplicateCommandService.
    ///
    /// # Arguments
    /// * `duplicates_repo` - The duplicate repository port.
    /// * `ledger` - The asset ledger for atomic asset mutations.
    /// * `event_bus` - The application event bus.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        ledger: Arc<dyn TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
    ) -> Self {
        Self {
            duplicates_repo,
            ledger,
            event_bus,
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
    /// * `selected_asset_id` - Optional ID of the primary asset kept.
    ///
    /// # Errors
    /// Returns `AppError::Database` if the resolution fails to be saved.
    pub async fn resolve_group(
        &self,
        group_id: &str,
        action: DuplicateResolutionAction,
        kept_asset_ids: Option<Vec<String>>,
    ) -> AppResult<()> {
        // We will store the first kept ID in selected_asset_id if needed, 
        // and optionally save the full list in the payload
        let selected_asset_id = kept_asset_ids.as_ref().and_then(|ids| ids.first().cloned());
        
        // Record the resolution in the repository
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

        // Trashing non-selected assets for custom selection
        if matches!(action, DuplicateResolutionAction::CustomSelection) {
            if let Some(kept_ids) = &kept_asset_ids {
                let candidates = self.duplicates_repo.get_group_candidates(group_id).await?;
                for candidate in candidates {
                    if !kept_ids.contains(&candidate.asset_id) {
                        let _ = self.ledger.execute(crate::core::ledger::command::LedgerCommand::MoveToTrash(
                            crate::core::ledger::command::MoveToTrashPayload {
                                asset_id: candidate.asset_id.clone(),
                            }
                        )).await;
                    }
                }
            }
        }

        // Publish event for UI
        let _ = self.event_bus.publish(DomainEvent::DuplicateGroupResolved {
            group_id: group_id.to_string(),
            action: action.to_string(),
        });

        Ok(())
    }

    /// Triggers a scan to find new duplicates.
    ///
    /// First rehashes any fingerprints that still have pending/placeholder hashes
    /// (e.g. assets created via BatchCreate which doesn't emit individual AssetCreated events).
    /// Then runs the exact match scan to group files with identical content.
    ///
    /// # Errors
    /// Returns `AppError::DatabaseError` if the scan fails.
    pub async fn start_duplicate_scan(&self) -> AppResult<()> {
        // Step 1: Ensure all fingerprints have real Blake3 hashes
        let rehashed = self.duplicates_repo.rehash_pending_fingerprints().await?;
        tracing::info!("DuplicateCommandService: rehashed {} pending fingerprints before scan", rehashed);

        // Step 2: Run the exact match grouping
        self.duplicates_repo.run_exact_match_scan().await?;
        
        // Let the system know new groups might be available
        let _ = self.event_bus.publish(DomainEvent::DuplicateScanProgressed {
            processed: 0,
            matched: 0,
            groups_created: 0,
        });
        
        Ok(())
    }
}
