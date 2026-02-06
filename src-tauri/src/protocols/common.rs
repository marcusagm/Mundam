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
    let mime = if let Some(format) = crate::formats::FileFormat::detect(path) {
        format.mime_types.first().unwrap_or(&"application/octet-stream").to_string()
    } else {
        from_path(path).first_or_octet_stream().to_string()
    };

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
                        let mut real_end = end;
                        let mut chunk_size = (real_end - start) + 1;
                        
                        // Cap chunk size to 100MB to prevent memory exhaustion.
                        // Browsers will automatically request the remaining data in subsequent requests.
                        if chunk_size > 100 * 1024 * 1024 {
                             chunk_size = 100 * 1024 * 1024;
                             real_end = start + chunk_size - 1;
                        }

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
        }
    }

    // Default: Serve entire file (for non-range requests, e.g. small images/fonts)
    // Prevent serving massive files without Range to avoid OOM panics.
    if file_size > 200 * 1024 * 1024 {
        return Err(error_response(StatusCode::PAYLOAD_TOO_LARGE, b"File too large for full download. Use Range requests.".to_vec()));
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
