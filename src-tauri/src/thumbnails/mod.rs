use std::path::Path;
use crate::formats::{FileFormat, ThumbnailStrategy};

pub mod native;
pub mod archive;
pub mod icon;

/// Determines the best strategy for generating a thumbnail based on file detection.
///
/// This uses the `FileFormat::detect` system to identify the file type (magic bytes or extensions)
/// and returns the associated `ThumbnailStrategy`.
///
/// # Arguments
///
/// * `path` - A reference to the file path to inspect.
///
/// # Returns
///
/// * `ThumbnailStrategy` - The recommended strategy (e.g., Ffmpeg, NativeImage, Icon).
///   Defaults to `ThumbnailStrategy::Icon` if detection fails.
pub fn get_thumbnail_strategy(path: &Path) -> ThumbnailStrategy {
    match FileFormat::detect(path) {
        Some(format) => format.strategy.clone(),
        None => ThumbnailStrategy::Icon, 
    }
}

/// Main entry point for thumbnail generation.
///
/// Routes the request to the appropriate sub-module (native, ffmpeg, archive, icon) based on the detected strategy.
///
/// # Priority Logic
/// 1. **FFmpeg Priority:** If FFmpeg is available, it is prioritized for `Image` and `Video` strategies.
///    This offloads processing to an external process, improving stability and performance for large files.
/// 2. **Original Strategy:** If FFmpeg fails or is unavailable, it proceeds with the strategy defined in `formats.rs`.
/// 3. **Fallback:** If the primary strategy fails, it attempts to generate a generic file icon.
///
/// # Arguments
///
/// * `input_path` - Path to the source file.
/// * `output_path` - Path where the resulting WebP thumbnail will be saved.
/// * `size_px` - The target maximum dimension (width or height) in pixels.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if successful, Err otherwise.
pub fn generate_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let strategy = get_thumbnail_strategy(input_path);
    let start = std::time::Instant::now();
    
    // Debug log
    // println!("THUMB: {:?} | {:?} | {:?}", strategy, size_px, input_path.file_name().unwrap_or_default());
    
    // OPTIMIZATION: Try external FFmpeg FIRST if available for Image/Video
    // This offloads the decoding to an external process, which is often faster and safer for large files
    let ffmpeg_available = crate::ffmpeg::is_ffmpeg_available();
    
    // Only try FFmpeg first if the assigned strategy is NOT Icon/Zip (we know ffmpeg can't do those)
    // AND it's a media type (Image/Video/Project which might be PS)
    if ffmpeg_available && matches!(strategy, ThumbnailStrategy::Ffmpeg | ThumbnailStrategy::NativeImage) {
         if let Ok(_) = crate::ffmpeg::generate_thumbnail_ffmpeg_full(None, input_path, output_path, size_px) {
             let elapsed = start.elapsed();
             println!("THUMB (FFmpeg Priority): SUCCESS | {:?} | {:?}", elapsed, input_path.file_name().unwrap_or_default());
             return Ok(());
         }
         println!("THUMB (FFmpeg Priority): FAILED - Falling back to Native");
    }

    let result = match strategy {
        ThumbnailStrategy::Ffmpeg => {
            // We already tried and failed above, or logic missed it.
            // If we are here, it means the PRIMARY strategy is forced to Ffmpeg but it failed/not available?
            Err("FFmpeg strategy failed or unavailable".into())
        },
        ThumbnailStrategy::NativeImage => native::generate_thumbnail_fast(input_path, output_path, size_px),
        ThumbnailStrategy::ZipPreview => archive::generate_thumbnail_zip_preview(input_path, output_path, size_px),
        ThumbnailStrategy::Webview => icon::generate_thumbnail_icon(input_path, output_path, size_px), 
        ThumbnailStrategy::Icon | ThumbnailStrategy::None => icon::generate_thumbnail_icon(input_path, output_path, size_px),
    };
    
    let final_result = match result {
        Ok(_) => Ok(()),
        Err(e) => {
             // eprintln!("Primary strategy {:?} failed: {}", strategy, e);
             // Fallback attempt: If not already Icon, try Icon
             if !matches!(strategy, ThumbnailStrategy::Icon) {
                  icon::generate_thumbnail_icon(input_path, output_path, size_px)
             } else {
                  Err(e)
             }
        }
    };
    
    let elapsed = start.elapsed();
    // Only log if it took significantly long (e100ms)
    if elapsed.as_millis() > 100 {
        println!("THUMB: {:?} | {:?} | {:?}", strategy, elapsed, input_path.file_name().unwrap_or_default());
    }
    
    final_result
}

/// Generate a unique thumbnail filename from image path
pub fn get_thumbnail_filename(image_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    image_path.hash(&mut hasher);
    format!("{:x}.webp", hasher.finish())
}
