//! Atomic writes and keyset progress reads for staged parts.
use super::{
    check_active, lookup, parse_manifest,
    publication::{current_attempt, item_committed},
    SqliteImportStaging, StoredBatch,
};
use crate::core::{
    imports::staging::{
        ConfirmedPart, UploadError, UploadItem, UploadOwner, UploadReceipt, UploadState, LIMITS,
        PART_BYTES,
    },
    AppError,
};
use sqlx::SqliteConnection;

pub(super) const PARTS_PER_ITEM: i64 = (LIMITS.max_item_bytes / PART_BYTES as u64) as i64;

impl SqliteImportStaging {
    /// Durably accepts one part or confirms that the same digest was already accepted.
    pub async fn store(
        &self,
        owner: &UploadOwner<'_>,
        batch: &str,
        item_id: &str,
        part_number: u32,
        digest: &str,
        bytes: &[u8],
    ) -> Result<(), UploadError> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let row = lookup(&mut transaction, owner, batch).await?;
        check_active(&row)?;
        let manifest = parse_manifest(&row.manifest)?;
        let (position, item) = manifest
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| item.item_id == item_id)
            .ok_or(UploadError::Missing)?;
        if item_committed(&mut transaction, batch, item_id).await? {
            return Err(UploadError::PublicationComplete);
        }
        let offset = u64::from(part_number) * PART_BYTES as u64;
        if offset >= item.byte_length
            || bytes.len() as u64 != (item.byte_length - offset).min(PART_BYTES as u64)
        {
            return Err(UploadError::InvalidPart);
        }
        let sequence = position as i64 * PARTS_PER_ITEM + i64::from(part_number);
        let inserted = sqlx::query(
            "INSERT INTO import_parts (batch_id, sequence, item_id, part_number, digest, content)
             VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(batch_id, sequence) DO NOTHING",
        )
        .bind(batch)
        .bind(sequence)
        .bind(item_id)
        .bind(part_number)
        .bind(digest)
        .bind(bytes)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            let previous: String = sqlx::query_scalar(
                "SELECT digest FROM import_parts WHERE batch_id = ? AND sequence = ?",
            )
            .bind(batch)
            .bind(sequence)
            .fetch_one(&mut *transaction)
            .await?;
            if previous != digest {
                return Err(UploadError::PartConflict);
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Returns one page of confirmed parts; item status reflects all confirmed parts.
    pub async fn receipt(
        &self,
        owner: &UploadOwner<'_>,
        batch: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<UploadReceipt, UploadError> {
        let limit = i64::from(limit.unwrap_or(50));
        if !(1..=100).contains(&limit) {
            return Err(AppError::ValidationFailed("Invalid receipt page limit".into()).into());
        }
        let after = match cursor {
            Some(cursor) => cursor
                .parse::<i64>()
                .ok()
                .filter(|value| (0..LIMITS.max_items as i64 * PARTS_PER_ITEM).contains(value))
                .ok_or_else(|| AppError::ValidationFailed("Invalid receipt cursor".into()))?,
            None => -1,
        };
        let mut transaction = self.0.begin().await?;
        let row = lookup(&mut transaction, owner, batch).await?;
        let result = receipt(&mut transaction, batch, row, after, limit).await?;
        transaction.commit().await?;
        Ok(result)
    }

    /// Cancels the batch and frees its reservation and stored contents in one transaction.
    pub async fn cancel(
        &self,
        owner: &UploadOwner<'_>,
        batch: &str,
    ) -> Result<UploadReceipt, UploadError> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let mut row = lookup(&mut transaction, owner, batch).await?;
        if current_attempt(&mut transaction, batch).await?.is_some() {
            return Err(UploadError::PublicationPending);
        }
        sqlx::query("UPDATE import_batches SET cancelled = 1 WHERE id = ?")
            .bind(batch)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM import_parts WHERE batch_id = ?")
            .bind(batch)
            .execute(&mut *transaction)
            .await?;
        row.cancelled = true;
        let result = receipt(&mut transaction, batch, row, -1, 1).await?;
        transaction.commit().await?;
        Ok(result)
    }
}

async fn receipt(
    connection: &mut SqliteConnection,
    batch_id: &str,
    batch: StoredBatch,
    after: i64,
    limit: i64,
) -> Result<UploadReceipt, UploadError> {
    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_id, COUNT(*) FROM import_parts WHERE batch_id = ? GROUP BY item_id",
    )
    .bind(batch_id)
    .fetch_all(&mut *connection)
    .await?;
    let published: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT item_id, asset_id, file_name FROM import_published_items WHERE batch_id = ?",
    )
    .bind(batch_id)
    .fetch_all(&mut *connection)
    .await?;
    let active_attempt = current_attempt(connection, batch_id).await?;
    let mut page: Vec<(i64, String, u32)> = sqlx::query_as(
        "SELECT sequence, item_id, part_number FROM import_parts WHERE batch_id = ? AND sequence > ?
         ORDER BY sequence LIMIT ?",
    ).bind(batch_id).bind(after).bind(limit + 1).fetch_all(connection).await?;
    let mut next_cursor = None;
    if page.len() > limit as usize {
        page.pop();
        next_cursor = page.last().map(|(sequence, _, _)| sequence.to_string());
    }
    let items: Vec<_> = parse_manifest(&batch.manifest)?
        .items
        .into_iter()
        .map(|item| {
            let count = counts
                .iter()
                .find(|(id, _)| id == &item.item_id)
                .map_or(0, |(_, count)| *count as u64);
            let result = published.iter().find(|(id, _, _)| id == &item.item_id);
            let state = if result.is_some() {
                UploadState::Committed
            } else if active_attempt
                .as_ref()
                .is_some_and(|attempt| attempt.item_id == item.item_id)
            {
                UploadState::Publishing
            } else if batch.cancelled {
                UploadState::Cancelled
            } else if count == item.byte_length.div_ceil(PART_BYTES as u64) {
                UploadState::Ready
            } else {
                UploadState::Receiving
            };
            UploadItem {
                item_id: item.item_id,
                state,
                asset_id: result.map(|(_, id, _)| id.clone()),
                file_name: result.map(|(_, _, name)| name.clone()),
            }
        })
        .collect();
    let state = if items
        .iter()
        .all(|item| matches!(item.state, UploadState::Committed))
    {
        UploadState::Committed
    } else if active_attempt.is_some() {
        UploadState::Publishing
    } else if batch.cancelled {
        UploadState::Cancelled
    } else if items
        .iter()
        .all(|item| matches!(item.state, UploadState::Ready | UploadState::Committed))
    {
        UploadState::Ready
    } else {
        UploadState::Receiving
    };
    Ok(UploadReceipt {
        batch_id: batch_id.to_owned(),
        state,
        items,
        next_cursor,
        confirmed_parts: page
            .into_iter()
            .map(|(_, item_id, part_number)| ConfirmedPart {
                item_id,
                part_number,
            })
            .collect(),
    })
}
