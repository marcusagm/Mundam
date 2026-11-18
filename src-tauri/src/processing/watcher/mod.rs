//! # FileSystem Watcher Module
//!
//! This module implements the "Blind Sensor" for filesystem events.
//! It detects OS level changes and uses a Debouncer to group rapid events
//! into clean Domain Events for the application.

pub mod debouncer;
mod native;
mod runtime;
pub mod sensor;

pub use sensor::WatcherService;
