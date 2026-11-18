//! Awaited text transfers used by theme import/export and taxonomy export.

use crate::core::error::AppResult;
use std::path::Path;

/// Reads the selected file to completion.
/// # Errors
/// Propagates filesystem and UTF-8 decoding errors.
pub async fn read(path: &Path) -> AppResult<String> {
    Ok(tokio::fs::read_to_string(path).await?)
}

/// Writes the selected destination to completion under the command's admission scope.
/// # Errors
/// Propagates filesystem errors; the caller retains the existing overwrite semantics.
pub async fn write(path: &Path, contents: &str) -> AppResult<()> {
    Ok(tokio::fs::write(path, contents).await?)
}
