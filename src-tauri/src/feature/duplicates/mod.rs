//! Duplicate commands, queries, coordination and matching capabilities.
//! Automatic queue consumption is owned by `processing::workers::DuplicateWorker`.

pub mod commands;
pub mod coordinator;
pub mod fingerprints;
pub mod matcher;
pub mod queries;
