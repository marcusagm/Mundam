//! FFmpeg subprocess wrapper for thumbnail generation
//!
//! Handles complex formats that Rust crates can't decode:
//! - RAW: cr2, cr3, arw, nef, dng, raf, orf, etc.
//! - Modern: heic, heif, avif, jxl
//! - Design: psd, psb, ai, eps, svg, tiff

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Read;
use std::time::Duration;
use wait_timeout::ChildExt;
use tauri::Manager;
use crate::error::{AppError, AppResult};

/// Get the path to the FFmpeg binary
pub fn get_ffmpeg_path<R: tauri::Runtime>(app_handle: Option<&tauri::AppHandle<R>>) -> Option<PathBuf> {
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

    if let Ok(exe_path) = std::env::current_exe() {
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

    if Command::new("ffmpeg").arg("-version").output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

pub fn is_ffmpeg_available() -> bool {
    get_ffmpeg_path::<tauri::Wry>(None).is_some()
}

/// Helper to run a command with a timeout to avoid application freezes.
fn run_command_with_timeout(mut cmd: Command, timeout_secs: u64) -> AppResult<std::process::Output> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    match child.wait_timeout(Duration::from_secs(timeout_secs))? {
        Some(status) => {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut s) = child.stdout {
                 s.read_to_end(&mut stdout).ok();
            }
            if let Some(mut s) = child.stderr {
                 s.read_to_end(&mut stderr).ok();
            }
            Ok(std::process::Output { status, stdout, stderr })
        }
        None => {
            child.kill().ok();
            Err(AppError::Transcoding(format!("Command timed out after {}s", timeout_secs)))
        }
    }
}

pub fn generate_with_ffmpeg(
    ffmpeg_path: &Path,
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
    is_video: bool,
) -> AppResult<()> {
    let input_str = input_path.to_string_lossy();
    let output_str = output_path.to_string_lossy();

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

        let mut cmd = Command::new(ffmpeg_path);
        cmd.args(&args);

        let output = run_command_with_timeout(cmd, 15)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Transcoding(stderr.to_string()));
        }
        Ok(())
    };

    if !is_video {
        if let Err(e) = run_ffmpeg(None) {
             eprintln!("FFmpeg image conversion failed for {}: {}", input_str, e);
             return Err(AppError::Transcoding(format!("FFmpeg failed: {}", e)));
        }
        if !output_path.exists() {
            return Err(AppError::Transcoding("FFmpeg did not create output file".to_string()));
        }
        return Ok(());
    }

    if let Err(e1) = run_ffmpeg(Some("00:00:01")) {
        if let Err(e2) = run_ffmpeg(Some("00:00:00")) {
            if let Err(e3) = run_ffmpeg(None) {
                 eprintln!("Thumbnail ffmpeg failed for {}: 1s err: {}, 0s err: {}, no-seek err: {}", input_str, e1, e2, e3);
                 return Err(AppError::Transcoding(format!("FFmpeg failed: {}", e3)));
            }
        }
    }

    if !output_path.exists() {
        return Err(AppError::Transcoding("FFmpeg did not create output file".to_string()));
    }

    Ok(())
}

pub fn generate_thumbnail_ffmpeg_full<R: tauri::Runtime>(
    app_handle: Option<&tauri::AppHandle<R>>,
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

pub fn get_audio_waveform<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    input_path: &Path,
) -> AppResult<Vec<f32>> {
    let ffmpeg_path = get_ffmpeg_path(Some(app_handle))
        .ok_or_else(|| AppError::Transcoding("FFmpeg not found".to_string()))?;

    let mut cmd = Command::new(ffmpeg_path);
    cmd.args([
        "-hide_banner",
        "-loglevel", "error",
        "-i", &input_path.to_string_lossy(),
        "-ar", "100",
        "-ac", "1",
        "-f", "f32le",
        "-",
    ]);

    let output = run_command_with_timeout(cmd, 30)?;

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

    let max = result.iter().fold(0.0f32, |max, &val| max.max(val));
    if max > 0.0 {
        Ok(result.iter().map(|&v| v / max).collect())
    } else {
        Ok(result)
    }
}

pub fn extract_frame_to_memory<R: tauri::Runtime>(app_handle: Option<&tauri::AppHandle<R>>, input_path: &Path) -> AppResult<Vec<u8>> {
    let ffmpeg_path = get_ffmpeg_path(app_handle)
        .ok_or_else(|| AppError::Transcoding("FFmpeg not found".to_string()))?;

    let input_str = input_path.to_string_lossy();
    let ext = input_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(), "error".to_string(),
    ];

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

    if ext == "hdr" || ext == "exr" {
        args.push("-vf".to_string());
        args.push("zscale=t=linear:npl=100,tonemap=tonemap=hable,zscale=p=709:t=709".to_string());
    }

    args.extend_from_slice(&[
        "-vframes".to_string(), "1".to_string(),
        "-f".to_string(), "image2".to_string(),
        "-c:v".to_string(), "mjpeg".to_string(),
        "-".to_string(),
    ]);

    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args(&args);
    let output = run_command_with_timeout(cmd, 15)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
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
            let mut retry_cmd = Command::new(&ffmpeg_path);
            retry_cmd.args(&retry_args);
            let retry_output = run_command_with_timeout(retry_cmd, 10)?;

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
        let available = is_ffmpeg_available();
    }
}
