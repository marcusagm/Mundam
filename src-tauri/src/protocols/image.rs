use super::common::{decode_path, extract_path_part, serve_file};
use tauri::http::{header, Response, StatusCode, Request};
use std::path::PathBuf;
pub fn handler(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "image");
    let decoded_path = decode_path(&path_part);
    let mut full_path = PathBuf::from(&decoded_path);

    if !full_path.is_absolute() && cfg!(unix) {
        if !path_part.starts_with('/') {
            full_path = PathBuf::from("/").join(full_path);
        }
    }

    // NATIVE EXTRACTORS: Handle formats the browser cannot render natively (RAW, etc)
    if let Ok((preview_data, mime)) = crate::thumbnails::extractors::extract_preview(&full_path) {
        let len = preview_data.len();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, len)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(preview_data)
            .unwrap_or_else(|_| Response::default());
    }

    let range = request.headers().get(header::RANGE);
    match serve_file(&full_path, range) {
        Ok(res) => res,
        Err(res) => res,
    }
}
