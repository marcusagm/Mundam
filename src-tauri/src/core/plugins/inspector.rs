//! Data-only contract between package permission policy and inspector renderers.

use super::PublisherAnchor;
use mundam_plugin_sdk::Permission;
use serde::Serialize;

/// Current package and authorization as read from a single catalog-scoped registry query.
pub struct ActiveInspectorPackage {
    pub id: String,
    pub publisher: PublisherAnchor,
    pub digest: [u8; 32],
    pub bytes: Vec<u8>,
    pub permissions: Vec<Permission>,
}

/// A declarative section the host can render for a single selected asset.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectorSection {
    pub plugin_id: String,
    pub plugin_name: String,
    pub section_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport_action_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_link_label: Option<String>,
    pub fields: Vec<PluginInspectorField>,
}

/// A field definition displays its declared default until the data broker supplies values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectorField {
    pub field_id: String,
    pub label: String,
    pub kind: InspectorFieldKind,
    pub default_value: bool,
}

/// Presentation controls accepted by the first inspector contract.
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InspectorFieldKind {
    Checkbox,
}
