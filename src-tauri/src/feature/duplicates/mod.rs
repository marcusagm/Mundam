//! Duplicate commands, queries, coordination and matching capabilities.
//! Automatic queue consumption is owned by `processing::workers::DuplicateWorker`.

pub mod commands;
pub mod coordinator;
/// Shared fingerprint execution for both scan entry points.
pub mod fingerprint_processor;
pub mod fingerprints;
pub mod matcher;
pub mod matching;
pub mod matching_processor;
pub mod queries;
mod scan_progress;
