//! Application entry point for user-authored discovery metadata.
use crate::core::error::AppResult;
use crate::core::ledger::command::{LedgerCommand, UpdateAssetDiscoveryPayload};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::models::asset::Asset;

/// Updates user metadata through the ledger so events follow a successful commit.
///
/// # Errors
/// Propagates invalid origin, missing asset and persistence failures.
#[tracing::instrument(skip(ledger, payload))]
pub async fn update_discovery(
    ledger: &dyn TransactionalAssetLedger,
    payload: UpdateAssetDiscoveryPayload,
) -> AppResult<Asset> {
    ledger
        .execute(LedgerCommand::UpdateAssetDiscovery(payload))
        .await
}
