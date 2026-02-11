use std::path::Path;
use crate::formats::{FileFormat, ThumbnailStrategy};

pub mod native;
pub mod archive;
pub mod affinity;
pub mod extractors;

pub mod icon;
pub mod svg;
pub mod font;
pub mod model;
pub mod commands;
pub mod worker;
pub mod priority;
pub mod raw;

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
/// Returns
///
/// * `Result<String, Box<dyn std::error::Error>>` - The filename (relative to thumb root) of the generated/used thumbnail.
pub fn generate_thumbnail(
    input_path: &Path,
    thumbnails_dir: &Path,
    hashed_filename: &str,
    size_px: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let output_path = thumbnails_dir.join(hashed_filename);

    // OPTIMIZATION: Open file handle ONCE here to avoid re-opening in detection and native generation
    let mut open_file = std::fs::File::open(input_path).ok();

    let (strategy, is_video) = if let Some(ref mut file) = open_file {
        FileFormat::detect_header(file, input_path)
            .map(|f| (f.strategy.clone(), f.type_category == crate::formats::MediaType::Video))
            .unwrap_or_else(|| (get_thumbnail_strategy(input_path), false))
    } else {
        FileFormat::detect(input_path)
            .map(|f| (f.strategy.clone(), f.type_category == crate::formats::MediaType::Video))
            .unwrap_or_else(|| (ThumbnailStrategy::Icon, false))
    };

    let start = std::time::Instant::now();

    // OPTIMIZATION: Try external FFmpeg FIRST if available for Image/Video
    let ffmpeg_available = crate::media::ffmpeg::is_ffmpeg_available();

    // Note: If we use FFmpeg, the open_file handle is ignored (FFmpeg opens its own).
    // EXCLUSION: Affinity, Clip, XMind should always use NativeExtractor first as they are proprietary zip formats.
    let ext = input_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let is_zip_project = ["afphoto", "afdesign", "afpub", "clip", "xmind"].contains(&ext.as_str());

    // Explicitly exclude RAW formats from FFmpeg priority, as they should always go to rsraw.
    let is_raw_format = matches!(strategy, ThumbnailStrategy::Raw) || ["cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "dng", "raf", "orf", "rw2", "pef", "erf"].contains(&ext.as_str());

    if ffmpeg_available && !is_zip_project && !is_raw_format && matches!(strategy, ThumbnailStrategy::Ffmpeg | ThumbnailStrategy::NativeImage | ThumbnailStrategy::NativeExtractor) {
         if let Ok(_) = crate::media::ffmpeg::generate_thumbnail_ffmpeg_full(None, input_path, &output_path, size_px, is_video) {
             let elapsed = start.elapsed();
             println!("THUMB (FFmpeg Priority): SUCCESS | {:?} | {:?}", elapsed, input_path.file_name().unwrap_or_default());
             return Ok(hashed_filename.to_string());
         }
         println!("THUMB (FFmpeg Priority): FAILED - Falling back to Native");
    }

    let result = match strategy {
        ThumbnailStrategy::Ffmpeg => {
            println!("THUMB: Ffmpeg Strategy Final Failure for {:?}", input_path.file_name());
            Err("FFmpeg strategy failed or unavailable".into())
        },
        ThumbnailStrategy::NativeImage => native::generate_thumbnail_fast(input_path, &output_path, size_px, open_file.as_mut()).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::ZipPreview => archive::generate_thumbnail_zip_preview(input_path, &output_path, size_px).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::NativeExtractor => extractors::generate_thumbnail_extracted(input_path, &output_path, size_px).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::Raw => raw::generate_raw_thumbnail(input_path, &output_path, size_px).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::Webview => svg::generate_thumbnail_svg(input_path, &output_path, size_px).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::Font => font::generate_font_thumbnail(input_path, &output_path, size_px).map(|_| hashed_filename.to_string()),
        ThumbnailStrategy::Model3D => model::generate_model_preview(input_path, thumbnails_dir, hashed_filename, size_px),
        ThumbnailStrategy::Icon | ThumbnailStrategy::None => {
            // Use the shared icon generator logic
            icon::get_or_generate_icon(input_path, thumbnails_dir, size_px)
        },
    };

    let final_result = match result {
        Ok(path) => Ok(path),
        Err(e) => {
             // Fallback attempt: If not already Icon, try Icon
             if !matches!(strategy, ThumbnailStrategy::Icon) {
                  icon::get_or_generate_icon(input_path, thumbnails_dir, size_px)
             } else {
                  Err(e)
             }
        }
    };

    let elapsed = start.elapsed();
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
