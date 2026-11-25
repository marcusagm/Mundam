//! Bounded import staging policy and receipts, independent of HTTP and SQLite.
use super::ImportLimits;
use crate::core::AppError;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Binary part size selected by the host.
pub const PART_BYTES: usize = 256 * 1024;
/// Initial staging limits. Publication applies its own format and destination checks.
pub const LIMITS: ImportLimits = ImportLimits {
    max_items: 8,
    max_tags_per_item: 32,
    max_item_bytes: 16 * 1024 * 1024,
    max_batch_bytes: 32 * 1024 * 1024,
};

/// Identifies a batch without sharing ownership across catalogs or clients.
pub struct UploadOwner<'a> {
    pub catalog_id: &'a str,
    pub client_id: &'a str,
}

/// Instructions returned after a reservation is persisted.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadAdmission {
    pub batch_id: String,
    pub part_size_bytes: usize,
    pub expires_at: DateTime<Utc>,
    pub receipt_path: String,
}

/// Owner-visible progress of a staging batch.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadReceipt {
    pub batch_id: String,
    pub state: UploadState,
    pub items: Vec<UploadItem>,
    pub confirmed_parts: Vec<ConfirmedPart>,
    pub next_cursor: Option<String>,
}

/// Staging states returned for a batch or an item.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadState {
    Receiving,
    Ready,
    Cancelled,
}

/// Progress of one declared asset.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadItem {
    pub item_id: String,
    pub state: UploadState,
}

/// A part whose content and digest were committed together.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedPart {
    pub item_id: String,
    pub part_number: u32,
}

/// Staging outcomes that the transport can map to stable response codes.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("Import batch or item unavailable")]
    Missing,
    #[error("Import batch expired")]
    Expired,
    #[error("Import batch was cancelled")]
    Cancelled,
    #[error("The idempotency key was already used with another manifest")]
    IdempotencyConflict,
    #[error("The part already has different content")]
    PartConflict,
    #[error("The part number or length does not match the manifest")]
    InvalidPart,
    #[error("Import staging quota is full")]
    Quota,
    #[error("Import staging ran out of disk space")]
    DiskFull,
    #[error(transparent)]
    Other(#[from] AppError),
}
