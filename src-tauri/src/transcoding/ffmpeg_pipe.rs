use std::io::{Read, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, Child};

use super::quality::TranscodeQuality;
use super::detector::{MediaType, get_media_type};
use super::cache::TranscodeCache;

/// Transcoding status for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscodeStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

/// FFmpeg-based transcoder for media files
pub struct FfmpegTranscoder {
    ffmpeg_path: PathBuf,
    cache: TranscodeCache,
}

impl FfmpegTranscoder {
    /// Create a new transcoder with the given cache
    pub fn new(cache: TranscodeCache) -> Self {
        // Use the centralized FFmpeg path detection from crate::ffmpeg
        let ffmpeg_path = crate::ffmpeg::get_ffmpeg_path(None)
            .unwrap_or_else(|| PathBuf::from("ffmpeg"));
        Self { ffmpeg_path, cache }
    }

    /// Create a new transcoder with app handle for bundled FFmpeg detection
    pub fn with_app_handle(cache: TranscodeCache, app_handle: &tauri::AppHandle) -> Self {
        let ffmpeg_path = crate::ffmpeg::get_ffmpeg_path(Some(app_handle))
            .unwrap_or_else(|| PathBuf::from("ffmpeg"));
        Self { ffmpeg_path, cache }
    }

    /// Check if FFmpeg is available
    pub fn is_available(&self) -> bool {
        Command::new(&self.ffmpeg_path)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Transcode a file and return path to transcoded file
    /// This blocks until transcoding is complete
    pub fn transcode_sync(
        &self,
        source: &Path,
        quality: TranscodeQuality,
    ) -> Result<PathBuf, TranscodeError> {
        // Check cache first
        if let Some(cached) = self.cache.get(source, quality) {
            return Ok(cached);
        }

        // Verify source exists
        if !source.exists() {
            return Err(TranscodeError::SourceNotFound(source.to_path_buf()));
        }

        // Get output path
        let output = self.cache.get_cache_path(source, quality);

        // Build FFmpeg command based on media type
        let media_type = get_media_type(source);
        let mut cmd = self.build_ffmpeg_command(source, &output, quality, media_type);

        // Execute and capture output
        let result = cmd.output().map_err(|e| TranscodeError::FfmpegError(e.to_string()))?;

        if result.status.success() && output.exists() {
            Ok(output)
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            eprintln!("FFMPEG_STDERR: {}", stderr);
            Err(TranscodeError::TranscodeFailed(format!(
                "FFmpeg exited with status: {:?}, stderr: {}",
                result.status.code(),
                stderr.chars().take(500).collect::<String>()
            )))
        }
    }

    /// Build FFmpeg command for transcoding
    fn build_ffmpeg_command(
        &self,
        source: &Path,
        output: &Path,
        quality: TranscodeQuality,
        media_type: MediaType,
    ) -> Command {
        let mut cmd = Command::new(&self.ffmpeg_path);

        // Input options - increase analysis for complex/large files
        cmd.arg("-y")                        // Overwrite output
            .arg("-hide_banner")             // Cleaner output
            .arg("-loglevel").arg("warning") // Reduce verbosity
            .arg("-probesize").arg("100M")   // Analyze more data (for large files)
            .arg("-analyzeduration").arg("100M") // Longer analysis time
            .arg("-i")
            .arg(source);

        match media_type {
            MediaType::Audio => {
                // Audio-only transcoding to AAC
                cmd.args([
                    "-vn",                     // No video
                    "-c:a", "aac",             // AAC codec
                    "-b:a", &format!("{}k", quality.audio_bitrate() / 1000),
                    "-ar", "48000",            // Standard sample rate
                    "-f", "mp4",               // MP4 container (m4a is mp4 audio-only)
                ]);
            }
            MediaType::Video | MediaType::Unknown => {
                // Video transcoding to H.264 + AAC using CRF for quality
                // Map video and audio streams explicitly, ignore if missing
                cmd.args([
                    "-map", "0:v:0?",          // First video stream (optional)
                    "-map", "0:a:0?",          // First audio stream (optional)
                    // Video codec settings
                    "-c:v", "libx264",         // H.264 codec
                    "-profile:v", "high",      // H.264 High Profile (best quality)
                    "-level", "4.1",           // Level 4.1 (1080p@30fps compatible)
                    "-preset", quality.ffmpeg_preset(),
                    "-crf", &quality.crf().to_string(), // CRF-based quality
                    // Force even dimensions (required by most codecs)
                    "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                    "-pix_fmt", "yuv420p",     // Compatibility
                    // GOP settings for better seeking
                    "-g", "30",                // Keyframe every 30 frames (1s at 30fps)
                    "-bf", "2",                // 2 B-frames between I and P frames
                    // Audio settings
                    "-c:a", "aac",             // AAC codec
                    "-b:a", &format!("{}k", quality.audio_bitrate() / 1000),
                    "-ar", "48000",            // Standard sample rate
                    // Container settings
                    "-movflags", "+faststart", // Web optimization (moves moov atom to start)
                    "-max_muxing_queue_size", "9999", // Handle large/complex streams
                    "-f", "mp4",
                ]);
            }
        }

        cmd.arg(output);
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        cmd
    }

    /// Start streaming transcoding (returns child process)
    /// The output is written to stdout in fragmented MP4 format
    pub fn start_streaming(
        &self,
        source: &Path,
        quality: TranscodeQuality,
    ) -> Result<TranscodeStream, TranscodeError> {
        if !source.exists() {
            return Err(TranscodeError::SourceNotFound(source.to_path_buf()));
        }

        let media_type = get_media_type(source);
        let mut cmd = Command::new(&self.ffmpeg_path);

        // Input
        cmd.arg("-i").arg(source);

        match media_type {
            MediaType::Audio => {
                cmd.args([
                    "-vn",
                    "-c:a", "aac",
                    "-b:a", &format!("{}k", quality.audio_bitrate() / 1000),
                    "-f", "adts", // ADTS for streaming AAC
                    "pipe:1",
                ]);
            }
            MediaType::Video | MediaType::Unknown => {
                cmd.args([
                    "-c:v", "libx264",
                    "-preset", "ultrafast", // Always ultrafast for streaming
                    "-tune", "zerolatency",
                    "-b:v", &format!("{}k", quality.video_bitrate() / 1000),
                    "-c:a", "aac",
                    "-b:a", &format!("{}k", quality.audio_bitrate() / 1000),
                    "-movflags", "frag_keyframe+empty_moov+faststart",
                    "-pix_fmt", "yuv420p",
                    "-f", "mp4",
                    "pipe:1",
                ]);
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| TranscodeError::FfmpegError(e.to_string()))?;

        Ok(TranscodeStream {
            child,
            media_type,
        })
    }

    /// Get the cache manager
    pub fn cache(&self) -> &TranscodeCache {
        &self.cache
    }
}

/// Streaming transcoder output
pub struct TranscodeStream {
    child: Child,
    media_type: MediaType,
}

impl TranscodeStream {
    /// Read transcoded data into buffer
    /// Returns number of bytes read, 0 if finished
    pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(stdout) = self.child.stdout.as_mut() {
            stdout.read(buf)
        } else {
            Ok(0)
        }
    }

    /// Get buffered reader for the stream
    pub fn reader(&mut self) -> Option<BufReader<&mut std::process::ChildStdout>> {
        self.child.stdout.as_mut().map(BufReader::new)
    }

    /// Get the media type being transcoded
    pub fn media_type(&self) -> MediaType {
        self.media_type
    }

    /// Get MIME type for the output
    pub fn mime_type(&self) -> &'static str {
        match self.media_type {
            MediaType::Audio => "audio/aac",
            MediaType::Video | MediaType::Unknown => "video/mp4",
        }
    }

    /// Wait for transcoding to finish
    pub fn wait(mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait()
    }

    /// Kill the transcoding process
    pub fn kill(mut self) -> std::io::Result<()> {
        self.child.kill()
    }
}

/// Transcoding errors
#[derive(Debug, Clone)]
pub enum TranscodeError {
    SourceNotFound(PathBuf),
    FfmpegNotFound,
    FfmpegError(String),
    TranscodeFailed(String),
    IoError(String),
}

impl std::fmt::Display for TranscodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscodeError::SourceNotFound(p) => write!(f, "Source file not found: {:?}", p),
            TranscodeError::FfmpegNotFound => write!(f, "FFmpeg not found in system"),
            TranscodeError::FfmpegError(e) => write!(f, "FFmpeg error: {}", e),
            TranscodeError::TranscodeFailed(e) => write!(f, "Transcoding failed: {}", e),
            TranscodeError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for TranscodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_detection() {
        // This test checks if we can find FFmpeg via the centralized path detection
        let found = crate::ffmpeg::get_ffmpeg_path(None);
        println!("FFmpeg found at: {:?}", found);
    }
}
