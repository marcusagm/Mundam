//! # Library Feature Module
//!
//! Handles the management of assets in the library, including indexing and lifecycle.
//!
//! This module is organized around the Single Responsibility Principle:
//!
//! - **`indexer`**: Core `LibraryIndexer` struct, parallel directory scanning, and repair operations.
//! - **`event_handler`**: Real-time filesystem event processing (add, rename, delete, move recovery).
//! - **`classifier`**: Pure file classification logic for the fan-out producer pipeline.

pub mod classifier;
pub mod event_handler;
pub mod indexer;
pub mod remove_location;
