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

use crate::delivery::streaming::http_diagnostics::{track_handler, ActiveHandlers};
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
    shutdown_token: CancellationToken,
}

/// Helper for Axum error responses in streaming.
struct StreamError(AppError);

impl IntoResponse for StreamError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Cancelled => StatusCode::SERVICE_UNAVAILABLE,
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
    let lifecycle = app_handle
        .state::<Arc<crate::lifecycle::LifecycleRegistry>>()
        .inner()
        .clone();

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
        shutdown_token: shutdown_token.clone(),
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

    let active_handlers = Arc::new(ActiveHandlers::default());
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
        .layer(middleware::from_fn_with_state(
            active_handlers.clone(),
            track_handler,
        ))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Streaming server listening on http://{}", addr);

    let shutdown_process_manager = process_manager.clone();
    let shutdown_linear_manager = linear_manager.clone();
    let shutdown_lifecycle = lifecycle.clone();
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(error) => {
                tracing::error!("Failed to bind streaming server to {}: {}", addr, error);
                return;
            }
        };
        let server = serve_http_until_shutdown(listener, app, shutdown_token.clone());
        let cleanup_token = shutdown_token.clone();
        let cleanup = async move {
            cleanup_token.cancelled().await;
            tracing::info!("Streaming server: graceful shutdown initiated");
            let processes = shutdown_process_manager.write().await.cancel_all();
            let sessions = shutdown_linear_manager.shutdown().await;
            if !sessions.is_clean() {
                shutdown_lifecycle.report_failure("linear streaming cleanup");
            }
            tracing::info!(
                process_cancellations = processes,
                linear_sessions = sessions.session_count,
                "Streaming server: cleanup attempts completed; draining HTTP requests"
            );
            loop {
                let manager = shutdown_process_manager.read().await;
                let active = manager.active_count();
                if active == 0 {
                    let failures = manager.owner_failures();
                    if failures > 0 {
                        tracing::error!(failures, "Streaming server: process supervision failures");
                        shutdown_lifecycle.report_failure("streaming process supervision");
                    }
                    tracing::info!("Streaming server: all registered process owners finished");
                    break;
                }
                drop(manager);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        };
        tokio::pin!(server);
        tokio::pin!(cleanup);
        let result = tokio::select! {
            result = &mut server => {
                if shutdown_token.is_cancelled() {
                    cleanup.await;
                }
                result
            }
            _ = &mut cleanup => {
                active_handlers.log_active();
                let mut interval = tokio::time::interval_at(
                    tokio::time::Instant::now() + Duration::from_secs(1),
                    Duration::from_secs(1),
                );
                loop {
                    tokio::select! {
                        result = &mut server => break result,
                        _ = interval.tick() => active_handlers.log_active(),
                    }
                }
            }
        };
        if let Err(error) = result {
            tracing::error!("Streaming server exited with error: {}", error);
            lifecycle.report_failure("streaming server");
        }
        tracing::info!("Streaming server stopped");
    })
}

async fn serve_http_until_shutdown(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown_token: CancellationToken,
) -> std::io::Result<()> {
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_token.cancelled_owned())
        .await
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
    let started = std::time::Instant::now();
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
    tracing::debug!(
        elapsed_ms = started.elapsed().as_millis(),
        "Streaming asset path canonicalized"
    );

    // Use AssetQueryHandler to list root locations
    let roots: Vec<Folder> = asset_query_handler.list_folders(None).await.map_err(|e| {
        error!("Failed to list root folders: {:?}", e);
        AppError::Internal(format!("Failed to list root folders: {:?}", e))
    })?;

    let root_count = roots.len();
    let is_within_scope = tokio::task::spawn_blocking(move || {
        roots.iter().any(|root| {
            let canonical_root = root
                .path
                .canonicalize()
                .unwrap_or_else(|_| root.path.clone());
            is_subpath(&canonical_path, &canonical_root) || is_subpath(&canonical_path, &root.path)
        })
    })
    .await
    .map_err(|error| AppError::Internal(format!("Root scope validation task failed: {error}")))?;
    tracing::debug!(
        root_count,
        elapsed_ms = started.elapsed().as_millis(),
        "Streaming path scope validated"
    );

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

    match probe::get_video_info(
        &state.app_handle,
        &state.registry,
        &file_path,
        &state.process_manager,
        &state.shutdown_token,
    )
    .await
    {
        Ok(info) => {
            let json = serde_json::to_string(&info).unwrap_or_default();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap_or_default())
        }
        Err(probe_error) => {
            if probe::is_shutdown_cancellation(probe_error.as_ref()) {
                return Err(StreamError(AppError::Cancelled));
            }
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
#[instrument(skip(state, params))]
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
    let info = match probe::get_video_info(
        &state.app_handle,
        &state.registry,
        &file_path,
        &state.process_manager,
        &state.shutdown_token,
    )
    .await
    {
        Ok(video_info) => video_info,
        Err(probe_error) => {
            if probe::is_shutdown_cancellation(probe_error.as_ref()) {
                return Err(StreamError(AppError::Cancelled));
            }
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
#[instrument(skip(state, params))]
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
        segment::SegmentRequest {
            file_path: &file_path,
            segment_index: index,
            segment_duration: SEGMENT_DURATION as f64,
            quality,
        },
    )
    .await
    {
        Ok(data) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "video/MP2T")
            .header(header::CACHE_CONTROL, "max-age=3600")
            .body(Body::from(data))
            .unwrap_or_default()),
        Err(segment_error) => segment_error_response(segment_error.as_ref()),
    }
}

fn segment_error_response(
    segment_error: &(dyn std::error::Error + Send + Sync + 'static),
) -> Result<Response, StreamError> {
    if segment_error
        .downcast_ref::<segment::SegmentAdmissionClosed>()
        .is_some()
    {
        tracing::info!("Segment request rejected during shutdown");
        return Ok((StatusCode::SERVICE_UNAVAILABLE, "Segment admission closed").into_response());
    }
    if let Some(cancelled) = segment_error.downcast_ref::<segment::SegmentCancelled>() {
        tracing::info!(reason = ?cancelled.0, "Segment request cancelled");
        return Ok((StatusCode::SERVICE_UNAVAILABLE, "Segment request cancelled").into_response());
    }
    error!("SEGMENT_ERROR: {}", segment_error);
    Err(StreamError(AppError::Generic(format!(
        "Segment failed: {}",
        segment_error
    ))))
}

/// Linear HLS Handler using /hls-live/*path
/// Request can be:
/// 1. .../{asset_id}/index.m3u8 -> Starts transcode, returns playlist
/// 2. .../{asset_id}/segment_00001.ts -> Returns segment
#[instrument(skip(state, params))]
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

                let wait = wait_for_stream_file(
                    &playlist_path,
                    Duration::from_secs(60),
                    &state.shutdown_token,
                )
                .await
                .map_err(|error| {
                    StreamError(AppError::Generic(format!(
                        "Failed to check playlist: {error}"
                    )))
                })?;
                if wait == FileWait::Cancelled {
                    tracing::info!("Linear streaming: playlist wait cancelled during shutdown");
                    return Ok(StatusCode::SERVICE_UNAVAILABLE.into_response());
                }
                if wait == FileWait::Ready {
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

        let wait =
            wait_for_stream_file(&segment_path, Duration::from_secs(5), &state.shutdown_token)
                .await
                .map_err(|error| {
                    StreamError(AppError::Generic(format!(
                        "Failed to check segment: {error}"
                    )))
                })?;
        if wait == FileWait::Cancelled {
            tracing::info!("Linear streaming: segment wait cancelled during shutdown");
            return Ok(StatusCode::SERVICE_UNAVAILABLE.into_response());
        }
        if wait == FileWait::TimedOut {
            return Err(StreamError(AppError::Generic("Segment timed out".into())));
        }

        serve_file(&segment_path, None)
            .await
            .map_err(|s| StreamError(AppError::Generic(format!("Serve error: {}", s))))
    } else {
        Err(StreamError(AppError::Generic("Invalid HLS path".into())))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FileWait {
    Ready,
    TimedOut,
    Cancelled,
}

async fn wait_for_stream_file(
    path: &StdPath,
    timeout: Duration,
    shutdown_token: &CancellationToken,
) -> std::io::Result<FileWait> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if shutdown_token.is_cancelled() {
            return Ok(FileWait::Cancelled);
        }
        if tokio::fs::try_exists(path).await? {
            return Ok(FileWait::Ready);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(FileWait::TimedOut);
        }
        tokio::select! {
            biased;
            _ = shutdown_token.cancelled() => return Ok(FileWait::Cancelled),
            _ = tokio::time::sleep_until((tokio::time::Instant::now() + Duration::from_millis(100)).min(deadline)) => {}
        }
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
    fn segment_admission_and_real_failures_have_distinct_http_statuses() {
        let response = match segment_error_response(&segment::SegmentAdmissionClosed) {
            Ok(response) => response,
            Err(_) => panic!("closed admission should produce a response"),
        };
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let error = std::io::Error::other("transcoding failed");
        let response = match segment_error_response(&error) {
            Ok(_) => panic!("a real failure must not be classified as closed admission"),
            Err(error) => error.into_response(),
        };
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn http_shutdown_stops_accepting_while_existing_request_drains(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let entered_tx = Arc::new(tokio::sync::Mutex::new(Some(entered_tx)));
        let release = Arc::new(tokio::sync::Notify::new());
        let handler_release = release.clone();
        let app = Router::new().route(
            "/test",
            get(move || {
                let entered_tx = entered_tx.clone();
                let release = handler_release.clone();
                async move {
                    if let Some(sender) = entered_tx.lock().await.take() {
                        let _ = sender.send(());
                    }
                    release.notified().await;
                    StatusCode::OK
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let token = CancellationToken::new();
        let server = tokio::spawn(serve_http_until_shutdown(listener, app, token.clone()));
        let mut client = tokio::net::TcpStream::connect(address).await?;
        client
            .write_all(b"GET /test HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        tokio::time::timeout(Duration::from_secs(2), entered_rx)
            .await
            .map_err(|_| "HTTP handler did not start")??;
        token.cancel();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match tokio::time::timeout(
                    Duration::from_millis(500),
                    tokio::net::TcpStream::connect(address),
                )
                .await
                {
                    Ok(Ok(_)) => tokio::time::sleep(Duration::from_millis(10)).await,
                    Ok(Err(_)) | Err(_) => break,
                }
            }
        })
        .await
        .map_err(|_| "HTTP listener remained reachable after shutdown")?;
        assert!(!server.is_finished());
        release.notify_one();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), client.read_to_end(&mut response))
            .await
            .map_err(|_| "HTTP response did not drain")??;
        assert!(response.starts_with(b"HTTP/1.1 200"));
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .map_err(|_| "HTTP server did not stop after the request drained")???;
        Ok(())
    }

    #[tokio::test]
    async fn stream_file_wait_handles_ready_timeout_and_cancellation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let file = directory.path().join("playlist.m3u8");
        let token = CancellationToken::new();
        assert_eq!(
            wait_for_stream_file(&file, Duration::from_millis(30), &token).await?,
            FileWait::TimedOut
        );
        tokio::fs::write(&file, b"playlist").await?;
        assert_eq!(
            wait_for_stream_file(&file, Duration::from_secs(1), &token).await?,
            FileWait::Ready
        );
        token.cancel();
        assert_eq!(
            wait_for_stream_file(&file, Duration::from_secs(1), &token).await?,
            FileWait::Cancelled
        );
        Ok(())
    }

    #[tokio::test]
    async fn cancellation_releases_a_waiting_http_request() -> Result<(), Box<dyn std::error::Error>>
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let directory = tempfile::tempdir()?;
        let path = directory.path().join("missing.m3u8");
        let token = CancellationToken::new();
        let request_token = token.clone();
        let (entered_sender, entered_receiver) = tokio::sync::oneshot::channel();
        let entered_sender = Arc::new(tokio::sync::Mutex::new(Some(entered_sender)));
        let handlers = Arc::new(ActiveHandlers::default());
        let app = Router::new()
            .route(
                "/hls-live/test",
                get(move || {
                    let path = path.clone();
                    let token = request_token.clone();
                    let entered_sender = entered_sender.clone();
                    async move {
                        if let Some(sender) = entered_sender.lock().await.take() {
                            let _ = sender.send(());
                        }
                        match wait_for_stream_file(&path, Duration::from_secs(60), &token).await {
                            Ok(FileWait::Cancelled) => StatusCode::SERVICE_UNAVAILABLE,
                            Ok(FileWait::Ready) => StatusCode::OK,
                            _ => StatusCode::INTERNAL_SERVER_ERROR,
                        }
                    }
                }),
            )
            .layer(middleware::from_fn_with_state(
                handlers.clone(),
                track_handler,
            ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let shutdown_token = token.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_token.cancelled_owned())
                .await
        });
        let mut client = tokio::net::TcpStream::connect(address).await?;
        client
            .write_all(
                b"GET /hls-live/test HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            )
            .await?;
        tokio::time::timeout(Duration::from_secs(2), entered_receiver).await??;
        assert_eq!(handlers.count(5), 1);
        token.cancel();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), client.read_to_end(&mut response)).await??;
        assert!(response.starts_with(b"HTTP/1.1 503"));
        tokio::time::timeout(Duration::from_secs(2), server).await???;
        assert_eq!(handlers.count(5), 0);
        Ok(())
    }

    #[test]
    fn test_strip_unc_prefix() {
        let unc = Path::new(r"\\?\D:\folder\video.mp4");
        assert_eq!(strip_unc_prefix(unc), Path::new(r"D:\folder\video.mp4"));

        let normal = Path::new(r"D:\folder\video.mp4");
        assert_eq!(strip_unc_prefix(normal), Path::new(r"D:\folder\video.mp4"));

        let network = Path::new(r"\\?\UNC\server\share\file.mp4");
        assert_eq!(
            strip_unc_prefix(network),
            Path::new(r"\\server\share\file.mp4")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn test_is_subpath_unix() {
        let parent = Path::new("/library/folder");
        assert!(is_subpath(Path::new("/library/folder/video.mp4"), parent));
        assert!(!is_subpath(
            Path::new("/library/folder-other/video.mp4"),
            parent
        ));
        assert!(!is_subpath(Path::new("/LIBRARY/folder/video.mp4"), parent));
    }

    #[cfg(windows)]
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
