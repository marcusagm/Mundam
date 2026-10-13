//! Application Composition Root.
//!
//! This module orchestrates the initialization of all backend subsystems,
//! ensuring strict adherence to the Hexagonal Architecture. It acts as the
//! single authoritative location that understands both the frontend interface
//! framework (Tauri via `AppHandle`) and the concrete infrastructure instances.

pub mod database;
pub mod library;
pub mod streaming;
pub mod system;
pub mod workers;

/// Central data structure containing all core filesystem paths resolved
/// exactly once during application boot.
///
/// Injected into Tauri's state (`AppHandle::manage`) to provide O(1) access
/// to critical directories across the application without path re-resolution or I/O.
#[derive(Clone)]
pub struct AppDirectories {
    pub app_data: std::path::PathBuf,
    pub db_path: std::path::PathBuf,
    pub settings_path: std::path::PathBuf,
    pub thumbnails_dir: std::path::PathBuf,
    pub previews_dir: std::path::PathBuf,
}
