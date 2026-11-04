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
            let belongs_to_asset =
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| {
                        asset_ids.contains(stem)
                            || stem.split_once('.').is_some_and(|(asset_id, lease)| {
                                asset_ids.contains(asset_id) && uuid::Uuid::parse_str(lease).is_ok()
                            })
                    });
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

/// Reconciles a finished attempt directly, or periodically enumerates all abandoned versions.
/// Enumeration precedes the database snapshot, preserving later writers and published pins.
///
/// # Errors
/// Propagates cache enumeration, database or deletion failures; originals never qualify.
pub(crate) async fn prune_processing_artifacts(
    pool: &sqlx::SqlitePool,
    app_data_directory: &Path,
    ticket: Option<&crate::core::processing::ProcessingTicket>,
) -> AppResult<()> {
    let directory = app_data_directory.join("thumbnails");
    let mut candidates = Vec::new();
    if let Some(ticket) = ticket {
        for extension in ["webp", "glb", "gltf"] {
            candidates.push((
                ticket.asset_id.clone(),
                directory.join(format!(
                    "{}.{}.{}",
                    ticket.asset_id, ticket.lease_id, extension
                )),
            ));
        }
    } else {
        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if let Some((owner, lease)) = stem.split_once('.') {
                if uuid::Uuid::parse_str(lease).is_ok() {
                    candidates.push((owner.to_string(), path));
                }
            }
        }
    }
    if candidates.is_empty() {
        return Ok(());
    }
    let asset_ids: HashSet<&str> = candidates
        .iter()
        .map(|(asset_id, _)| asset_id.as_str())
        .collect();
    let identities = serde_json::to_string(&asset_ids)
        .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT a.id, a.thumbnail_path, task.preview_path,
            CASE WHEN task.lease_expires_at > unixepoch() THEN task.lease_id ELSE NULL END
         FROM assets a LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = 'thumbnail'
         WHERE a.id IN (SELECT value FROM json_each(?))",
    ).bind(identities).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    let pins: std::collections::HashMap<_, _> = rows
        .into_iter()
        .map(|(asset_id, thumbnail, preview, lease)| (asset_id, (thumbnail, preview, lease)))
        .collect();
    for (asset_id, path) in candidates {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let protected = pins
            .get(&asset_id)
            .is_some_and(|(thumbnail, preview, lease)| {
                thumbnail.as_deref() == Some(name)
                    || preview.as_deref() == Some(name)
                    || lease.as_ref().is_some_and(|lease| {
                        path.file_stem().and_then(|stem| stem.to_str())
                            == Some(format!("{asset_id}.{lease}").as_str())
                    })
            });
        if !protected {
            if let Err(error) = tokio::fs::remove_file(&path).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(error.into());
                }
            }
        }
    }
    Ok(())
}
