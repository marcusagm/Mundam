/// Module for handling delivery mechanisms in the Tauri application.
///
/// This module provides a way to register custom URI schemes with the Tauri
/// application, allowing the frontend to request resources using custom
/// URI formats.
///
/// # Submodules
///
/// * `asset`: Handles the `asset://` URI scheme for serving assets.
/// * `tauri`: Handles the `tauri://` URI scheme for serving assets.
///
/// # Examples
///
/// ```no_run
/// use tauri::{AppHandle, Manager};
/// use crate::delivery::protocols::asset::handler as asset_handler;
/// use crate::delivery::tauri::handler as tauri_handler;
///
/// let mut app = tauri::Builder::default()
///     .register_uri_scheme_protocol("asset", Box::new(asset_handler))
///     .register_uri_scheme_protocol("tauri", Box::new(tauri_handler))
///     .build()
///     .unwrap();
/// ```
pub mod external_api;
pub mod protocols;
pub mod streaming;
pub mod tauri;
