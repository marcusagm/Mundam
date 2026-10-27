use std::path::Path;
use tauri::http::{header, Response, StatusCode};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::error;

/// Generic error response for protocol handlers
pub fn error_response(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .unwrap_or_default()
}

/// Helper to decode URI paths and handle potential issues
pub fn decode_uri_path(uri: &str, scheme: &str) -> String {
    let prefixes = [
        format!("{}://localhost/", scheme),
        format!("{}://", scheme),
        format!("http://{}.localhost/", scheme),
        format!("https://{}.localhost/", scheme),
    ];
    let mut path = uri;
    for prefix in &prefixes {
        if let Some(rest) = uri.strip_prefix(prefix) {
            path = rest;
            break;
        }
    }

    // Remove query parameters if any
    let path = path.split('?').next().unwrap_or(path);

    let decoded = percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .into_owned();

    // On Windows, if a path starts with "/C:/" or "/C:\", strip the leading slash
    if cfg!(windows) && decoded.starts_with('/') && decoded.len() > 2 && decoded.as_bytes()[2] == b':' {
        decoded[1..].to_string()
    } else {
        decoded
    }
}

/// Asynchronously serves a file with HTTP 206 Partial Content (Range) support.
/// Reusable by asset://, video:// and audio:// protocols.
pub async fn serve_file(
    path: &Path,
    range: Option<&header::HeaderValue>,
) -> Result<Response<Vec<u8>>, Response<Vec<u8>>> {
    if !path.exists() {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            format!("File not found: {:?}", path).into_bytes(),
        ));
    }

    let mut file = File::open(path).await.map_err(|e| {
        error!("Protocol disk error: {:?}", e);
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            b"Disk access error".to_vec(),
        )
    })?;

    let metadata = file.metadata().await.map_err(|_| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            b"Metadata error".to_vec(),
        )
    })?;

    let file_size = metadata.len();
    if file_size == 0 {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, 0)
            .body(Vec::new())
            .unwrap_or_default());
    }

    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    let builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Range, Content-Length, Accept-Ranges",
        );

    // Partial Content Read (Chunking limit active)
    if let Some(range_value) = range {
        if let Ok(range_str) = range_value.to_str() {
            if let Some(range_spec) = range_str.strip_prefix("bytes=") {
                let mut start: u64 = 0;
                let mut end: u64 = file_size.saturating_sub(1);

                let parts: Vec<&str> = range_spec.split('-').collect();
                if !parts.is_empty() && !parts[0].is_empty() {
                    if let Ok(s) = parts[0].parse::<u64>() {
                        start = s;
                    }
                }

                if parts.len() >= 2 && !parts[1].is_empty() {
                    if let Ok(e) = parts[1].parse::<u64>() {
                        end = e;
                    }
                } else if parts.len() == 1 && range_spec.starts_with('-') {
                    if let Ok(suffix) = range_spec[1..].parse::<u64>() {
                        start = file_size.saturating_sub(suffix);
                        end = file_size.saturating_sub(1);
                    }
                }

                if start >= file_size {
                    return Err(error_response(
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        format!("bytes */{}", file_size).into_bytes(),
                    ));
                }
                if end >= file_size {
                    end = file_size.saturating_sub(1);
                }
                if start > end {
                    end = start;
                }

                // Chunk Size Rule: Max limit bounded (10 MB chunks per UI Network Request).
                let max_chunk = 10 * 1024 * 1024;
                let requested_size = (end - start) + 1;
                let chunk_size = std::cmp::min(requested_size, max_chunk);

                let mut buffer = vec![0u8; chunk_size as usize];
                if file.seek(std::io::SeekFrom::Start(start)).await.is_ok() {
                    if let Ok(bytes_read) = file.read_exact(&mut buffer).await {
                        buffer.truncate(bytes_read);
                        return Ok(builder
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header(
                                header::CONTENT_RANGE,
                                format!(
                                    "bytes {}-{}/{}",
                                    start,
                                    start + bytes_read as u64 - 1,
                                    file_size
                                ),
                            )
                            .header(header::CONTENT_LENGTH, bytes_read as u64)
                            .body(buffer)
                            .unwrap_or_else(|_| Response::default()));
                    }
                }
            }
        }
    }

    // Default Request Strategy for large files (Chunked)
    if file_size > 500 * 1024 * 1024 {
        let chunk_size = std::cmp::min(file_size, 10 * 1024 * 1024);
        let mut buffer = vec![0u8; chunk_size as usize];
        if file.seek(std::io::SeekFrom::Start(0)).await.is_ok() {
            if let Ok(bytes_read) = file.read_exact(&mut buffer).await {
                buffer.truncate(bytes_read);
                return Ok(builder
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes 0-{}/{}", bytes_read as u64 - 1, file_size),
                    )
                    .header(header::CONTENT_LENGTH, bytes_read as u64)
                    .body(buffer)
                    .unwrap_or_else(|_| Response::default()));
            }
        }
    }

    // Standard small file strategy
    let mut all_data = Vec::with_capacity(file_size as usize);
    if file.read_to_end(&mut all_data).await.is_err() {
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            b"Read disk error".to_vec(),
        ));
    }

    Ok(builder
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(all_data)
        .unwrap_or_else(|_| Response::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_uri_path_macos() {
        let decoded = decode_uri_path("video://localhost/Movies/sample.mp4", "video");
        assert_eq!(decoded, "Movies/sample.mp4");

        let decoded_audio = decode_uri_path("audio://localhost/Music/song.mp3?query=1", "audio");
        assert_eq!(decoded_audio, "Music/song.mp3");
    }

    #[test]
    fn test_decode_uri_path_windows() {
        let decoded = decode_uri_path("http://video.localhost/Movies/sample.mp4", "video");
        assert_eq!(decoded, "Movies/sample.mp4");

        let decoded_audio = decode_uri_path("http://audio.localhost/Music/song.mp3?query=1", "audio");
        assert_eq!(decoded_audio, "Music/song.mp3");
    }
}
