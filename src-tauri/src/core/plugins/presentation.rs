//! Data-only presentation definitions approved for an active catalog.

use super::{inspector::PluginInspectorSection, PublisherAnchor};
use mundam_plugin_sdk::{CommandAction, Permission, ScreenBlock};
use serde::Serialize;

/// A signed package and its current catalog grants read in one registry query.
pub struct ApprovedPluginPackage {
    pub id: String,
    pub publisher: PublisherAnchor,
    pub digest: [u8; 32],
    pub bytes: Vec<u8>,
    pub permissions: Vec<Permission>,
}

/// A single snapshot used to render all declarative plugin surfaces.
#[derive(Debug, Default, Serialize)]
pub struct PluginPresentation {
    pub sections: Vec<PluginInspectorSection>,
    pub screens: Vec<PluginScreen>,
    pub commands: Vec<PluginCommand>,
}

/// A host command qualified by the package that owns its target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommand {
    pub plugin_id: String,
    pub plugin_name: String,
    pub command_id: String,
    pub title: String,
    pub action: CommandAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_shortcut: Option<String>,
}

/// A screen identified by its plugin and local declaration.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginScreen {
    pub plugin_id: String,
    pub plugin_name: String,
    pub screen_id: String,
    pub title: String,
    pub blocks: Vec<ScreenBlock>,
}
