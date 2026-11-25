//! Keeps import directories anchored across the catalog transaction and recovery.
#[cfg(windows)]
mod windows;

use crate::core::{library::FileIdentity, AppError, AppResult};
use std::path::Path;

/// Native identities recorded before the first move and checked on recovery.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(crate) struct ImportParents {
    pub source: FileIdentity,
    pub destination: FileIdentity,
}

/// The selected parents and their Windows directory handles.
pub(crate) struct ImportDirectories {
    pub parents: ImportParents,
    #[cfg(windows)]
    native: std::sync::Arc<windows::Directories>,
}

impl ImportDirectories {
    /// Opens the two directory paths and holds the Windows handles until this guard drops.
    ///
    /// # Errors
    /// Returns invalid paths, redirected directories, conflicts or native IO failures.
    pub(crate) async fn open(source: &Path, destination: &Path) -> AppResult<Self> {
        let source_parent = parent(source)?;
        let destination_parent = parent(destination)?;
        #[cfg(windows)]
        {
            let source_parent = source_parent.to_path_buf();
            let destination_parent = destination_parent.to_path_buf();
            let native = tokio::task::spawn_blocking(move || {
                windows::Directories::open(&source_parent, &destination_parent)
            })
            .await
            .map_err(|error| AppError::Internal(error.to_string()))??;
            let parents = native.identities()?;
            Ok(Self {
                parents,
                native: std::sync::Arc::new(native),
            })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {
                parents: ImportParents {
                    source: super::identity::read(source_parent).await?,
                    destination: super::identity::read(destination_parent).await?,
                },
            })
        }
    }

    /// Moves the verified source entry to the selected directory without overwriting an entry.
    ///
    /// # Errors
    /// Returns identity changes, destination conflicts and native IO failures.
    pub(crate) async fn move_file(
        &self,
        source: &Path,
        destination: &Path,
        expected: &FileIdentity,
        reverse: bool,
    ) -> AppResult<()> {
        #[cfg(windows)]
        {
            let source = source.file_name().ok_or_else(invalid_path)?.to_os_string();
            let destination = destination
                .file_name()
                .ok_or_else(invalid_path)?
                .to_os_string();
            let native = self.native.clone();
            let expected = expected.clone();
            tokio::task::spawn_blocking(move || {
                native.move_file(&source, &destination, &expected, reverse)
            })
            .await
            .map_err(|error| AppError::Internal(error.to_string()))??;
        }
        #[cfg(not(windows))]
        {
            let (source_parent, destination_parent) = if reverse {
                (&self.parents.destination, &self.parents.source)
            } else {
                (&self.parents.source, &self.parents.destination)
            };
            if &super::identity::read(parent(source)?).await? != source_parent
                || &super::identity::read(parent(destination)?).await? != destination_parent
                || &super::identity::read(source).await? != expected
            {
                return Err(AppError::ValidationFailed(
                    "Import move paths changed".into(),
                ));
            }
            super::exclusive_move::move_path(source, destination).await?;
        }
        Ok(())
    }
}

fn parent(path: &Path) -> AppResult<&Path> {
    if !path.is_absolute() {
        return Err(invalid_path());
    }
    path.parent()
        .filter(|_| path.file_name().is_some())
        .ok_or_else(invalid_path)
}

fn invalid_path() -> AppError {
    AppError::ValidationFailed("Import moves require absolute file paths".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn import_move_pins_ancestors_and_never_replaces_a_destination(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = tempfile::tempdir()?;
        let ancestor = storage.path().join("ancestor");
        let nested = ancestor.join("nested");
        tokio::fs::create_dir_all(&nested).await?;
        let prepared = storage.path().join("prepared");
        let target = nested.join("cover.png");
        let renamed = storage.path().join("renamed");
        tokio::fs::write(&prepared, b"staged content").await?;
        tokio::fs::write(&target, b"original content").await?;
        let identity = super::super::identity::read(&prepared).await?;
        let guard = ImportDirectories::open(&prepared, &target).await?;
        assert!(tokio::fs::rename(&ancestor, &renamed).await.is_err());
        assert!(guard
            .move_file(&prepared, &target, &identity, false)
            .await
            .is_err());
        assert_eq!(tokio::fs::read(&target).await?, b"original content");
        assert_eq!(tokio::fs::read(&prepared).await?, b"staged content");
        let free_target = nested.join("success.png");
        guard
            .move_file(&prepared, &free_target, &identity, false)
            .await?;
        assert_eq!(tokio::fs::read(&free_target).await?, b"staged content");
        drop(guard);
        tokio::fs::rename(&ancestor, &renamed).await?;
        assert_eq!(
            tokio::fs::read(renamed.join("nested/success.png")).await?,
            b"staged content"
        );
        Ok(())
    }
}
