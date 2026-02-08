//! Video Probe Module
//!
//! Uses ffprobe to extract video metadata and determine if format is native.

use serde::Serialize;
use std::path::Path;
use std::process::Command;


use crate::ffmpeg::get_ffmpeg_path;
use crate::transcoding::detector;

/// Video information returned by probe
#[derive(Debug, Clone, Serialize)]
pub struct VideoInfo {
    /// Duration in seconds
    pub duration_secs: f64,
    /// Whether the format is natively playable in WebView
    pub is_native: bool,
    /// Video codec (e.g., "h264", "vp9")
    pub video_codec: Option<String>,
    /// Audio codec (e.g., "aac", "opus")
    pub audio_codec: Option<String>,
    /// Container format (e.g., "mp4", "mkv")
    pub container: Option<String>,
    /// Resolution width
    pub width: Option<u32>,
    /// Resolution height
    pub height: Option<u32>,
}

/// Get video information using ffprobe
pub async fn get_video_info(
    app_handle: &tauri::AppHandle,
    path: &Path,
) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
    // Check if file exists
    if !path.exists() {
        return Err(format!("File not found: {:?}", path).into());
    }

    // Get ffprobe path (same location as ffmpeg)
    let ffmpeg_path = get_ffmpeg_path(Some(app_handle))
        .ok_or("FFmpeg/FFprobe not found")?;

    // ffprobe is usually next to ffmpeg
    let ffprobe_path = ffmpeg_path.with_file_name(
        if cfg!(target_os = "windows") { "ffprobe.exe" } else { "ffprobe" }
    );

    // If ffprobe doesn't exist, try system path
    let probe_cmd = if ffprobe_path.exists() {
        ffprobe_path.to_string_lossy().to_string()
    } else {
        "ffprobe".to_string()
    };

    // Run ffprobe with JSON output
    let output = Command::new(&probe_cmd)
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            &path.to_string_lossy(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFprobe failed: {}", stderr).into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    // Extract duration from format
    let duration_secs = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Extract format name
    let container = json["format"]["format_name"]
        .as_str()
        .map(|s| s.split(',').next().unwrap_or(s).to_string());

    // Find video and audio streams
    let streams = json["streams"].as_array();

    let mut video_codec = None;
    let mut audio_codec = None;
    let mut width = None;
    let mut height = None;

    if let Some(streams) = streams {
        for stream in streams {
            let codec_type = stream["codec_type"].as_str().unwrap_or("");
            let codec_name = stream["codec_name"].as_str();

            match codec_type {
                "video" if video_codec.is_none() => {
                    video_codec = codec_name.map(String::from);
                    width = stream["width"].as_u64().map(|v| v as u32);
                    height = stream["height"].as_u64().map(|v| v as u32);
                }
                "audio" if audio_codec.is_none() => {
                    audio_codec = codec_name.map(String::from);
                }
                _ => {}
            }
        }
    }

    // Determine if native using existing detector (extension-based)
    // Plus additional codec-based check for better accuracy
    // Also mark HLS-problematic formats as "native" to use fallback transcoding
    let is_native = (detector::is_native_format(path) && is_codec_native(&video_codec, &audio_codec))
        || is_hls_problematic(path, &container);

    Ok(VideoInfo {
        duration_secs,
        is_native,
        video_codec,
        audio_codec,
        container,
        width,
        height,
    })
}

/// Check if video/audio codecs are natively supported in WebView
fn is_codec_native(video_codec: &Option<String>, audio_codec: &Option<String>) -> bool {
    // Native video codecs
    let native_video = match video_codec {
        Some(codec) => matches!(
            codec.as_str(),
            "h264" | "avc1" | "avc" | // H.264
            "vp8"                      // VP8 (limited support)
        ),
        None => true, // No video stream is OK
    };

    // Native audio codecs
    let native_audio = match audio_codec {
        Some(codec) => matches!(
            codec.as_str(),
            "aac" | "mp3" | "mp2" |    // Common web codecs
            "flac" |                    // FLAC
            "pcm_s16le" | "pcm_s24le" | "pcm_f32le" // PCM variants (WAV)
        ),
        None => true, // No audio stream is OK
    };

    native_video && native_audio
}

/// Check if a format has issues with HLS streaming and should use fallback
/// These formats either don't seek well or have FFmpeg processing issues
fn is_hls_problematic(path: &Path, container: &Option<String>) -> bool {
    // Check by extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if matches!(ext_lower.as_str(),
            // Flash formats - obsolete and problematic
            "swf" |
            // Raw video streams without proper container
            "m2v" | "mpv" |
            // Some MPEG containers have seeking issues
            "mpg" | "mpeg"
        ) {
            return true;
        }
    }

    // Check by container format from ffprobe
    if let Some(fmt) = container {
        if matches!(fmt.as_str(),
            "swf" |
            "mpeg1video" | "mpegvideo" |
            "m2v" | "mpegps"
        ) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_native() {
        assert!(is_codec_native(&Some("h264".to_string()), &Some("aac".to_string())));
        assert!(!is_codec_native(&Some("hevc".to_string()), &Some("aac".to_string())));
        assert!(!is_codec_native(&Some("h264".to_string()), &Some("opus".to_string())));
        assert!(!is_codec_native(&Some("vp9".to_string()), &Some("opus".to_string())));
    }
}
