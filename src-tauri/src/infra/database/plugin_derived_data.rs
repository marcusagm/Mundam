//! SQLite operations for native thumbnail source identity and the plugin result cache.

use super::processing::selection::{ACTIVE_ASSET, SOURCE_STAMP};
use crate::core::{
    plugins::derived_data::{
        PluginDerivedResultKey, PluginPreviewArtifact, PluginPreviewRevision, PluginPreviewSource,
        MAX_DERIVED_RESULT_BYTES,
    },
    AppError, AppResult,
};
use serde_json::{Map, Value};
use sqlx::SqliteConnection;

pub(super) async fn preview_source(
    connection: &mut SqliteConnection,
    asset_id: &str,
) -> AppResult<Option<PluginPreviewSource>> {
    Ok(preview_artifact(connection, asset_id)
        .await?
        .map(|artifact| artifact.source))
}

pub(super) async fn preview_artifact(
    connection: &mut SqliteConnection,
    asset_id: &str,
) -> AppResult<Option<PluginPreviewArtifact>> {
    let statement = format!(
        "SELECT task.thumbnail_artifact, task.thumbnail_digest, a.family
         FROM assets a JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = 'thumbnail'
         WHERE a.id = ? AND {ACTIVE_ASSET}
           AND task.state = 'succeeded' AND task.source_stamp = ({SOURCE_STAMP})
           AND task.thumbnail_artifact = a.thumbnail_path AND task.thumbnail_digest IS NOT NULL"
    );
    let Some((file_name, digest, family)) =
        sqlx::query_as::<_, (String, Vec<u8>, String)>(&statement)
            .bind(asset_id)
            .fetch_optional(connection)
            .await?
    else {
        return Ok(None);
    };
    let content_digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| AppError::Internal("Invalid digest for native thumbnail".into()))?;
    let mut revision = blake3::Hasher::new_derive_key("Mundam plugin thumbnail revision v1");
    revision.update(&content_digest);
    revision.update(file_name.as_bytes());
    Ok(Some(PluginPreviewArtifact {
        source: PluginPreviewSource {
            revision: PluginPreviewRevision(*revision.finalize().as_bytes()),
            content_digest,
        },
        file_name,
        family,
    }))
}

pub(super) async fn read_result(
    connection: &mut SqliteConnection,
    key: &PluginDerivedResultKey,
    source: PluginPreviewRevision,
) -> AppResult<Option<Map<String, Value>>> {
    if !is_current(connection, key, source).await? {
        return Ok(None);
    }
    let encoded: Option<String> = sqlx::query_scalar(
        "SELECT value_json FROM plugin_derived_results
         WHERE asset_id = ? AND plugin_id = ? AND publisher_fingerprint = ? AND result_id = ?
           AND package_digest = ? AND schema_version = ? AND source_revision = ?",
    )
    .bind(&key.asset_id)
    .bind(&key.plugin_id)
    .bind(key.publisher_fingerprint.as_slice())
    .bind(&key.result_id)
    .bind(key.package_digest.as_slice())
    .bind(i64::from(key.schema_version))
    .bind(source.0.as_slice())
    .fetch_optional(connection)
    .await?;
    encoded
        .map(|value| {
            serde_json::from_str(&value)
                .map_err(|_| AppError::Internal("Invalid stored plugin result".into()))
        })
        .transpose()
}

pub(super) fn encode_result(
    key: &PluginDerivedResultKey,
    value: &Map<String, Value>,
) -> AppResult<String> {
    if key.schema_version == 0 {
        return Err(AppError::ValidationFailed(
            "Invalid plugin result schema version".into(),
        ));
    }
    let encoded = serde_json::to_string(value)
        .map_err(|_| AppError::ValidationFailed("Invalid plugin result".into()))?;
    if encoded.len() > MAX_DERIVED_RESULT_BYTES {
        return Err(AppError::ValidationFailed(
            "Plugin result exceeds the 64 KiB limit".into(),
        ));
    }
    if serde_json::from_str::<Map<String, Value>>(&encoded).is_err() {
        return Err(AppError::ValidationFailed(
            "Plugin result cannot be decoded".into(),
        ));
    }
    Ok(encoded)
}

pub(super) async fn write_result(
    connection: &mut SqliteConnection,
    key: &PluginDerivedResultKey,
    source: PluginPreviewRevision,
    encoded: String,
) -> AppResult<bool> {
    if !is_current(connection, key, source).await? {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO plugin_derived_results (asset_id, plugin_id, publisher_fingerprint, result_id,
             package_digest, schema_version, source_revision, value_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (asset_id, plugin_id, publisher_fingerprint, result_id) DO UPDATE SET
            package_digest = excluded.package_digest, schema_version = excluded.schema_version,
            source_revision = excluded.source_revision, value_json = excluded.value_json",
    )
    .bind(&key.asset_id)
    .bind(&key.plugin_id)
    .bind(key.publisher_fingerprint.as_slice())
    .bind(&key.result_id)
    .bind(key.package_digest.as_slice())
    .bind(i64::from(key.schema_version))
    .bind(source.0.as_slice())
    .bind(encoded)
    .execute(connection)
    .await?;
    Ok(true)
}

async fn is_current(
    connection: &mut SqliteConnection,
    key: &PluginDerivedResultKey,
    source: PluginPreviewRevision,
) -> AppResult<bool> {
    Ok(preview_source(connection, &key.asset_id)
        .await?
        .is_some_and(|published| published.revision == source))
}

#[cfg(test)]
mod tests;
