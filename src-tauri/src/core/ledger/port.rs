use super::command::LedgerCommand;
use crate::core::error::AppResult;
use crate::core::models::asset::Asset;
use async_trait::async_trait;

/// Port (Interface) for the Transactional Asset Ledger.
///
/// The Ledger is the Single Source of Truth for all state mutations.
/// It commits catalog changes and coordinates recoverable filesystem effects.
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
    /// Saves observed native identities only for the paths still owned by their catalog rows.
    ///
    /// # Errors
    /// Returns persistence failures; implementations without identity support may skip recording.
    async fn remember_file_identities(
        &self,
        _identities: Vec<(std::path::PathBuf, crate::core::library::FileIdentity)>,
    ) -> AppResult<()> {
        Ok(())
    }

    /// Records native identities of indexed files in bounded batches.
    ///
    /// # Errors
    /// Returns persistence errors; unavailable paths are skipped for the next scan.
    async fn observe_file_identities(&self, _paths: Vec<std::path::PathBuf>) -> AppResult<()> {
        Ok(())
    }

    /// Executes a managed intention with a durable operation receipt.
    ///
    /// # Errors
    /// Returns validation or storage failures; unsupported adapters fail closed.
    async fn execute_library(
        &self,
        _operation: super::library_operation::LibraryOperation,
    ) -> AppResult<crate::core::library::LibraryOperationResult> {
        Err(crate::core::error::AppError::Internal(
            "Managed library operations are unsupported".into(),
        ))
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
