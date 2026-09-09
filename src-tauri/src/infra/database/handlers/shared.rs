//! Shared infrastructure utilities used by all Ledger handlers.
//!
//! These free functions were extracted from `SqliteAssetLedger` to break the
//! compile-time coupling between individual handlers and the parent Ledger module.
//! Each handler can now be compiled and unit-tested independently by importing
//! only this module, without pulling the entire `ledger` dependency graph.
use crate::core::error::{AppError, AppResult};
use crate::core::models::asset::Asset;
use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use tracing::{info, warn};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

/// Records an operation in the `asset_operations_log` audit table within the
/// current transaction boundary.
///
/// This function is the backbone of the Saga/Outbox pattern: every mutating
/// handler writes its intent here before the transaction commits. If the status
/// is `PENDING`, the `SagaRecoveryService` will attempt to complete the
/// associated filesystem operation on next startup.
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `operation_type` - A short label identifying the mutation (e.g. `DELETE_ASSET`).
/// * `asset_id` - The entity targeted by the operation.
/// * `payload` - Full JSON payload for recovery context and post-mortem debugging.
/// * `status` - Initial status (`PENDING`, `COMPLETED`, or `FAILED`).
/// * `error_note` - Optional error description when the status is `FAILED`.
///
/// # Errors
///
/// Returns `AppError` if the INSERT into `asset_operations_log` fails.
pub async fn log_operation(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_type: &str,
    asset_id: &str,
    payload: serde_json::Value,
    status: &str,
    error_note: Option<&str>,
) -> AppResult<()> {
    let operation_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO asset_operations_log (id, operation_type, asset_id, payload, status, error_note, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        operation_id,
        operation_type,
        asset_id,
        payload,
        status,
        error_note,
        now
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

/// Updates the status and error note of an existing operation in the `asset_operations_log`.
///
/// This is used by the post-commit Saga step to transition an operation from `PENDING`
/// to either `COMPLETED` or `FAILED`.
///
/// # Arguments
///
/// * `pool` - The database connection pool (executes outside the main transaction).
/// * `asset_id` - The asset associated with the operation.
/// * `operation_type` - The type of operation (e.g. `DELETE_ASSET`).
/// * `status` - The new status (`COMPLETED` or `FAILED`).
/// * `error_note` - Optional error message if the status is `FAILED`.
pub async fn update_operation_status(
    pool: &sqlx::SqlitePool,
    asset_id: &str,
    operation_type: &str,
    status: &str,
    error_note: Option<&str>,
) -> AppResult<()> {
    sqlx::query!(
        "UPDATE asset_operations_log SET status = ?, error_note = ? WHERE asset_id = ? AND status = 'PENDING' AND operation_type = ?",
        status,
        error_note,
        asset_id,
        operation_type
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetches a complete `Asset` entity by its unique ID within the current transaction.
///
/// Performs a LEFT JOIN with `asset_metadata_envelope` to include technical metadata
/// (dimensions, duration, JSON payloads) in the returned entity. This is the canonical
/// way for handlers to return a fully hydrated `Asset` after mutation.
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `asset_id` - The unique identifier of the asset to fetch.
///
/// # Errors
///
/// Returns `AppError::NotFound` if no asset exists with the given ID.
pub async fn fetch_asset_by_id(
    transaction: &mut Transaction<'_, Sqlite>,
    asset_id: &str,
) -> AppResult<Asset> {
    let row = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            a.rating as "rating: i64",
            a.notes as "notes?",
            ame.width as "width: i64",
            ame.height as "height: i64",
            ame.duration_secs as "duration_secs: f64",
            ame.technical_payload as "technical_payload: serde_json::Value",
            ame.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>"
        FROM assets a
        LEFT JOIN asset_metadata_envelope ame ON a.id = ame.asset_id
        WHERE a.id = ?
        "#,
        asset_id
    )
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(|| AppError::NotFound(asset_id.to_string()))?;

    let asset_db = crate::infra::database::models::AssetDb {
        id: row.id,
        name: row.name,
        path: row.path,
        state: row.state,
        format_type: row.format_type,
        family: row.family,
        file_size: row.file_size,
        created_at: row.created_at,
        modified_at: row.modified_at,
        added_at: row.added_at,
        updated_at: row.updated_at,
        folder_id: row.folder_id,
        thumbnail_path: row.thumbnail_path,
        rating: row.rating,
        notes: row.notes,
        is_favorite: row.is_favorite,
        deleted_at: row.deleted_at,
        width: row.width,
        height: row.height,
        duration_secs: row.duration_secs,
        technical_payload: row.technical_payload,
        semantic_payload: row.semantic_payload,
        dominant_color: row.dominant_color,
    };

    Ok(asset_db.into())
}

/// Robustly resolves an asset ID from a filesystem path, handling macOS NFC/NFD
/// Unicode normalization mismatches.
///
/// First attempts a direct `COLLATE NOCASE` path match. If that fails, falls back
/// to a two-step resolution: find the parent folder by path, then find the asset
/// by name within that folder. This guarantees correct resolution even when the OS
/// reports paths in NFD while the database stores them in NFC (or vice-versa).
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `path` - The filesystem path to resolve.
///
/// # Errors
///
/// Returns `AppError` only on database query failure, not on "not found" (returns `None`).
pub async fn resolve_asset_id_robust(
    transaction: &mut Transaction<'_, Sqlite>,
    path: &std::path::Path,
) -> AppResult<Option<String>> {
    let path_string = path.to_string_lossy().to_string();

    let row = sqlx::query!(
        r#"SELECT id as "id!" FROM assets WHERE path = ? COLLATE NOCASE"#,
        path_string
    )
    .fetch_optional(&mut **transaction)
    .await?;

    if let Some(result) = row {
        return Ok(Some(result.id));
    }

    info!(
        "Ledger: Direct path match FAILED for '{}'. Attempting robust folder+name resolution...",
        path_string
    );

    if let (Some(parent_path), Some(name)) = (path.parent(), path.file_name()) {
        let name_string = name.to_string_lossy().to_string();

        if let Some(folder_id) =
            Box::pin(resolve_folder_id_robust(transaction, parent_path)).await?
        {
            let asset_row = sqlx::query!(
                r#"SELECT id as "id!" FROM assets WHERE folder_id = ? AND name = ? COLLATE NOCASE"#,
                folder_id,
                name_string
            )
            .fetch_optional(&mut **transaction)
            .await?;

            if let Some(asset_result) = asset_row {
                info!(
                    "Ledger: Robust resolution SUCCESS. Found asset ID {} via folder {} + name '{}'",
                    asset_result.id, folder_id, name_string
                );
                return Ok(Some(asset_result.id));
            } else {
                warn!(
                    "Ledger: Robust resolution FAILED: Asset '{}' not found in folder ID {} (Normalization mismatch?)",
                    name_string, folder_id
                );
            }
        } else {
            warn!(
                "Ledger: Robust resolution FAILED: Could not resolve folder ID for path '{}'",
                parent_path.display()
            );
        }
    }

    Ok(None)
}

/// Robustly resolves a folder ID by filesystem path, handling NFC/NFD normalization.
///
/// Uses the same two-step strategy as `resolve_asset_id_robust`: direct path match
/// first, then recursive parent-ID + name match as fallback. The recursion is bounded
/// by the filesystem path depth.
///
/// # Arguments
///
/// * `transaction` - The active database transaction.
/// * `path` - The filesystem path of the folder to resolve.
///
/// # Errors
///
/// Returns `AppError` only on database query failure, not on "not found" (returns `None`).
pub async fn resolve_folder_id_robust(
    transaction: &mut Transaction<'_, Sqlite>,
    path: &std::path::Path,
) -> AppResult<Option<String>> {
    let path_string = path.to_string_lossy().to_string();

    let row = sqlx::query!(
        r#"SELECT id as "id!" FROM folders WHERE path = ? COLLATE NOCASE"#,
        path_string
    )
    .fetch_optional(&mut **transaction)
    .await?;

    if let Some(result) = row {
        return Ok(Some(result.id));
    }

    if let (Some(parent_path), Some(name)) = (path.parent(), path.file_name()) {
        let name_string = name.to_string_lossy().to_string();

        if let Some(parent_id) =
            Box::pin(resolve_folder_id_robust(transaction, parent_path)).await?
        {
            let folder_row = sqlx::query!(
                r#"SELECT id as "id!" FROM folders WHERE parent_id = ? AND name = ? COLLATE NOCASE"#,
                parent_id,
                name_string
            )
            .fetch_optional(&mut **transaction)
            .await?;

            if let Some(folder_result) = folder_row {
                info!(
                    "Ledger: Robust folder resolution SUCCESS. Resolved '{}' -> ID {}",
                    path_string, folder_result.id
                );
                return Ok(Some(folder_result.id));
            }
        }
    }

    Ok(None)
}

/// Performs a one-time database-wide path normalization to NFC.
///
/// Resolves legacy "ghost records" on macOS where paths were stored in NFD
/// by the filesystem watcher before the NFC normalization was enforced.
/// This function should be called once at startup, before any other operations.
///
/// # Arguments
///
/// * `pool` - The database connection pool.
///
/// # Errors
///
/// Returns `AppError` if the database transaction or any UPDATE query fails.
pub async fn normalize_database_paths(pool: &sqlx::SqlitePool) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let now = Utc::now();

    info!("Ledger: Starting database path normalization (NFC)...");

    let asset_rows = sqlx::query!(r#"SELECT id, path FROM assets"#)
        .fetch_all(&mut *transaction)
        .await?;

    let mut asset_fix_count = 0;
    for row in asset_rows {
        let nfc_path = row.path.nfc().collect::<String>();
        if nfc_path != row.path {
            sqlx::query!(
                "UPDATE assets SET path = ?, updated_at = ? WHERE id = ?",
                nfc_path,
                now,
                row.id
            )
            .execute(&mut *transaction)
            .await?;
            asset_fix_count += 1;
        }
    }

    let folder_rows = sqlx::query!(r#"SELECT id, path FROM folders"#)
        .fetch_all(&mut *transaction)
        .await?;

    let mut folder_fix_count = 0;
    for row in folder_rows {
        let nfc_path = row.path.nfc().collect::<String>();
        if nfc_path != row.path {
            sqlx::query!(
                "UPDATE folders SET path = ?, updated_at = ? WHERE id = ?",
                nfc_path,
                now,
                row.id
            )
            .execute(&mut *transaction)
            .await?;
            folder_fix_count += 1;
        }
    }

    transaction.commit().await?;

    if asset_fix_count > 0 || folder_fix_count > 0 {
        info!(
            "Ledger: Path normalization COMPLETED. Fixed {} assets and {} folders.",
            asset_fix_count, folder_fix_count
        );
    } else {
        info!("Ledger: Database is already normalized.");
    }

    Ok(())
}
