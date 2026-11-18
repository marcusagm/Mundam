//! Streaming and Transcoding IPC Gateway
//!
//! Provides the Tauri commands required for the frontend to interact with
//! the streaming server and transcoding engine.

use crate::core::error::AppResult;
use crate::core::repository::AssetQueryHandler;
use crate::delivery::tauri::admission::IpcRequest;
use crate::delivery::tauri::commands::queries::StreamingSessionToken;
use crate::feature::transcoding::cache::{CacheStats, TranscodeCache};
use crate::feature::transcoding::detector::{self, MediaKind};
use crate::feature::transcoding::profiles::TranscodeQuality;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// Checks if a file needs transcoding for playback.
#[tauri::command]
pub async fn needs_transcoding(
    _ipc: IpcRequest,
    registry: State<'_, Arc<crate::core::formats::registry::FormatRegistry>>,
    path: String,
) -> AppResult<bool> {
    Ok(detector::needs_transcoding(
        &registry,
        std::path::Path::new(&path),
    ))
}

/// Checks if a file is natively supported by the webview.
#[tauri::command]
pub async fn is_native_format(
    _ipc: IpcRequest,
    registry: State<'_, Arc<crate::core::formats::registry::FormatRegistry>>,
    path: String,
) -> AppResult<bool> {
    Ok(detector::is_native_format(
        &registry,
        std::path::Path::new(&path),
    ))
}

/// Generates a streaming URL for an asset.
#[tauri::command]
pub async fn get_stream_url(
    _ipc: IpcRequest,
    app_handle: AppHandle,
    asset_id: String,
) -> AppResult<String> {
    let session_token = app_handle.state::<StreamingSessionToken>();
    let registry = app_handle.state::<Arc<crate::core::formats::registry::FormatRegistry>>();
    let port = 9876; // Default port

    // Check if it's native or needs HLS
    let query_handler = app_handle.state::<Arc<dyn AssetQueryHandler>>();
    let asset = query_handler
        .get_by_id(&asset_id)
        .await?
        .ok_or_else(|| crate::core::error::AppError::NotFound(asset_id.clone()))?;

    let strategy = detector::get_playback_strategy(&registry, &asset.path);
    let media_kind = detector::get_media_kind(&registry, &asset.path);

    use crate::core::formats::types::PlaybackStrategy;

    match strategy {
        PlaybackStrategy::Native => Ok(format!(
            "http://localhost:{}/stream/{}?token={}",
            port, asset.id, session_token.0
        )),
        PlaybackStrategy::LinearHls | PlaybackStrategy::AudioLinearHls => Ok(format!(
            "http://localhost:{}/hls-live/{}/index.m3u8?token={}",
            port, asset.id, session_token.0
        )),
        PlaybackStrategy::Hls | PlaybackStrategy::AudioHls => Ok(format!(
            "http://localhost:{}/playlist/{}/playlist.m3u8?token={}",
            port, asset.id, session_token.0
        )),
        _ => {
            if media_kind != MediaKind::Unknown {
                // Fallback to standard HLS if strategy isn't None but it's a media format
                Ok(format!(
                    "http://localhost:{}/playlist/{}/playlist.m3u8?token={}",
                    port, asset.id, session_token.0
                ))
            } else {
                Err(crate::core::error::AppError::UnsupportedFormat(
                    asset.path.to_string_lossy().to_string(),
                ))
            }
        }
    }
}

/// Returns available quality options for a video asset.
#[tauri::command]
pub async fn get_quality_options(_ipc: IpcRequest, _asset_id: String) -> Vec<TranscodeQuality> {
    vec![
        TranscodeQuality::Low,
        TranscodeQuality::Medium,
        TranscodeQuality::High,
        TranscodeQuality::Original,
    ]
}

/// Checks if FFmpeg is available on the system.
#[tauri::command]
pub async fn ffmpeg_available(_ipc: IpcRequest) -> bool {
    crate::processing::transcoding::check_transcoding_availability()
}

/// Checks if a cached version of the file exists for a given quality.
#[tauri::command]
pub async fn is_cached(
    _ipc: IpcRequest,
    app_handle: AppHandle,
    asset_id: String,
    quality: TranscodeQuality,
) -> AppResult<bool> {
    let cache = app_handle.state::<Arc<TranscodeCache>>();
    let query_handler = app_handle.state::<Arc<dyn AssetQueryHandler>>();

    let asset = query_handler
        .get_by_id(&asset_id)
        .await?
        .ok_or_else(|| crate::core::error::AppError::NotFound(asset_id))?;

    Ok(cache.exists(&asset.path, quality))
}

/// Returns statistics about the transcode cache.
#[tauri::command]
pub async fn get_streaming_cache_stats(
    _ipc: IpcRequest,
    cache: State<'_, Arc<TranscodeCache>>,
) -> AppResult<CacheStats> {
    Ok(cache.get_stats())
}

/// Manually triggers transcoding for a file (useful for pre-caching).
#[tauri::command]
pub async fn transcode_file(
    _ipc: IpcRequest,
    app_handle: AppHandle,
    asset_id: String,
    _quality: TranscodeQuality,
) -> AppResult<()> {
    let query_handler = app_handle.state::<Arc<dyn AssetQueryHandler>>();
    let hls_manager =
        app_handle.state::<Arc<crate::feature::transcoding::hls_manager::HlsManager>>();

    let asset = query_handler
        .get_by_id(&asset_id)
        .await?
        .ok_or_else(|| crate::core::error::AppError::NotFound(asset_id))?;

    let mime = mime_guess::from_path(&asset.path)
        .first_or_octet_stream()
        .to_string();

    hls_manager
        .get_or_start_stream(&asset.id, &asset.path, Some(&mime))
        .await?;

    Ok(())
}

/// Cleans up the transcode cache.
#[tauri::command]
pub async fn cleanup_cache_streaming(
    _ipc: IpcRequest,
    cache: State<'_, Arc<TranscodeCache>>,
    max_age_days: u64,
) -> AppResult<usize> {
    Ok(cache.cleanup(max_age_days))
}

/// Clears the entire transcode cache.
#[tauri::command]
pub async fn clear_cache_streaming(
    _ipc: IpcRequest,
    cache: State<'_, Arc<TranscodeCache>>,
) -> AppResult<usize> {
    Ok(cache.clear_all())
}
