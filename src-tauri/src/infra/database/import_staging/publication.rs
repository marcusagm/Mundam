//! Coordinates durable staging claims with the Ledger's separate commit transaction.
use super::{
    check_active, lookup, lookup_owned,
    materialization::{complete_manifest, materialize},
    parse_manifest, volume_preparation, SqliteImportStaging, StoredBatch,
};
use crate::core::{
    imports::{
        publication::{
            ImportPublicationAttempt, PublicationAuthority, PublishPreparedImport, PublishedImport,
        },
        staging::{UploadError, UploadOwner},
    },
    ledger::port::TransactionalAssetLedger,
    AppError,
};
use crate::infra::filesystem::prepared_imports;
use sqlx::SqliteConnection;
use std::{path::Path, sync::Arc};
use uuid::Uuid;

impl SqliteImportStaging {
    /// Owns preparation, publication and cleanup for one staged item.
    ///
    /// # Errors
    /// Returns claim, validation, native storage and recovery failures.
    pub async fn publish_item(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: Uuid,
        item_id: &str,
        authority: PublicationAuthority,
        app_data: &Path,
        ledger: Arc<dyn TransactionalAssetLedger>,
    ) -> Result<PublishedImport, UploadError> {
        if !matches_owner(&authority, owner) {
            return Err(UploadError::Missing);
        }
        let staging = self.clone();
        let catalog_id = owner.catalog_id.to_owned();
        let client_id = owner.client_id.to_owned();
        let item_id = item_id.to_owned();
        let app_data = app_data.to_path_buf();
        tokio::spawn(async move {
            let owner = UploadOwner {
                catalog_id: &catalog_id,
                client_id: &client_id,
            };
            let attempt = staging
                .claim_publication(&owner, batch_id, &item_id)
                .await?;
            let batch = batch_id.to_string();
            let mut transaction = staging.0.begin_with("BEGIN IMMEDIATE").await?;
            let row = active_claim(&mut transaction, &owner, &attempt).await?;
            let has_tags = parse_manifest(&row.manifest)?
                .items
                .into_iter()
                .find(|item| item.item_id == item_id)
                .is_some_and(|item| !item.tag_ids.is_empty());
            let destination = match ledger
                .import_preparation_destination(&authority, &row.folder_id, has_tags)
                .await
            {
                Ok(destination) => destination,
                Err(error) => {
                    settle(&mut transaction, &attempt, None, &row.manifest).await?;
                    transaction.commit().await?;
                    return Err(error.into());
                }
            };
            let app_identity = crate::infra::filesystem::identity::read(&app_data).await?;
            let (mut temporary, volume_prepared, _volume_directory) = if app_identity.device
                != destination.identity.device
            {
                let (writer, temporary, prepared, directory) = volume_preparation::prepare_volume(
                    &staging,
                    transaction,
                    attempt.attempt_id,
                    destination,
                )
                .await?;
                transaction = writer;
                (temporary, Some(prepared), Some(directory))
            } else {
                (
                    prepared_imports::create(&app_data, attempt.attempt_id).await?,
                    None,
                    None,
                )
            };
            let prepared = async {
                let sealed =
                    materialize(&mut transaction, &owner, &batch, &item_id, &mut temporary).await?;
                if volume_prepared.is_some() {
                    temporary.sync_all().await.map_err(AppError::from)?;
                } else {
                    prepared_imports::sync(&temporary, &app_data).await?;
                }
                Ok::<_, UploadError>(sealed)
            }
            .await;
            drop(temporary);
            let sealed = match prepared {
                Ok(sealed) => sealed,
                Err(error) => {
                    volume_preparation::cleanup(&mut transaction, &app_data, attempt.attempt_id)
                        .await?;
                    settle(&mut transaction, &attempt, None, &row.manifest).await?;
                    transaction.commit().await?;
                    return Err(error);
                }
            };
            let command = PublishPreparedImport {
                authority,
                attempt_id: None,
                prepared_id: attempt.attempt_id,
                volume_prepared,
                batch_id,
                folder_id: sealed.folder_id,
                item: sealed.item,
                sha256: sealed.sha256,
            };
            let published = commit_claimed(
                &mut transaction,
                &owner,
                &attempt,
                &row,
                ledger.as_ref(),
                command,
                Some(&app_data),
            )
            .await?;
            transaction.commit().await?;
            published.map_err(UploadError::from)
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    }

    /// Records the publication intent before any filesystem or catalog write.
    ///
    /// # Errors
    /// Rejects inaccessible, inactive, incomplete or already claimed items.
    pub async fn claim_publication(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: Uuid,
        item_id: &str,
    ) -> Result<ImportPublicationAttempt, UploadError> {
        let batch = batch_id.to_string();
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        check_active(&lookup(&mut transaction, owner, &batch).await?)?;
        if item_finished(&mut transaction, &batch, item_id).await? {
            return Err(UploadError::PublicationComplete);
        }
        if current_attempt(&mut transaction, &batch).await?.is_some() {
            return Err(UploadError::PublicationPending);
        }
        let manifest = complete_manifest(&mut transaction, owner, &batch).await?;
        if !manifest.items.iter().any(|item| item.item_id == item_id) {
            return Err(UploadError::Missing);
        }
        let attempt_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO import_publication_claims (batch_id, item_id, attempt_id) VALUES (?, ?, ?)",
        )
        .bind(&batch)
        .bind(item_id)
        .bind(attempt_id.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(ImportPublicationAttempt {
            batch_id,
            item_id: item_id.into(),
            attempt_id,
        })
    }

    /// Publishes a prepared file under an active claim and reconciles the catalog outcome.
    ///
    /// The coordinator owns the write lock until the Ledger returns, even if a caller disconnects.
    /// A caller prepares and synchronizes the temporary before entering this operation.
    ///
    /// # Errors
    /// Returns invalid claim, publication, journal recovery and storage failures.
    pub async fn commit_prepared(
        &self,
        owner: &UploadOwner<'_>,
        attempt: ImportPublicationAttempt,
        ledger: Arc<dyn TransactionalAssetLedger>,
        command: PublishPreparedImport,
    ) -> Result<PublishedImport, UploadError> {
        let staging = self.clone();
        let catalog_id = owner.catalog_id.to_owned();
        let client_id = owner.client_id.to_owned();
        tokio::spawn(async move {
            let owner = UploadOwner {
                catalog_id: &catalog_id,
                client_id: &client_id,
            };
            let mut transaction = staging.0.begin_with("BEGIN IMMEDIATE").await?;
            let row = active_claim(&mut transaction, &owner, &attempt).await?;
            let published = commit_claimed(
                &mut transaction,
                &owner,
                &attempt,
                &row,
                ledger.as_ref(),
                command,
                None,
            )
            .await?;
            transaction.commit().await?;
            published.map_err(UploadError::from)
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    }

    /// Restores the staging receipt from a recovered catalog without rerunning publication.
    ///
    /// # Errors
    /// Returns ownership or recovery errors, leaving the claim intact when proof is ambiguous.
    pub async fn reconcile_publication(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: Uuid,
        app_data: &Path,
        ledger: &dyn TransactionalAssetLedger,
    ) -> Result<Option<PublishedImport>, UploadError> {
        let batch = batch_id.to_string();
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let row = lookup_owned(&mut transaction, owner, &batch).await?;
        let Some(attempt) = current_attempt(&mut transaction, &batch).await? else {
            return Ok(None);
        };
        let proof = ledger
            .reconcile_import_attempt(owner.catalog_id, &attempt)
            .await?;
        volume_preparation::cleanup(&mut transaction, app_data, attempt.attempt_id).await?;
        settle(&mut transaction, &attempt, proof.as_ref(), &row.manifest).await?;
        transaction.commit().await?;
        Ok(proof)
    }

    /// Records a sanitized failure after the publication claim has been settled.
    ///
    /// # Errors
    /// Returns storage and authorization failures without changing committed items.
    pub(crate) async fn record_publication_error(
        &self,
        owner: &UploadOwner<'_>,
        batch_id: Uuid,
        item_id: &str,
        code: &str,
        retryable: bool,
    ) -> Result<(), UploadError> {
        let batch = batch_id.to_string();
        let mut writer = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let row = lookup(&mut writer, owner, &batch).await?;
        check_active(&row)?;
        if current_attempt(&mut writer, &batch).await?.is_some()
            || item_finished(&mut writer, &batch, item_id).await?
        {
            return Ok(());
        }
        let manifest = parse_manifest(&row.manifest)?;
        if !manifest.items.iter().any(|item| item.item_id == item_id) {
            return Err(UploadError::Missing);
        }
        sqlx::query(
            "INSERT INTO import_publication_errors (batch_id, item_id, code, retryable) VALUES (?, ?, ?, ?)
             ON CONFLICT(batch_id, item_id) DO UPDATE SET code = excluded.code, retryable = excluded.retryable",
        ).bind(&batch).bind(item_id).bind(code).bind(retryable).execute(&mut *writer).await?;
        release_terminal_parts(&mut writer, &batch, manifest.items.len()).await?;
        writer.commit().await?;
        Ok(())
    }

    /// Resolves claims belonging to the open catalog before serving external clients.
    ///
    /// # Errors
    /// Returns discovery or recovery failures. Unresolved claims remain in staging.
    pub(crate) async fn recover_for_catalog(
        &self,
        catalog_id: &str,
        app_data: &Path,
        ledger: &dyn TransactionalAssetLedger,
    ) -> Result<(), UploadError> {
        let pending: Vec<(String, String)> = sqlx::query_as(
            "SELECT batches.id, batches.owner FROM import_publication_claims AS claims
             JOIN import_batches AS batches ON batches.id = claims.batch_id
             WHERE batches.catalog = ? ORDER BY batches.id",
        )
        .bind(catalog_id)
        .fetch_all(&self.0)
        .await?;
        let mut failures = 0;
        for (batch, client_id) in pending {
            let owner = UploadOwner {
                catalog_id,
                client_id: &client_id,
            };
            let result = match Uuid::parse_str(&batch) {
                Ok(batch_id) => {
                    self.reconcile_publication(&owner, batch_id, app_data, ledger)
                        .await
                }
                Err(_) => Err(UploadError::StagingCorrupt),
            };
            if let Err(error) = result {
                failures += 1;
                tracing::warn!(%batch, %error, "Import publication reconciliation remains pending");
            }
        }
        if failures > 0 {
            return Err(AppError::Internal(format!(
                "{failures} import publication claims need recovery"
            ))
            .into());
        }
        Ok(())
    }
}

async fn active_claim(
    connection: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    attempt: &ImportPublicationAttempt,
) -> Result<StoredBatch, UploadError> {
    let batch = attempt.batch_id.to_string();
    let row = lookup(connection, owner, &batch).await?;
    check_active(&row)?;
    if current_attempt(connection, &batch).await?.as_ref() != Some(attempt) {
        return Err(UploadError::Missing);
    }
    Ok(row)
}

async fn commit_claimed(
    connection: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    attempt: &ImportPublicationAttempt,
    row: &StoredBatch,
    ledger: &dyn TransactionalAssetLedger,
    mut command: PublishPreparedImport,
    app_data: Option<&Path>,
) -> Result<Result<PublishedImport, AppError>, UploadError> {
    if !matches_command(attempt, &command, owner, &row.folder_id)
        || !parse_manifest(&row.manifest)?.items.contains(&command.item)
    {
        return Err(UploadError::IdempotencyConflict);
    }
    command.attempt_id = Some(attempt.attempt_id);
    ledger
        .reconcile_import_attempt(owner.catalog_id, attempt)
        .await?;
    let published = ledger.publish_prepared_import(command).await;
    let proof = ledger
        .reconcile_import_attempt(owner.catalog_id, attempt)
        .await?;
    if published
        .as_ref()
        .is_ok_and(|receipt| proof.as_ref() != Some(receipt))
    {
        return Err(
            AppError::Internal("Import result has no matching catalog proof".into()).into(),
        );
    }
    if let Some(app_data) = app_data {
        volume_preparation::cleanup(connection, app_data, attempt.attempt_id).await?;
    }
    settle(connection, attempt, proof.as_ref(), &row.manifest).await?;
    Ok(published)
}

fn matches_command(
    attempt: &ImportPublicationAttempt,
    command: &PublishPreparedImport,
    owner: &UploadOwner<'_>,
    folder_id: &str,
) -> bool {
    matches_owner(&command.authority, owner)
        && command.batch_id == attempt.batch_id
        && command.item.item_id == attempt.item_id
        && command.folder_id == folder_id
}

fn matches_owner(authority: &PublicationAuthority, owner: &UploadOwner<'_>) -> bool {
    match authority {
        PublicationAuthority::Host => true,
        PublicationAuthority::Paired(consent) => {
            consent.catalog_id == owner.catalog_id && consent.client_id == owner.client_id
        }
    }
}

async fn settle(
    connection: &mut SqliteConnection,
    attempt: &ImportPublicationAttempt,
    proof: Option<&PublishedImport>,
    manifest: &str,
) -> Result<(), UploadError> {
    let batch = attempt.batch_id.to_string();
    if let Some(proof) = proof {
        sqlx::query(
            "INSERT INTO import_published_items (batch_id, item_id, asset_id, file_name)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&batch)
        .bind(&attempt.item_id)
        .bind(&proof.asset_id)
        .bind(&proof.file_name)
        .execute(&mut *connection)
        .await?;
        sqlx::query("DELETE FROM import_publication_errors WHERE batch_id = ? AND item_id = ?")
            .bind(&batch)
            .bind(&attempt.item_id)
            .execute(&mut *connection)
            .await?;
        release_terminal_parts(connection, &batch, parse_manifest(manifest)?.items.len()).await?;
    }
    sqlx::query("DELETE FROM import_publication_claims WHERE batch_id = ?")
        .bind(&batch)
        .execute(connection)
        .await?;
    Ok(())
}

async fn release_terminal_parts(
    connection: &mut SqliteConnection,
    batch: &str,
    item_count: usize,
) -> Result<(), UploadError> {
    let terminal: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM import_published_items WHERE batch_id = ?1)
          + (SELECT COUNT(*) FROM import_publication_errors WHERE batch_id = ?1 AND retryable = 0)",
    )
    .bind(batch)
    .fetch_one(&mut *connection)
    .await?;
    if terminal as usize == item_count {
        sqlx::query("DELETE FROM import_parts WHERE batch_id = ?")
            .bind(batch)
            .execute(&mut *connection)
            .await?;
        sqlx::query("UPDATE import_batches SET reserved_bytes = 0 WHERE id = ?")
            .bind(batch)
            .execute(connection)
            .await?;
    }
    Ok(())
}

pub(super) async fn current_attempt(
    connection: &mut SqliteConnection,
    batch_id: &str,
) -> Result<Option<ImportPublicationAttempt>, UploadError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT item_id, attempt_id FROM import_publication_claims WHERE batch_id = ?",
    )
    .bind(batch_id)
    .fetch_optional(connection)
    .await?;
    row.map(|(item_id, attempt_id)| {
        Ok(ImportPublicationAttempt {
            batch_id: Uuid::parse_str(batch_id).map_err(|_| UploadError::StagingCorrupt)?,
            item_id,
            attempt_id: Uuid::parse_str(&attempt_id).map_err(|_| UploadError::StagingCorrupt)?,
        })
    })
    .transpose()
}

pub(super) async fn item_finished(
    connection: &mut SqliteConnection,
    batch_id: &str,
    item_id: &str,
) -> Result<bool, UploadError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM import_published_items WHERE batch_id = ?1 AND item_id = ?2
        UNION ALL SELECT 1 FROM import_publication_errors WHERE batch_id = ?1 AND item_id = ?2 AND retryable = 0)",
    )
    .bind(batch_id)
    .bind(item_id)
    .fetch_one(connection)
    .await?)
}
