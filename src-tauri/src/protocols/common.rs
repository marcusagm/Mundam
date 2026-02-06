use mime_guess::from_path;
use percent_encoding::percent_decode_str;
use std::path::Path;
use tauri::http::{header, Response, StatusCode};

pub fn error_response(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .unwrap_or_else(|_| Response::default())
}

pub fn extract_path_part(uri: &str, scheme: &str) -> String {
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

pub fn decode_path(path: &str) -> String {
    percent_decode_str(path).decode_utf8_lossy().into_owned()
}

pub fn serve_file(path: &Path, range: Option<&header::HeaderValue>) -> Result<Response<Vec<u8>>, Response<Vec<u8>>> {
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
    
    // Use the central format registry for MIME detection
    let mut mime = if let Some(format) = crate::formats::FileFormat::detect(path) {
        format.mime_types.first().unwrap_or(&"application/octet-stream").to_string()
    } else {
        from_path(path).first_or_octet_stream().to_string()
    };

    // Specific fix for MPEG transport streams which Safari likes as video/mp2t
    if path.extension().and_then(|e| e.to_str()) == Some("m2ts") {
        mime = "video/mp2t".to_string();
    }

    let builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ACCESS_CONTROL_EXPOSE_HEADERS, "Content-Range, Content-Length, Accept-Ranges");

    if let Some(range_value) = range {
        if let Ok(range_str) = range_value.to_str() {
            if range_str.starts_with("bytes=") {
                let range_spec = &range_str["bytes=".len()..];
                let mut start: u64 = 0;
                let mut end: u64 = file_size - 1;

                let parts: Vec<&str> = range_spec.split('-').collect();
                if parts.len() >= 1 && !parts[0].is_empty() {
                    if let Ok(s) = parts[0].parse::<u64>() {
                        start = s;
                    }
                }
                if parts.len() >= 2 && !parts[1].is_empty() {
                    if let Ok(e) = parts[1].parse::<u64>() {
                        end = e;
                    }
                } else if parts.len() == 1 && range_spec.starts_with('-') {
                    // Suffix: -500 -> last 500 bytes
                    if let Ok(suffix) = range_spec[1..].parse::<u64>() {
                        start = file_size.saturating_sub(suffix);
                        end = file_size - 1;
                    }
                }

                // Sanitize range
                if start >= file_size {
                    return Err(error_response(StatusCode::RANGE_NOT_SATISFIABLE, format!("bytes */{}", file_size).into_bytes()));
                }
                if end >= file_size {
                    end = file_size - 1;
                }
                if start > end {
                    end = start; // Handle zero-length ranges somewhat gracefully
                }

                let max_chunk = 10 * 1024 * 1024; // 10MB chunks
                let requested_size = (end - start) + 1;
                let chunk_size = std::cmp::min(requested_size, max_chunk);
                let real_end = start + chunk_size - 1;

                let mut buffer = vec![0u8; chunk_size as usize];
                if file.seek(std::io::SeekFrom::Start(start)).is_ok() && file.read_exact(&mut buffer).is_ok() {
                    return Ok(builder
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, real_end, file_size))
                        .header(header::CONTENT_LENGTH, chunk_size)
                        .body(buffer)
                        .unwrap_or_else(|_| Response::default()));
                }
            }
        }
    }

    // Default: Serve entire file (for non-range requests)
    // If the file is very large and no range was requested, we might be in an initial "probe" request.
    // For videos, instead of returning 413, we serve the first chunk as 206 to encourage the browser to use Ranges.
    let is_video = path.extension().and_then(|e| e.to_str()).map(|e| ["mp4", "webm", "mov", "m4v", "mkv", "m2ts"].contains(&e)).unwrap_or(false);

    if file_size > 500 * 1024 * 1024 && is_video {
        // Automatic Range fallthrough for large videos
        let chunk_size = std::cmp::min(file_size, 10 * 1024 * 1024);
        let mut buffer = vec![0u8; chunk_size as usize];
        if file.seek(std::io::SeekFrom::Start(0)).is_ok() && file.read_exact(&mut buffer).is_ok() {
             return Ok(builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_RANGE, format!("bytes 0-{}/{}", chunk_size - 1, file_size))
                .header(header::CONTENT_LENGTH, chunk_size)
                .body(buffer)
                .unwrap_or_else(|_| Response::default()));
        }
    }

    // High Memory Limit for full download (Images, Fonts, small Videos)
    if file_size > 1024 * 1024 * 1024 {
        return Err(error_response(StatusCode::PAYLOAD_TOO_LARGE, b"File too large".to_vec()));
    }

    let mut all_data = Vec::with_capacity(file_size as usize);
    if file.read_to_end(&mut all_data).is_err() {
        return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, b"Read error".to_vec()));
    }

    Ok(builder
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(all_data)
        .unwrap_or_else(|_| Response::default()))
}
