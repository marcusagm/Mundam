//! Writes a versioned document from the catalog snapshot to the user-selected destination.

use crate::core::{plugins::export::PluginAnnotationReader, AppError, AppResult};
use futures_util::TryStreamExt;
use std::path::Path;
use tokio::io::{AsyncWriteExt, BufWriter};

/// Exports to a sibling temporary file and publishes only after the snapshot is fully written.
///
/// # Errors
/// Rejects non-JSON destinations and propagates catalog, serialization and filesystem failures.
pub async fn annotations(
    reader: &dyn PluginAnnotationReader,
    catalog_id: &str,
    destination: &Path,
) -> AppResult<u64> {
    if !destination
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(AppError::ValidationFailed(
            "Choose a JSON file for the annotation export".into(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| {
            AppError::ValidationFailed("Choose a directory for the annotation export".into())
        })?
        .to_path_buf();
    let (file, temporary_path) = tokio::task::spawn_blocking(move || {
        tempfile::NamedTempFile::new_in(parent).map(tempfile::NamedTempFile::into_parts)
    })
    .await
    .map_err(|_| AppError::Internal("Could not prepare the annotation export".into()))??;
    let mut output = BufWriter::new(tokio::fs::File::from_std(file));

    let catalog_json = serde_json::to_string(catalog_id)
        .map_err(|_| AppError::Internal("Could not encode the export catalog identity".into()))?;
    let header =
        format!("{{\n  \"formatVersion\": 1,\n  \"catalogId\": {catalog_json},\n  \"fields\": [");
    output.write_all(header.as_bytes()).await?;
    let mut count = 0;
    let mut fields = reader.annotations();
    while let Some(field) = fields.try_next().await? {
        let encoded = serde_json::to_vec(&field)
            .map_err(|_| AppError::Internal("Could not encode a plugin annotation".into()))?;
        let separator: &[u8] = if count == 0 { b"\n    " } else { b",\n    " };
        output.write_all(separator).await?;
        output.write_all(&encoded).await?;
        count += 1;
    }
    drop(fields);
    output.write_all(b"\n  ]\n}\n").await?;
    output.flush().await?;
    output.get_ref().sync_all().await?;
    drop(output);

    let destination = destination.to_path_buf();
    tokio::task::spawn_blocking(move || {
        temporary_path
            .persist(destination)
            .map_err(|failure| failure.error)
    })
    .await
    .map_err(|_| AppError::Internal("Could not publish the annotation export".into()))??;
    Ok(count)
}

#[cfg(test)]
mod tests;
