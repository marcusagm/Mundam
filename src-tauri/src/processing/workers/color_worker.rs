//! Palette extraction reconciles durable work; events only reduce wake-up latency.

use crate::core::error::{AppError, AppResult};
use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::ledger::command::{LedgerCommand, UpdateAssetColorsPayload};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::processing::{
    ProcessingKind, ProcessingOutcome, ProcessingRepository, ProcessingTicket,
};
use crate::core::repository::AssetQueryHandler;
use crate::feature::analysis::colors::extract_color_palette;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

/// Serial palette executor backed by recoverable, generation-aware pending work.
pub struct ColorWorker {
    ledger: Arc<dyn TransactionalAssetLedger>,
    event_bus: Arc<dyn AppEventBus>,
    queries: Arc<dyn AssetQueryHandler>,
    processing: Arc<dyn ProcessingRepository>,
    thumbnails_dir: PathBuf,
}

impl ColorWorker {
    /// Injects persistence and queries without coupling extraction to the desktop handle.
    pub fn new(
        ledger: Arc<dyn TransactionalAssetLedger>,
        event_bus: Arc<dyn AppEventBus>,
        queries: Arc<dyn AssetQueryHandler>,
        processing: Arc<dyn ProcessingRepository>,
        thumbnails_dir: PathBuf,
    ) -> Self {
        Self {
            ledger,
            event_bus,
            queries,
            processing,
            thumbnails_dir,
        }
    }

    /// Reconciles on startup, periodically and after relevant domain notifications.
    pub fn start(self, token: CancellationToken) -> tauri::async_runtime::JoinHandle<()> {
        let mut subscriber = self.event_bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {},
                    event = subscriber.recv() => match event {
                        Ok(DomainEvent::ThumbnailGenerated { .. } | DomainEvent::ReextractAssetColors { .. }) => {},
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            warn!(count, "Color events lagged; reconciling durable work");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                if let Err(error) = self.process_pending(&token).await {
                    error!(%error, "Color processing failed");
                }
            }
        })
    }

    async fn process_pending(&self, token: &CancellationToken) -> AppResult<()> {
        if token.is_cancelled() {
            return Ok(());
        }
        for asset_id in self.processing.pending(ProcessingKind::Colors, 1).await? {
            let Some(ticket) = self
                .processing
                .begin(ProcessingKind::Colors, &asset_id)
                .await?
            else {
                continue;
            };
            let result = self.extract(&ticket, token).await;
            if token.is_cancelled() {
                return Ok(());
            }
            let outcome = match result {
                Ok(outcome) => outcome,
                Err(error) => {
                    warn!(asset_id, %error, "Palette extraction will follow the bounded retry policy");
                    ProcessingOutcome::Failed(error.to_string())
                }
            };
            self.processing.finish(&ticket, outcome).await?;
        }
        Ok(())
    }

    async fn extract(
        &self,
        ticket: &ProcessingTicket,
        token: &CancellationToken,
    ) -> AppResult<ProcessingOutcome> {
        let asset = self.queries.get_asset_by_id(&ticket.asset_id).await?;
        let Some(thumbnail) = asset.thumbnail_path.filter(|path| !path.is_empty()) else {
            return Ok(ProcessingOutcome::Skipped(
                "A valid thumbnail is required".into(),
            ));
        };
        let path = self.thumbnails_dir.join(thumbnail);
        let colors = tokio::task::spawn_blocking(move || extract_color_palette(&path, None))
            .await
            .map_err(|error| {
                AppError::Internal(format!("Color extraction task failed: {error}"))
            })??;
        if token.is_cancelled() || !self.processing.is_current(ticket).await? {
            return Ok(ProcessingOutcome::Skipped(
                "Attempt was superseded or canceled".into(),
            ));
        }
        self.ledger
            .execute(LedgerCommand::UpdateAssetColors(UpdateAssetColorsPayload {
                asset_id: ticket.asset_id.clone(),
                colors,
            }))
            .await?;
        Ok(ProcessingOutcome::Completed)
    }
}

#[cfg(test)]
#[path = "color_worker_tests.rs"]
mod tests;
