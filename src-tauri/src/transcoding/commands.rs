use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use super::cache::TranscodeCache;
use super::detector;
use super::ffmpeg_pipe::FfmpegTranscoder;
use super::quality::TranscodeQuality;

/// Check if a file needs transcoding for playback
#[tauri::command]
pub fn needs_transcoding(path: String) -> bool {
    detector::needs_transcoding(Path::new(&path))
}

/// Check if a file is natively supported
#[tauri::command]
pub fn is_native_format(path: String) -> bool {
    detector::is_native_format(Path::new(&path))
}

/// Get the appropriate stream URL for a file
/// Returns `audio://` or `video://` for native formats
/// Returns `audio-stream://` or `video-stream://` for transcoded formats
#[tauri::command]
pub fn get_stream_url(path: String, quality: Option<String>) -> String {
    let file_path = Path::new(&path);
    let quality_param = quality.unwrap_or_else(|| "preview".to_string());

    if detector::needs_transcoding(file_path) {
        // Use streaming protocol
        let media_type = detector::get_media_type(file_path);
        match media_type {
            detector::MediaType::Audio => {
                format!("audio-stream://localhost/{}?quality={}",
                    urlencoding::encode(&path), quality_param)
            }
            detector::MediaType::Video | detector::MediaType::Unknown => {
                format!("video-stream://localhost/{}?quality={}",
                    urlencoding::encode(&path), quality_param)
            }
        }
    } else {
        // Use native protocol
        let media_type = detector::get_media_type(file_path);
        match media_type {
            detector::MediaType::Audio => {
                format!("audio://localhost/{}", urlencoding::encode(&path))
            }
            detector::MediaType::Video | detector::MediaType::Unknown => {
                format!("video://localhost/{}", urlencoding::encode(&path))
            }
        }
    }
}

/// Get available quality options
#[tauri::command]
pub fn get_quality_options() -> Vec<QualityOption> {
    TranscodeQuality::all()
        .iter()
        .map(|q| QualityOption {
            id: format!("{:?}", q).to_lowercase(),
            label: q.label().to_string(),
            video_bitrate: q.video_bitrate(),
            audio_bitrate: q.audio_bitrate(),
        })
        .collect()
}

/// Transcode a file and return the cached path
/// This is a blocking operation used for "pre-transcoding"
#[tauri::command]
pub async fn transcode_file(
    app: AppHandle,
    path: String,
    quality: Option<String>,
) -> AppResult<String> {
    let file_path = PathBuf::from(&path);
    let quality = quality
        .and_then(|q| TranscodeQuality::from_str(&q))
        .unwrap_or_default();

    // Get app data dir for cache
    let app_data = app
        .path()
        .app_local_data_dir()?;

    let cache = TranscodeCache::new(&app_data);
    let transcoder = FfmpegTranscoder::new_with_app(cache, &app);

    // Check if FFmpeg is available
    if !transcoder.is_available() {
        return Err(AppError::Transcoding("FFmpeg is not installed or not found in PATH".to_string()));
    }

    // Transcode synchronously (in background thread)
    let result = tokio::task::spawn_blocking(move || {
        transcoder.transcode_sync(&file_path, quality)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    match result {
        Ok(output_path) => Ok(output_path.to_string_lossy().to_string()),
        Err(e) => Err(AppError::Transcoding(e.to_string())),
    }
}

/// Check if a transcoded version is already cached
#[tauri::command]
pub fn is_cached(app: AppHandle, path: String, quality: Option<String>) -> bool {
    let file_path = Path::new(&path);
    let quality = quality
        .and_then(|q| TranscodeQuality::from_str(&q))
        .unwrap_or_default();

    if let Ok(app_data) = app.path().app_local_data_dir() {
        let cache = TranscodeCache::new(&app_data);
        cache.exists(file_path, quality)
    } else {
        false
    }
}

/// Get cache statistics
#[tauri::command]
pub fn get_cache_stats(app: AppHandle) -> AppResult<CacheStats> {
    let app_data = app.path().app_local_data_dir()?;
    let cache = TranscodeCache::new(&app_data);

    Ok(CacheStats {
        directory: cache.dir().to_string_lossy().to_string(),
        size_bytes: cache.get_cache_size(),
        file_count: cache.get_file_count(),
    })
}

/// Clean up old cache entries
#[tauri::command]
pub fn cleanup_cache(app: AppHandle, max_age_days: Option<u64>) -> AppResult<usize> {
    let app_data = app.path().app_local_data_dir()?;
    let cache = TranscodeCache::new(&app_data);

    let days = max_age_days.unwrap_or(30);
    Ok(cache.cleanup(days))
}

/// Clear all cache entries
#[tauri::command]
pub fn clear_cache(app: AppHandle) -> AppResult<usize> {
    let app_data = app.path().app_local_data_dir()?;
    let cache = TranscodeCache::new(&app_data);
    Ok(cache.clear_all())
}

/// Check if FFmpeg is available
#[tauri::command]
pub fn ffmpeg_available(app: AppHandle) -> bool {
    if let Ok(app_data) = app.path().app_local_data_dir() {
        let cache = TranscodeCache::new(&app_data);
        let transcoder = FfmpegTranscoder::new_with_app(cache, &app);
        transcoder.is_available()
    } else {
        false
    }
}

// --- Response Types ---

#[derive(serde::Serialize)]
pub struct QualityOption {
    id: String,
    label: String,
    video_bitrate: u32,
    audio_bitrate: u32,
}

#[derive(serde::Serialize)]
pub struct CacheStats {
    directory: String,
    size_bytes: u64,
    file_count: usize,
}
