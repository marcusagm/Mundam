use std::path::Path;
use fast_image_resize as fr;
use zune_jpeg::JpegDecoder;

/// Generates a thumbnail using native Rust libraries.
///
/// Optimized for performance using:
/// - `zune-jpeg` for fast JPEG decoding (SIMD-optimized).
/// - `fast_image_resize` for high-performance resizing (SIMD: SSE2, AVX2, NEON, WASM).
/// - `webp` crate for native encoding.
/// - **Buffered Reader** for efficient file IO.
/// - **Bilinear Filter** for resize speed (vs Lanczos3).
///
/// # Arguments
///
/// * `input_path` - Path to the image file.
/// * `output_path` - Destination path for the WebP thumbnail.
/// * `size_px` - Target size in pixels.
pub fn generate_thumbnail_fast(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
    open_file: Option<&mut std::fs::File>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_total = std::time::Instant::now();
    
    // We already know it's a supported format from detect, but let's double check extension for the decoder optimization
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Decode based on format - use optimized decoder for JPEG
    let start_decode = std::time::Instant::now();
    let (rgba_data, width, height) = match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => decode_jpeg_fast(input_path)?,
        _ => {
            // Fallback to image crate for other formats
            // Use BufReader for potentially better IO performance
            let file = if let Some(f) = open_file {
                // Rewind just in case
                let _ = std::io::Seek::seek(f, std::io::SeekFrom::Start(0));
                // We must clone the handle or use reference, but image::load consumes reader.
                // Since we passed Option<File>, and we are in a block, we can't easily consume it if we want to support the 'None' case cleanly without duplication.
                // However, File requires strict ownership for BufReader if we return it.
                // Actually, let's just use try_clone() which is cheap for file descriptors.
                f.try_clone()?
            } else {
                std::fs::File::open(input_path)?
            };
            
            let reader = std::io::BufReader::new(file);
            let img = image::load(reader, image::ImageFormat::from_path(input_path).unwrap_or(image::ImageFormat::Png))?;
            
            let w = img.width();
            let h = img.height();
            (img.to_rgba8().into_raw(), w, h)
        }
    };
    println!("DEBUG: Native Decode took: {:?}", start_decode.elapsed());

    // Calculate new dimensions maintaining aspect ratio
    let aspect = width as f32 / height as f32;
    let (new_w, new_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect).max(1.0) as u32)
    } else {
        (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
    };

    // Resize using fast_image_resize (SIMD optimized)
    let start_resize = std::time::Instant::now();
    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        rgba_data,
        fr::PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;

    let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    
    // Use Bilinear filter which is much faster than the default Lanczos3
    // Especially important for debug builds or large images
    let options = fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Bilinear));
    
    resizer
        .resize(&src_image, &mut dst_image, Some(&options))
        .map_err(|e| e.to_string())?;
    println!("DEBUG: Native Resize took: {:?}", start_resize.elapsed());

    // Encode to WebP using native webp crate
    let start_encode = std::time::Instant::now();
    let buffer = dst_image.buffer();
    encode_webp_native(buffer, new_w, new_h, output_path)?;
    println!("DEBUG: Native Encode took: {:?}", start_encode.elapsed());
    
    println!("DEBUG: Native Total took: {:?}", start_total.elapsed());

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
pub fn encode_webp_native(
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
