use fast_image_resize as fr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zune_jpeg::JpegDecoder;

/// Thumbnail generation strategy based on file format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThumbnailStrategy {
    /// Fast Rust-native path for common formats (JPEG, PNG, WebP, GIF, BMP)
    FastPath,
    /// FFmpeg subprocess for complex formats (RAW, HEIC, PSD, etc.)
    FFmpeg,
    /// Extract embedded preview from ZIP-based formats (Affinity, XMind, etc.)
    ZipPreview,
    /// Generate icon-based thumbnail for unsupported formats
    IconFallback,
}

/// Determines the best strategy for generating a thumbnail based on file extension
pub fn get_strategy(path: &Path) -> ThumbnailStrategy {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Tier 1: Fast path - Rust native (most common formats)
        "jpg" | "jpeg" | "jpe" | "jfif" | "png" | "webp" | "gif" | "bmp" | "ico" => {
            ThumbnailStrategy::FastPath
        }

        // Tier 2: FFmpeg - Complex formats requiring external decoder
        "heic" | "heif" | "hif" | "avif" | "jxl" |  // Modern codecs
        "cr2" | "cr3" | "arw" | "nef" | "dng" | "raf" | "orf" | "pef" | "rw2" | 
        "3fr" | "mrw" | "nrw" | "sr2" | "srw" | "x3f" | "erf" | "crw" | "raw" |  // RAW
        "psd" | "psb" | "ai" | "eps" | "svg" | "tif" | "tiff" => {
            ThumbnailStrategy::FFmpeg
        }

        // Tier 3: ZIP Preview - Extract embedded thumbnail
        "af" | "afdesign" | "afphoto" | "afpub" |  // Affinity
        "clip" | "xmind" | "graffle" => {
            ThumbnailStrategy::ZipPreview
        }

        // Tier 4: Icon fallback - No thumbnail possible
        _ => ThumbnailStrategy::IconFallback,
    }
}

/// Main entry point for thumbnail generation - routes to appropriate strategy
pub fn generate_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let strategy = get_strategy(input_path);
    let start = std::time::Instant::now();
    
    // Prefer FFmpeg for ALL image formats when available (much faster for large images)
    let result = if crate::ffmpeg::is_ffmpeg_available() {
        match crate::ffmpeg::generate_thumbnail_ffmpeg_full(None, input_path, output_path, size_px) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("FFmpeg failed, falling back to Rust: {}", e);
                // Fallback to Rust implementation
                match strategy {
                    ThumbnailStrategy::FastPath => generate_thumbnail_fast(input_path, output_path, size_px),
                    ThumbnailStrategy::ZipPreview => generate_thumbnail_zip_preview(input_path, output_path, size_px),
                    ThumbnailStrategy::IconFallback => generate_thumbnail_icon(input_path, output_path, size_px),
                    ThumbnailStrategy::FFmpeg => Err(e), // Already tried FFmpeg
                }
            }
        }
    } else {
        // No FFmpeg available, use Rust implementations
        match strategy {
            ThumbnailStrategy::FastPath => generate_thumbnail_fast(input_path, output_path, size_px),
            ThumbnailStrategy::FFmpeg => generate_thumbnail_ffmpeg(input_path, output_path, size_px),
            ThumbnailStrategy::ZipPreview => generate_thumbnail_zip_preview(input_path, output_path, size_px),
            ThumbnailStrategy::IconFallback => generate_thumbnail_icon(input_path, output_path, size_px),
        }
    };
    
    let elapsed = start.elapsed();
    println!("THUMB: {:?} | {:?} | {:?}", strategy, elapsed, input_path.file_name().unwrap_or_default());
    
    result
}


/// Fast path using zune-jpeg for JPEG decode and webp crate for encode
pub fn generate_thumbnail_fast(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Decode based on format - use optimized decoder for JPEG
    let (rgba_data, width, height) = match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => decode_jpeg_fast(input_path)?,
        _ => {
            // Fallback to image crate for other formats (png, gif, bmp, ico, webp)
            let img = image::open(input_path)?;
            let w = img.width();
            let h = img.height();
            (img.to_rgba8().into_raw(), w, h)
        }
    };

    // Calculate new dimensions maintaining aspect ratio
    let aspect = width as f32 / height as f32;
    let (new_w, new_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect).max(1.0) as u32)
    } else {
        (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
    };

    // Resize using fast_image_resize (SIMD optimized)
    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        rgba_data,
        fr::PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;

    let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, None)
        .map_err(|e| e.to_string())?;

    // Encode to WebP using native webp crate
    let buffer = dst_image.buffer();
    encode_webp_native(buffer, new_w, new_h, output_path)?;

    Ok(())
}

/// Decode JPEG using zune-jpeg (faster pure Rust decoder, ~2-3x faster than image crate)
fn decode_jpeg_fast(path: &Path) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let jpeg_data = std::fs::read(path)?;
    
    let mut decoder = JpegDecoder::new(&jpeg_data);
    
    // Decode to RGB
    let pixels = decoder.decode()
        .map_err(|e| format!("JPEG decode error: {:?}", e))?;
    
    let info = decoder.info()
        .ok_or("Failed to get JPEG info")?;
    
    let width = info.width as u32;
    let height = info.height as u32;
    
    // Convert RGB to RGBA
    let rgba = rgb_to_rgba(&pixels);
    
    Ok((rgba, width, height))
}

/// Convert RGB pixels to RGBA (add alpha channel)
fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let pixel_count = rgb.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    
    for chunk in rgb.chunks_exact(3) {
        rgba.push(chunk[0]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[2]); // B
        rgba.push(255);      // A (opaque)
    }
    
    rgba
}

/// Encode image data to WebP using native libwebp
fn encode_webp_native(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let encoder = webp::Encoder::from_rgba(rgba_data, width, height);
    let webp_data = encoder.encode(80.0); // Quality 80
    
    std::fs::write(output_path, &*webp_data)?;
    Ok(())
}

/// FFmpeg-based thumbnail generation for complex formats
/// Supports: RAW, HEIC, AVIF, PSD, SVG, TIFF, etc.
pub fn generate_thumbnail_ffmpeg(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try FFmpeg first if available
    if crate::ffmpeg::is_ffmpeg_available() {
        match crate::ffmpeg::generate_thumbnail_ffmpeg_full(None, input_path, output_path, size_px) {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("FFmpeg failed, trying fallback: {}", e);
            }
        }
    }
    
    // Fallback to image crate for formats it supports (tiff, etc.)
    // This won't work for RAW/HEIC but will for some formats
    let img = image::open(input_path)?;
    let width = img.width();
    let height = img.height();

    let aspect = width as f32 / height as f32;
    let (new_w, new_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect).max(1.0) as u32)
    } else {
        (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
    };

    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;

    let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, None)
        .map_err(|e| e.to_string())?;

    let buffer = dst_image.buffer();
    encode_webp_native(buffer, new_w, new_h, output_path)?;

    Ok(())
}


/// Extract preview from ZIP-based formats (Affinity, XMind, etc.)
pub fn generate_thumbnail_zip_preview(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    // Common preview paths in ZIP-based design files
    let preview_paths = [
        "preview.png",
        "Thumbnails/thumbnail.png", 
        "QuickLook/Preview.png",
        "QuickLook/Thumbnail.png",
        "icon.png",
    ];
    
    for preview_path in &preview_paths {
        if let Ok(mut entry) = archive.by_name(preview_path) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            
            // Decode the preview image
            let img = image::load_from_memory(&buf)?;
            let width = img.width();
            let height = img.height();
            
            let aspect = width as f32 / height as f32;
            let (new_w, new_h) = if aspect > 1.0 {
                (size_px, (size_px as f32 / aspect).max(1.0) as u32)
            } else {
                (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
            };
            
            let src_image = fr::images::Image::from_vec_u8(
                width,
                height,
                img.to_rgba8().into_raw(),
                fr::PixelType::U8x4,
            )
            .map_err(|e| e.to_string())?;
            
            let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
            let mut resizer = fr::Resizer::new();
            resizer
                .resize(&src_image, &mut dst_image, None)
                .map_err(|e| e.to_string())?;
            
            let buffer = dst_image.buffer();
            encode_webp_native(buffer, new_w, new_h, output_path)?;
            
            return Ok(());
        }
    }
    
    Err("No preview found in archive".into())
}

/// Icon category for unsupported file types
#[derive(Debug, Clone, Copy)]
pub enum IconCategory {
    File3D,
    Font,
    Design,
    Generic,
}

/// Determine icon category based on file extension
fn get_icon_category(path: &Path) -> IconCategory {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        // 3D formats
        "c4d" | "3ds" | "obj" | "fbx" | "blend" | "stl" | "dae" | 
        "skp" | "dwg" | "dxf" | "max" | "lwo" | "lws" | "ma" | "mb" => {
            IconCategory::File3D
        }
        
        // Font formats
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "fon" | "fnt" => {
            IconCategory::Font
        }
        
        // Design formats (non-ZIP based)
        "cdr" | "indd" | "xd" | "fig" | "sketch" => {
            IconCategory::Design
        }
        
        _ => IconCategory::Generic,
    }
}

/// Generate icon-based thumbnail for unsupported formats
/// Creates a simple colored placeholder with file extension
pub fn generate_thumbnail_icon(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let category = get_icon_category(input_path);
    let _ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("?")
        .to_uppercase();
    
    // Create a simple colored thumbnail based on category
    // Using a solid color background with extension text
    let (bg_color, text_color) = match category {
        IconCategory::File3D => ([0x1a, 0x3a, 0x4e, 0xff], [0x6a, 0xaa, 0xee, 0xff]),
        IconCategory::Font => ([0x3e, 0x3a, 0x2a, 0xff], [0xca, 0xba, 0x8a, 0xff]),
        IconCategory::Design => ([0x3a, 0x2a, 0x4e, 0xff], [0xba, 0x9a, 0xde, 0xff]),
        IconCategory::Generic => ([0x2a, 0x2a, 0x3e, 0xff], [0x88, 0x88, 0xaa, 0xff]),
    };
    
    // Create image buffer (RGBA)
    let mut pixels = vec![0u8; (size_px * size_px * 4) as usize];
    
    // Fill with background color and rounded corners effect
    for y in 0..size_px {
        for x in 0..size_px {
            let idx = ((y * size_px + x) * 4) as usize;
            
            // Simple rounded corner check (corner radius ~10% of size)
            let corner_radius = (size_px / 10) as i32;
            let in_corner = is_in_rounded_corner(x as i32, y as i32, size_px as i32, corner_radius);
            
            if in_corner {
                // Transparent for corners
                pixels[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                pixels[idx..idx + 4].copy_from_slice(&bg_color);
            }
        }
    }
    
    // Draw a simple file icon shape in the center
    let icon_size = size_px * 60 / 100;
    let icon_x = (size_px - icon_size) / 2;
    let icon_y = (size_px - icon_size) / 2 - size_px / 10;
    draw_file_icon(&mut pixels, size_px, icon_x, icon_y, icon_size, &text_color);
    
    // Encode to WebP
    encode_webp_native(&pixels, size_px, size_px, output_path)?;
    
    Ok(())
}

/// Check if pixel is in a rounded corner region
fn is_in_rounded_corner(x: i32, y: i32, size: i32, radius: i32) -> bool {
    // Top-left corner
    if x < radius && y < radius {
        let dx = radius - x;
        let dy = radius - y;
        return dx * dx + dy * dy > radius * radius;
    }
    // Top-right corner
    if x >= size - radius && y < radius {
        let dx = x - (size - radius - 1);
        let dy = radius - y;
        return dx * dx + dy * dy > radius * radius;
    }
    // Bottom-left corner
    if x < radius && y >= size - radius {
        let dx = radius - x;
        let dy = y - (size - radius - 1);
        return dx * dx + dy * dy > radius * radius;
    }
    // Bottom-right corner
    if x >= size - radius && y >= size - radius {
        let dx = x - (size - radius - 1);
        let dy = y - (size - radius - 1);
        return dx * dx + dy * dy > radius * radius;
    }
    false
}

/// Draw a simple file icon shape
fn draw_file_icon(pixels: &mut [u8], img_size: u32, x: u32, y: u32, size: u32, color: &[u8; 4]) {
    let fold_size = size / 4;
    
    for py in 0..size {
        for px in 0..size {
            let abs_x = x + px;
            let abs_y = y + py;
            
            if abs_x >= img_size || abs_y >= img_size {
                continue;
            }
            
            // File body (simple rectangle with folded corner)
            let in_body = px < size && py < size;
            let in_fold = px >= size - fold_size && py < fold_size;
            
            if in_body && !in_fold {
                // Draw border (2px)
                let is_border = px < 2 || px >= size - 2 || py < 2 || py >= size - 2
                    || (px >= size - fold_size - 2 && py < fold_size);
                
                if is_border {
                    let idx = ((abs_y * img_size + abs_x) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(color);
                }
            }
        }
    }
}

/// Generate a unique thumbnail filename from image path
pub fn get_thumbnail_filename(image_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    image_path.hash(&mut hasher);
    format!("{:x}.webp", hasher.finish())
}
