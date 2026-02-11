use std::path::Path;
use image::load_from_memory;
use quickraw::Export;
use memmap2::MmapOptions;
use std::panic;

/// Generates a thumbnail for a RAW image using a hybrid approach:
/// 1. Tries `quickraw` for high-fidelity extraction.
/// 2. Falls back to a high-speed Brute-force JPEG scan if `quickraw` fails or panics.
pub fn generate_raw_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // We use catch_unwind because some underlying libraries (quickexif)
    // tend to panic on malformed or specific legacy RAW headers (like CRW).
    let path_clone = input_path.to_path_buf();

    let quick_result = panic::catch_unwind(|| {
        if let Ok(raw_data) = std::fs::read(&path_clone) {
            // We must copy the data out to avoid lifetime issues with the closure
            return Export::export_thumbnail_data(&raw_data)
                .map(|(data, _)| data.to_vec())
                .ok();
        }
        None
    });

    if let Ok(Some(thumbnail_data)) = quick_result {
        println!("THUMB: Extracted with quickraw (Fidelity): {:?}", input_path);
        if let Ok(img) = load_from_memory(&thumbnail_data) {
            return process_image(img, output_path, size_px);
        }
    } else {
        // Panic or Error occurred in quickraw. Fall back to Brute-force.
        println!("THUMB: quickraw failed/panicked. Falling back to Brute-force: {:?}", input_path);
    }

    // --- FALLBACK: BRUTE-FORCE JPEG SCAN ---
    brute_force_fallback(input_path, output_path, size_px)
}

/// Scans the file for the largest embedded JPEG preview.
fn brute_force_fallback(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(input_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    let mut best_img: Option<image::DynamicImage> = None;
    let mut best_size = 0u64;

    let scan_limit = mmap.len().min(4 * 1024 * 1024); // Scan first 4MB
    let mut i = 0;
    while i < scan_limit - 4 {
        if mmap[i] == 0xFF && mmap[i+1] == 0xD8 && mmap[i+2] == 0xFF {
            if let Ok(img) = load_from_memory(&mmap[i..]) {
                let s = (img.width() as u64) * (img.height() as u64);
                if s > best_size {
                    best_size = s;
                    best_img = Some(img);
                }
                i += 1024; // Skip block to accelerate scan
                continue;
            }
        }
        i += 1;
    }

    if let Some(img) = best_img {
        println!("THUMB: Brute-force found preview ({}x{}) in {:?}", img.width(), img.height(), input_path);
        return process_image(img, output_path, size_px);
    }

    Err(format!("Hybrid extraction failed for {:?}", input_path).into())
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
