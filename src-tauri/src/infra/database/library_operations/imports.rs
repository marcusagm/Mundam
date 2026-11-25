//! Publishes one prepared file using the same journal and writer as managed library edits.
mod authorization;
mod content;
mod records;

use super::{finish_commit, journal, rollback_error, DiskChanges, LIBRARY_GATE};
use crate::core::{
    error::{AppError, AppResult},
    events::{AppEventBus, DomainEvent},
    imports::{
        publication::{
            ImportPublicationAttempt, PublicationAuthority, PublishPreparedImport, PublishedImport,
        },
        staging::LIMITS,
        ImportManifest,
    },
    path_guard::LedgerPathGuard,
};
use crate::infra::filesystem::{import_move::ImportDirectories, prepared_imports};
use sha2::{Digest, Sha256};
use sqlx::{Connection, SqliteConnection, SqlitePool};
use std::path::{Path, PathBuf};

/// Repairs interrupted moves before reading the catalog outcome of a staging claim.
///
/// # Errors
/// Returns recovery or database errors so ambiguous claims remain outstanding.
pub(in crate::infra::database) async fn reconcile_attempt(
    pool: &SqlitePool,
    app_data: &Path,
    catalog_id: &str,
    attempt: &ImportPublicationAttempt,
) -> AppResult<Option<PublishedImport>> {
    let _operation = LIBRARY_GATE.lock().await;
    let active_catalog: String =
        sqlx::query_scalar("SELECT catalog_id FROM import_preferences WHERE singleton = 1")
            .fetch_one(pool)
            .await?;
    if active_catalog != catalog_id {
        return Err(AppError::ValidationFailed(
            "Import claim belongs to a different catalog".into(),
        ));
    }
    journal::recover(pool, app_data).await?;
    let receipt: Option<(String, String)> = sqlx::query_as(
        "SELECT asset_id, file_name FROM import_publication_receipts
         WHERE batch_id = ? AND item_id = ? AND attempt_id = ?",
    )
    .bind(attempt.batch_id.to_string())
    .bind(&attempt.item_id)
    .bind(attempt.attempt_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(receipt.map(|(asset_id, file_name)| PublishedImport {
        asset_id,
        file_name,
    }))
}

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
    let mut fingerprint = Sha256::new().chain_update(command.sha256).chain_update(
        serde_json::to_vec(&manifest).map_err(|error| AppError::Internal(error.to_string()))?,
    );
    if let PublicationAuthority::Paired(consent) = &command.authority {
        // Rotation changes the live grant version, while the receipt belongs to the same client.
        let owner = (&consent.catalog_id, &consent.client_id, &consent.server_key);
        fingerprint.update(b"mundam.imports.paired.v1");
        fingerprint.update(
            serde_json::to_vec(&owner).map_err(|error| AppError::Internal(error.to_string()))?,
        );
    }
    let fingerprint: [u8; 32] = fingerprint.finalize().into();
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
        let mut snapshot = pool.begin().await?;
        authorization::revalidate(&mut snapshot, &command).await?;
        let receipt = records::replay(&mut snapshot, &batch_id, item_id, &fingerprint).await?;
        snapshot.commit().await?;
        receipt
    };
    if let Some(receipt) = receipt {
        return Ok(receipt);
    }

    let source = prepared_imports::path(app_data, command.prepared_id);
    let inspected = content::inspect(&source, &command).await;
    let _operation = LIBRARY_GATE.lock().await;
    let mut connection = pool.acquire().await?;
    let previous: i64 = sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(&mut *connection)
        .await?;
    let restore = match previous {
        0 => Some("PRAGMA synchronous = OFF"),
        1 => Some("PRAGMA synchronous = NORMAL"),
        _ => None,
    };
    if restore.is_some() {
        if let Err(error) = sqlx::query("PRAGMA synchronous = FULL")
            .execute(&mut *connection)
            .await
        {
            connection.close_on_drop();
            return Err(error.into());
        }
    }
    let outcome = write(
        &mut connection,
        app_data,
        guard,
        &source,
        inspected,
        &command,
        &fingerprint,
    )
    .await;
    if let Some(restore) = restore {
        if let Err(error) = sqlx::query(restore).execute(&mut *connection).await {
            connection.close_on_drop();
            tracing::warn!(%error, "Import writer discarded after failing to restore SQLite settings");
        }
    }
    // A failed COMMIT can need a fresh connection to resolve its durable outcome.
    drop(connection);
    let movement = match outcome? {
        PublicationWrite::Replay(receipt) => return Ok(receipt),
        PublicationWrite::Moved(movement) => movement,
    };
    let PublishedMovement {
        receipt,
        journal,
        destination,
        commit,
        _directories,
    } = movement;
    finish_commit(commit, pool, &journal, app_data).await?;
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

enum PublicationWrite {
    Replay(PublishedImport),
    Moved(PublishedMovement),
}

struct PublishedMovement {
    receipt: PublishedImport,
    journal: journal::Journal,
    destination: PathBuf,
    commit: Result<(), sqlx::Error>,
    _directories: ImportDirectories,
}

async fn write(
    connection: &mut SqliteConnection,
    app_data: &Path,
    guard: &LedgerPathGuard,
    source: &Path,
    inspected: AppResult<content::InspectedFile>,
    command: &PublishPreparedImport,
    fingerprint: &[u8; 32],
) -> AppResult<PublicationWrite> {
    let batch_id = command.batch_id.to_string();
    let item_id = &command.item.item_id;
    let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
    authorization::revalidate(&mut transaction, command).await?;
    if let Some(receipt) =
        records::replay(&mut transaction, &batch_id, item_id, fingerprint).await?
    {
        return Ok(PublicationWrite::Replay(receipt));
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
    let directories = ImportDirectories::open(source, &destination).await?;
    if directories.parents.destination != folder.identity
        || directories.parents.source.device != folder.identity.device
    {
        return Err(AppError::ValidationFailed(
            "Import parent directories changed".into(),
        ));
    }
    let receipt = records::record(
        &mut transaction,
        command,
        &batch_id,
        fingerprint,
        &destination,
        &inspected,
    )
    .await?;
    let changes = DiskChanges {
        moves: vec![(source.to_path_buf(), destination.clone())],
        ..DiskChanges::default()
    };
    let journal = journal::Journal::prepare_import(
        app_data,
        &changes,
        inspected.length,
        command.sha256,
        directories.parents.clone(),
    )
    .await?;
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
        content::recheck_source(source, &inspected).await?;
        content::recheck_directory(&folder.path, &folder.identity).await?;
        journal.verify_source(0).await?;
        authorization::revalidate(&mut transaction, command).await
    }
    .await;
    if let Err(error) = apply {
        if let Err(cleanup) = journal.remove(app_data).await {
            tracing::warn!(%cleanup, "Unstarted import journal retained");
        }
        return Err(error);
    }

    guard.mark(&destination);
    if let Err(error) = directories
        .move_file(source, &destination, &inspected.identity, false)
        .await
    {
        return Err(rollback_error(&journal, app_data, error).await);
    }
    let final_identity = match journal.published_import_identity().await {
        Ok(identity) => identity,
        Err(error) => return Err(rollback_error(&journal, app_data, error).await),
    };
    if let Err(error) =
        records::record_identity(&mut transaction, &receipt.asset_id, &final_identity).await
    {
        return Err(rollback_error(&journal, app_data, error).await);
    }
    let commit = transaction.commit().await;
    Ok(PublicationWrite::Moved(PublishedMovement {
        receipt,
        journal,
        destination,
        commit,
        _directories: directories,
    }))
}
