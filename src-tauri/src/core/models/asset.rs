use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The various lifecycle states of an asset in the Mundam system.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]

/// Enum representing the various lifecycle states of an asset in the Mundam system.
pub enum AssetState {
    /// FS Emit (Create/Rename)
    Discovered,
    /// Format-Kit is reading "Magic Bytes"
    Probing,
    /// Capability [Metadata] extraction succeeded
    Indexed,
    /// ThumbnailWorker extraction succeeded
    Thumbnailed,
    /// Status Gold/Perfect
    Idle,
    /// FS Emit "Modified" (External edit)
    Stale,
    /// HD Disconnected / Folder deleted from project
    Offline,
    /// Fallback for unidentified formats
    Unknown,
}

/// A Domain entity representing a media asset.
/// This is decoupled from the database storage format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Name of the asset
    #[serde(rename = "filename")]
    pub name: String,
    /// Path to the asset
    pub path: PathBuf,
    /// State of the asset
    pub state: AssetState,
    /// Format type of the asset
    #[serde(rename = "format")]
    pub format_type: String,
    /// Family of the asset
    #[serde(rename = "media_type")]
    pub family: String,
    /// File size of the asset
    #[serde(rename = "size")]
    pub file_size: u64,
    /// Timestamp of when the file was created
    pub created_at: Option<DateTime<Utc>>,
    /// Timestamp of when the file was last modified
    pub modified_at: Option<DateTime<Utc>>,
    /// Timestamp of when the asset was added to Mundam
    pub added_at: Option<DateTime<Utc>>,
    /// Timestamp of when the asset record was last updated in Mundam
    pub updated_at: Option<DateTime<Utc>>,

    /// Width of the asset
    pub width: Option<i32>,
    /// Height of the asset
    pub height: Option<i32>,
    /// Duration of the asset in seconds
    pub duration_secs: Option<f64>,
    /// Technical payload of the asset
    pub technical_payload: Option<serde_json::Value>,
    /// Semantic payload of the asset
    pub semantic_payload: Option<serde_json::Value>,
    /// Dominant colors of the asset
    pub dominant_color: Option<serde_json::Value>,
    /// Reference to the parent folder in recursive tree
    pub folder_id: Option<String>,
    /// Path to the generated thumbnail file
    pub thumbnail_path: Option<String>,
    /// User-assigned rating (0-5 stars)
    pub rating: Option<i32>,
    /// Free-text personal notes
    pub notes: Option<String>,
    /// Is favorite
    pub is_favorite: bool,
    /// Timestamp of when the asset was sent to trash
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A Domain entity representing a single color extracted from an asset's palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetColor {
    /// Unique identifier for the color record (optional for new records).
    pub id: Option<i64>,
    /// The hexadecimal representation of the color (e.g., "#FFFFFF").
    pub hex_color: String,
    /// CIE-LAB L* component (lightness).
    pub lab_lightness: f64,
    /// CIE-LAB a* component (green-red axis).
    pub lab_green_red: f64,
    /// CIE-LAB b* component (blue-yellow axis).
    pub lab_blue_yellow: f64,
    /// Proportion of the asset this color represents (0.0-1.0).
    pub percentage: f64,
    /// Dominance rank (1 = most dominant).
    pub rank: i32,
}

/// A Domain entity representing a recursive folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// ID of the parent folder
    pub parent_id: Option<String>,
    /// Name of the folder
    pub name: String,
    /// Path of the folder
    pub path: PathBuf,
    /// Timestamp of when the folder was created
    pub created_at: DateTime<Utc>,
    /// Timestamp of when the folder was last updated
    pub updated_at: DateTime<Utc>,
}

/// A Domain entity representing a taxonomy Tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Name of the tag
    pub name: String,
    /// Color of the tag
    pub color: Option<String>,
    /// ID of the parent tag
    pub parent_id: Option<String>,
    /// Sorting order index for UI display ordering
    pub order_index: i64,
}

/// A lightweight projection of an asset for grid listings and infinite scroll.
/// Focuses on visual performance and minimal bridge overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSummaryDto {
    /// Unique identifier (UUID/ULID)
    pub id: String,
    /// Name of the asset
    #[serde(rename = "filename")]
    pub name: String,
    /// Absolute filesystem path
    pub path: PathBuf,
    /// State of the asset
    pub state: AssetState,
    /// Format type of the asset
    #[serde(rename = "format")]
    pub format_type: String,
    /// Family of the asset
    #[serde(rename = "media_type")]
    pub family: String,
    /// Timestamp of when the file was created
    pub created_at: Option<DateTime<Utc>>,
    /// Timestamp of when the file was last modified
    pub modified_at: Option<DateTime<Utc>>,
    /// Timestamp of when the asset was added to Mundam
    pub added_at: Option<DateTime<Utc>>,
    /// Parent folder ID
    pub folder_id: Option<String>,
    /// Path to the generated thumbnail file
    pub thumbnail_path: Option<String>,
    /// File size in bytes
    #[serde(rename = "size")]
    pub file_size: i64,
    /// Width in pixels
    pub width: Option<i32>,
    /// Height in pixels
    pub height: Option<i32>,
    /// User-assigned rating (0-5 stars)
    pub rating: i32,
    /// Free-text personal notes
    pub notes: Option<String>,
    /// Is favorite
    pub is_favorite: bool,
    /// Timestamp of when the asset was sent to trash
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A wrapper for paginated asset summaries including total count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedAssetsDto {
    /// The batch of assets for the current page.
    pub items: Vec<AssetSummaryDto>,
    /// The total number of assets matching the filter across all pages.
    pub total_items: i64,
}

/// Count of assets associated with a specific tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCount {
    pub tag_id: String,
    pub count: i64,
}

/// Count of assets within a specific folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderCount {
    pub folder_id: String,
    pub count: i64,
}

/// Statistics for assets grouped by media family (e.g., image, video, audio, document).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyStat {
    /// Media family identifier (e.g. "image", "video", "audio", "document", "project")
    pub family: String,
    /// Total number of active assets in this family
    pub count: i64,
    /// Total file size in bytes for this family
    pub total_size_bytes: i64,
}

/// Comprehensive statistics about the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    /// Total number of assets in the library.
    pub total_assets: i64,
    /// Total number of folders in the library.
    pub total_folders: i64,
    /// Total number of tags in the library.
    pub total_tags: i64,
    /// Total size of all assets in bytes.
    pub total_size_bytes: i64,
    /// Number of assets that have no tags assigned.
    pub untagged_assets: i64,
    /// Number of assets that have at least one tag.
    pub has_tags_assets: i64,
    /// Number of assets marked as favorite.
    pub favorite_assets: i64,
    /// Number of assets in the trash.
    pub trash_assets: i64,
    /// Number of smart folders.
    pub smart_folders: i64,
    /// Number of duplicate assets identified (or candidate assets).
    pub duplicate_assets: i64,
    /// Distribution of assets across tags.
    pub tag_counts: Vec<TagCount>,
    /// Direct asset counts per folder.
    pub folder_counts: Vec<FolderCount>,
    /// Asset counts per folder including all subfolders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_counts_recursive: Option<Vec<FolderCount>>,
    /// Breakdown of assets and disk usage per media family.
    #[serde(default)]
    pub family_stats: Vec<FamilyStat>,
}

/// Parameters for filtering asset listings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetFilter {
    /// Family of the asset
    pub family: Option<String>,
    /// State of the asset
    pub state: Option<AssetState>,
    /// Search query
    pub search_query: Option<String>,
    /// Whether to use FTS5 trigram fuzzy matching for the search query
    pub search_fuzzy: Option<bool>,
    /// Filter by folder
    pub folder_id: Option<String>,
    /// Filter by tags (any of)
    pub tags: Option<Vec<String>>,
    /// Filter to only get assets without any tags
    pub untagged: Option<bool>,
    /// Filter to only get assets with tags
    pub has_tags: Option<bool>,
    /// Filter to only get favorite assets
    pub favorites_only: Option<bool>,
    /// Filter to only get trashed assets
    pub trash_only: Option<bool>,
    /// Whether to include assets from subfolders recursively
    pub recursive: Option<bool>,
    /// Field to sort by (e.g., "filename", "size", "created_at")
    pub sort_by: Option<String>,
    /// Sort order ("asc" or "desc")
    pub sort_order: Option<String>,
}

/// Pagination parameters for the read model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageParams {
    /// Page number
    pub page: u32,
    /// Page size
    pub page_size: u32,
}

/// Default implementation for PageParams.
impl Default for PageParams {
    /// Returns the default PageParams.
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 50,
        }
    }
}

/// Implementation of the PageParams struct.
impl PageParams {
    /// Returns the OFFSET for SQL queries.
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// Returns the LIMIT for SQL queries.
    pub fn limit(&self) -> u32 {
        self.page_size
    }
}
