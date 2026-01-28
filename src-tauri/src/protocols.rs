use mime_guess::from_path;
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};
use tauri::{
    http::{header, Response, StatusCode},
    Manager,
};

fn serve_file(path: &Path) -> Result<Response<Vec<u8>>, Response<Vec<u8>>> {
    if !path.exists() {
        eprintln!("ERROR: File does not exist: {:?}", path);
        return Err(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(Vec::new())
            .unwrap());
    }

    match std::fs::read(path) {
        Ok(data) => {
            let mime = from_path(path).first_or_octet_stream();
            println!("DEBUG: Serving {:?} as {}", path, mime.essence_str());
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.essence_str())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(data)
                .map_err(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(Vec::new())
                        .unwrap()
                })
        }
        Err(e) => {
            eprintln!("ERROR: Failed to read file {:?}: {}", path, e);
            Err(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Vec::new())
                .unwrap())
        }
    }
}

pub fn thumb_handler(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    println!("DEBUG: thumb_handler called for URI: {}", uri);

    let path_part = if uri.contains("://localhost/") {
        uri.split("://localhost/").nth(1).unwrap_or("")
    } else {
        uri.trim_start_matches("thumb://")
    };
    
    // Strip query params if present (e.g., ?v=0 for cache busting)
    let path_part = path_part.split('?').next().unwrap_or(path_part);

    let thumb_dir = app
        .path()
        .app_local_data_dir()
        .expect("Failed to get app data dir")
        .join("thumbnails");

    let decoded_filename = percent_decode_str(path_part).decode_utf8_lossy();
    let full_path = thumb_dir.join(decoded_filename.as_ref());

    match serve_file(&full_path) {
        Ok(res) => res,
        Err(res) => res,
    }
}

pub fn orig_handler(
    _app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();

    let path_part = if uri.contains("://localhost/") {
        uri.split("://localhost/").nth(1).unwrap_or("")
    } else {
        uri.trim_start_matches("orig://")
    };

    let decoded_path = percent_decode_str(path_part).decode_utf8_lossy();
    let mut full_path = PathBuf::from(decoded_path.as_ref());

    // Ensure absolute path on Unix
    if !full_path.is_absolute() && cfg!(unix) {
        full_path = PathBuf::from("/").join(full_path);
    }

    match serve_file(&full_path) {
        Ok(res) => res,
        Err(res) => res,
    }
}
