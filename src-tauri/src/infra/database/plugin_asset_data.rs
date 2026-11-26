//! SQL operations run through the catalog's Asset Ledger.

use crate::core::{
    plugins::asset_data::{
        PluginAssetFieldKey, PluginBooleanField, PluginBooleanWrite, PluginBooleanWriteOutcome,
    },
    AppError, AppResult,
};
use sqlx::{Row, SqliteConnection};

fn fingerprint_hex(fingerprint: &[u8; 32]) -> String {
    fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) async fn read_boolean(
    connection: &mut SqliteConnection,
    key: &PluginAssetFieldKey,
) -> AppResult<Option<PluginBooleanField>> {
    let row = sqlx::query(
        "SELECT field.schema_version, field.revision, field.value_json
         FROM assets AS asset
         LEFT JOIN plugin_asset_data AS field ON field.asset_id = asset.id
            AND field.plugin_id = ? AND field.publisher_fingerprint = ? AND field.field_id = ?
         WHERE asset.id = ? AND asset.deleted_at IS NULL",
    )
    .bind(&key.plugin_id)
    .bind(fingerprint_hex(&key.publisher_fingerprint))
    .bind(&key.field_id)
    .bind(&key.asset_id)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| AppError::NotFound("Active asset".into()))?;

    let Some(schema_version) = row.try_get::<Option<u32>, _>("schema_version")? else {
        return Ok(None);
    };
    let encoded: String = row.try_get("value_json")?;
    let value = serde_json::from_str::<bool>(&encoded)
        .map_err(|_| AppError::Internal("Stored plugin field is not a boolean".into()))?;
    Ok(Some(PluginBooleanField {
        schema_version,
        revision: row.try_get("revision")?,
        value,
    }))
}

pub(super) async fn write_boolean(
    connection: &mut SqliteConnection,
    key: &PluginAssetFieldKey,
    write: PluginBooleanWrite,
) -> AppResult<PluginBooleanWriteOutcome> {
    if write.schema_version == 0 || write.expected_revision < 0 {
        return Err(AppError::ValidationFailed(
            "Invalid plugin field schema version or revision".into(),
        ));
    }

    let current = read_boolean(connection, key).await?;
    if let Some(field) = current.filter(|field| field.schema_version != write.schema_version) {
        return Ok(PluginBooleanWriteOutcome::SchemaMismatch {
            stored_version: field.schema_version,
        });
    }
    let current_revision = current.map_or(0, |field| field.revision);
    if current_revision != write.expected_revision {
        return Ok(PluginBooleanWriteOutcome::Conflict { current_revision });
    }
    let revision = current_revision
        .checked_add(1)
        .ok_or_else(|| AppError::Internal("Plugin field revision exhausted".into()))?;

    sqlx::query(
        "INSERT INTO plugin_asset_data (asset_id, plugin_id, publisher_fingerprint, field_id,
                                       schema_version, revision, value_json)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (asset_id, plugin_id, publisher_fingerprint, field_id)
         DO UPDATE SET revision = excluded.revision, value_json = excluded.value_json",
    )
    .bind(&key.asset_id)
    .bind(&key.plugin_id)
    .bind(fingerprint_hex(&key.publisher_fingerprint))
    .bind(&key.field_id)
    .bind(i64::from(write.schema_version))
    .bind(revision)
    .bind(if write.value { "true" } else { "false" })
    .execute(connection)
    .await?;

    Ok(PluginBooleanWriteOutcome::Stored(PluginBooleanField {
        schema_version: write.schema_version,
        revision,
        value: write.value,
    }))
}

#[cfg(test)]
mod tests;
