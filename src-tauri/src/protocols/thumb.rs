use super::common::{decode_path, extract_path_part, serve_file, error_response};
use tauri::{http::{header, Response, StatusCode, Request}, Manager, AppHandle};


pub fn handler<R: tauri::Runtime>(app: &AppHandle<R>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "thumb");
    let path_part = path_part.split('?').next().unwrap_or(&path_part);

    let thumb_dir = match app.path().app_local_data_dir() {
        Ok(dir) => dir.join("thumbnails"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, b"Data dir not found".to_vec()),
    };

    let decoded_filename = decode_path(path_part);
    let mut full_path = thumb_dir.join(&decoded_filename);

    if !full_path.exists() && decoded_filename.starts_with("icon_") {
        let alt_path = thumb_dir.join("extensions").join(&decoded_filename);
        if alt_path.exists() {
            full_path = alt_path;
        }
    }

    let range = request.headers().get(header::RANGE);
    match serve_file(&full_path, range) {
        Ok(res) => res,
        Err(res) => res,
    }
}
