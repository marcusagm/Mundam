//! Creates the app-owned recovery directory only on an explicit advanced-user request.
use crate::core::error::AppResult;
use std::path::{Path, PathBuf};

/// Returns an existing recovery directory for the settings panel.
///
/// # Errors
/// Returns filesystem errors when the app data directory is unavailable.
pub async fn directory(app_data: &Path) -> AppResult<PathBuf> {
    let directory = app_data.join("deleted-folders");
    tokio::fs::create_dir_all(&directory).await?;
    Ok(directory)
}
