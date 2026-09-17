use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Representa a impressão digital completa de um asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFingerprint {
    pub asset_id: String,
    pub content_hash: Option<String>,
    pub perceptual_hash: Option<String>,
    pub block_hash: Option<String>,
    pub thumb_hash: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub format_family: Option<String>,
    pub color_profile: Option<String>,
    pub orientation: Option<i32>,
    pub fingerprint_version: i32,
    pub updated_at: DateTime<Utc>,
}

/// Define os tipos de duplicidade que um grupo pode ter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DuplicateGroupType {
    Exact,
    Near,
    Derived,
}

/// Define o status atual de um grupo de duplicados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DuplicateGroupStatus {
    Open,
    Reviewed,
    Ignored,
    Resolved,
}

/// Agrupa múltiplos assets candidatos que correspondem a uma regra de duplicidade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: String,
    pub rule_set_id: String,
    pub group_type: DuplicateGroupType,
    pub canonical_asset_id: Option<String>,
    pub confidence: f64,
    pub status: DuplicateGroupStatus,
    pub candidate_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Representa um candidato dentro de um grupo de duplicados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub group_id: String,
    pub asset_id: String,
    pub score: f64,
    pub reasons: String, // Trated as JSON string to simplify SQLite integration
    pub is_selected: bool,
}

/// Define a ação tomada sobre um grupo de duplicados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DuplicateResolutionAction {
    KeepOne,
    DeleteSelected,
    MergeMetadata,
    IgnoreGroup,
    CustomSelection,
}

/// Registra a decisão tomada pelo usuário sobre um grupo de duplicados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateResolution {
    pub id: String,
    pub group_id: String,
    pub action: DuplicateResolutionAction,
    pub selected_asset_id: Option<String>,
    pub payload: Option<String>, // Trated as JSON string
    pub resolved_by: Option<String>,
    pub resolved_at: DateTime<Utc>,
}

/// Define as configurações para uma detecção específica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateRuleSet {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub consider_exact_match: bool,
    pub consider_visual_match: bool,
    pub consider_crop_match: bool,
    pub ignore_resolution_difference: bool,
    pub ignore_recompression: bool,
    pub allow_rotation: bool,
    pub allow_mirroring: bool,
    pub min_score: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Carries the user-confirmed merge decisions for a duplicate resolution.
///
/// Every field is `Option` — `None` means the user chose to skip merging
/// that particular piece of metadata. Only non-None fields are written to
/// the kept assets via the Asset Ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataMergePayload {
    /// The rating value the user chose to propagate (0–5).
    /// None if the user opted to keep the kept asset's existing rating.
    pub rating: Option<i32>,
    /// Whether the kept asset(s) should be marked as favorite.
    /// None means no change. True forces favorite on; false forces it off.
    pub is_favorite: Option<bool>,
    /// The final notes string the user confirmed (may be a concatenation of
    /// notes from multiple candidates, edited freely by the user).
    /// None if the user opted to skip notes merging.
    pub notes: Option<String>,
    /// Tag IDs to add to the kept asset(s). These are collected from the
    /// discarded candidates and pre-selected in the merge UI.
    pub tags_to_add: Vec<String>,
    /// A JSON object containing technical metadata fields selected by the user
    /// from the discarded candidates (e.g. EXIF Artist, Copyright, GPS data).
    /// Written via `LedgerCommand::UpdateTechnicalMetadata`.
    /// None if the user chose not to override any technical metadata.
    pub technical_payload_override: Option<serde_json::Value>,
}

