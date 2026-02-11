//! FFmpeg subprocess wrapper for thumbnail generation
//!
//! Handles complex formats that Rust crates can't decode:
//! - RAW: cr2, cr3, arw, nef, dng, raf, orf, etc.
//! - Modern: heic, heif, avif, jxl
//! - Design: psd, psb, ai, eps, svg, tiff

use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use crate::error::{AppError, AppResult};


/// Get the path to the FFmpeg binary
///
/// Searches in order:
/// 1. Bundled in resources (production)
/// 2. Bundled in src-tauri/ffmpeg (development)
/// 3. System PATH
pub fn get_ffmpeg_path(app_handle: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    // Try bundled FFmpeg in resources (production)
    if let Some(handle) = app_handle {
        if let Ok(resource_dir) = handle.path().resource_dir() {
            let bundled_path = if cfg!(target_os = "windows") {
                resource_dir.join("ffmpeg").join("ffmpeg.exe")
            } else {
                resource_dir.join("ffmpeg").join("ffmpeg")
            };

            if bundled_path.exists() {
                return Some(bundled_path);
            }
        }
    }

    // Try bundled FFmpeg in src-tauri/ffmpeg (development)
    // The binary is at: src-tauri/ffmpeg/ffmpeg
    // Current exe is at: src-tauri/target/debug/mundam
    if let Ok(exe_path) = std::env::current_exe() {
        // Go up from target/debug to src-tauri
        if let Some(target_dir) = exe_path.parent() {
            if let Some(debug_dir) = target_dir.parent() {
                if let Some(src_tauri) = debug_dir.parent() {
                    let bundled_path = src_tauri.join("ffmpeg").join("ffmpeg");
                    if bundled_path.exists() {
                        return Some(bundled_path);
                    }
                }
            }
        }
    }

    // Fallback to system FFmpeg
    if Command::new("ffmpeg").arg("-version").output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}


/// Check if FFmpeg is available (system or bundled)
pub fn is_ffmpeg_available() -> bool {
    get_ffmpeg_path(None).is_some()
}


/// Generate a thumbnail using FFmpeg subprocess
///
/// This handles:
/// - RAW camera files (cr2, arw, nef, etc.)
/// - Modern codecs (heic, avif, jxl)
/// - Design files (psd first layer, svg, tiff)
pub fn generate_with_ffmpeg(
    ffmpeg_path: &Path,
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
    is_video: bool,
) -> AppResult<()> {
    let input_str = input_path.to_string_lossy();
    let output_str = output_path.to_string_lossy();

    // FFmpeg command for thumbnail generation:
    // Try seeking 1 second first (better for movies)
    // If that fails, try 0 seconds (better for short clips/SWF)
    let run_ffmpeg = |time: Option<&str>| -> AppResult<()> {
        let mut args = vec![
            "-hide_banner".to_string(),
            "-loglevel".to_string(), "error".to_string(),
        ];

        if let Some(t) = time {
            args.push("-ss".to_string());
            args.push(t.to_string());
        }

        args.extend_from_slice(&[
            "-i".to_string(), input_str.to_string(),
            "-vf".to_string(), format!("scale={}:-1:flags=lanczos", size_px),
            "-vframes".to_string(), "1".to_string(),
            "-c:v".to_string(), "libwebp".to_string(),
            "-strict".to_string(), "unofficial".to_string(),
            "-q:v".to_string(), "80".to_string(),
            "-y".to_string(),
            output_str.to_string(),
        ]);

        let output = Command::new(ffmpeg_path)
            .args(&args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Transcoding(stderr.to_string()));
        }
        Ok(())
    };

    // If it's an image, skip seeking logic entirely and just convert
    if !is_video {
        if let Err(e) = run_ffmpeg(None) {
             eprintln!("FFmpeg image conversion failed for {}: {}", input_str, e);
             return Err(AppError::Transcoding(format!("FFmpeg failed: {}", e)));
        }
        // Verify output was created
        if !output_path.exists() {
            return Err(AppError::Transcoding("FFmpeg did not create output file".to_string()));
        }
        return Ok(());
    }

    // First attempt at 1s
    if let Err(e1) = run_ffmpeg(Some("00:00:01")) {
        // Second attempt at 0s
        if let Err(e2) = run_ffmpeg(Some("00:00:00")) {
            // Third attempt: No seek (let ffmpeg find first frame automatically)
            if let Err(e3) = run_ffmpeg(None) {
                 eprintln!("Thumbnail ffmpeg failed for {}: 1s err: {}, 0s err: {}, no-seek err: {}", input_str, e1, e2, e3);
                 return Err(AppError::Transcoding(format!("FFmpeg failed: {}", e3)));
            }
        }
    }

    // Verify output was created
    if !output_path.exists() {
        return Err(AppError::Transcoding("FFmpeg did not create output file".to_string()));
    }

    Ok(())
}

/// Generate thumbnail with fallback strategies
///
/// 1. Try FFmpeg from bundled resources
/// 2. Try system FFmpeg
/// 3. Return error if neither works
pub fn generate_thumbnail_ffmpeg_full(
    app_handle: Option<&tauri::AppHandle>,
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
    is_video: bool,
) -> AppResult<()> {
    let ffmpeg_path = get_ffmpeg_path(app_handle)
        .ok_or_else(|| AppError::Transcoding("FFmpeg not found (neither bundled nor in system PATH)".to_string()))?;

    generate_with_ffmpeg(&ffmpeg_path, input_path, output_path, size_px, is_video)
        .map_err(|e| AppError::Transcoding(e.to_string()))
}

/// Extract normalized audio waveform data using FFmpeg
pub fn get_audio_waveform(
    app_handle: &tauri::AppHandle,
    input_path: &Path,
) -> AppResult<Vec<f32>> {
    let ffmpeg_path = get_ffmpeg_path(Some(app_handle))
        .ok_or_else(|| AppError::Transcoding("FFmpeg not found".to_string()))?;

    // Use a low sample rate (100Hz) to extract peaks quickly
    // -ar 100: Resample to 100Hz
    // -ac 1: Downmix to mono
    // -f f32le: Output 32-bit floats
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel", "error",
            "-i", &input_path.to_string_lossy(),
            "-ar", "100",
            "-ac", "1",
            "-f", "f32le",
            "-",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Transcoding(format!("FFmpeg waveform extraction failed: {}", stderr)));
    }

    let raw_data = output.stdout;
    let floats: Vec<f32> = raw_data
        .chunks_exact(4)
        .map(|chunk| {
            let b = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(b).abs()
        })
        .collect();

    if floats.is_empty() {
        return Ok(vec![]);
    }

    // Downsample to exactly 500 points for a consistent UI waveform
    let target_points = 500;
    let result = if floats.len() <= target_points {
        floats
    } else {
        let chunk_size = floats.len() / target_points;
        floats.chunks(chunk_size)
            .map(|chunk| chunk.iter().fold(0.0f32, |max, &val| max.max(val)))
            .take(target_points)
            .collect()
    };

    // Global normalization
    let max = result.iter().fold(0.0f32, |max, &val| max.max(val));
    if max > 0.0 {
        Ok(result.iter().map(|&v| v / max).collect())
    } else {
        Ok(result)
    }
}

/// Extract a single frame (usually the first) to memory as JPEG data.
/// Handles tone mapping for HDR files automatically.
pub fn extract_frame_to_memory(input_path: &Path) -> AppResult<Vec<u8>> {
    let ffmpeg_path = get_ffmpeg_path(None)
        .ok_or_else(|| AppError::Transcoding("FFmpeg not found".to_string()))?;

    let input_str = input_path.to_string_lossy();
    let ext = input_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(), "error".to_string(),
    ];

    // Seek logic for videos (same as generate_with_ffmpeg)
    let is_video = matches!(ext.as_str(),
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "wmv" | "flv" | "m4v" | "mxf" |
        "asf" | "ts" | "mts" | "m2ts" | "vob" | "3gp" | "rm" | "ogv" | "swf" | "mpg" | "mpeg" | "m2v"
    );
    if is_video {
        args.push("-ss".to_string());
        args.push("00:00:01".to_string());
    }

    args.push("-i".to_string());
    args.push(input_str.to_string());

    // Tone mapping for HDR (Radiance HDR, OpenEXR, or HDR video)
    if ext == "hdr" || ext == "exr" {
        // Simple but robust tonemapping
        args.push("-vf".to_string());
        args.push("zscale=t=linear:npl=100,tonemap=tonemap=hable,zscale=p=709:t=709".to_string());
    }

    args.extend_from_slice(&[
        "-vframes".to_string(), "1".to_string(),
        "-f".to_string(), "image2".to_string(),
        "-c:v".to_string(), "mjpeg".to_string(),
        "-".to_string(),
    ]);

    let output = Command::new(ffmpeg_path)
        .args(&args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If it failed and we sought to 1s, try again without seek
        if is_video {
             let retry_args = vec![
                "-hide_banner".to_string(),
                "-loglevel".to_string(), "error".to_string(),
                "-i".to_string(), input_str.to_string(),
                "-vframes".to_string(), "1".to_string(),
                "-f".to_string(), "image2".to_string(),
                "-c:v".to_string(), "mjpeg".to_string(),
                "-".to_string(),
            ];
            let retry_output = Command::new(get_ffmpeg_path(None).unwrap())
                .args(&retry_args)
                .output()?;

            if retry_output.status.success() {
                return Ok(retry_output.stdout);
            }
        }
        return Err(AppError::Transcoding(format!("FFmpeg frame extraction failed: {}", stderr)));
    }

    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_available() {
        // This test will pass if FFmpeg is installed on the system
        let available = is_ffmpeg_available();
        // println!("FFmpeg available: {}", available);
    }
}
