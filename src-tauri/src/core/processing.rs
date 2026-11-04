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
    /// Unique reservation identity; reclaimed attempts cannot reuse an older ticket.
    pub lease_id: String,
}

/// Derived results accepted only together with the owning attempt's completion.
pub enum ProcessingResult {
    /// Optional metadata and model preview remain useful without thumbnail support.
    Thumbnail {
        thumbnail_path: Option<String>,
        preview_path: Option<String>,
        metadata: Option<crate::core::ledger::command::UpdateTechnicalMetadataPayload>,
        unsupported_reason: Option<String>,
    },
    /// Complete replacement palette for the ticket's asset.
    Colors(Vec<crate::core::models::asset::AssetColor>),
    /// Exact and optional visual representation of the ticket's asset.
    Fingerprint(crate::core::models::DuplicateFingerprint),
}

/// Distinguishes valid results, unsupported work and retryable processing failures.
pub enum ProcessingOutcome {
    /// Typed cause with private log context, for bounded retries and grouped diagnostics.
    DiagnosedFailure {
        issue: crate::core::processing_diagnostics::ProcessingIssue,
        message: String,
    },
    /// Unsupported output is terminal until the user explicitly requests reevaluation.
    Unavailable(crate::core::processing_diagnostics::ProcessingIssue),
    /// The result was persisted successfully.
    Completed,
    /// This stage is not applicable to the asset or provider.
    Skipped(String),
    /// A processing error is subject to the bounded retry policy.
    Failed(String),
}

impl ProcessingOutcome {
    /// Preserves private failure details while exposing only a stable diagnostic cause.
    pub fn failure(error: &crate::core::AppError) -> Self {
        Self::DiagnosedFailure {
            issue: crate::core::processing_diagnostics::ProcessingIssue::from_error(error),
            message: error.to_string(),
        }
    }
}

/// Durable pending work and attempt outcomes shared by specialized executors.
///
/// Runtime concurrency remains owned by each worker. Durable reservations fence
/// concurrent executors and expire after crashes; renewal never revives an expired lease.
#[async_trait]
pub trait ProcessingRepository: Send + Sync {
    /// Groups current active outcomes by a finite, safe cause whitelist.
    /// # Errors
    /// Propagates storage failures instead of claiming healthy processing.
    async fn diagnostics(
        &self,
        kind: ProcessingKind,
    ) -> AppResult<Vec<crate::core::processing_diagnostics::ProcessingDiagnostic>>;

    /// Queues terminal outcomes atomically, preserving valid outputs and active leases.
    /// # Errors
    /// Rolls back the complete request on a storage error.
    async fn retry(
        &self,
        kind: ProcessingKind,
        scope: crate::core::processing_diagnostics::ProcessingRetryScope,
    ) -> AppResult<crate::core::processing_diagnostics::ProcessingRetryCounts>;
    /// Counts durable outcomes, excluding the supplied running asset identities.
    ///
    /// # Errors
    /// Returns a storage error when the queue read model is unavailable.
    async fn queue_counts(
        &self,
        kind: ProcessingKind,
        active_assets: &[String],
    ) -> AppResult<crate::core::processing_control::ProcessingQueueCounts>;
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

    /// Renews an unexpired reservation without reviving superseded ownership.
    ///
    /// # Errors
    /// Returns a storage error when renewal cannot be recorded.
    async fn renew(&self, ticket: &ProcessingTicket) -> AppResult<bool>;

    /// Releases ownership without consuming a retry; stale releases are harmless.
    ///
    /// # Errors
    /// Returns a storage error when the reservation cannot be released.
    async fn release(&self, ticket: &ProcessingTicket) -> AppResult<()>;

    /// Resolves only the committed model preview, independently of an active attempt.
    ///
    /// # Errors
    /// Returns a storage error when the artifact pointer cannot be read.
    async fn preview_path(&self, asset_id: &str) -> AppResult<Option<String>>;

    /// Records an outcome without overwriting a newer request or recreating removed work.
    ///
    /// # Errors
    /// Returns a storage error if the outcome cannot be persisted.
    async fn finish(&self, ticket: &ProcessingTicket, outcome: ProcessingOutcome) -> AppResult<()>;
}
