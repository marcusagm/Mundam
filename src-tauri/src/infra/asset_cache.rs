//! Deletes generated asset artifacts without following database paths to original media.

use crate::core::error::AppResult;
use std::collections::HashSet;
use std::path::Path;

/// Removes every cached thumbnail and preview variant owned by the supplied IDs.
///
/// Only direct files inside the application's managed cache directories qualify.
/// Database thumbnail paths are deliberately not deletion targets: original media
/// must remain intact even if an old record contains an unexpected path.
///
/// # Errors
/// Returns the first filesystem error after attempting all remaining artifacts.
pub async fn remove_asset_artifacts(
    app_data_directory: &Path,
    asset_ids: &HashSet<String>,
) -> AppResult<()> {
    if asset_ids.is_empty() {
        return Ok(());
    }
    let mut first_error = None;
    for directory_name in ["thumbnails", "previews"] {
        let directory = app_data_directory.join(directory_name);
        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    first_error.get_or_insert(error);
                    break;
                }
            };
            let path = entry.path();
            let belongs_to_asset = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| asset_ids.contains(stem));
            if !belongs_to_asset {
                continue;
            }
            if let Err(error) = tokio::fs::remove_file(&path).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(path = %path.display(), %error, "Failed to remove asset artifact");
                    first_error.get_or_insert(error);
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error.into());
    }
    Ok(())
}
