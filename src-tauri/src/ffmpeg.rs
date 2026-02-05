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
    // -i: input file
    // -vf scale: resize maintaining aspect ratio (using -1 for auto)
    // -vframes 1: only output one frame (for video/animated files)
    // -q:v 80: quality for WebP output
    // -y: overwrite output without asking
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel", "error",
            "-i", &input_str,
            "-vf", &format!("scale={}:-1:flags=lanczos", size_px),
            "-vframes", "1",
            "-q:v", "80",
            "-y",
            &output_str,
        ])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg failed: {}", stderr).into());
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
