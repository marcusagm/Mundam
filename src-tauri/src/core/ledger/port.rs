use super::command::LedgerCommand;
use crate::core::error::AppResult;
use crate::core::models::asset::Asset;
use async_trait::async_trait;

/// Port (Interface) for the Transactional Asset Ledger.
///
/// The Ledger is the Single Source of Truth for all state mutations.
/// It ensures atomicity between database updates and filesystem operations.
#[async_trait]
pub trait TransactionalAssetLedger: Send + Sync {
    /// Reclaims unreferenced processing artifacts while preserving published and leased files.
    ///
    /// # Errors
    /// Returns a storage or cache cleanup error.
    async fn reconcile_processing_artifacts(
        &self,
        _ticket: Option<&crate::core::processing::ProcessingTicket>,
    ) -> AppResult<()> {
        Ok(())
    }
    /// Executes a mutation command and returns the resulting Asset entity.
    ///
    /// # Arguments
    /// * `command` - The mutation intent (Create, Update, Delete, etc).
    ///
    /// # Errors
    /// Returns `AppError` if validation fails, a state transition is illegal,
    /// or if the underlying storage/filesystem operation fails.
    async fn execute(&self, command: LedgerCommand) -> AppResult<Asset>;

    /// Atomically fences ownership, saves derived data and completes its attempt.
    /// Returns false for an expired, removed or superseded attempt, without mutations or events.
    ///
    /// # Errors
    /// Returns a storage or validation error; unsupported adapters must fail closed.
    async fn commit_processing(
        &self,
        _ticket: &crate::core::processing::ProcessingTicket,
        _result: crate::core::processing::ProcessingResult,
    ) -> AppResult<bool> {
        Err(crate::core::error::AppError::Internal(
            "This Ledger adapter does not support fenced processing commits".into(),
        ))
    }
}
