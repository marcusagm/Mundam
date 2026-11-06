//! Explicit archive access and bounded inventory keep filesystem details behind Core ports.
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

/// Measures logical archive bytes without following symbolic links or blocking Tokio.
///
/// # Errors
/// Returns inaccessible entries, task failures or an inventory exceeding one million entries.
pub(crate) async fn measure(path: &Path) -> AppResult<i64> {
    let directory = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut bytes = 0i64;
        for (index, entry) in walkdir::WalkDir::new(directory)
            .follow_links(false)
            .into_iter()
            .enumerate()
        {
            if index >= 1_000_000 {
                return Err(crate::core::error::AppError::ValidationFailed(
                    "Folder exceeds the recovery inventory budget".into(),
                ));
            }
            let entry =
                entry.map_err(|error| crate::core::error::AppError::Internal(error.to_string()))?;
            if entry.file_type().is_file() {
                bytes = bytes.saturating_add(
                    i64::try_from(
                        entry
                            .metadata()
                            .map_err(|error| {
                                crate::core::error::AppError::Internal(error.to_string())
                            })?
                            .len(),
                    )
                    .unwrap_or(i64::MAX),
                );
            }
        }
        Ok(bytes)
    })
    .await
    .map_err(|error| crate::core::error::AppError::Internal(error.to_string()))?
}

/// App-local storage adapter shared by settings and recovery orchestration.
pub struct FolderRecoveryStorage(pub PathBuf);

#[async_trait::async_trait]
impl crate::core::library::RecoveryStorage for FolderRecoveryStorage {
    async fn directory(&self) -> AppResult<PathBuf> {
        directory(&self.0).await
    }
}
