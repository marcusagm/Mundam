use std::path::Path;
use image::load_from_memory;
use memmap2::MmapOptions;

/// Generates a thumbnail for a RAW image using a high-performance brute-force JPEG extraction.
///
/// Almost all RAW files (CR2, NEF, ARW, DNG, ORF, etc.) embed one or more full-size
/// JPEG previews. This function scans the file for JPEG SOI markers and attempts
/// to decode them, which is significantly faster than full RAW development.
pub fn generate_raw_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("THUMB: Brute-force extracting preview from RAW: {:?}", input_path);

    // 1. Open file with memory mapping for maximum speed
    let file = std::fs::File::open(input_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    // 2. Scan for JPEG START OF IMAGE (SOI) marker: FF D8 FF
    // We scan the first 2MB which usually contains all previews.
    // We prioritize the LARGEST JPEG found, as RAWs often contain multiple
    // small thumbnails and one large preview.

    let mut best_img: Option<image::DynamicImage> = None;
    let mut best_size = 0;

    let scan_limit = mmap.len().min(4 * 1024 * 1024); // Scan first 4MB
    let mut i = 0;
    while i < scan_limit - 4 {
        if mmap[i] == 0xFF && mmap[i+1] == 0xD8 && mmap[i+2] == 0xFF {
            // Found a potential JPEG start. Try to decode metatada first if possible,
            // or just try to decode the whole thing.
            // Note: image::load_from_memory is quite fast and safe.
            if let Ok(img) = load_from_memory(&mmap[i..]) {
                let s = img.width() * img.height();
                if s > best_size {
                    best_size = s;
                    best_img = Some(img);
                }
                // Skip ahead a bit to avoid overlapping JPEGs if any
                i += 1024;
                continue;
            }
        }
        i += 1;
    }

    if let Some(img) = best_img {
        println!("THUMB: Found preview ({}x{}) in {:?}", img.width(), img.height(), input_path);
        return process_image(img, output_path, size_px);
    }

    Err(format!("No valid embedded JPEG preview found in {:?}", input_path).into())
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
