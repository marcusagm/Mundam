//! Taxonomy application commands validate input before dispatching to the ledger.

use crate::core::error::AppResult;
use crate::core::ledger::command::{ImportTagsPayload, LedgerCommand};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::ledger::tag_validation::validate_import;

/// Applies the entire outline atomically through the domain ledger.
///
/// # Errors
/// Returns outline validation errors or errors reported by the ledger.
#[tracing::instrument(skip(ledger, payload))]
pub async fn import_tags(
    ledger: &dyn TransactionalAssetLedger,
    payload: ImportTagsPayload,
) -> AppResult<()> {
    validate_import(&payload)?;
    ledger.execute(LedgerCommand::ImportTags(payload)).await?;
    Ok(())
}
