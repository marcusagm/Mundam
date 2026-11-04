//! Durable, coalesced duplicate comparisons, independent of storage and desktop delivery.

use crate::core::{
    models::{DuplicateFingerprint, DuplicateGroupType, DuplicateRuleSet},
    AppResult,
};
use async_trait::async_trait;
use serde::Serialize;

/// Exact and visual comparisons retain independent completion and retry outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingKind {
    Exact,
    Visual,
}

impl MatchingKind {
    /// Stable queue identity also used as the runtime phase.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact_matching",
            Self::Visual => "visual_matching",
        }
    }
}

/// One ownership token fences a captured catalog and policy revision.
#[derive(Clone)]
pub struct MatchingTicket {
    pub kind: MatchingKind,
    pub lease_id: String,
    pub revision: i64,
    pub manual_run_id: Option<String>,
}

/// Immutable inputs are collected together, then compared off the async executor.
pub struct MatchingInput {
    pub ticket: MatchingTicket,
    pub policy: DuplicateRuleSet,
    pub fingerprints: Vec<DuplicateFingerprint>,
}

/// An evaluated relationship; asset identities are validated again at publication.
pub struct MatchingCandidate {
    pub asset_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// A complete group, never a partially published prefix of a comparison.
pub struct MatchingGroup {
    pub group_type: DuplicateGroupType,
    pub confidence: f64,
    pub candidates: Vec<MatchingCandidate>,
}

/// Bounded telemetry for two coalesced comparison jobs, separate from per-asset queues.
#[derive(Default, Clone, Serialize)]
pub struct MatchingStatus {
    /// Changes only when a complete comparison is published, including empty results.
    pub publication_sequence: u64,
    pub completed: Vec<String>,
    pub failed_phases: Vec<String>,
    pub pending: u64,
    pub retry_wait: u64,
    pub failed: u64,
    pub manual_pending: bool,
    pub manual_cancel_requested: bool,
    pub manual_phase: Option<String>,
}

/// Specialized persistence port; claims and publication are atomic database operations.
#[async_trait]
pub trait MatchingRepository: Send + Sync {
    /// Reconciles durable intent, including expired claims and manual recovery.
    /// # Errors
    /// Returns storage errors rather than inferring an idle queue.
    async fn matching_status(&self) -> AppResult<MatchingStatus>;
    /// Requests both phases without stealing a live reservation.
    /// # Errors
    /// Rejects a second unfinished manual request or propagates storage errors.
    async fn request_matching(&self, run_id: &str) -> AppResult<()>;
    /// Captures policy and eligible fingerprints under an exclusive reservation.
    /// # Errors
    /// Returns storage errors; unavailable work returns no claim.
    async fn claim_matching(
        &self,
        kind: MatchingKind,
        automatic_enabled: bool,
    ) -> AppResult<Option<MatchingInput>>;
    /// Never revives expired, canceled or superseded ownership.
    /// # Errors
    /// Propagates storage failures.
    async fn renew_matching(&self, ticket: &MatchingTicket) -> AppResult<bool>;
    /// Publishes a full phase and its completion in one fenced transaction.
    /// # Errors
    /// Rolls back invalid groups and failed writes; obsolete results return false.
    async fn publish_matching(
        &self,
        input: &MatchingInput,
        groups: Vec<MatchingGroup>,
    ) -> AppResult<bool>;
    /// Records bounded retry failures or releases canceled work without consuming an attempt.
    /// # Errors
    /// Propagates storage errors; stale ownership never changes a newer claim.
    async fn release_matching(
        &self,
        ticket: &MatchingTicket,
        error: Option<String>,
    ) -> AppResult<()>;
    /// Cancels manual intent while retaining independent automatic requests.
    /// # Errors
    /// Propagates storage failures. A run ID limits cancellation to its owning scan.
    async fn cancel_matching(&self, run_id: Option<&str>) -> AppResult<()>;
}
