use mime_guess::from_path;
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};
use tauri::{
    http::{header, Response, StatusCode},
    Manager,
};

fn serve_file(path: &Path, range: Option<&header::HeaderValue>) -> Result<Response<Vec<u8>>, Response<Vec<u8>>> {
    use std::io::{Read, Seek};
    
    let mut file = std::fs::File::open(path).map_err(|e| {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(e.to_string().into_bytes())
            .unwrap()
    })?;

    let metadata = file.metadata().map_err(|_| {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .unwrap()
    })?;
    
    let file_size = metadata.len();
    let mut mime = from_path(path).first_or_octet_stream().to_string();

    // Fallbacks for Safari/Tauri picky video and special formats
    if mime == "application/octet-stream" {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "mxf" => mime = "video/mxf".to_string(),
                "mkv" => mime = "video/x-matroska".to_string(),
                "heic" => mime = "image/heic".to_string(),
                "heif" => mime = "image/heif".to_string(),
                "psd" | "psb" => mime = "image/vnd.adobe.photoshop".to_string(),
                "ai" => mime = "application/pdf".to_string(),
                "wav" => mime = "audio/wav".to_string(),
                "opus" => mime = "audio/opus".to_string(),
                "oga" | "ogg" => mime = "audio/ogg".to_string(),
                _ => {}
            }
        }
    }

    let response = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCEPT_RANGES, "bytes");

    if let Some(range_value) = range {
        if let Ok(range_str) = range_value.to_str() {
            if range_str.starts_with("bytes=") {
                let range_parts: Vec<&str> = range_str["bytes=".len()..].split('-').collect();
                if range_parts.len() == 2 {
                    let start = range_parts[0].parse::<u64>().unwrap_or(0);
                    let end = range_parts[1].parse::<u64>().unwrap_or(file_size - 1);
                    let end = if end >= file_size { file_size - 1 } else { end };

                    if start < file_size && start <= end {
                        let chunk_size = (end - start) + 1;
                        let mut buffer = vec![0u8; chunk_size as usize];
                        
                        file.seek(std::io::SeekFrom::Start(start)).map_err(|_| {
                             Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Vec::new()).unwrap()
                        })?;
                        
                        file.read_exact(&mut buffer).map_err(|_| {
                             Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Vec::new()).unwrap()
                        })?;

                        return Ok(response
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size))
                            .header(header::CONTENT_LENGTH, chunk_size)
                            .body(buffer)
                            .unwrap());
                    }
                }
            }
        }
    }

    // Default: Serve entire file (be careful with memory, but Vec<u8> is required by Response)
    // For very large files, this might still be an issue if Range wasn't used.
    let mut all_data = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut all_data).map_err(|_| {
        Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Vec::new()).unwrap()
    })?;

    Ok(response
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, file_size)
        .body(all_data)
        .unwrap())
}

pub fn thumb_handler(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    // println!("DEBUG: thumb_handler called for URI: {}", uri);

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
    let mut full_path = thumb_dir.join(decoded_filename.as_ref());

    // Fallback: If it's an icon that's missing the 'extensions/' prefix
    if !full_path.exists() && decoded_filename.starts_with("icon_") {
        let alt_path = thumb_dir.join("extensions").join(decoded_filename.as_ref());
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

    // NATIVE EXTRACTORS: Interpolate formats the browser cannot render natively.
    // We intercept and serve an extracted preview if available.
    if let Ok((preview_data, mime)) = crate::thumbnails::extractors::extract_preview(&full_path) {
        println!("PROTOCOL (orig_handler): Extracted {} preview for {:?}", mime, full_path.file_name().unwrap_or_default());
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(preview_data)
            .unwrap();
    }

    let range = request.headers().get(header::RANGE);

    match serve_file(&full_path, range) {
        Ok(res) => res,
        Err(res) => res,
    }
}
