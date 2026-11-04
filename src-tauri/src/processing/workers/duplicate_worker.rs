//! Automatic fingerprint processing uses one durable queue and shared scan admission.

use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::formats::FormatRegistry;
use crate::core::processing::ProcessingRepository;
use crate::core::processing_control::{ProcessingControl, WorkerKind};
use crate::core::repository::{AssetQueryHandler, DuplicatesRepository};
use crate::feature::duplicates::coordinator::DuplicateProcessingCoordinator;
use crate::feature::duplicates::fingerprint_processor::FingerprintProcessor;
use crate::feature::settings::SettingsService;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Events wake a single polling executor instead of launching a second hashing path.
pub struct DuplicateWorker {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    event_bus: Arc<dyn AppEventBus>,
    settings_service: SettingsService,
    processor: FingerprintProcessor,
    coordinator: Arc<DuplicateProcessingCoordinator>,
    control: Arc<ProcessingControl>,
}

#[cfg(test)]
#[path = "duplicate_worker_tests.rs"]
mod tests;

impl DuplicateWorker {
    /// Injects durable attempt bookkeeping and the coordinator used by manual scans.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        ledger: Arc<dyn crate::core::ledger::port::TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
        settings_service: SettingsService,
        format_registry: Arc<FormatRegistry>,
        processing: Arc<dyn ProcessingRepository>,
        queries: Arc<dyn AssetQueryHandler>,
        coordinator: Arc<DuplicateProcessingCoordinator>,
    ) -> Self {
        let processor = FingerprintProcessor::new(ledger, processing, queries, format_registry);
        Self {
            duplicates_repo,
            event_bus,
            settings_service,
            processor,
            coordinator,
            control: Arc::new(ProcessingControl::default()),
        }
    }

    /// Shares automatic and manual scan admission with statusbar controls.
    pub fn with_control(mut self, control: Arc<ProcessingControl>) -> Self {
        self.control = control;
        self
    }

    /// Processes recoverable work and coalesces comparisons after batches or idle transitions.
    pub fn start(
        self: Arc<Self>,
        token: CancellationToken,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let mut subscriber = self.event_bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let _lifetime = self.control.start(WorkerKind::Duplicates);
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut dirty_count = 0;
            let mut visual_dirty = false;
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {},
                    event = subscriber.recv() => match event {
                        Ok(DomainEvent::AssetCreated { .. } | DomainEvent::ScanCompleted { .. }) => {},
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            warn!(count, "Duplicate events lagged; reconciling durable work");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                if !self
                    .settings_service
                    .get_settings()
                    .await
                    .map(|settings| settings.auto_detect_duplicates)
                    .unwrap_or(true)
                {
                    continue;
                }
                match self.process_batch(&token).await {
                    Ok((processed, idle)) => {
                        dirty_count += processed;
                        visual_dirty |= processed > 0;
                        self.control
                            .set_scheduled(WorkerKind::Duplicates, visual_dirty || dirty_count > 0);
                        if ((idle && visual_dirty) || dirty_count >= 100) && !token.is_cancelled() {
                            match self.compare(&token, idle).await {
                                Ok(true) => {
                                    dirty_count = 0;
                                    if idle {
                                        visual_dirty = false;
                                    }
                                    self.control.set_scheduled(
                                        WorkerKind::Duplicates,
                                        visual_dirty || dirty_count > 0,
                                    );
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    warn!(%error, "Automatic duplicate comparison failed")
                                }
                            }
                        }
                    }
                    Err(error) => warn!(%error, "Automatic fingerprint processing failed"),
                }
            }
        })
    }

    async fn process_batch(
        &self,
        token: &CancellationToken,
    ) -> crate::core::error::AppResult<(usize, bool)> {
        if self.control.is_paused(WorkerKind::Duplicates) {
            return Ok((0, false));
        }
        let pending = self.processor.pending(10).await?;
        let idle = pending.is_empty();
        let mut processed = 0;
        for asset_id in pending {
            let _admission = tokio::select! {
                _ = token.cancelled() => break,
                admission = self.coordinator.acquire() => admission,
            };
            let Some(_activity) =
                self.control
                    .try_admit(WorkerKind::Duplicates, "fingerprinting", Some(&asset_id))
            else {
                break;
            };
            let result = self.processor.process_asset(&asset_id, token).await;
            if token.is_cancelled() {
                break;
            }
            match result {
                Ok(true) => processed += 1,
                Ok(false) => {}
                Err(error) => {
                    warn!(asset_id, %error, "Fingerprint failed; applying bounded retry policy")
                }
            }
        }
        Ok((processed, idle))
    }

    async fn compare(
        &self,
        token: &CancellationToken,
        include_visual: bool,
    ) -> crate::core::error::AppResult<bool> {
        let _admission = tokio::select! {
            _ = token.cancelled() => return Ok(false),
            admission = self.coordinator.acquire() => admission,
        };
        let Some(activity) = self
            .control
            .try_admit(WorkerKind::Duplicates, "exact_matching", None)
        else {
            return Ok(false);
        };
        let mut groups_processed = self
            .duplicates_repo
            .run_exact_match_scan(Some(token.clone()))
            .await?;
        if include_visual && !token.is_cancelled() {
            activity.set_phase("visual_matching");
            groups_processed += self
                .duplicates_repo
                .run_visual_match_scan(Some(token.clone()))
                .await?;
        }
        if !token.is_cancelled() {
            self.event_bus
                .publish(DomainEvent::DuplicateGroupsChanged { groups_processed })?;
        }
        Ok(true)
    }
}
