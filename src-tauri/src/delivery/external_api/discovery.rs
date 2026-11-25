//! Publishes the bound endpoint in the current user's Windows application data directory.
use crate::core::{AppError, AppResult};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

pub(super) const FILE_NAME: &str = "import-endpoint.json";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryDocument<'a> {
    version: u8,
    instance_id: uuid::Uuid,
    url: &'a str,
    public_key_sha256: &'a str,
}

/// Keeps the exclusive Windows file handle until this listener loses ownership.
pub(super) struct DiscoveryPublication {
    path: PathBuf,
    _lock: File,
}

impl DiscoveryPublication {
    /// Replaces any descriptor from a crashed instance after acquiring the lock.
    pub(super) fn publish(directory: &Path, url: &str, public_key_sha256: &str) -> AppResult<Self> {
        // Sharing mode zero is held by the kernel until the handle or process closes.
        // Leave the empty lock file in place so every instance locks the same path.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .share_mode(0)
            .open(directory.join("import-endpoint.lock"))
            .map_err(|error| {
                if error.raw_os_error() == Some(32) {
                    AppError::ValidationFailed(
                        "Another Mundam instance is already publishing the local import API".into(),
                    )
                } else {
                    error.into()
                }
            })?;
        let path = directory.join(FILE_NAME);
        let document = DiscoveryDocument {
            version: 1,
            instance_id: uuid::Uuid::new_v4(),
            url,
            public_key_sha256,
        };
        let bytes =
            serde_json::to_vec(&document).map_err(|error| AppError::Internal(error.to_string()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary.persist(&path).map_err(|failure| failure.error)?;
        Ok(Self { path, _lock: lock })
    }
}

impl Drop for DiscoveryPublication {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != ErrorKind::NotFound {
                tracing::warn!(kind = ?error.kind(), "Could not remove the import discovery document");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn import_discovery_replaces_stale_file_and_prevents_competing_publishers() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join(FILE_NAME);
        std::fs::write(&path, b"stale after process loss")?;
        let first =
            DiscoveryPublication::publish(directory.path(), "https://127.0.0.1:5000", "public")?;
        let published = std::fs::read(&path)?;
        assert!(
            DiscoveryPublication::publish(directory.path(), "https://127.0.0.1:5001", "other")
                .is_err()
        );
        assert_eq!(std::fs::read(&path)?, published);
        let document: Value = serde_json::from_slice(&published)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        assert_eq!(document["version"], 1);
        assert_eq!(document["url"], "https://127.0.0.1:5000");
        assert_eq!(document["publicKeySha256"], "public");
        assert_eq!(document.as_object().map(|fields| fields.len()), Some(4));
        drop(first);
        assert!(!path.exists());
        let second =
            DiscoveryPublication::publish(directory.path(), "https://127.0.0.1:5001", "public")?;
        let document_after: Value = serde_json::from_slice(&std::fs::read(path)?)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        assert_ne!(document_after["instanceId"], document["instanceId"]);
        drop(second);
        Ok(())
    }
}
