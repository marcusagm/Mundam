//! Media processing module.
//!
//! Contains format providers, content extractors, and utility functions for
//! handling all supported media types in the Mundam pipeline.

pub mod extractors;
pub mod providers;

pub mod fallback_format;
/// Shared content and visual fingerprint extraction.
pub mod fingerprint;
pub mod image_utils;
