//! Segment Transcoder
//!
//! Transcodes video segments on-demand using FFmpeg.
//! Segments are cached to disk for subsequent requests.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::ffmpeg::get_ffmpeg_path;
use crate::transcoding::cache::TranscodeCache;
use super::process_manager::ProcessManager;

/// Get or generate a video segment
///
/// Returns cached segment if available, otherwise transcodes on-demand.
pub async fn get_segment(
    app_handle: &tauri::AppHandle,
    cache: &Arc<TranscodeCache>,
    process_manager: &Arc<RwLock<ProcessManager>>,
    file_path: &Path,
    segment_index: u32,
    segment_duration: f64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Check if segment is already cached
    let cache_path = get_segment_cache_path(cache, file_path, segment_index);

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
    let data = transcode_segment(app_handle, file_path, segment_index, segment_duration).await?;

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
    file_path: &Path,
    segment_index: u32,
    segment_duration: f64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let ffmpeg_path = get_ffmpeg_path(Some(app_handle))
        .ok_or("FFmpeg not found")?;

    let start_time = segment_index as f64 * segment_duration;

    // FFmpeg command for HLS segment
    // Using -ss before -i for fast seeking
    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args([
        "-hide_banner",
        "-loglevel", "warning",
        // Fast seek (before input)
        "-ss", &format!("{:.3}", start_time),
        // Input file
        "-i", &file_path.to_string_lossy(),
        // Duration
        "-t", &format!("{:.3}", segment_duration),
        // Stream mapping (first video, first audio if exists)
        "-map", "0:v:0",
        "-map", "0:a:0?",
        // Video encoding - ultrafast for real-time
        "-c:v", "libx264",
        "-preset", "ultrafast",
        "-crf", "23",
        "-profile:v", "high",
        "-level", "4.1",
        // Force even dimensions
        "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
        "-pix_fmt", "yuv420p",
        // Audio encoding
        "-c:a", "aac",
        "-b:a", "128k",
        "-ar", "48000",
        // Output format
        "-f", "mpegts",
        // Output to stdout
        "-",
    ]);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let mut stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Read output
    let mut output_data = Vec::new();
    stdout.read_to_end(&mut output_data).await?;

    // Wait for process
    let status = child.wait().await?;

    if !status.success() {
        let mut err_output = String::new();
        stderr.read_to_string(&mut err_output).await.ok();
        return Err(format!("FFmpeg failed (segment {}): {}", segment_index, err_output).into());
    }

    if output_data.is_empty() {
        return Err(format!("FFmpeg produced empty output for segment {}", segment_index).into());
    }

    Ok(output_data)
}

/// Get the cache path for a segment
fn get_segment_cache_path(cache: &TranscodeCache, file_path: &Path, segment_index: u32) -> PathBuf {
    // Use the cache directory from TranscodeCache
    // Create a subdirectory for HLS segments
    let cache_dir = cache.dir().join("hls_segments");

    // Create a hash-based filename for the source file
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    file_path.to_string_lossy().hash(&mut hasher);

    // Include file modification time in hash for cache invalidation
    if let Ok(metadata) = std::fs::metadata(file_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
    }

    let file_hash = format!("{:016x}", hasher.finish());

    cache_dir.join(format!("{}-seg{:05}.ts", file_hash, segment_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_cache_path() {
        // Just verify the function doesn't panic
        let temp_dir = std::env::temp_dir().join("test_cache");
        let cache = TranscodeCache::new(&temp_dir);
        let path = get_segment_cache_path(&cache, Path::new("/test/video.mkv"), 42);

        assert!(path.to_string_lossy().contains("seg00042.ts"));
        assert!(path.to_string_lossy().contains("hls_segments"));
    }
}
