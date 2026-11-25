//! Disposable, bounded SQLite staging. The content and its receipt share a FULL-synchronous
//! commit, so recovery never needs to infer whether independently written bytes survived.
use super::routes::Failure;
use chrono::{DateTime, Utc};
use mundam_lib::core::imports::ImportManifest;
use serde_json::{json, Value};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{path::Path, time::Duration};
use uuid::Uuid;

pub const PART_BYTES: usize = 256 * 1024;

pub async fn open(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    sqlx::raw_sql(
        "
        CREATE TABLE IF NOT EXISTS batches (
            id TEXT PRIMARY KEY, owner TEXT NOT NULL, key TEXT NOT NULL, manifest TEXT NOT NULL,
            expires INTEGER NOT NULL, UNIQUE(owner, key)
        );
        CREATE TABLE IF NOT EXISTS parts (
            batch TEXT NOT NULL REFERENCES batches(id), item TEXT NOT NULL, number INTEGER NOT NULL,
            digest TEXT NOT NULL, content BLOB NOT NULL, PRIMARY KEY(batch, item, number)
        );
    ",
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}

pub async fn admit(
    pool: &SqlitePool,
    owner: &str,
    key: &str,
    manifest: ImportManifest,
) -> Result<Value, Failure> {
    let canonical = serde_json::to_string(&manifest).map_err(|_| Failure::Internal)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let existing: Option<(String, String, i64)> =
        sqlx::query_as("SELECT id, manifest, expires FROM batches WHERE owner = ? AND key = ?")
            .bind(owner)
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?;
    let (batch, expires) = if let Some((batch, previous, expires)) = existing {
        if canonical != previous {
            return Err(Failure::Conflict("IDEMPOTENCY_CONFLICT"));
        }
        check_expiry(expires)?;
        (batch, expires)
    } else {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM batches WHERE owner = ?")
            .bind(owner)
            .fetch_one(&mut *tx)
            .await?;
        if count >= 4 {
            return Err(Failure::Capacity);
        }
        let batch = Uuid::new_v4().to_string();
        let expires = Utc::now().timestamp() + 3600;
        sqlx::query("INSERT INTO batches (id,owner,key,manifest,expires) VALUES (?,?,?,?,?)")
            .bind(&batch)
            .bind(owner)
            .bind(key)
            .bind(canonical)
            .bind(expires)
            .execute(&mut *tx)
            .await?;
        (batch, expires)
    };
    tx.commit().await?;
    let deadline = DateTime::<Utc>::from_timestamp(expires, 0).ok_or(Failure::Internal)?;
    Ok(
        json!({"batchId": batch, "partSizeBytes": PART_BYTES, "expiresAt": deadline,
        "receiptPath": format!("/v1/imports/{batch}")}),
    )
}

fn check_expiry(expires: i64) -> Result<(), Failure> {
    if expires <= Utc::now().timestamp() {
        return Err(Failure::Expired);
    }
    Ok(())
}

async fn lookup(
    connection: &mut sqlx::SqliteConnection,
    owner: &str,
    batch: &str,
) -> Result<ImportManifest, Failure> {
    let (manifest, expires): (String, i64) =
        sqlx::query_as("SELECT manifest, expires FROM batches WHERE owner = ? AND id = ?")
            .bind(owner)
            .bind(batch)
            .fetch_optional(connection)
            .await?
            .ok_or(Failure::Missing)?;
    check_expiry(expires)?;
    serde_json::from_str(&manifest).map_err(|_| Failure::Internal)
}

pub async fn store(
    pool: &SqlitePool,
    owner: &str,
    batch: &str,
    item: &str,
    number: u32,
    digest: &str,
    bytes: &[u8],
) -> Result<(), Failure> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let manifest = lookup(&mut tx, owner, batch).await?;
    let file = manifest
        .items
        .iter()
        .find(|entry| entry.item_id == item)
        .ok_or(Failure::Missing)?;
    let offset = u64::from(number) * PART_BYTES as u64;
    if offset >= file.byte_length
        || bytes.len() as u64 != (file.byte_length - offset).min(PART_BYTES as u64)
    {
        return Err(Failure::Invalid("INVALID_PART_SIZE"));
    }
    let insertion = sqlx::query(
        "INSERT INTO parts (batch,item,number,digest,content) VALUES (?,?,?,?,?)
                   ON CONFLICT(batch,item,number) DO NOTHING",
    )
    .bind(batch)
    .bind(item)
    .bind(number)
    .bind(digest)
    .bind(bytes)
    .execute(&mut *tx)
    .await?;
    if insertion.rows_affected() == 0 {
        let previous: String =
            sqlx::query_scalar("SELECT digest FROM parts WHERE batch=? AND item=? AND number=?")
                .bind(batch)
                .bind(item)
                .bind(number)
                .fetch_one(&mut *tx)
                .await?;
        if previous != digest {
            return Err(Failure::Conflict("PART_CONFLICT"));
        }
    }
    tx.commit().await?;
    Ok(())
}

pub async fn receipt(pool: &SqlitePool, owner: &str, batch: &str) -> Result<Value, Failure> {
    let mut tx = pool.begin().await?;
    let manifest = lookup(&mut tx, owner, batch).await?;
    let parts: Vec<(String, u32)> =
        sqlx::query_as("SELECT item, number FROM parts WHERE batch=? ORDER BY item, number")
            .bind(batch)
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    let mut confirmed = Vec::with_capacity(parts.len());
    let mut items = Vec::with_capacity(manifest.items.len());
    for item in manifest.items {
        let before = confirmed.len();
        for (_, number) in parts.iter().filter(|(id, _)| id == &item.item_id) {
            confirmed.push(json!({"itemId": item.item_id, "partNumber": number}));
        }
        let complete =
            (confirmed.len() - before) as u64 == item.byte_length.div_ceil(PART_BYTES as u64);
        items.push(
            json!({"itemId": item.item_id, "state": if complete {"ready"} else {"receiving"}}),
        );
    }
    Ok(
        json!({"batchId": batch, "state": "receiving", "items": items, "confirmedParts": confirmed, "nextCursor": null}),
    )
}
