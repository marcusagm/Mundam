//! Shared per-asset execution and retry bookkeeping for manual and automatic fingerprints.

use crate::core::{
    formats::FormatRegistry,
    ledger::port::TransactionalAssetLedger,
    processing::{
        ProcessingKind, ProcessingOutcome, ProcessingRepository, ProcessingResult, ProcessingTicket,
    },
    repository::AssetQueryHandler,
    AppResult,
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Computes through one pipeline; callers own scan admission and user-visible progress.
#[derive(Clone)]
pub struct FingerprintProcessor {
    ledger: Arc<dyn TransactionalAssetLedger>,
    processing: Arc<dyn ProcessingRepository>,
    queries: Arc<dyn AssetQueryHandler>,
    registry: Arc<FormatRegistry>,
}

impl FingerprintProcessor {
    /// Shares the same repositories, source checks and provider registry across entry points.
    pub fn new(
        ledger: Arc<dyn TransactionalAssetLedger>,
        processing: Arc<dyn ProcessingRepository>,
        queries: Arc<dyn AssetQueryHandler>,
        registry: Arc<FormatRegistry>,
    ) -> Self {
        Self {
            ledger,
            processing,
            queries,
            registry,
        }
    }

    /// Reads the shared eligibility policy, including backoff, trash and offline exclusions.
    ///
    /// # Errors
    /// Propagates queue read failures.
    pub async fn pending(&self, limit: u32) -> AppResult<Vec<String>> {
        self.processing
            .pending(ProcessingKind::Fingerprint, limit)
            .await
    }

    /// Returns true only after a current result was saved and its attempt completed.
    ///
    /// # Errors
    /// Records extraction errors using bounded retries, then propagates the error.
    /// Persistence errors are also propagated. Canceled or superseded work returns false.
    pub async fn process_asset(
        &self,
        asset_id: &str,
        token: &CancellationToken,
    ) -> AppResult<bool> {
        if token.is_cancelled() {
            return Ok(false);
        }
        let Some(ticket) = self
            .processing
            .begin(ProcessingKind::Fingerprint, asset_id)
            .await?
        else {
            return Ok(false);
        };
        let _lease = crate::processing::lease::ProcessingLease::start(
            self.processing.clone(),
            ticket.clone(),
        );
        let result = self.calculate_and_save(&ticket, token).await;
        if let Err(error) = &result {
            if !token.is_cancelled() {
                self.processing
                    .finish(&ticket, ProcessingOutcome::failure(&error))
                    .await?;
            }
        }
        self.processing.release(&ticket).await?;
        if token.is_cancelled() {
            return Ok(false);
        }
        match result {
            Ok(completed) => Ok(completed),
            Err(error) => Err(error),
        }
    }

    async fn calculate_and_save(
        &self,
        ticket: &ProcessingTicket,
        token: &CancellationToken,
    ) -> AppResult<bool> {
        if !self.processing.is_current(ticket).await? {
            return Ok(false);
        }
        let asset = self.queries.get_asset_by_id(&ticket.asset_id).await?;
        let Some(fingerprint) = crate::processing::media::fingerprint::generate(
            asset,
            self.registry.clone(),
            token.clone(),
        )
        .await?
        else {
            return Ok(false);
        };
        if token.is_cancelled() || !self.processing.is_current(ticket).await? {
            return Ok(false);
        }
        self.ledger
            .commit_processing(ticket, ProcessingResult::Fingerprint(fingerprint))
            .await
    }

    /// Processes a fixed pending snapshot so progress totals do not grow during a manual scan.
    ///
    /// # Errors
    /// Propagates initial queue failures. Per-asset failures retain their retry outcome and
    /// do not prevent other candidates from advancing.
    pub async fn rehash_pending(
        &self,
        token: &CancellationToken,
        progress: Option<tokio::sync::mpsc::UnboundedSender<(usize, usize)>>,
    ) -> AppResult<usize> {
        let pending = self.pending(u32::MAX).await?;
        let total = pending.len();
        if let Some(sender) = &progress {
            let _ = sender.send((0, total));
        }
        let mut completed = 0;
        for (index, asset_id) in pending.into_iter().enumerate() {
            if token.is_cancelled() {
                break;
            }
            match self.process_asset(&asset_id, token).await {
                Ok(true) => completed += 1,
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(asset_id, %error, "Manual fingerprint follows the bounded retry policy")
                }
            }
            if token.is_cancelled() {
                break;
            }
            if let Some(sender) = &progress {
                let _ = sender.send((index + 1, total));
            }
        }
        Ok(completed)
    }
}

#[cfg(test)]
#[path = "fingerprint_processor_tests.rs"]
mod tests;
