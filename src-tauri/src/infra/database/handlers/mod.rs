//! Specialized Command Handlers for the Infra Database Layer.
//!
//! Each sub-module encapsulates all SQL mutations and audit-log entries for a
//! specific domain concept, keeping `SqliteAssetLedger` as a pure transactional
//! router rather than a "God Adapter".
//!
//! The `shared` module provides common infrastructure utilities (audit logging,
//! asset fetching, path resolution) as free functions, decoupled from the Ledger
//! struct to enable independent compilation and testing of each handler.
pub mod asset_handler;
pub mod duplicates_handler;
pub mod folder_handler;
pub mod metadata_handler;
pub mod shared;
pub mod smart_folder_handler;
pub mod tags_handler;
pub mod thumbnail_handler;
