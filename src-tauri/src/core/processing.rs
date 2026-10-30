//! Shared processing contracts, independent of desktop delivery and concrete storage.

use crate::core::error::AppResult;
use async_trait::async_trait;
use serde::Serialize;

/// Read-model state used to distinguish pending output from terminal extraction outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThumbnailProcessingState {
    /// Work is eligible or has been explicitly requested.
    Pending,
    /// A bounded retry is deferred.
    RetryWait,
    /// Automatic retry attempts are exhausted.
    Failed,
    /// The provider cannot produce a thumbnail.
    Skipped,
    /// A generated thumbnail is available.
    Succeeded,
    /// The asset is inactive or offline.
    Unavailable,
}

/// Compact per-asset status queried in viewport-sized batches without exposing error paths.
#[derive(Debug, Clone, Serialize)]
pub struct ThumbnailProcessingStatus {
    /// Identity of the requested asset.
    pub asset_id: String,
    /// Authoritative state of thumbnail processing.
    pub state: ThumbnailProcessingState,
    /// Generation used to invalidate image cache after regeneration.
    pub generation: i64,
    /// Current generated output, including recovery after a missed ready event.
    pub thumbnail_path: Option<String>,
}

/// Independent processing stages must not overload the asset's lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingKind {
    /// Thumbnail generation and its technical metadata extraction.
    Thumbnail,
    /// Palette extraction from a valid thumbnail.
    Colors,
    /// Content and visual fingerprint generation.
    Fingerprint,
}

impl ProcessingKind {
    /// Returns the stable storage identity of this processing stage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::Colors => "colors",
            Self::Fingerprint => "fingerprint",
        }
    }
}

/// Identifies the generation and source observed when an attempt was admitted.
#[derive(Debug, Clone)]
pub struct ProcessingTicket {
    /// Asset whose results this attempt may produce.
    pub asset_id: String,
    /// Stage being processed.
    pub kind: ProcessingKind,
    /// Explicit requests invalidate older attempt generations.
    pub generation: i64,
    /// Source identity used to discard outdated attempt bookkeeping.
    pub source_stamp: String,
}

/// Distinguishes valid results, unsupported work and retryable processing failures.
pub enum ProcessingOutcome {
    /// The result was persisted successfully.
    Completed,
    /// This stage is not applicable to the asset or provider.
    Skipped(String),
    /// A processing error is subject to the bounded retry policy.
    Failed(String),
}

/// Durable pending work and attempt outcomes shared by specialized executors.
///
/// Runtime concurrency remains owned by each worker; tickets are generation
/// checks, not cross-process execution leases.
#[async_trait]
pub trait ProcessingRepository: Send + Sync {
    /// Reads thumbnail outcomes for a bounded set of visible assets.
    ///
    /// # Errors
    /// Returns a storage error if statuses cannot be read.
    async fn thumbnail_statuses(
        &self,
        asset_ids: &[String],
    ) -> AppResult<Vec<ThumbnailProcessingStatus>>;

    /// Lists eligible work with due retries before untouched backlog in a stable order.
    /// Deferred retries and terminal failures are excluded.
    ///
    /// # Errors
    /// Returns a storage error if pending work cannot be queried.
    async fn pending(&self, kind: ProcessingKind, limit: u32) -> AppResult<Vec<String>>;

    /// Admits an eligible attempt and captures its source and request generation.
    ///
    /// # Errors
    /// Returns a storage error if the attempt cannot be recorded.
    async fn begin(
        &self,
        kind: ProcessingKind,
        asset_id: &str,
    ) -> AppResult<Option<ProcessingTicket>>;

    /// Checks that an attempt still targets an active asset and current generation.
    ///
    /// # Errors
    /// Returns a storage error if the generation cannot be checked.
    async fn is_current(&self, ticket: &ProcessingTicket) -> AppResult<bool>;

    /// Records an outcome without overwriting a newer request or recreating removed work.
    ///
    /// # Errors
    /// Returns a storage error if the outcome cannot be persisted.
    async fn finish(&self, ticket: &ProcessingTicket, outcome: ProcessingOutcome) -> AppResult<()>;
}
