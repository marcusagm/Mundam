use std::path::PathBuf;
use tauri::http::{header, Response, StatusCode, Request};
use tauri::{AppHandle, Manager};

use super::common::{decode_path, extract_path_part, error_response};
use crate::transcoding::cache::TranscodeCache;
use crate::transcoding::detector;
use crate::transcoding::ffmpeg_pipe::FfmpegTranscoder;
use crate::transcoding::quality::TranscodeQuality;

/// Handler for video-stream:// protocol
/// Transcodes unsupported video formats to MP4 (H.264) on-the-fly or serves from cache
pub fn handler<R: tauri::Runtime>(app: &AppHandle<R>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    
    // Parse path and quality from URI
    // Format: video-stream://localhost/path/to/file.mkv?quality=preview
    let (path_str, quality) = parse_stream_uri(&uri, "video-stream");
    let decoded_path = decode_path(&path_str);
    let mut full_path = PathBuf::from(&decoded_path);

    if !full_path.is_absolute() && cfg!(unix) {
        if !path_str.starts_with('/') {
            full_path = PathBuf::from("/").join(full_path);
        }
    }

    // Verify file exists
    if !full_path.exists() {
        return error_response(StatusCode::NOT_FOUND, b"File not found".to_vec());
    }

    // Check if this format needs transcoding
    if !detector::needs_transcoding(&full_path) {
        // Fallback to regular video serving for native formats
        let range = request.headers().get(header::RANGE);
        return match crate::protocols::common::serve_file(&full_path, range) {
            Ok(res) => res,
            Err(res) => res,
        };
    }

    // Get cache directory
    let app_data = match app.path().app_local_data_dir() {
        Ok(d) => d,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"Failed to get app data directory".to_vec(),
            );
        }
    };

    let cache = TranscodeCache::new(&app_data);

    // Check cache first
    if let Some(cached_path) = cache.get(&full_path, quality) {
        // Serve from cache with full range support
        let range = request.headers().get(header::RANGE);
        return match crate::protocols::common::serve_file(&cached_path, range) {
            Ok(res) => res,
            Err(res) => res,
        };
    }

    // Need to transcode
    let transcoder = FfmpegTranscoder::new(cache);

    if !transcoder.is_available() {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            b"FFmpeg is not available for transcoding".to_vec(),
        );
    }

    // For video, we always transcode to cache first (streaming is complex with seeking)
    // This may take a while for long videos, but provides better seeking experience
    match transcoder.transcode_sync(&full_path, quality) {
        Ok(output_path) => {
            // Serve the transcoded file with range support
            let range = request.headers().get(header::RANGE);
            match crate::protocols::common::serve_file(&output_path, range) {
                Ok(res) => res,
                Err(res) => res,
            }
        }
        Err(e) => {
            eprintln!("TRANSCODE_ERROR: {:?}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transcoding failed: {}", e).into_bytes(),
            )
        }
    }
}

/// Parse the stream URI to extract path and quality
fn parse_stream_uri(uri: &str, scheme: &str) -> (String, TranscodeQuality) {
    // First, extract the path part using the common function
    let path_with_query = extract_path_part(uri, scheme);
    
    // Split path and query string
    let (path, query) = if let Some(pos) = path_with_query.find('?') {
        (&path_with_query[..pos], Some(&path_with_query[pos + 1..]))
    } else {
        (path_with_query.as_str(), None)
    };

    // Parse quality from query string
    let quality = query
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("quality="))
                .map(|p| &p[8..])
        })
        .and_then(TranscodeQuality::from_str)
        .unwrap_or_default();

    (path.to_string(), quality)
}
