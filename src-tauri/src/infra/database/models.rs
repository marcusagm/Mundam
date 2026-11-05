// No imports needed here if unused
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for the main assets table in V2.
///
/// This struct directly maps to the `assets` table in the SQLite database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetDb {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Display name of the asset
    pub name: String,
    /// Absolute filesystem path
    pub path: String,
    /// Current state in the lifecycle machine
    pub state: String,
    /// Detected format (e.g. image/png)
    pub format_type: String,
    /// Media family (e.g. IMAGE, VIDEO)
    pub family: String,
    /// Size in bytes
    pub file_size: i64,
    /// File creation timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// File modification timestamp
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Ingestion timestamp
    pub added_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Database record update timestamp
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Parent folder ID
    pub folder_id: Option<String>,
    /// Path to the generated thumbnail file
    pub thumbnail_path: Option<String>,
    /// User-assigned rating (0-5 stars)
    pub rating: Option<i64>,
    /// Free-text personal notes
    pub notes: Option<String>,
    /// Is favorite
    pub is_favorite: bool,
    /// Deleted at
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Width of the asset
    pub width: Option<i64>,
    /// Height of the asset
    pub height: Option<i64>,
    /// Duration of the asset in seconds
    pub duration_secs: Option<f64>,
    /// Dominant colors of the asset
    pub dominant_color: Option<serde_json::Value>,
    /// Technical payload of the asset
    pub technical_payload: Option<serde_json::Value>,
    /// Semantic payload of the asset
    pub semantic_payload: Option<serde_json::Value>,
}

/// Lightweight database projection for asset listings.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetSummaryDb {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Display name of the asset
    pub name: String,
    /// Absolute filesystem path
    pub path: String,
    /// Current state in the lifecycle machine
    pub state: String,
    /// Detected format (e.g. image/png)
    pub format_type: String,
    /// Media family (e.g. IMAGE, VIDEO)
    pub family: String,
    /// Timestamp of when the file was created
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of when the file was last modified
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Ingestion timestamp
    pub added_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of when the record was last updated
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// ID of the parent folder
    pub folder_id: Option<String>,
    /// Path to the generated thumbnail file
    pub thumbnail_path: Option<String>,
    /// File size in bytes
    pub file_size: i64,
    /// Width in pixels
    pub width: Option<i64>,
    /// Height in pixels
    pub height: Option<i64>,
    /// User-assigned rating (0-5 stars)
    pub rating: Option<i64>,
    /// Free-text personal notes
    pub notes: Option<String>,
    /// Is favorite
    pub is_favorite: bool,
    /// Deleted at
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Dynamic metadata envelope for specific format capabilities.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetMetadataEnvelopeDb {
    /// Unique identifier (UUID/ULID)
    pub asset_id: String,
    /// Width of the asset
    pub width: Option<i64>,
    /// Height of the asset
    pub height: Option<i64>,
    /// Duration of the asset in seconds
    pub duration_secs: Option<f64>,
    /// Dominant colors of the asset
    pub dominant_color: Option<serde_json::Value>,
    /// Technical payload of the asset
    pub technical_payload: Option<serde_json::Value>,
    /// Semantic payload of the asset
    pub semantic_payload: Option<serde_json::Value>,
    /// User-assigned rating (0-5 stars)
    pub rating: Option<i64>,
    /// Free-text personal notes
    pub notes: Option<String>,
}

/// Record of an operation performed on an asset, used for audit and undo.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetOperationLogDb {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Type of operation (e.g. "tag", "move", "delete")
    pub operation_type: String,
    /// ID of the asset involved
    pub asset_id: String,
    /// Payload containing operation details
    pub payload: serde_json::Value,
    /// Status of the operation (e.g. "pending", "completed", "failed")
    pub status: String,
    /// Error note if the operation failed
    pub error_note: Option<String>,
    /// Timestamp of when the operation was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Database model for asset colors.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetColorDb {
    pub id: i64,
    pub asset_id: String,
    pub hex_color: String,
    pub lab_lightness: f64,
    pub lab_green_red: f64,
    pub lab_blue_yellow: f64,
    pub percentage: f64,
    pub rank: i64,
}

impl From<crate::core::models::asset::AssetColor> for AssetColorDb {
    fn from(color: crate::core::models::asset::AssetColor) -> Self {
        Self {
            id: color.id.unwrap_or(0),
            asset_id: String::new(),
            hex_color: color.hex_color,
            lab_lightness: color.lab_lightness,
            lab_green_red: color.lab_green_red,
            lab_blue_yellow: color.lab_blue_yellow,
            percentage: color.percentage,
            rank: color.rank as i64,
        }
    }
}

/// Database model for smart folders.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SmartFolderDb {
    pub id: String,
    pub name: String,
    pub query_json: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SmartFolderDb> for crate::core::models::SmartFolder {
    fn from(row: SmartFolderDb) -> Self {
        Self {
            id: row.id,
            name: row.name,
            query_json: row.query_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Converts an AssetDb to an Asset.
impl From<AssetDb> for crate::core::models::Asset {
    fn from(row: AssetDb) -> Self {
        use crate::core::models::asset::AssetState;
        use std::str::FromStr;
        Self {
            id: row.id,
            name: row.name,
            path: std::path::PathBuf::from(row.path),
            state: AssetState::from_str(&row.state).unwrap_or(AssetState::Unknown),
            format_type: row.format_type,
            family: row.family,
            file_size: row.file_size as u64,
            created_at: row.created_at,
            modified_at: row.modified_at,
            added_at: row.added_at,
            updated_at: row.updated_at,
            folder_id: row.folder_id,
            width: row.width.map(|v| v as i32),
            height: row.height.map(|v| v as i32),
            duration_secs: row.duration_secs,
            technical_payload: row.technical_payload,
            semantic_payload: row.semantic_payload,
            dominant_color: row.dominant_color,
            thumbnail_path: row.thumbnail_path,
            rating: row.rating.map(|v| v as i32),
            notes: row.notes,
            is_favorite: row.is_favorite,
            deleted_at: row.deleted_at,
        }
    }
}

/// Converts an AssetSummaryDb to an AssetSummaryDto.
impl From<AssetSummaryDb> for crate::core::models::AssetSummaryDto {
    fn from(row: AssetSummaryDb) -> Self {
        use crate::core::models::asset::AssetState;
        use std::str::FromStr;
        Self {
            id: row.id,
            name: row.name,
            path: std::path::PathBuf::from(row.path),
            state: AssetState::from_str(&row.state).unwrap_or(AssetState::Unknown),
            format_type: row.format_type,
            family: row.family,
            created_at: row.created_at,
            modified_at: row.modified_at,
            added_at: row.added_at,
            folder_id: row.folder_id,
            thumbnail_path: row.thumbnail_path,
            file_size: row.file_size,
            width: row.width.map(|v| v as i32),
            height: row.height.map(|v| v as i32),
            rating: row.rating.unwrap_or(0) as i32,
            notes: row.notes,
            is_favorite: row.is_favorite,
            deleted_at: row.deleted_at,
        }
    }
}

/// Database model for folders.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FolderDb {
    /// Explicit icon color, absent for the default appearance.
    pub color: Option<String>,
    /// Resolve the ancestor color when enabled.
    pub inherits_color: bool,
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<FolderDb> for crate::core::models::asset::Folder {
    fn from(row: FolderDb) -> Self {
        Self {
            color: row.color,
            inherits_color: row.inherits_color,
            id: row.id,
            parent_id: row.parent_id,
            name: row.name,
            path: std::path::PathBuf::from(row.path),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database model for tags.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TagDb {
    /// Distinguishes inherited colors from explicit user choices.
    pub inherits_color: bool,
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: i64,
}

impl From<TagDb> for crate::core::models::asset::Tag {
    fn from(row: TagDb) -> Self {
        Self {
            id: row.id,
            name: row.name,
            color: row.color,
            inherits_color: row.inherits_color,
            parent_id: row.parent_id,
            order_index: row.order_index,
        }
    }
}
