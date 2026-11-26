//! Framework-independent wire types for the first declarative inspector contribution.

use serde::{Deserialize, Serialize};

/// Declarative metadata and contributions supplied by a plugin author.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub icon: String,
    pub targets: Vec<Target>,
    pub version: String,
    pub publisher: String,
    pub license: String,
    pub engines: Engines,
    pub permissions: Vec<Permission>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_permissions: Vec<Permission>,
    pub contributes: Contributions,
}

/// A pair of constraints that must both match the same executor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub platform: Platform,
    pub architecture: Architecture,
}

/// The portable manifest vocabulary for operating systems.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    All,
    Windows,
    Mac,
    Linux,
}

/// The executor architecture vocabulary. Arm refers to 32-bit ARM.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    All,
    X64,
    Arm64,
    Arm,
}

/// Required stable host API interval, independent of the desktop version.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engines {
    pub mundam_api: String,
}

/// Capability names recognized by this SDK. Declaring one does not grant it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Permission {
    #[serde(rename = "selection.read")]
    SelectionRead,
    #[serde(rename = "assetData.own.read")]
    OwnDataRead,
    #[serde(rename = "assetData.own.write")]
    OwnDataWrite,
}

/// Initial extension points available as data to any host renderer.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contributions {
    pub asset_fields: Vec<AssetField>,
    pub inspector_sections: Vec<InspectorSection>,
}

/// A versioned value in the plugin's own asset namespace.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetField {
    pub id: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub schema_version: u32,
    pub default: bool,
}

/// Value type supported by the first declarative editor.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
}

/// Inspector composition for one selection context.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InspectorSection {
    pub id: String,
    pub title: String,
    pub when: SelectionCondition,
    pub fields: Vec<InspectorField>,
}

/// An explicit selection predicate without user-supplied expressions.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionCondition {
    pub selection_count: u32,
}

/// An editor bound to a field declared by the same manifest.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectorField {
    pub field: String,
    pub label: String,
    pub control: Control,
    #[serde(default)]
    pub read_only: bool,
}

/// Host-rendered editor control supplied by the v1 inspector.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Control {
    Checkbox,
}
