//! Transcoding Profiles
//!
//! Provides optimized FFmpeg parameters for on-the-fly HLS fragmentation.
//! The goal is to prioritize latency over master quality to allow instant playback
//! of heavy MKV and unsupported video files in the Solid.js UI.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Available transcoding quality presets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TranscodeQuality {
    Low,      // 480p, low bitrate
    Medium,   // 720p, medium bitrate
    High,     // 1080p, high bitrate
    Original, // Max resolution, high bitrate
}

/// Defines the FFmpeg arguments to execute a fragmented HLS conversion.
#[derive(Debug, Clone)]
pub struct TranscodingProfile {
    /// Arguments injected before the input file (e.g., hardware acceleration flags)
    pub input_args: Vec<String>,
    /// Arguments injected after the input file (e.g., video codecs, scaling)
    pub output_args: Vec<String>,
}

impl TranscodingProfile {
    /// Resolves the optimal FFmpeg arguments to stream a given file via HLS.
    ///
    /// Depending on the file's nature (audio vs video), it builds a stream copy
    /// or an ultrafast transcode targeting 1080p maximum height.
    ///
    /// # Arguments
    /// * `original_path` - The physical path to the source media.
    /// * `mime_type` - An optional MIME type to hint at audio-only sources.
    ///
    /// # Returns
    /// An instantiated `TranscodingProfile` ready to be fed into a `tokio::process::Command`.
    pub fn resolve_for_hls(
        original_path: &Path,
        mime_type: Option<&str>,
        video_encoder: &str,
    ) -> Self {
        let ext = original_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut output_args = Vec::new();

        // Universal HLS fragmentation parameters for low latency VOD
        // We output fragmented mp4 (fmp4) or mpegts. MPEG-TS is universally supported.
        output_args.extend(vec![
            "-hls_time".to_string(),
            "4".to_string(), // 4 seconds fragments for better latency/buffering balance
            "-hls_playlist_type".to_string(),
            "event".to_string(), // Event signifies an appending playlist, good for on-the-fly
            "-f".to_string(),
            "hls".to_string(),
            "-hls_segment_type".to_string(),
            "mpegts".to_string(),
            "-hls_flags".to_string(),
            "independent_segments+append_list".to_string(), // Allows UI to start playing immediately
        ]);

        let is_audio_only = mime_type.map_or(
            matches!(
                ext.as_str(),
                "flac" | "wav" | "mp3" | "m4a" | "ogg" | "aac" | "opus"
            ),
            |m| m.starts_with("audio/"),
        );

        if is_audio_only {
            // Audio-only pipeline (Pseudo-HLS)
            output_args.extend(vec![
                "-vn".to_string(), // Strip video stream completely
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                "256k".to_string(),
            ]);

            TranscodingProfile {
                input_args: vec![],
                output_args,
            }
        } else {
            // Video pipeline
            // Transcodes everything to H.264 ultrafast.
            // In a more sophisticated setup, we would read the stream codecs and use `-c:v copy` if already H.264.
            output_args.extend(["-c:v", video_encoder, "-b:v", "2500k"].map(str::to_string));
            if video_encoder == "libx264" {
                output_args.extend(["-preset", "ultrafast", "-crf", "23"].map(str::to_string));
            }
            output_args.extend(
                [
                    "-vf",
                    "scale=-2:'min(1080,ih)'",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    "-maxrate",
                    "5M",
                    "-bufsize",
                    "10M",
                ]
                .map(str::to_string),
            );

            TranscodingProfile {
                input_args: vec![
                    // Mild hardware acceleration attempts natively
                    "-hwaccel".to_string(),
                    "auto".to_string(),
                ],
                output_args,
            }
        }
    }
}
