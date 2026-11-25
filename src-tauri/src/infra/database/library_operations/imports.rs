//! Publishes one prepared file using the same journal and writer as managed library edits.
mod content;
mod records;

use super::{
    commit_or_repair, journal, move_without_replace, rollback_error, DiskChanges, LIBRARY_GATE,
};
use crate::core::{
    error::{AppError, AppResult},
    events::{AppEventBus, DomainEvent},
    imports::{
        publication::{PublishPreparedImport, PublishedImport},
        staging::LIMITS,
        ImportManifest,
    },
    path_guard::LedgerPathGuard,
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::Path;

const PREPARED_DIRECTORY: &str = "import-prepared";

/// Commits an asset and its receipt, compensating any uncommitted native movement.
///
/// # Errors
/// Rejects changed requests, invalid content, unavailable destinations and storage failures.
pub(in crate::infra::database) async fn publish(
    pool: &SqlitePool,
    events: &dyn AppEventBus,
    guard: &LedgerPathGuard,
    app_data: &Path,
    mut command: PublishPreparedImport,
) -> AppResult<PublishedImport> {
    let manifest = ImportManifest {
        folder_id: Some(command.folder_id.clone()),
        tag_ids: Vec::new(),
        items: vec![command.item],
    }
    .normalize(&LIMITS)?;
    let fingerprint: [u8; 32] = Sha256::new()
        .chain_update(command.sha256)
        .chain_update(
            serde_json::to_vec(&manifest).map_err(|error| AppError::Internal(error.to_string()))?,
        )
        .finalize()
        .into();
    command.item = manifest
        .items
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Internal("Normalized import has no item".into()))?;
    let batch_id = command.batch_id.to_string();
    let item_id = &command.item.item_id;
    let receipt = {
        let _operation = LIBRARY_GATE.lock().await;
        journal::recover(pool, app_data).await?;
        let mut connection = pool.acquire().await?;
        records::replay(&mut connection, &batch_id, item_id, &fingerprint).await?
    };
    if let Some(receipt) = receipt {
        return Ok(receipt);
    }

    let source = app_data
        .join(PREPARED_DIRECTORY)
        .join(command.prepared_id.to_string());
    let inspected = content::inspect(&source, &command).await;
    let _operation = LIBRARY_GATE.lock().await;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(receipt) =
        records::replay(&mut transaction, &batch_id, item_id, &fingerprint).await?
    {
        return Ok(receipt);
    }

    let inspected = inspected?;
    let folder = records::destination(&mut transaction, &command.folder_id).await?;
    if folder.identity.device != inspected.identity.device {
        return Err(AppError::ValidationFailed(
            "Prepared import must be on the destination volume".into(),
        ));
    }
    let destination =
        records::allocate(&mut transaction, &folder.path, &command.item.file_name).await?;
    let receipt = records::record(
        &mut transaction,
        &command,
        &batch_id,
        &fingerprint,
        &destination,
        &inspected,
    )
    .await?;
    let changes = DiskChanges {
        moves: vec![(source.clone(), destination.clone())],
        ..DiskChanges::default()
    };
    let journal = journal::Journal::prepare(app_data, &changes).await?;
    let apply = async {
        journal.require_source_identity(0, &inspected.identity)?;
        super::super::handlers::library::receipts::prepare(
            &mut transaction,
            &journal.operation_id,
            &receipt.asset_id,
            "importAsset",
            &destination,
            &Default::default(),
        )
        .await?;
        content::recheck_source(&source, &inspected).await?;
        content::recheck_directory(&folder.path, &folder.identity).await?;
        journal.verify_source(0).await
    }
    .await;
    if let Err(error) = apply {
        if let Err(cleanup) = journal.remove(app_data).await {
            tracing::warn!(%cleanup, "Unstarted import journal retained");
        }
        return Err(error);
    }

    guard.mark(&destination);
    if let Err(error) = move_without_replace(&source, &destination).await {
        return Err(rollback_error(&journal, app_data, error).await);
    }
    if let Err(error) = content::recheck_file(&destination, &inspected.identity).await {
        return Err(rollback_error(&journal, app_data, error).await);
    }
    commit_or_repair(transaction, pool, &journal, app_data).await?;
    if let Err(error) = journal.remove(app_data).await {
        tracing::warn!(%error, operation_id = %journal.operation_id, "Committed import journal retained");
    }
    if let Err(error) = events.publish(DomainEvent::AssetCreated {
        asset_id: receipt.asset_id.clone(),
        path: destination.to_string_lossy().into_owned(),
        format: "png".into(),
    }) {
        tracing::warn!(%error, asset_id = %receipt.asset_id, "Committed import processing hint was lost");
    }
    Ok(receipt)
}
