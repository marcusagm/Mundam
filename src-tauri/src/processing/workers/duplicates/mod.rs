//! Automatic fingerprint processing uses one durable queue and shared scan admission.

use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::processing::control::{ProcessingControl, WorkerKind};
use crate::core::repository::DuplicatesRepository;
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
#[path = "tests.rs"]
mod tests;

impl DuplicateWorker {
    /// Injects durable attempt bookkeeping and the coordinator used by manual scans.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        event_bus: Arc<dyn AppEventBus>,
        settings_service: SettingsService,
        processor: FingerprintProcessor,
        coordinator: Arc<DuplicateProcessingCoordinator>,
    ) -> Self {
        let processor = processor.with_budget(coordinator.budget.clone());
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
                let matching = match self.duplicates_repo.matching_status().await {
                    Ok(status) => status,
                    Err(error) => {
                        warn!(%error, "Could not read durable matching intent");
                        continue;
                    }
                };
                self.control.set_scheduled(
                    WorkerKind::Duplicates,
                    matching.pending > 0 || matching.retry_wait > 0 || matching.manual_pending,
                );
                if self.control.is_paused(WorkerKind::Duplicates) {
                    continue;
                }
                let automatic = match self.settings_service.get_settings().await {
                    Ok(settings) => settings.auto_detect_duplicates,
                    Err(error) => {
                        warn!(%error, "Could not confirm automatic duplicate policy");
                        continue;
                    }
                };
                let mut idle = true;
                if automatic {
                    match observe_cancellation(
                        &token,
                        "fingerprint_batch",
                        self.process_batch(&token),
                    )
                    .await
                    {
                        Ok((processed, queue_idle)) => {
                            dirty_count += processed;
                            idle = queue_idle;
                        }
                        Err(error) => {
                            warn!(%error, "Automatic fingerprint processing failed");
                            continue;
                        }
                    }
                }
                let current = self.duplicates_repo.matching_status().await;
                let Ok(current) = current else {
                    continue;
                };
                self.control.set_scheduled(
                    WorkerKind::Duplicates,
                    current.pending > 0 || current.retry_wait > 0 || current.manual_pending,
                );
                if (automatic && (idle || dirty_count >= 100)) || current.manual_pending {
                    match observe_cancellation(
                        &token,
                        "matching",
                        self.compare(&token, idle || current.manual_pending),
                    )
                    .await
                    {
                        Ok(true) => {
                            dirty_count = 0;
                        }

                        Ok(false) => {}
                        Err(error) => warn!(%error, "Durable duplicate comparison failed"),
                    }
                    if let Ok(status) = self.duplicates_repo.matching_status().await {
                        self.control.set_scheduled(
                            WorkerKind::Duplicates,
                            status.pending > 0 || status.retry_wait > 0 || status.manual_pending,
                        );
                    }
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
        let automatic = self
            .settings_service
            .get_settings()
            .await?
            .auto_detect_duplicates;
        let mut published = false;
        let mut groups_processed = 0;
        for kind in [
            crate::core::duplicates::matching::MatchingKind::Exact,
            crate::core::duplicates::matching::MatchingKind::Visual,
        ] {
            if token.is_cancelled()
                || (!include_visual
                    && kind == crate::core::duplicates::matching::MatchingKind::Visual)
            {
                break;
            }
            activity.set_phase(kind.as_str());
            if let Some(count) = crate::feature::duplicates::matching_processor::process(
                self.duplicates_repo.clone(),
                self.coordinator.clone(),
                kind,
                automatic,
                token,
            )
            .await?
            {
                published = true;
                groups_processed += count;
            }
        }
        if published {
            self.event_bus
                .publish(DomainEvent::DuplicateGroupsChanged {
                    groups_processed,
                    publication_sequence: self
                        .duplicates_repo
                        .matching_status()
                        .await
                        .unwrap_or_default()
                        .publication_sequence,
                })?;
        }
        Ok(published)
    }
}

async fn observe_cancellation<T>(
    token: &CancellationToken,
    phase: &'static str,
    work: impl std::future::Future<Output = T>,
) -> T {
    tokio::pin!(work);
    tokio::select! {
        result = &mut work => result,
        _ = token.cancelled() => {
            tracing::info!(phase, "DuplicateWorker: Waiting for in-flight work after cancellation");
            let result = work.await;
            tracing::info!(phase, "DuplicateWorker: In-flight work finished after cancellation");
            result
        }
    }
}
