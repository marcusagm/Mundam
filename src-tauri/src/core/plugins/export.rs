//! Durable catalog annotations available to the host's export workflow.

use crate::core::AppResult;
use futures_util::stream::BoxStream;
use serde::Serialize;

/// Portable row from one catalog snapshot, identified without an original filesystem path.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAnnotationExport {
    pub asset_id: String,
    pub asset_name: String,
    pub plugin_id: String,
    pub publisher_fingerprint: String,
    pub field_id: String,
    pub schema_version: u32,
    /// Decimal string preserves full SQLite revision precision for JSON consumers.
    pub revision: String,
    pub value: serde_json::Value,
}

/// The host consumes a single ordered snapshot without holding all annotations in memory.
pub trait PluginAnnotationReader: Send + Sync {
    /// Streams durable rows including logically trashed assets, in stable identifier order.
    /// Storage and decoding failures are returned as stream items.
    fn annotations(&self) -> BoxStream<'_, AppResult<PluginAnnotationExport>>;
}
