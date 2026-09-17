//! Core domain models for the application.

pub mod asset;
pub mod duplicates;
pub mod search;
pub mod smart_folder;

pub use duplicates::{
    DuplicateCandidate, DuplicateFingerprint, DuplicateGroup, DuplicateGroupStatus,
    DuplicateGroupType, DuplicateResolution, DuplicateResolutionAction, DuplicateRuleSet,
    MetadataMergePayload,
};

pub use asset::{
    Asset, AssetColor, AssetFilter, AssetState, AssetSummaryDto, Folder, FolderCount, LibraryStats,
    PageParams, PaginatedAssetsDto, Tag, TagCount,
};
pub use search::{LogicalOperator, SearchCriteria, SearchCriterion, SearchGroup, SearchItem};
pub use smart_folder::SmartFolder;
