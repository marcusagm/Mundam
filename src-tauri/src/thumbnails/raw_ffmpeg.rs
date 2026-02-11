use std::path::Path;
use std::process::Command;
use crate::media::ffmpeg::get_ffmpeg_path;

/// Generates a thumbnail for a RAW image using FFmpeg specialized for RAW formats.
///
/// This implementation follows the "Scenario 2" in docs/idea/ffmpeg-raw-images.md,
/// explicitly mapping the embedded preview stream (0:v:0) which is much faster
/// and more reliable than trying to decode the sensor data.
pub fn generate_raw_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("THUMB: Processing RAW via Specialized FFmpeg: {:?}", input_path);

    let ffmpeg_path = get_ffmpeg_path(None)
        .ok_or("FFmpeg binary not found")?;

    let input_str = input_path.to_string_lossy();
    let output_str = output_path.to_string_lossy();

    // Specialized args for RAW:
    // -map 0:v:0 -> Select the embedded JPEG preview stream (if exists)
    // -vf scale=... -> High quality lanczos scaling
    // -vframes 1 -> Just one frame
    // -c:v libwebp -> Use WebP for Mundam internal format
    let args = [
        "-hide_banner",
        "-loglevel", "error",
        "-i", &input_str,
        "-map", "0:v:0",
        "-vf", &format!("scale={}:-1:flags=lanczos", size_px),
        "-vframes", "1",
        "-c:v", "libwebp",
        "-q:v", "80", // Good quality/size balance
        "-y",
        &output_str,
    ];

    let output = Command::new(ffmpeg_path)
        .args(&args)
        .output()?;

    if !output.status.success() {
        let _stderr = String::from_utf8_lossy(&output.stderr);
        // Fallback: If -map 0:v:0 fails, try without it (for RAWs that FFmpeg decodes directly like some DNGs)
        println!("THUMB: FFmpeg -map 0:v:0 failed, trying simple conversion for {:?}", input_path);

        let simple_args = [
            "-hide_banner",
            "-loglevel", "error",
            "-i", &input_str,
            "-vf", &format!("scale={}:-1:flags=lanczos", size_px),
            "-vframes", "1",
            "-c:v", "libwebp",
            "-q:v", "80",
            "-y",
            &output_str,
        ];

        let retry_output = Command::new(get_ffmpeg_path(None).unwrap())
            .args(&simple_args)
            .output()?;

        if !retry_output.status.success() {
            let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
            return Err(format!("FFmpeg failed even on retry: {}", retry_stderr).into());
        }
    }

    if !output_path.exists() {
        return Err("FFmpeg claim success but output file missing".into());
    }

    Ok(())
}
