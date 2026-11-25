//! Owns the private, UUID-addressed files created for staging publication attempts.
use crate::core::{AppError, AppResult};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tokio::fs::{File, OpenOptions};
use uuid::Uuid;

const DIRECTORY: &str = "import-prepared";

/// Derives the path used by the host and the Ledger for this attempt.
pub(crate) fn path(app_data: &Path, id: Uuid) -> PathBuf {
    app_data.join(DIRECTORY).join(id.to_string())
}

/// Opens an exclusive temporary file in validated private storage.
///
/// # Errors
/// Returns filesystem failures, conflicting entries and redirected-directory failures.
pub(crate) async fn create(app_data: &Path, id: Uuid) -> AppResult<File> {
    let directory = app_data.join(DIRECTORY);
    let mut builder = tokio::fs::DirBuilder::new();
    builder.recursive(false);
    #[cfg(unix)]
    builder.mode(0o700);
    match builder.create(&directory).await {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    verify_directory(&directory).await?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    Ok(options.open(path(app_data, id)).await?)
}

/// Synchronizes prepared content before the Ledger can move it.
///
/// # Errors
/// Returns failures synchronizing the file or its directory entries.
pub(crate) async fn sync(file: &File, _app_data: &Path) -> AppResult<()> {
    file.sync_all().await?;
    #[cfg(unix)]
    {
        File::open(_app_data.join(DIRECTORY))
            .await?
            .sync_all()
            .await?;
        File::open(_app_data).await?.sync_all().await?;
    }
    Ok(())
}

/// Clears a single attempt file after the journal has settled.
///
/// # Errors
/// Rejects redirected storage and entries that are not regular files.
pub(crate) async fn remove(app_data: &Path, id: Uuid) -> AppResult<()> {
    let directory = app_data.join(DIRECTORY);
    match verify_directory(&directory).await {
        Ok(()) => {}
        Err(AppError::Io(error)) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    let path = path(app_data, id);
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Err(AppError::ValidationFailed(
            "Import temporary is not a regular file".into(),
        ));
    }
    super::identity::read(&path).await?;
    tokio::fs::remove_file(path).await?;
    #[cfg(unix)]
    File::open(directory).await?.sync_all().await?;
    Ok(())
}

async fn verify_directory(directory: &Path) -> AppResult<()> {
    if !tokio::fs::symlink_metadata(directory).await?.is_dir() {
        return Err(AppError::ValidationFailed(
            "Import temporary storage is not a directory".into(),
        ));
    }
    super::identity::read(directory).await?;
    Ok(())
}
