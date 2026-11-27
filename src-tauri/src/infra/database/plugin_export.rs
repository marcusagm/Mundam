//! Catalog projection that deliberately has no dependency on installed packages or the plugin registry.

use crate::core::{
    plugins::export::{PluginAnnotationExport, PluginAnnotationReader},
    AppError, AppResult,
};
use futures_util::{stream::BoxStream, StreamExt};
use sqlx::SqlitePool;

/// Borrows the pool of the catalog selected by the host.
pub struct SqlitePluginAnnotationReader<'catalog>(pub &'catalog SqlitePool);

impl PluginAnnotationReader for SqlitePluginAnnotationReader<'_> {
    fn annotations(&self) -> BoxStream<'_, AppResult<PluginAnnotationExport>> {
        sqlx::query_as(
            "SELECT field.asset_id, asset.name, field.plugin_id, field.publisher_fingerprint,
                    field.field_id, field.schema_version, field.revision, field.value_json
             FROM plugin_asset_data AS field JOIN assets AS asset ON asset.id = field.asset_id
             ORDER BY field.asset_id, field.plugin_id, field.publisher_fingerprint, field.field_id",
        )
        .fetch(self.0)
        .map(|row| {
            let (
                asset_id,
                asset_name,
                plugin_id,
                publisher_fingerprint,
                field_id,
                schema_version,
                revision,
                value_json,
            ): (_, _, _, _, _, _, i64, String) = row?;
            let value = serde_json::from_str(&value_json).map_err(|_| {
                AppError::Internal("Could not decode a stored plugin annotation".into())
            })?;
            Ok(PluginAnnotationExport {
                asset_id,
                asset_name,
                plugin_id,
                publisher_fingerprint,
                field_id,
                schema_version,
                revision: revision.to_string(),
                value,
            })
        })
        .boxed()
    }
}
