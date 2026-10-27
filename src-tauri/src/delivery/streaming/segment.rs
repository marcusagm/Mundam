//! Segment Transcoder
//!
//! Transcodes video segments on-demand using FFmpeg.
//! Segments are cached to disk for subsequent requests.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::info;

use super::process_manager::ProcessManager;
use crate::feature::transcoding::cache::TranscodeCache;
use crate::feature::transcoding::detector;
use crate::processing::transcoding::resolve_transcoding_tools;

/// Get or generate a video segment
pub async fn get_segment(
    app_handle: &tauri::AppHandle,
    registry: &Arc<crate::core::formats::registry::FormatRegistry>,
    cache: &Arc<TranscodeCache>,
    process_manager: &Arc<RwLock<ProcessManager>>,
    file_path: &Path,
    segment_index: u32,
    segment_duration: f64,
    quality: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Check if segment is already cached
    let cache_path = get_segment_cache_path(cache, file_path, segment_index, quality);

    if cache_path.exists() {
        // Serve from cache
        let data = tokio::fs::read(&cache_path).await?;
        return Ok(data);
    }

    // Generate segment key for process management
    let segment_key = format!("{}:{}", file_path.display(), segment_index);

    // Cancel any previous transcoding for this segment (in case of rapid seeking)
    {
        let mut pm = process_manager.write().await;
        pm.cancel(&segment_key);
    }

    // Transcode the segment
    let data = transcode_segment(
        app_handle,
        registry,
        process_manager,
        &segment_key,
        file_path,
        segment_index,
        segment_duration,
        quality,
    )
    .await?;

    // Cache the segment to disk
    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&cache_path, &data).await.ok();

    Ok(data)
}

/// Transcode a single segment using FFmpeg
async fn transcode_segment(
    app_handle: &tauri::AppHandle,
    registry: &Arc<crate::core::formats::registry::FormatRegistry>,
    process_manager: &Arc<RwLock<ProcessManager>>,
    segment_key: &str,
    file_path: &Path,
    segment_index: u32,
    segment_duration: f64,
    quality: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let tools = resolve_transcoding_tools(Some(app_handle))?;
    let ffmpeg_path = tools.ffmpeg;

    let start_time = segment_index as f64 * segment_duration;

    // Detect media kind to adjust FFmpeg flags
    let media_kind = detector::get_media_kind(registry, file_path);
    let is_audio = media_kind == detector::MediaKind::Audio;

    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "warning",
        "-analyzeduration",
        "100M",
        "-probesize",
        "50M",
        "-ignore_unknown",
        "-fflags",
        "+genpts",
        "-ss",
        &format!("{:.3}", start_time),
        "-i",
        &file_path.to_string_lossy(),
        "-t",
        &format!("{:.3}", segment_duration),
    ]);

    if is_audio {
        // Audio-only configuration
        cmd.args([
            "-map", "0:a:0?", "-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-ac", "2",
        ]);
    } else {
        // Video configuration
        cmd.args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-sn",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
        ]);

        match quality {
            "preview" => {
                cmd.args(["-crf", "30", "-vf", "scale=-2:480"]);
            }
            "high" => {
                cmd.args(["-crf", "18", "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2"]);
            }
            _ => {
                // standard
                cmd.args(["-crf", "23", "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2"]);
            }
        }

        cmd.args([
            "-profile:v",
            "high",
            "-level",
            "4.1",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-ar",
            "48000",
        ]);
    }

    cmd.args(["-f", "mpegts", "-"]);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(0x08000000);

    let child = cmd.spawn()?;

    if let Some(id) = child.id() {
        let mut pm = process_manager.write().await;
        pm.register(segment_key.to_string(), id);
    }

    let output = child.wait_with_output().await?;
    
    {
        let mut pm = process_manager.write().await;
        pm.remove(segment_key);
    }
    
    let status = output.status;
    let output_data = output.stdout;

    if !status.success() {
        let err_output = String::from_utf8_lossy(&output.stderr);
        info!("FFmpeg failed (segment {}): {}", segment_index, err_output);
        return Err(format!("FFmpeg failed (segment {}): {}", segment_index, err_output).into());
    }

    if output_data.is_empty() {
        return Err(format!("FFmpeg produced empty output for segment {}", segment_index).into());
    }

    Ok(output_data)
}

/// Get the cache path for a segment
fn get_segment_cache_path(
    cache: &TranscodeCache,
    file_path: &Path,
    segment_index: u32,
    quality: &str,
) -> PathBuf {
    let cache_dir = cache.dir().join("hls_segments");

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    file_path.to_string_lossy().hash(&mut hasher);

    if let Ok(metadata) = std::fs::metadata(file_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
    }

    let file_hash = format!("{:016x}", hasher.finish());

    cache_dir.join(format!(
        "{}-{}-seg{:05}.ts",
        file_hash, quality, segment_index
    ))
}
