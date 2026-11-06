//! Native and catalog validation stays inside the owning transaction; pure name policy lives in Core.
use crate::core::error::{AppError, AppResult};
use sqlx::{Sqlite, Transaction};
use std::path::{Path, PathBuf};
pub(super) fn invalid(message: &str) -> AppError {
    AppError::ValidationFailed(message.into())
}

/// Reads catalog ownership and validates the live directory in the same write transaction.
///
/// # Errors
/// Returns missing, offline, symbolic-link or inaccessible folders.
pub(super) async fn folder(
    transaction: &mut Transaction<'_, Sqlite>,
    identifier: &str,
) -> AppResult<(PathBuf, Option<String>)> {
    let (path, parent): (String, Option<String>) =
        sqlx::query_as("SELECT path, parent_id FROM folders WHERE id = ?")
            .bind(identifier)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("Folder not found".into()))?;
    let path = PathBuf::from(path);
    let metadata = tokio::fs::symlink_metadata(&path).await?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("Folder is unavailable or is a symbolic link"));
    }
    Ok((path, parent))
}

/// Rejects occupied disk destinations and exact catalog collisions inside the write transaction.
///
/// # Errors
/// Returns occupied destinations and native/catalog inspection failures.
pub(crate) async fn available(
    transaction: &mut Transaction<'_, Sqlite>,
    path: &Path,
) -> AppResult<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => {
            return Err(invalid(
                "A file or folder already exists at the destination",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let occupied: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM assets WHERE path = ?) + (SELECT COUNT(*) FROM folders WHERE path = ?)")
        .bind(path.to_string_lossy().as_ref()).bind(path.to_string_lossy().as_ref()).fetch_one(&mut **transaction).await?;
    if occupied > 0 {
        return Err(invalid("Destination is already registered in the library"));
    }
    Ok(())
}
