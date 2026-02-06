use mime_guess::from_path;
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};
use tauri::{
    http::{header, Response, StatusCode},
    Manager,
};

fn error_response(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .unwrap_or_else(|_| Response::default())
}

fn serve_file(path: &Path, range: Option<&header::HeaderValue>) -> Result<Response<Vec<u8>>, Response<Vec<u8>>> {
    use std::io::{Read, Seek};
    
    if !path.exists() {
        return Err(error_response(StatusCode::NOT_FOUND, b"File not found".to_vec()));
    }

    if path.is_dir() {
        return Err(error_response(StatusCode::FORBIDDEN, b"Cannot serve directory".to_vec()));
    }

    let mut file = std::fs::File::open(path).map_err(|e| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string().into_bytes())
    })?;

    let metadata = file.metadata().map_err(|_| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
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

    let builder = Response::builder()
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
                        
                        // Security check: don't allocate more than 100MB per chunk
                        if chunk_size > 100 * 1024 * 1024 {
                             return Err(error_response(StatusCode::PAYLOAD_TOO_LARGE, b"Chunk too large".to_vec()));
                        }

                        let mut buffer = vec![0u8; chunk_size as usize];
                        if file.seek(std::io::SeekFrom::Start(start)).is_ok() && file.read_exact(&mut buffer).is_ok() {
                            return Ok(builder
                                .status(StatusCode::PARTIAL_CONTENT)
                                .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size))
                                .header(header::CONTENT_LENGTH, chunk_size)
                                .body(buffer)
                                .unwrap_or_else(|_| Response::default()));
                        }
                    }
                }
            }
        }
    }

    // Default: Serve entire file. Prevent serving massive files without Range to avoid OOM panics.
    if file_size > 150 * 1024 * 1024 {
        return Err(error_response(StatusCode::PAYLOAD_TOO_LARGE, b"File too large. Use Range requests.".to_vec()));
    }

    let mut all_data = Vec::new();
    if file.read_to_end(&mut all_data).is_err() {
        return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, b"Read error".to_vec()));
    }

    Ok(builder
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, file_size)
        .body(all_data)
        .unwrap_or_else(|_| Response::default()))
}

fn extract_path_part(uri: &str, scheme: &str) -> String {
    // Standardize URL parsing for custom protocols in Tauri 2
    let prefix_with_host = format!("{}://localhost/", scheme);
    let prefix_simple = format!("{}://", scheme);
    let prefix_short = format!("{}:", scheme);

    if let Some(pos) = uri.find(&prefix_with_host) {
        uri[pos + prefix_with_host.len()..].to_string()
    } else if let Some(pos) = uri.find(&prefix_simple) {
        uri[pos + prefix_simple.len()..].to_string()
    } else if let Some(pos) = uri.find(&prefix_short) {
        uri[pos + prefix_short.len()..].to_string()
    } else {
        uri.to_string()
    }
}

pub fn thumb_handler(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "thumb");
    let path_part = path_part.split('?').next().unwrap_or(&path_part);

    let thumb_dir = match app.path().app_local_data_dir() {
        Ok(dir) => dir.join("thumbnails"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, b"Data dir not found".to_vec()),
    };

    let decoded_filename = percent_decode_str(path_part).decode_utf8_lossy();
    let mut full_path = thumb_dir.join(decoded_filename.as_ref());

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
    let path_part = extract_path_part(&uri, "orig");

    let decoded_path = percent_decode_str(&path_part).decode_utf8_lossy();
    let mut full_path = PathBuf::from(decoded_path.as_ref());

    if !full_path.is_absolute() && cfg!(unix) {
        // Handle leading slash removal by browsers
        if !path_part.starts_with('/') {
             full_path = PathBuf::from("/").join(full_path);
        }
    }

    // NATIVE EXTRACTORS: Interpolate formats the browser cannot render natively.
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
