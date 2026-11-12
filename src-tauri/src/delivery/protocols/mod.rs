pub mod admission;
/// Module for handling custom URI protocols for the Tauri application.
///
/// This module provides a way to register custom URI schemes with the Tauri
/// application, allowing the frontend to request resources using custom
/// URI formats.
///
/// # Submodules
///
/// * `asset`: Handles the `asset://` URI scheme for serving assets.
///
/// # Examples
///
/// ```no_run
/// use tauri::{AppHandle, Manager};
/// use crate::delivery::protocols::asset::handler as asset_handler;
///
/// let mut app = tauri::Builder::default()
///     .register_uri_scheme_protocol("asset", Box::new(asset_handler))
///     .build()
///     .unwrap();
/// ```
pub mod asset;
pub mod audio;
pub mod common;
pub mod video;
