//! Axum HTTP Server for HLS Streaming
//!
//! Runs on a separate thread and provides endpoints for:
//! - /health - Health check
//! - /probe/{path} - Get video metadata and native format detection
//! - /playlist/{path} - Generate M3U8 playlist dynamically
//! - /segment/{path}/{index} - Transcode and serve video segments

use axum::{
    routing::get,
    Router,
    extract::{Path, State},
    response::{IntoResponse, Response},
    http::{StatusCode, header},
    body::Body,
};
use axum::extract::Query;
use std::collections::HashMap;
use tower_http::cors::{CorsLayer, Any};
use std::time::Duration;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tauri::Manager;

use super::{probe, playlist, segment, process_manager::ProcessManager, linear::LinearManager};
use crate::transcoding::cache::TranscodeCache;

/// Default port for the HLS streaming server
pub const DEFAULT_PORT: u16 = 9876;

/// Segment duration in seconds
pub const SEGMENT_DURATION: f64 = 10.0;

/// Shared state for the streaming server
#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<TranscodeCache>,
    pub process_manager: Arc<RwLock<ProcessManager>>,
    pub linear_manager: LinearManager,
    pub app_handle: tauri::AppHandle,
}

/// The HLS Streaming Server
pub struct StreamingServer {
    port: u16,
    app_handle: tauri::AppHandle,
}

impl StreamingServer {
    /// Create a new streaming server instance
    pub fn new(port: u16, app_handle: tauri::AppHandle) -> Self {
        Self { port, app_handle }
    }

    /// Start the server on a background task
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app_data = self.app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        let cache = Arc::new(TranscodeCache::new(&app_data));
        let process_manager = Arc::new(RwLock::new(ProcessManager::new()));
        let linear_manager = LinearManager::new(self.app_handle.clone());

        let state = AppState {
            cache,
            process_manager: process_manager.clone(),
            linear_manager: linear_manager.clone(),
            app_handle: self.app_handle.clone(),
        };

        // Spawn cleanup task
        let pm_clone = process_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut pm = pm_clone.write().await;
                pm.cleanup_stale(30); // 30 seconds timeout
            }
        });

        // Spawn cleanup task for linear sessions
        let lm_clone = linear_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                // Cleanup sessions inactive for 60 seconds
                lm_clone.cleanup(Duration::from_secs(60)).await;
            }
        });

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/probe/*path", get(probe_handler))
            .route("/playlist/*path", get(playlist_handler))
            .route("/segment/*path", get(segment_handler))
            // New routes for linear HLS
            .route("/hls-live/*path", get(linear_hls_handler))
            .layer(cors)
            .with_state(state);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        println!("INFO: HLS streaming server started on http://{}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Probe endpoint - returns video metadata
async fn probe_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Response {
    let file_path = decode_path(&path);
    println!("DEBUG: Probe request for: {:?}", file_path);

    match probe::get_video_info(&state.app_handle, &file_path).await {
        Ok(info) => {
            println!("DEBUG: Probe success - native: {}, codec: {:?}", info.is_native, info.video_codec);
            let json = serde_json::to_string(&info).unwrap_or_default();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap()
        }
        Err(e) => {
            eprintln!("PROBE_ERROR for {:?}: {}", file_path, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Probe failed: {}", e)))
                .unwrap()
        }
    }
}

/// Playlist endpoint - generates M3U8 dynamically
async fn playlist_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let file_path = decode_path(&path);
    let quality = params.get("quality").map(|s| s.as_str()).unwrap_or("standard");

    // First, probe the video to get duration
    let info = match probe::get_video_info(&state.app_handle, &file_path).await {
        Ok(i) => i,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Failed to probe video: {}", e)))
                .unwrap();
        }
    };

    let m3u8 = playlist::generate_m3u8(&path, info.duration_secs, SEGMENT_DURATION, quality);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(m3u8))
        .unwrap()
}

/// Segment endpoint - transcodes and serves a video segment
async fn segment_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let quality = params.get("quality").map(|s| s.as_str()).unwrap_or("standard");
    // Path format: /segment/{encoded_file_path}/{index}
    // We need to parse out the index from the end
    let (file_path, index) = match parse_segment_path(&path) {
        Some((p, i)) => (p, i),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Invalid segment path format"))
                .unwrap();
        }
    };

    match segment::get_segment(
        &state.app_handle,
        &state.cache,
        &state.process_manager,
        &file_path,
        index,
        SEGMENT_DURATION,
        quality,
    ).await {
        Ok(data) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "video/MP2T")
                .header(header::CACHE_CONTROL, "max-age=3600")
                .body(Body::from(data))
                .unwrap()
        }
        Err(e) => {
            eprintln!("SEGMENT_ERROR: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Segment failed: {}", e)))
                .unwrap()
        }
    }
}

/// Linear HLS Handler using /hls-live/*path
/// Request can be:
/// 1. .../video.swf/index.m3u8 -> Starts transcode, returns playlist
/// 2. .../video.swf/segment_00001.ts -> Returns segment
async fn linear_hls_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    // 1. Parse path
    // Path might look like "Users/me/video.swf/index.m3u8"
    // We need to split the "resource" (last part) from "file path"

    let decoded_full = urlencoding::decode(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));

    // Check if valid request
    if decoded_full.ends_with("/index.m3u8") {
        // Playlist request
        let file_path_str = decoded_full.trim_end_matches("/index.m3u8");
        // Check for double slash issues
        let file_path_str = if file_path_str.ends_with('/') { &file_path_str[..file_path_str.len()-1] } else { file_path_str };

        // Handle absolute path restoration
        let file_path = if cfg!(windows) {
             PathBuf::from(file_path_str)
        } else {
             if file_path_str.starts_with('/') {
                PathBuf::from(file_path_str)
             } else {
                PathBuf::from("/").join(file_path_str)
             }
        };

        if !file_path.exists() {
             return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("File not found: {:?}", file_path)))
                .unwrap();
        }

        let quality = params.get("quality").map(|s| s.as_str()).unwrap_or("standard");

        match state.linear_manager.get_or_start(&file_path, quality).await {
            Ok(temp_dir) => {
                let playlist_path = temp_dir.join("index.m3u8");

                // Poll for playlist existence (ffmpeg might take a second to create it)
                let mut tries = 0;
                while !playlist_path.exists() && tries < 100 { // Wait up to 10s
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    tries += 1;
                }

                if playlist_path.exists() {
                    match tokio::fs::read_to_string(&playlist_path).await {
                         Ok(content) => {
                            Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
                                .body(Body::from(content))
                                .unwrap()
                         }
                         Err(e) => {
                             Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("Failed to read playlist: {}", e)))
                                .unwrap()
                         }
                    }
                } else {
                    Response::builder()
                        .status(StatusCode::REQUEST_TIMEOUT)
                        .body(Body::from("Playlist generation timed out"))
                        .unwrap()
                }
            }
            Err(e) => {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to start streaming: {}", e)))
                    .unwrap()
            }
        }

    } else if decoded_full.ends_with(".ts") {
        // Segment request: .../video.swf/segment_00001.ts
        // Extract filename and file path
        // Assume format: {file_path}/{segment_name}

        let path_buf = PathBuf::from(decoded_full.as_ref());
        let segment_name = path_buf.file_name().unwrap().to_string_lossy();
        let file_path_part = path_buf.parent().unwrap();

        // Convert file_path_part back to absolute path
        let file_path_str = file_path_part.to_string_lossy();
        let file_path = if cfg!(unix) && !file_path_str.starts_with('/') {
             PathBuf::from("/").join(file_path_str.as_ref())
        } else {
             PathBuf::from(file_path_str.as_ref())
        };

        if let Some(temp_dir) = state.linear_manager.get_temp_dir(&file_path).await {
             let segment_path = temp_dir.join(segment_name.as_ref());
             if segment_path.exists() {
                 match tokio::fs::read(&segment_path).await {
                     Ok(data) => {
                         Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "video/MP2T")
                            .header(header::CACHE_CONTROL, "no-cache")
                            .body(Body::from(data))
                            .unwrap()
                     }
                     Err(_) => {
                         Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
                     }
                 }
             } else {
                 Response::builder().status(StatusCode::NOT_FOUND).body(Body::from("Segment not found")).unwrap()
             }
        } else {
             Response::builder().status(StatusCode::NOT_FOUND).body(Body::from("Session not active")).unwrap()
        }

    } else {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid HLS Live request"))
            .unwrap()
    }
}

/// Decode URL-encoded path
fn decode_path(path: &str) -> PathBuf {
    // URL decode the path first
    let decoded = urlencoding::decode(path)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| path.to_string());

    // The path comes as /path/to/file (the leading slash is part of the route)
    // For Unix absolute paths like /Users/..., we need to preserve them
    // For Windows paths like C:\..., they would be encoded differently

    // If path starts with "/" and next char is not "/", it's an absolute Unix path
    // that was passed directly, so preserve it
    if decoded.starts_with('/') {
        // Check if it looks like a Unix absolute path (e.g., /Users, /home, /tmp)
        let parts: Vec<&str> = decoded.splitn(3, '/').collect();
        if parts.len() >= 2 && !parts[1].is_empty() {
            // Valid Unix path like /Users/...
            return PathBuf::from(&decoded);
        }
    }

    // Fallback: treat as relative path
    PathBuf::from(decoded)
}

/// Parse segment path to extract file path and index
/// Format: {url_encoded_path}/{index}
fn parse_segment_path(path: &str) -> Option<(PathBuf, u32)> {
    // URL decode first
    let decoded = urlencoding::decode(path)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| path.to_string());

    // Find the last slash to separate index from path
    if let Some(last_slash) = decoded.rfind('/') {
        let file_part = &decoded[..last_slash];
        let index_part = &decoded[last_slash + 1..];

        // Try to parse index (might have .ts extension)
        let index_str = index_part.trim_end_matches(".ts");
        if let Ok(index) = index_str.parse::<u32>() {
            return Some((PathBuf::from(file_part), index));
        }
    }

    None
}

/// Start the streaming server in a background task
pub fn spawn_server(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let server = StreamingServer::new(DEFAULT_PORT, app_handle);
        if let Err(e) = server.start().await {
            eprintln!("ERROR: HLS streaming server failed: {}", e);
        }
    });
}
