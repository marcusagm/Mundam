//! Per-user, bounded import bytes and receipts in a separate FULL-synchronous SQLite store.
mod materialization;
mod parts;
mod publication;

use crate::core::{
    imports::{
        staging::{UploadAdmission, UploadError, UploadOwner, PART_BYTES},
        ImportManifest,
    },
    AppError, AppResult,
};
use chrono::{DateTime, Utc};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqliteConnection, SqlitePool,
};
use std::{path::Path, time::Duration};

const CLIENT_QUOTA: i64 = 64 * 1024 * 1024;
const TOTAL_QUOTA: i64 = 128 * 1024 * 1024;
const RECEIVE_SECONDS: i64 = 24 * 3600;
const RECEIPT_SECONDS: i64 = 7 * 24 * 3600;

/// File shared by the desktop listener and startup recovery.
pub(crate) const DATABASE_NAME: &str = "import-staging.sqlite";

/// Holds the isolated staging database, with the catalog and client included in each owner key.
#[derive(Clone)]
pub struct SqliteImportStaging(SqlitePool);

impl SqliteImportStaging {
    /// Opens staging and removes expired part contents before the listener starts.
    ///
    /// # Errors
    /// Returns database, storage or setup failures.
    pub async fn open(path: &Path) -> AppResult<Self> {
        let settings = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3));
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(settings)
            .await?;
        sqlx::raw_sql(include_str!("import_staging/schema.sql"))
            .execute(&pool)
            .await?;
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        purge(&mut transaction, Utc::now().timestamp()).await?;
        transaction.commit().await?;
        Ok(Self(pool))
    }

    /// Returns a previously reserved batch before the current default folder is resolved.
    pub async fn replay(
        &self,
        owner: &UploadOwner<'_>,
        key: &str,
        manifest: &str,
    ) -> Result<Option<UploadAdmission>, UploadError> {
        let mut connection = self.0.acquire().await?;
        replay(
            &mut connection,
            owner,
            key,
            manifest,
            Utc::now().timestamp(),
        )
        .await
    }

    /// Reserves the entire batch before accepting bytes, with a second replay check inside
    /// the write transaction to serialize retries and quota decisions.
    pub async fn reserve(
        &self,
        owner: &UploadOwner<'_>,
        key: &str,
        canonical: &str,
        manifest: &ImportManifest,
        folder_id: &str,
    ) -> Result<UploadAdmission, UploadError> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let now = Utc::now().timestamp();
        if let Some(existing) = replay(&mut transaction, owner, key, canonical, now).await? {
            transaction.commit().await?;
            return Ok(existing);
        }
        purge(&mut transaction, now).await?;
        let (records, total, client_total): (i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN (expires > ?1 OR pending.batch_id IS NOT NULL) AND cancelled = 0
                    THEN reserved_bytes END), 0),
                COALESCE(SUM(CASE WHEN (expires > ?1 OR pending.batch_id IS NOT NULL)
                    AND cancelled = 0 AND catalog = ?2 AND owner = ?3
                    THEN reserved_bytes END), 0)
             FROM import_batches LEFT JOIN import_publication_claims AS pending ON pending.batch_id = import_batches.id",
        ).bind(now).bind(owner.catalog_id).bind(owner.client_id)
        .fetch_one(&mut *transaction).await?;
        let bytes = manifest
            .items
            .iter()
            .map(|item| item.byte_length as i64)
            .sum::<i64>();
        if records >= 128 || total + bytes > TOTAL_QUOTA || client_total + bytes > CLIENT_QUOTA {
            transaction.commit().await?;
            return Err(UploadError::Quota);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let expires = now + RECEIVE_SECONDS;
        sqlx::query(
            "INSERT INTO import_batches (id, catalog, owner, request_key, manifest, folder_id, expires, reserved_bytes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        ).bind(&id).bind(owner.catalog_id).bind(owner.client_id).bind(key)
        .bind(canonical).bind(folder_id).bind(expires).bind(bytes).execute(&mut *transaction).await?;
        transaction.commit().await?;
        admission(id, expires)
    }

    /// Gets the resolved destination of an active batch so a caller can recheck its grant before a write.
    pub async fn destination(
        &self,
        owner: &UploadOwner<'_>,
        batch: &str,
    ) -> Result<String, UploadError> {
        let mut connection = self.0.acquire().await?;
        let row = lookup(&mut connection, owner, batch).await?;
        check_active(&row)?;
        Ok(row.folder_id)
    }
}

#[derive(sqlx::FromRow)]
struct StoredBatch {
    manifest: String,
    folder_id: String,
    expires: i64,
    cancelled: bool,
}

async fn lookup(
    connection: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    batch: &str,
) -> Result<StoredBatch, UploadError> {
    let row = lookup_owned(connection, owner, batch).await?;
    if row.expires <= Utc::now().timestamp() {
        return Err(UploadError::Expired);
    }
    Ok(row)
}

async fn lookup_owned(
    connection: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    batch: &str,
) -> Result<StoredBatch, UploadError> {
    let row: StoredBatch = sqlx::query_as(
        "SELECT manifest, folder_id, expires, cancelled FROM import_batches
         WHERE id = ? AND catalog = ? AND owner = ?",
    )
    .bind(batch)
    .bind(owner.catalog_id)
    .bind(owner.client_id)
    .fetch_optional(connection)
    .await?
    .ok_or(UploadError::Missing)?;
    Ok(row)
}

fn check_active(batch: &StoredBatch) -> Result<(), UploadError> {
    if batch.cancelled {
        return Err(UploadError::Cancelled);
    }
    Ok(())
}

async fn replay(
    connection: &mut SqliteConnection,
    owner: &UploadOwner<'_>,
    key: &str,
    manifest: &str,
    now: i64,
) -> Result<Option<UploadAdmission>, UploadError> {
    let existing: Option<(String, String, i64, bool)> = sqlx::query_as(
        "SELECT id, manifest, expires, EXISTS(
            SELECT 1 FROM import_publication_claims WHERE batch_id = import_batches.id
         ) FROM import_batches WHERE catalog = ? AND owner = ? AND request_key = ?",
    )
    .bind(owner.catalog_id)
    .bind(owner.client_id)
    .bind(key)
    .fetch_optional(connection)
    .await?;
    let Some((id, original, expires, pending)) = existing else {
        return Ok(None);
    };
    if expires <= now {
        if !pending && expires + RECEIPT_SECONDS <= now {
            return Ok(None);
        }
        return Err(UploadError::Expired);
    }
    if manifest != original {
        return Err(UploadError::IdempotencyConflict);
    }
    Ok(Some(admission(id, expires)?))
}

async fn purge(connection: &mut SqliteConnection, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM import_parts WHERE batch_id IN (
        SELECT id FROM import_batches WHERE expires <= ?
        AND id NOT IN (SELECT batch_id FROM import_publication_claims)
    )",
    )
    .bind(now)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "DELETE FROM import_batches WHERE expires <= ?
        AND id NOT IN (SELECT batch_id FROM import_publication_claims)",
    )
    .bind(now - RECEIPT_SECONDS)
    .execute(connection)
    .await?;
    Ok(())
}

fn admission(batch_id: String, expires: i64) -> Result<UploadAdmission, UploadError> {
    let expires_at = DateTime::from_timestamp(expires, 0)
        .ok_or_else(|| AppError::Internal("Invalid staging expiry".into()))?;
    Ok(UploadAdmission {
        receipt_path: format!("/v1/imports/{batch_id}"),
        batch_id,
        part_size_bytes: PART_BYTES,
        expires_at,
    })
}

fn parse_manifest(manifest: &str) -> Result<ImportManifest, UploadError> {
    serde_json::from_str(manifest).map_err(|error| AppError::Internal(error.to_string()).into())
}

impl From<sqlx::Error> for UploadError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .and_then(|database| database.code())
            .as_deref()
            == Some("13")
        {
            Self::DiskFull
        } else {
            Self::Other(error.into())
        }
    }
}
