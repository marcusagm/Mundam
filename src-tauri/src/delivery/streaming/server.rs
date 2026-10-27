//! HTTP Streaming Server (Axum)
//!
//! A high-performance embedded HTTP server for media delivery.
//! Supported features:
//! - Range Requests (HTTP 206) for MP4/WebM.
//! - HLS Playlist (.m3u8) and Segment (.ts) serving.
//! - Session Token Authentication.
//! - Asset ID to Physical Path resolution via AssetQueryHandler.

use crate::core::error::AppError;
use crate::feature::transcoding::cache::TranscodeCache;
use crate::infra::database::manager::DbManager;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, instrument, warn};

use axum::http::HeaderValue;
use std::net::SocketAddr;
use std::path::Path as StdPath;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use crate::core::repository::AssetQueryHandler;
use crate::delivery::tauri::commands::queries::StreamingSessionToken;

use crate::core::models::asset::Folder;
use crate::delivery::streaming::{playlist, probe, segment};

use crate::delivery::streaming::linear_manager::LinearManager;
use crate::delivery::streaming::process_manager::ProcessManager;
use tokio::sync::RwLock;

/// Shared server state for Axum handlers.
#[derive(Clone)]
struct AppState {
    app_handle: AppHandle,
    cache: Arc<TranscodeCache>,

    registry: Arc<crate::core::formats::registry::FormatRegistry>,
    process_manager: Arc<RwLock<ProcessManager>>,
    linear_manager: LinearManager,
    _database: Arc<DbManager>,
    asset_query_handler: Arc<dyn AssetQueryHandler>,
    session_token: String,
}

/// Helper for Axum error responses in streaming.
struct StreamError(AppError);

impl IntoResponse for StreamError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.0.to_string()).into_response()
    }
}

const SEGMENT_DURATION: u32 = 4;

/// Initializes and starts the Axum streaming server.
pub async fn start_server(
    app_handle: AppHandle,
    port: u16,
    shutdown_token: CancellationToken,
) -> tauri::async_runtime::JoinHandle<()> {
    let asset_query = app_handle
        .state::<Arc<dyn AssetQueryHandler>>()
        .inner()
        .clone();

    let database = app_handle.state::<Arc<DbManager>>().inner().clone();
    let registry = app_handle
        .state::<Arc<crate::core::formats::registry::FormatRegistry>>()
        .inner()
        .clone();
    let session_token = app_handle.state::<StreamingSessionToken>().0.clone();

    // Initialize Delivery-layer managers (Parity with V1)
    let process_manager = Arc::new(RwLock::new(ProcessManager::new()));
    let linear_manager = LinearManager::new(app_handle.clone());
    let cache = app_handle.state::<Arc<TranscodeCache>>().inner().clone();

    let state = AppState {
        app_handle: app_handle.clone(),
        cache,

        registry,
        process_manager: process_manager.clone(),
        linear_manager: linear_manager.clone(),
        _database: database,
        asset_query_handler: asset_query,
        session_token,
    };

    // Spawn cleanup tasks inspired by V1
    let process_cleanup_token = shutdown_token.child_token();
    let pm_clone = process_manager.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            tokio::select! {
                _ = process_cleanup_token.cancelled() => break,
                _ = interval.tick() => {
                    let mut pm = pm_clone.write().await;
                    pm.cleanup_stale(30);
                }
            }
        }
    });

    let linear_cleanup_token = shutdown_token.child_token();
    let lm_clone = linear_manager.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = linear_cleanup_token.cancelled() => break,
                _ = interval.tick() => {
                    lm_clone.cleanup(std::time::Duration::from_secs(60)).await;
                }
            }
        }
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/probe/:asset_id", get(probe_handler))
        .route("/stream/:asset_id", get(stream_handler))
        .route("/playlist/:asset_id/playlist.m3u8", get(playlist_handler))
        .route("/segment/:asset_id/:segment", get(segment_handler))
        .route("/hls-live/*path", get(linear_hls_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(build_cors_layer())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Streaming server listening on http://{}", addr);

    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(error) => {
                tracing::error!("Failed to bind streaming server to {}: {}", addr, error);
                return;
            }
        };
        if let Err(error) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_token.cancelled().await;
                tracing::info!("Streaming server: graceful shutdown initiated");
            })
            .await
        {
            tracing::error!("Streaming server exited with error: {}", error);
        }
        tracing::info!("Streaming server stopped");
    })
}

/// Builds a restricted CORS layer for the streaming server.
fn build_cors_layer() -> CorsLayer {
    let allowed_origins = [
        "tauri://localhost",
        "https://tauri.localhost",
        "http://localhost:1420",
    ]
    .into_iter()
    .filter_map(|origin| origin.parse::<HeaderValue>().ok())
    .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::HEAD,
            axum::http::Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            axum::http::header::RANGE,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::ORIGIN,
        ]))
        .expose_headers([
            axum::http::header::CONTENT_RANGE,
            axum::http::header::CONTENT_LENGTH,
            axum::http::header::ACCEPT_RANGES,
        ])
}

/// Middleware to validate the session token.
async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Health check doesn't need token
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    // CORS pre-flight requests don't need authentication
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // Try to extract token from query parameters
    let query_string = req.uri().query().unwrap_or("");
    let provided_token = query_string.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        match (parts.next(), parts.next()) {
            (Some("token"), Some(value)) => Some(value),
            _ => None,
        }
    });

    match provided_token {
        Some(token) if token == state.session_token => Ok(next.run(req).await),
        _ => {
            tracing::warn!(
                "Unauthorized streaming request: invalid or missing token in URI: {}",
                req.uri()
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn forbidden_response(e: AppError) -> StreamError {
    warn!("Access denied: {:?}", e);
    StreamError(e)
}

/// Strips the Windows verbatim/UNC prefix (e.g. `\\?\` or `\\?\UNC\`) added by `canonicalize()`.
fn strip_unc_prefix(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        std::path::PathBuf::from(format!(r"\\{}", stripped))
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Checks whether `child` is inside `parent` directory, accounting for OS path semantics
/// (e.g. case-insensitivity on Windows and UNC prefix stripping).
fn is_subpath(child: &std::path::Path, parent: &std::path::Path) -> bool {
    let clean_child = strip_unc_prefix(child);
    let clean_parent = strip_unc_prefix(parent);

    #[cfg(windows)]
    {
        use std::path::Component;
        let child_components: Vec<_> = clean_child.components().collect();
        let parent_components: Vec<_> = clean_parent.components().collect();

        if child_components.len() < parent_components.len() {
            return false;
        }

        for (c, p) in child_components.iter().zip(parent_components.iter()) {
            match (c, p) {
                (Component::Prefix(c_pref), Component::Prefix(p_pref)) => {
                    let c_str = c_pref.as_os_str().to_string_lossy().to_lowercase();
                    let p_str = p_pref.as_os_str().to_string_lossy().to_lowercase();
                    let c_norm = c_str.strip_prefix(r"\\?\").unwrap_or(&c_str);
                    let p_norm = p_str.strip_prefix(r"\\?\").unwrap_or(&p_str);
                    if c_norm != p_norm {
                        return false;
                    }
                }
                (Component::Normal(c_name), Component::Normal(p_name)) => {
                    if c_name.to_string_lossy().to_lowercase()
                        != p_name.to_string_lossy().to_lowercase()
                    {
                        return false;
                    }
                }
                (Component::RootDir, Component::RootDir) => {}
                (Component::CurDir, Component::CurDir) => {}
                _ => return false,
            }
        }
        true
    }

    #[cfg(not(windows))]
    {
        clean_child.starts_with(&clean_parent)
    }
}

/// Validates that the asset path is within authorized roots.
async fn validate_path_scope(
    asset_query_handler: &Arc<dyn AssetQueryHandler>,
    path: &std::path::Path,
) -> Result<(), AppError> {
    let path_clone = path.to_path_buf();

    // Canonicalize to resolve symlinks and '..'
    let canonical_path = tokio::task::spawn_blocking(move || path_clone.canonicalize())
        .await
        .map_err(|e| {
            error!("Blocking task join error: {:?}", e);
            AppError::Internal(format!("Blocking task join error: {:?}", e))
        })?
        .map_err(|e| {
            warn!(
                "Path scope validation: cannot resolve path {:?}: {:?}",
                path, e
            );
            AppError::Forbidden(format!("Cannot resolve path: {:?}", e))
        })?;

    // Use AssetQueryHandler to list root locations
    let roots: Vec<Folder> = asset_query_handler.list_folders(None).await.map_err(|e| {
        error!("Failed to list root folders: {:?}", e);
        AppError::Internal(format!("Failed to list root folders: {:?}", e))
    })?;

    let is_within_scope = roots.iter().any(|root| {
        let canonical_root = root.path.canonicalize().unwrap_or_else(|_| root.path.clone());
        is_subpath(&canonical_path, &canonical_root) || is_subpath(&canonical_path, &root.path)
    });

    if !is_within_scope {
        warn!("Access denied: path {:?} is outside authorized roots", path);
        return Err(AppError::Forbidden(format!(
            "Path {:?} is outside authorized roots",
            path
        )));
    }

    Ok(())
}

/// Resolves the physical path for streaming and validates the scope.
/// Bypasses scope validation if the asset is in the internal trash.
async fn resolve_and_validate_path(
    asset: &crate::core::models::asset::Asset,
    state: &AppState,
) -> Result<std::path::PathBuf, StreamError> {
    if asset.deleted_at.is_some() {
        if let Ok(dir) = state.app_handle.path().app_local_data_dir() {
            return Ok(crate::core::trash::resolve_physical_path(asset, &dir));
        }
    }
    
    validate_path_scope(&state.asset_query_handler, &asset.path)
        .await
        .map_err(forbidden_response)?;
        
    Ok(asset.path.clone())
}

async fn health_handler() -> &'static str {
    "OK"
}

/// Probe endpoint - returns video metadata
#[instrument(skip(state))]
async fn probe_handler(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> Result<Response, StreamError> {
    // Resolve asset_id to physical path
    let asset = state
        .asset_query_handler
        .get_by_id(&asset_id)
        .await
        .map_err(|e| StreamError(AppError::Generic(format!("DB error: {}", e))))?
        .ok_or_else(|| StreamError(AppError::NotFound(asset_id)))?;

    let file_path = resolve_and_validate_path(&asset, &state).await?;

    match probe::get_video_info(&state.app_handle, &state.registry, &file_path).await {
        Ok(info) => {
            let json = serde_json::to_string(&info).unwrap_or_default();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap_or_default())
        }
        Err(probe_error) => {
            error!(path = ?file_path, "Probe failed: {}", probe_error);
            Err(StreamError(AppError::Generic(format!(
                "Probe failed: {}",
                probe_error
            ))))
        }
    }
}

/// Handler for direct media streaming with Range support.
async fn stream_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> Result<Response, StreamError> {
    let asset = state
        .asset_query_handler
        .get_by_id(&asset_id)
        .await
        .map_err(|e| StreamError(AppError::Generic(format!("DB error: {}", e))))?
        .ok_or_else(|| StreamError(AppError::NotFound(asset_id)))?;

    let file_path = resolve_and_validate_path(&asset, &state).await?;

    let range = headers.get(header::RANGE).cloned();
    serve_file(&file_path, range)
        .await
        .map_err(|s| StreamError(AppError::Generic(format!("Serve error: {}", s))))
}

/// Playlist endpoint - generates M3U8 dynamically
#[instrument(skip(state))]
async fn playlist_handler(
    State(state): State<AppState>,
    Path(id_and_ext): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, StreamError> {
    tracing::debug!("Playlist request: asset_id_or_path={}", id_and_ext);
    // Expected format: {asset_id}/playlist.m3u8
    let asset_id = id_and_ext
        .split('/')
        .next()
        .unwrap_or(&id_and_ext)
        .to_string();

    // Resolve asset_id to physical path
    let asset = state
        .asset_query_handler
        .get_by_id(&asset_id)
        .await
        .map_err(|e| StreamError(AppError::Generic(format!("DB error: {}", e))))?
        .ok_or_else(|| StreamError(AppError::NotFound(asset_id)))?;

    let file_path = resolve_and_validate_path(&asset, &state).await?;

    let quality = params
        .get("quality")
        .map(|s| s.as_str())
        .unwrap_or("standard");

    // Preserve the token in segment URLs so HLS.js can authenticate each segment request
    let token_param = params
        .get("token")
        .map(|token| format!("&token={}", token))
        .unwrap_or_default();

    // First, probe the video to get duration
    let info = match probe::get_video_info(&state.app_handle, &state.registry, &file_path).await {
        Ok(video_info) => video_info,
        Err(probe_error) => {
            return Err(StreamError(AppError::Generic(format!(
                "Failed to probe video: {}",
                probe_error
            ))));
        }
    };

    let m3u8 = playlist::generate_m3u8_with_token(
        &asset.id,
        info.duration_secs,
        SEGMENT_DURATION as f64,
        quality,
        &token_param,
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(m3u8))
        .unwrap_or_default())
}

/// Segment endpoint - transcodes and serves a video segment
#[instrument(skip(state))]
async fn segment_handler(
    State(state): State<AppState>,
    Path((asset_id, segment)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, StreamError> {
    tracing::debug!(
        "Segment request: asset_id={}, segment={}",
        asset_id,
        segment
    );
    let quality = params
        .get("quality")
        .map(|s| s.as_str())
        .unwrap_or("standard");

    // Extract index from segment part
    let index_str = segment.trim_end_matches(".ts");
    let index = index_str
        .parse::<u32>()
        .map_err(|_| StreamError(AppError::Generic("Invalid segment index".to_string())))?;

    // Resolve asset_id to physical path
    let asset = state
        .asset_query_handler
        .get_by_id(&asset_id)
        .await
        .map_err(|e| StreamError(AppError::Generic(format!("DB error: {}", e))))?
        .ok_or_else(|| StreamError(AppError::NotFound(asset_id)))?;

    let file_path = resolve_and_validate_path(&asset, &state).await?;

    match segment::get_segment(
        &state.app_handle,
        &state.registry,
        &state.cache,
        &state.process_manager,
        &file_path,
        index,
        SEGMENT_DURATION as f64,
        quality,
    )
    .await
    {
        Ok(data) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "video/MP2T")
            .header(header::CACHE_CONTROL, "max-age=3600")
            .body(Body::from(data))
            .unwrap_or_default()),
        Err(segment_error) => {
            error!("SEGMENT_ERROR: {}", segment_error);
            Err(StreamError(AppError::Generic(format!(
                "Segment failed: {}",
                segment_error
            ))))
        }
    }
}

/// Linear HLS Handler using /hls-live/*path
/// Request can be:
/// 1. .../{asset_id}/index.m3u8 -> Starts transcode, returns playlist
/// 2. .../{asset_id}/segment_00001.ts -> Returns segment
#[instrument(skip(state))]
async fn linear_hls_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, StreamError> {
    // 1. Parse Asset ID from path
    let parts: Vec<&str> = path.split('/').collect();
    if parts.is_empty() {
        return Err(StreamError(AppError::Generic(
            "Invalid live path".to_string(),
        )));
    }
    let asset_id = parts[0].to_string();

    // Resolve asset_id to physical path
    let asset = state
        .asset_query_handler
        .get_by_id(&asset_id)
        .await
        .map_err(|e| StreamError(AppError::Generic(format!("DB error: {}", e))))?
        .ok_or_else(|| StreamError(AppError::NotFound(asset_id)))?;

    let file_path = resolve_and_validate_path(&asset, &state).await?;

    // 2. Handle Playlist Request
    if path.ends_with("/index.m3u8") {
        if !file_path.exists() {
            return Err(StreamError(AppError::NotFound(format!(
                "File not found: {:?}",
                file_path
            ))));
        }

        let quality = params
            .get("quality")
            .map(|s| s.as_str())
            .unwrap_or("standard");

        match state
            .linear_manager
            .get_or_start(&asset.id, &file_path, quality)
            .await
        {
            Ok(temp_dir) => {
                let playlist_path = temp_dir.join("index.m3u8");

                // Poll for playlist existence (ffmpeg might take a second to create it, or up to 60s for MIDI synthesis)
                let mut tries = 0;
                while !playlist_path.exists() && tries < 600 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    tries += 1;
                }

                if playlist_path.exists() {
                    match tokio::fs::read_to_string(&playlist_path).await {
                        Ok(content) => {
                            // Inject the session token into all segment URLs
                            let tokenized_content = content
                                .lines()
                                .map(|line| {
                                    if line.ends_with(".ts") {
                                        format!("{}?token={}\n", line, state.session_token)
                                    } else {
                                        format!("{}\n", line)
                                    }
                                })
                                .collect::<String>();

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
                                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                .body(Body::from(tokenized_content))
                                .unwrap_or_default())
                        }
                        Err(read_error) => Err(StreamError(AppError::Generic(format!(
                            "Failed to read playlist: {}",
                            read_error
                        )))),
                    }
                } else {
                    Err(StreamError(AppError::Generic(
                        "Playlist generation timed out".into(),
                    )))
                }
            }
            Err(start_error) => Err(StreamError(AppError::Generic(format!(
                "Failed to start streaming: {}",
                start_error
            )))),
        }
    }
    // 3. Handle Segment Request
    else if path.ends_with(".ts") {
        // Extract asset_id from the path (e.g., "asset_id/segment_index.ts")
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 2 {
            return Err(StreamError(AppError::Generic(
                "Invalid segment path".into(),
            )));
        }
        let asset_id = parts[0].to_string();

        let temp_dir = state
            .linear_manager
            .get_temp_dir(&asset_id)
            .await
            .ok_or_else(|| StreamError(AppError::Generic("Session not found".into())))?;

        // Update the session's last_access to prevent timeout while streaming
        state.linear_manager.update_access(&asset_id).await;

        let segment_path = temp_dir.join(parts[1]);

        // Wait for segment to be ready (it might be being transcoded)
        let mut attempts = 0;
        while !segment_path.exists() && attempts < 50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        if !segment_path.exists() {
            return Err(StreamError(AppError::Generic("Segment timed out".into())));
        }

        serve_file(&segment_path, None)
            .await
            .map_err(|s| StreamError(AppError::Generic(format!("Serve error: {}", s))))
    } else {
        Err(StreamError(AppError::Generic("Invalid HLS path".into())))
    }
}

/// Helper to serve a file using tokio and support Range headers.
async fn serve_file(
    path: &StdPath,
    range: Option<header::HeaderValue>,
) -> Result<Response, StatusCode> {
    let file = File::open(path).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let metadata = file
        .metadata()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let size = metadata.len();

    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    // Simple Range handling (can be optimized with tower_http::services::ServeFile if preferred)
    // For now mirroring the logic from V1/Asset protocol but in Axum style.

    if let Some(range_header) = range.and_then(|h| h.to_str().ok().map(|s| s.to_owned())) {
        if let Some(range_spec) = range_header.strip_prefix("bytes=") {
            let parts: Vec<&str> = range_spec.split('-').collect();
            let start = parts[0].parse::<u64>().unwrap_or(0);
            let end = parts
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(size - 1);

            if start >= size {
                return Ok((
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    format!("bytes */{}", size),
                )
                    .into_response());
            }

            let end = end.min(size - 1);
            let length = end - start + 1;

            let mut file = file;
            use tokio::io::AsyncSeekExt;
            file.seek(tokio::io::SeekFrom::Start(start))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Use explicit take from AsyncReadExt
            let stream = ReaderStream::new(tokio::io::AsyncReadExt::take(file, length));
            let body = Body::from_stream(stream);

            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime)
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, size),
                )
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, length)
                .body(body)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, size)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_strip_unc_prefix() {
        let unc = Path::new(r"\\?\D:\folder\video.mp4");
        assert_eq!(strip_unc_prefix(unc), Path::new(r"D:\folder\video.mp4"));

        let normal = Path::new(r"D:\folder\video.mp4");
        assert_eq!(strip_unc_prefix(normal), Path::new(r"D:\folder\video.mp4"));

        let network = Path::new(r"\\?\UNC\server\share\file.mp4");
        assert_eq!(strip_unc_prefix(network), Path::new(r"\\server\share\file.mp4"));
    }

    #[test]
    fn test_is_subpath() {
        // Windows UNC canonicalized child vs standard root
        let child = Path::new(r"\\?\D:\Instagram\Mulheres\video.mp4");
        let parent = Path::new(r"D:\Instagram\Mulheres");
        assert!(is_subpath(child, parent));

        // Case insensitivity
        let child_lower = Path::new(r"d:\instagram\mulheres\video.mp4");
        let parent_upper = Path::new(r"D:\Instagram\Mulheres");
        assert!(is_subpath(child_lower, parent_upper));

        // Outside scope
        let child_outside = Path::new(r"D:\OtherFolder\video.mp4");
        assert!(!is_subpath(child_outside, parent));

        // Different drive
        let child_diff_drive = Path::new(r"C:\Instagram\Mulheres\video.mp4");
        assert!(!is_subpath(child_diff_drive, parent));
    }
}

