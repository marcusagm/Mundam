use std::path::PathBuf;
use tauri::http::{Request, Response};
use tauri::AppHandle;

use crate::delivery::protocols::common::{decode_uri_path, serve_file};

/// Tauri Protocol Handler for audio://
///
/// Legacy alias to serve audio files directly by path.
pub fn handler(_app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let decoded_path = decode_uri_path(&uri, "audio");
    let path = PathBuf::from(decoded_path);
    let range = request.headers().get(tauri::http::header::RANGE);

    let response = tauri::async_runtime::block_on(async { serve_file(&path, range).await });

    match response {
        Ok(res) => res,
        Err(res) => *res,
    }
}
