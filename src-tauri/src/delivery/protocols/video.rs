use std::path::PathBuf;
use tauri::http::{Request, Response};
use tauri::AppHandle;

use crate::delivery::protocols::common::{decode_uri_path, serve_file};

/// Tauri Protocol Handler for video://
///
/// Legacy alias to serve video files directly by path.
/// Useful for V1 compatibility and native players that expect video:// scheme.
pub fn handler(_app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let decoded_path = decode_uri_path(&uri, "video");
    let path = PathBuf::from(decoded_path);
    let range = request.headers().get(tauri::http::header::RANGE);

    let response = tauri::async_runtime::block_on(async { serve_file(&path, range).await });

    match response {
        Ok(res) => res,
        Err(res) => *res,
    }
}
