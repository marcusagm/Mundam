//! FFmpeg subprocess wrapper for thumbnail generation
//!
//! Handles complex formats that Rust crates can't decode:
//! - RAW: cr2, cr3, arw, nef, dng, raf, orf, etc.
//! - Modern: heic, heif, avif, jxl
//! - Design: psd, psb, ai, eps, svg, tiff

use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;


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
) -> Result<(), Box<dyn std::error::Error>> {
    let input_str = input_path.to_string_lossy();
    let output_str = output_path.to_string_lossy();

    // FFmpeg command for thumbnail generation:
    // Try seeking 1 second first (better for movies)
    // If that fails, try 0 seconds (better for short clips/SWF)
    let run_ffmpeg = |time: &str| -> Result<(), String> {
        let output = Command::new(ffmpeg_path)
            .args([
                "-hide_banner",
                "-loglevel", "error",
                "-ss", time,
                "-i", &input_str,
                "-vf", &format!("scale={}:-1:flags=lanczos", size_px),
                "-vframes", "1",
                "-c:v", "libwebp",
                "-strict", "unofficial",
                "-q:v", "80",
                "-y",
                &output_str,
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(stderr.to_string());
        }
        Ok(())
    };

    // First attempt at 1s
    if let Err(e1) = run_ffmpeg("00:00:01") {
        // Second attempt at 0s
        if let Err(e2) = run_ffmpeg("00:00:00") {
            // Third attempt: No seek (let ffmpeg find first frame automatically)
            // This often helps with corrupted headers where seeking logic fails
            // but reading sequentially works.
            let output_no_seek = Command::new(ffmpeg_path)
                .args([
                    "-hide_banner",
                    "-loglevel", "error",
                    "-i", &input_str,
                    "-vf", &format!("scale={}:-1:flags=lanczos", size_px),
                    "-vframes", "1",
                    "-c:v", "libwebp",
                    "-strict", "unofficial",
                    "-q:v", "80",
                    "-y",
                    &output_str,
                ])
                .output()
                .map_err(|e| e.to_string())?;

            if !output_no_seek.status.success() {
                 let e3 = String::from_utf8_lossy(&output_no_seek.stderr);
                 eprintln!("Thumbnail ffmpeg failed for {}: 1s err: {}, 0s err: {}, no-seek err: {}", input_str, e1, e2, e3);
                 return Err(format!("FFmpeg failed: {}", e3).into());
            }
        }
    }

    // Verify output was created
    if !output_path.exists() {
        return Err("FFmpeg did not create output file".into());
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
) -> Result<(), Box<dyn std::error::Error>> {
    let ffmpeg_path = get_ffmpeg_path(app_handle)
        .ok_or("FFmpeg not found (neither bundled nor in system PATH)")?;

    generate_with_ffmpeg(&ffmpeg_path, input_path, output_path, size_px)
}

/// Extract normalized audio waveform data using FFmpeg
pub fn get_audio_waveform(
    app_handle: &tauri::AppHandle,
    input_path: &Path,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let ffmpeg_path = get_ffmpeg_path(Some(app_handle))
        .ok_or("FFmpeg not found")?;

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
        return Err(format!("FFmpeg waveform extraction failed: {}", stderr).into());
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
