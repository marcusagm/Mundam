//! Owns the private, UUID-addressed files created for staging publication attempts.
use crate::core::{imports::publication::volume_temporary_name, AppError, AppResult};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tokio::fs::{File, OpenOptions};
use uuid::Uuid;

const DIRECTORY: &str = "import-prepared";

/// Derives the only filename accepted for a destination-volume temporary.
pub(crate) fn volume_path(directory: &Path, id: Uuid) -> PathBuf {
    directory.join(volume_temporary_name(id))
}

/// Keeps the authorized directory chain open while its temporary is being written.
pub(crate) struct VolumeDirectoryLease {
    #[cfg(windows)]
    _directory: super::import_move::windows::DirectoryChain,
}

/// Opens an exclusive attempt file relative to the authorized destination handle.
///
/// # Errors
/// Returns conflicts, invalid directory identity and native storage errors.
#[cfg(windows)]
pub(crate) async fn create_on_volume(
    directory: &crate::core::imports::publication::ImportPreparationDestination,
    id: Uuid,
) -> AppResult<(
    File,
    crate::core::library::FileIdentity,
    VolumeDirectoryLease,
)> {
    let directory = directory.clone();
    let (file, identity, directory) = tokio::task::spawn_blocking(move || {
        let anchor = super::import_move::windows::DirectoryChain::open(&directory.path)?;
        if anchor.identity()? != directory.identity {
            return Err(AppError::ValidationFailed(
                "Import destination identity changed during preparation".into(),
            ));
        }
        let (file, identity) = anchor.create_temporary(volume_temporary_name(id).as_ref())?;
        Ok::<_, AppError>((file, identity, VolumeDirectoryLease { _directory: anchor }))
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;
    Ok((File::from_std(file), identity, directory))
}

/// Deletes a destination-volume attempt only with its persisted native identity.
///
/// # Errors
/// Returns unresolved ownership, directory changes and native storage errors.
#[cfg(windows)]
pub(crate) async fn remove_on_volume(
    directory: &crate::core::imports::publication::ImportPreparationDestination,
    id: Uuid,
    expected: Option<&crate::core::library::FileIdentity>,
) -> AppResult<()> {
    let directory = directory.clone();
    let expected = expected.cloned();
    tokio::task::spawn_blocking(move || {
        let anchor = super::import_move::windows::DirectoryChain::open(&directory.path)?;
        if anchor.identity()? != directory.identity {
            return Err(AppError::ValidationFailed(
                "Import destination identity changed during cleanup".into(),
            ));
        }
        Ok(anchor.cleanup_temporary(volume_temporary_name(id).as_ref(), expected.as_ref())?)
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
}

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

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::core::imports::publication::{is_volume_temporary, ImportPreparationDestination};
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn import_temporary_cleanup_requires_exact_ownership(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = tempfile::tempdir()?;
        let directory = ImportPreparationDestination {
            path: storage.path().to_path_buf(),
            identity: super::super::identity::read(storage.path()).await?,
        };
        let id = Uuid::new_v4();
        let path = volume_path(&directory.path, id);
        assert!(is_volume_temporary(&path));
        assert!(!is_volume_temporary(Path::new(
            ".mundam-import-example.part"
        )));
        let (mut file, identity, _directory_lease) = create_on_volume(&directory, id).await?;
        file.write_all(b"staged content").await?;
        file.sync_all().await?;
        drop(file);
        assert!(create_on_volume(&directory, id).await.is_err());
        assert!(remove_on_volume(&directory, id, None).await.is_err());
        assert!(remove_on_volume(&directory, id, Some(&directory.identity))
            .await
            .is_err());
        assert_eq!(tokio::fs::read(&path).await?, b"staged content");
        remove_on_volume(&directory, id, Some(&identity)).await?;
        remove_on_volume(&directory, id, Some(&identity)).await?;
        assert!(!path.try_exists()?);
        Ok(())
    }
}
