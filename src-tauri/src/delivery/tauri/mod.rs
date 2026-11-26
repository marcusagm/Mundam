//! Delivery layer for the application.

pub mod admission;
pub mod commands;
pub(crate) mod plugin_admin;
/// Global activity snapshots and cooperative admission controls.
pub mod processing;
pub mod shutdown;
pub mod startup;
pub mod thumbnails;
