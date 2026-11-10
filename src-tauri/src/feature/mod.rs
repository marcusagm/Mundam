//! Application Layer / CQRS Handlers.

/// Module for asset analysis.
pub mod analysis;
/// Module for asset management.
pub mod assets;
/// Module for duplicates management.
pub mod duplicates;
/// Module for library management.
pub mod library;
/// Module for media-related features (waveforms, etc).
pub mod media;
/// Bounded processing status queries.
pub mod processing;
/// Module for search functionality.
pub mod search;
/// Module for settings management.
pub mod settings;
/// Application shutdown coordination and presentation state.
pub mod shutdown;
/// Application startup readiness and retained progress.
pub mod startup;
/// Module for transcoding functionality.
pub mod transcoding;
