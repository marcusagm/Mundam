use std::path::Path;
use crate::formats::{FileFormat, ThumbnailStrategy};

pub mod native;
pub mod archive;
pub mod icon;

/// Determines the best strategy for generating a thumbnail based on file detection
pub fn get_thumbnail_strategy(path: &Path) -> ThumbnailStrategy {
    match FileFormat::detect(path) {
        Some(format) => format.strategy.clone(),
        None => ThumbnailStrategy::Icon, 
    }
}

/// Main entry point for thumbnail generation - routes to appropriate strategy
pub fn generate_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let strategy = get_thumbnail_strategy(input_path);
    let start = std::time::Instant::now();
    
    // Debug log
    // println!("THUMB: {:?} | {:?} | {:?}", strategy, size_px, input_path.file_name().unwrap_or_default());
    
    let result = match strategy {
        ThumbnailStrategy::Ffmpeg => {
            // Need to verify if FFmpeg IS actually available
            if crate::ffmpeg::is_ffmpeg_available() {
                crate::ffmpeg::generate_thumbnail_ffmpeg_full(None, input_path, output_path, size_px)
            } else {
                Err("FFmpeg required but not available".into())
            }
        },
        ThumbnailStrategy::NativeImage => native::generate_thumbnail_fast(input_path, output_path, size_px),
        ThumbnailStrategy::ZipPreview => archive::generate_thumbnail_zip_preview(input_path, output_path, size_px),
        ThumbnailStrategy::Webview => icon::generate_thumbnail_icon(input_path, output_path, size_px), // TODO: Webview capture not implemented yet, fallback to icon
        ThumbnailStrategy::Icon | ThumbnailStrategy::None => icon::generate_thumbnail_icon(input_path, output_path, size_px),
    };
    
    let final_result = match result {
        Ok(_) => Ok(()),
        Err(e) => {
             eprintln!("Primary strategy {:?} failed: {}", strategy, e);
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
