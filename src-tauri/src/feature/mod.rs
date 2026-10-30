//! Application Layer / CQRS Handlers.

/// Module for asset analysis.
pub mod analysis;
/// Module for asset management.
pub mod assets;
/// Module for library management.
pub mod library;
/// Module for media-related features (waveforms, etc).
pub mod media;
/// Module for search functionality.
pub mod search;
/// Module for settings management.
pub mod settings;
/// Module for transcoding functionality.
pub mod transcoding;
/// Module for duplicates management.
pub mod duplicates;
/// Module for trash management.
pub mod trash;
/// Application startup readiness and retained progress.
pub mod startup;
