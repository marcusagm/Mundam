//! Video Probe Module
//!
//! Uses ffprobe to extract video metadata and determine if format is native.

use serde::Serialize;
use std::path::Path;
use std::process::Command;
use tracing::error;

use crate::feature::transcoding::detector;
use crate::processing::transcoding::resolve_transcoding_tools;

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
    registry: &crate::core::formats::registry::FormatRegistry,
    path: &Path,
) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
    // Check if file exists
    if !path.exists() {
        return Err(format!("File not found: {:?}", path).into());
    }

    // Get transcoding tools (ffprobe)
    let tools = resolve_transcoding_tools(Some(app_handle))?;
    let ffprobe_path = tools.ffprobe;

    // Fast path for MIDI: bypass ffprobe since it fails on sequence files
    if let Some(ext) = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        if ext == "mid" || ext == "midi" {
            let duration_secs =
                crate::processing::media::extractors::midi_renderer::get_midi_length(path)
                    .unwrap_or(0.0);
            return Ok(VideoInfo {
                duration_secs,
                is_native: false, // requires transcoding
                video_codec: None,
                audio_codec: Some("midi".to_string()),
                container: Some("midi".to_string()),
                width: None,
                height: None,
            });
        }
    }

    // Run ffprobe with JSON output
    let mut cmd = Command::new(&ffprobe_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &path.to_string_lossy(),
        ])
        .stdin(std::process::Stdio::null())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("FFprobe failed for {:?}: {}", path, stderr);
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

    // Determine if native using existing detector
    let is_native =
        detector::is_native_format(registry, path) && is_codec_native(&video_codec, &audio_codec);

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
            "vp8" // VP8
        ),
        None => true, // No video stream is OK
    };

    // Native audio codecs
    let native_audio = match audio_codec {
        Some(codec) => matches!(
            codec.as_str(),
            "aac" | "mp3" | "mp2" |    // Common web codecs
            "flac" |                    // FLAC
            "pcm_s16le" | "pcm_s24le" | "pcm_f32le" | // PCM variants (WAV)
            "opus" | "vorbis" // Modern open codecs
        ),
        None => true, // No audio stream is OK
    };

    native_video && native_audio
}
