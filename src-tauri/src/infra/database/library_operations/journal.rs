//! Durable intent files bridge the crash window between native moves and SQLite commit.
use super::{move_without_replace, DiskChanges};
use crate::core::{
    error::{AppError, AppResult},
    library::FileIdentity,
};
use crate::infra::filesystem::import_move::{ImportDirectories, ImportParents};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

#[derive(Serialize, Deserialize)]
struct Movement {
    source: PathBuf,
    destination: PathBuf,
    identity: FileIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    import_content: Option<ImportContent>,
}

#[derive(Serialize, Deserialize)]
struct ImportContent {
    length: u64,
    sha256: [u8; 32],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parents: Option<ImportParents>,
}

impl Movement {
    async fn verified_identity(&self, path: &Path) -> AppResult<FileIdentity> {
        let actual = crate::infra::filesystem::identity::read(path).await?;
        if actual == self.identity {
            return Ok(actual);
        }
        if cfg!(windows)
            && actual.device == self.identity.device
            && actual.object == self.identity.object
        {
            if let Some(content) = &self.import_content {
                if verifies_import_bytes(path, content.length, content.sha256).await?
                    && crate::infra::filesystem::identity::read(path).await? == actual
                {
                    return Ok(actual);
                }
            }
        }
        Err(AppError::ValidationFailed(
            "Moved file does not match the prepared identity".into(),
        ))
    }
}

async fn verifies_import_bytes(path: &Path, length: u64, sha256: [u8; 32]) -> AppResult<bool> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;
    if length > crate::core::imports::staging::LIMITS.max_item_bytes
        || !tokio::fs::symlink_metadata(path).await?.is_file()
    {
        return Ok(false);
    }
    let mut file = tokio::fs::File::open(path).await?.take(length + 1);
    let mut digest = Sha256::new();
    let mut remaining = length;
    let mut chunk = vec![0; 64 * 1024];
    loop {
        let count = file.read(&mut chunk).await?;
        if count == 0 {
            return Ok(remaining == 0 && <[u8; 32]>::from(digest.finalize()) == sha256);
        }
        let Some(left) = remaining.checked_sub(count as u64) else {
            return Ok(false);
        };
        remaining = left;
        digest.update(&chunk[..count]);
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct Journal {
    version: u32,
    pub operation_id: String,
    movements: Vec<Movement>,
    creation: Option<PathBuf>,
    pub creation_identity: Option<FileIdentity>,
    #[serde(default)]
    removal: Option<PathBuf>,
}

impl Journal {
    /// Rejects a prepared movement whose recorded source differs from the inspected file.
    ///
    /// # Errors
    /// Returns a validation error before any native move if source ownership has changed.
    pub(super) fn require_source_identity(
        &self,
        index: usize,
        expected: &FileIdentity,
    ) -> AppResult<()> {
        if self.movements.get(index).map(|movement| &movement.identity) != Some(expected) {
            return Err(AppError::ValidationFailed(
                "Prepared file identity changed".into(),
            ));
        }
        Ok(())
    }

    /// Checks source ownership again after preparation and before executing a movement.
    ///
    /// # Errors
    /// Returns missing, replaced or inaccessible sources without moving an unrelated entry.
    pub async fn verify_source(&self, index: usize) -> AppResult<()> {
        let movement = self
            .movements
            .get(index)
            .ok_or_else(|| AppError::Internal("Missing journal movement".into()))?;
        if crate::infra::filesystem::identity::read(&movement.source).await? != movement.identity {
            return Err(AppError::ValidationFailed(
                "Source identity changed during library preparation".into(),
            ));
        }
        Ok(())
    }
    /// Flushes a versioned plan before the first native effect.
    ///
    /// # Errors
    /// Returns identity, serialization and durable-storage failures.
    pub async fn prepare(app_data: &Path, changes: &DiskChanges) -> AppResult<Self> {
        Self::prepare_movements(app_data, changes, None).await
    }

    /// Persists content evidence for a single import move before it can touch the destination.
    ///
    /// # Errors
    /// Returns validation, identity or durable-storage failures.
    pub(super) async fn prepare_import(
        app_data: &Path,
        changes: &DiskChanges,
        length: u64,
        sha256: [u8; 32],
        parents: ImportParents,
    ) -> AppResult<Self> {
        if changes.moves.len() != 1 {
            return Err(AppError::Internal(
                "An import journal requires exactly one movement".into(),
            ));
        }
        Self::prepare_movements(
            app_data,
            changes,
            Some(ImportContent {
                length,
                sha256,
                parents: Some(parents),
            }),
        )
        .await
    }

    /// Reads the final identity for the catalog, accounting for NTFS timestamp inheritance.
    ///
    /// # Errors
    /// Rejects an unverifiable destination or missing journal movement.
    pub(super) async fn published_import_identity(&self) -> AppResult<FileIdentity> {
        let movement = self
            .movements
            .first()
            .ok_or_else(|| AppError::Internal("Missing import movement".into()))?;
        movement.verified_identity(&movement.destination).await
    }

    async fn prepare_movements(
        app_data: &Path,
        changes: &DiskChanges,
        mut import_content: Option<ImportContent>,
    ) -> AppResult<Self> {
        let mut movements = Vec::new();
        for (source, destination) in &changes.moves {
            let identity = match movements
                .iter()
                .find(|movement: &&Movement| movement.destination == *source)
            {
                Some(previous) => previous.identity.clone(),
                None => crate::infra::filesystem::identity::read(source).await?,
            };
            movements.push(Movement {
                source: source.clone(),
                destination: destination.clone(),
                identity,
                import_content: import_content.take(),
            });
        }
        let journal = Self {
            version: 1,
            operation_id: uuid::Uuid::new_v4().to_string(),
            movements,
            creation: changes.create.clone(),
            creation_identity: None,
            removal: changes.remove_empty.clone(),
        };
        journal.persist(app_data).await?;
        Ok(journal)
    }

    /// Records ownership after creation so rollback never recursively removes external files.
    ///
    /// # Errors
    /// Returns ownership-read or durable-storage failures; ambiguous gaps require repair.
    pub async fn record_creation(&mut self, app_data: &Path) -> AppResult<()> {
        if let Some(path) = &self.creation {
            self.creation_identity = Some(crate::infra::filesystem::identity::read(path).await?);
            self.persist(app_data).await?;
        }
        Ok(())
    }

    async fn persist(&self, app_data: &Path) -> AppResult<()> {
        let directory = app_data.join("library-journal");
        let path = directory.join(format!("{}.json", self.operation_id));
        let bytes =
            serde_json::to_vec(self).map_err(|error| AppError::Internal(error.to_string()))?;
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            use std::io::Write;
            std::fs::create_dir_all(&directory)?;
            let temporary = path.with_extension("pending");
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            // Replacement is limited to this operation's own manifest, never original media.
            std::fs::rename(temporary, path)?;
            #[cfg(unix)]
            {
                std::fs::File::open(&directory)?.sync_all()?;
                if let Some(parent) = directory.parent() {
                    std::fs::File::open(parent)?.sync_all()?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
        Ok(())
    }

    /// Attempts every independent inverse movement without replacing external data.
    ///
    /// # Errors
    /// Aggregates unresolved paths, permissions and ownership conflicts for manual repair.
    pub async fn rollback(&self) -> AppResult<()> {
        let mut failures = Vec::new();
        for (index, movement) in self.movements.iter().enumerate().rev() {
            let result = async {
                let directories = if let Some(parents) = movement
                    .import_content
                    .as_ref()
                    .and_then(|content| content.parents.as_ref())
                {
                    let directories =
                        ImportDirectories::open(&movement.source, &movement.destination).await?;
                    if &directories.parents != parents {
                        return Err(AppError::ValidationFailed(
                            "Import recovery directory identity changed".into(),
                        ));
                    }
                    Some(directories)
                } else {
                    None
                };
                if owns_exact_path(&movement.source, movement).await {
                    return Ok(());
                }
                if !owns_exact_path(&movement.destination, movement).await {
                    for previous in &self.movements[..index] {
                        if previous.identity == movement.identity
                            && owns_exact_path(&previous.source, movement).await
                        {
                            return Ok(());
                        }
                    }
                    return Err(AppError::Internal(
                        "Neither path contains the expected file identity".into(),
                    ));
                }
                if let Some(directories) = directories {
                    let identity = movement.verified_identity(&movement.destination).await?;
                    directories
                        .move_file(&movement.destination, &movement.source, &identity, true)
                        .await
                } else {
                    move_without_replace(&movement.destination, &movement.source).await
                }
            }
            .await;
            if let Err(error) = result {
                failures.push(format!(
                    "{} -> {}: {error}",
                    movement.destination.display(),
                    movement.source.display()
                ));
            }
        }
        if let Some(path) = &self.creation {
            if tokio::fs::try_exists(path).await? {
                match (
                    &self.creation_identity,
                    crate::infra::filesystem::identity::read(path).await,
                ) {
                    (Some(expected), Ok(actual)) if *expected == actual => {
                        if let Err(error) = tokio::fs::remove_dir(path).await {
                            failures.push(error.to_string());
                        }
                    }
                    _ => failures.push(format!(
                        "Creation ownership cannot be established: {}",
                        path.display()
                    )),
                }
            }
        }
        if let Some(path) = &self.removal {
            if !tokio::fs::try_exists(path).await? {
                if let Err(error) = tokio::fs::create_dir(path).await {
                    failures.push(error.to_string());
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Internal(failures.join("; ")))
        }
    }

    /// Finalizes a journal only after commit proof or complete compensation.
    ///
    /// # Errors
    /// Returns filesystem errors; a retained completed journal can be retried safely.
    pub async fn remove(&self, app_data: &Path) -> AppResult<()> {
        tokio::fs::remove_file(
            app_data
                .join("library-journal")
                .join(format!("{}.json", self.operation_id)),
        )
        .await?;
        Ok(())
    }
}

/// Repairs interrupted operations before sensors or index scans can observe partial state.
///
/// # Errors
/// Returns a repair diagnostic without removing an ambiguous or incompatible manifest.
pub(crate) async fn recover(pool: &SqlitePool, app_data: &Path) -> AppResult<()> {
    let directory = app_data.join("library-journal");
    tokio::fs::create_dir_all(&directory).await?;
    let mut entries = tokio::fs::read_dir(directory).await?;
    let mut failures = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }
        let result = async {
            let journal: Journal = serde_json::from_slice(&tokio::fs::read(entry.path()).await?)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            if journal.version != 1 {
                return Err(AppError::ValidationFailed(
                    "Unsupported library journal version".into(),
                ));
            }
            if !super::library_receipts::committed(pool, &journal.operation_id).await? {
                journal.rollback().await?;
            }
            journal.remove(app_data).await
        }
        .await;
        if let Err(error) = result {
            failures.push(format!("{}: {error}", entry.path().display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Internal(format!(
            "Library repair required: {}",
            failures.join("; ")
        )))
    }
}

async fn owns_exact_path(path: &Path, movement: &Movement) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(mut entries) = tokio::fs::read_dir(parent).await else {
        return false;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if path.file_name().is_some_and(|name| {
            entry
                .file_name()
                .to_string_lossy()
                .nfc()
                .eq(name.to_string_lossy().nfc())
        }) {
            return movement.verified_identity(path).await.is_ok();
        }
    }
    false
}
