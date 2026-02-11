use std::path::Path;

/// Generates a thumbnail for a RAW image using rsraw (LibRaw).
///
/// This leverages LibRaw's robust decoding to support formats that are difficult
/// to parse manually (like CR3, RAF, newer ARW).
///
/// # Arguments
///
/// * `input_path` - Path to the RAW file.
/// * `output_path` - Destination path for the WebP thumbnail.
/// * `size_px` - Target size in pixels.
/// Generates a thumbnail for a RAW image using rsraw (LibRaw).
///
/// This leverages LibRaw's robust decoding to support formats that are difficult
/// to parse manually (like CR3, RAF, newer ARW).
pub fn generate_raw_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = extract_raw_preview_data(input_path)?;

    use image::load_from_memory;
    let img = load_from_memory(&data)
        .map_err(|e| format!("Failed to decode extracted RAW preview: {}", e))?;

    // Resize and save
    process_image(img, output_path, size_px)?;
    Ok(())
}

/// Extracts the largest embedded preview from a RAW file using rsraw.
pub fn extract_raw_preview_data(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Load the RAW file
    let file = std::fs::File::open(path)?;

    // Use memory mapping instead of reading entire file to heap
    let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

    let mut raw = rsraw::RawImage::open(&mmap)
        .map_err(|e| format!("LibRaw open error: {:?}", e))?;

    let thumbs = raw.extract_thumbs()
        .map_err(|e| format!("LibRaw extract_thumbs error: {:?}", e))?;

    // Find the largest thumbnail
    if let Some(thumb) = thumbs.iter().max_by_key(|t| t.width * t.height) {
        if thumb.data.is_empty() {
            return Err("Extracted thumbnail is empty".into());
        }
        Ok(thumb.data.clone())
    } else {
        Err("No embedded thumbnails found in RAW file".into())
    }
}

/// Helper to resize and save the image
fn process_image(
    img: image::DynamicImage,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use fast_image_resize as fr;
    use crate::thumbnails::native::encode_webp_native;

    let width = img.width();
    let height = img.height();

    // Calculate target dimensions
    let aspect = width as f32 / height as f32;
    let (new_w, new_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect).max(1.0) as u32)
    } else {
        (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
    };

    // Prepare source for resizing
    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;

    // Resize
    let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    let options = fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Bilinear));

    resizer
        .resize(&src_image, &mut dst_image, Some(&options))
        .map_err(|e| e.to_string())?;

    // Save as WebP
    let buffer = dst_image.buffer();
    encode_webp_native(buffer, new_w, new_h, output_path)?;

    Ok(())
}
