use std::path::Path;
use image::DynamicImage;
use rawloader;

/// Generates a thumbnail for a RAW image using `rawloader`.
///
/// This leverages a pure Rust implementation to decode RAW files.
/// `rawloader` is generally safer and easier to compile than C bindings,
/// but might not support every camera model perfectly.
///
/// # Arguments
///
/// * `input_path` - Path to the RAW file.
/// * `output_path` - Destination path for the WebP thumbnail.
/// * `size_px` - Target size in pixels.
pub fn generate_raw_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("THUMB: Processing RAW with rawloader: {:?}", input_path);

    // Decode the RAW file using rawloader
    // rawloader handles opening the file internally
    // Note: decode_file returns a Result<RawImage, custom_error>
    let raw_image = rawloader::decode_file(input_path)
        .map_err(|e| format!("rawloader decode error: {:?}", e))?;

    // rawloader returns raw sensor data (typically 12-14 bit packed).
    // We need to demosaic and convert to RGB8 for thumbnail generation.
    // However, rawloader's `RawImage` structural data is complex.
    // BUT! Most RAW files have embedded JPEG previews which are infinitely faster.
    // Does rawloader expose them?

    // Looking at rawloader docs/source would tell us.
    // Based on common knowledge of rawloader, it primarily decodes the raw data.
    // If it *only* decodes full raw data, it might still be slow (seconds).
    // But let's try the full decode path first as requested to test the library.

    // Convert RawImage to DynamicImage
    // This is non-trivial because raw data is CFA (Bayer) mosaic.
    // We need a simple debayering. `rawloader` provides raw data, not processed RGB.

    // WAIT: Does rawloader process it?
    // "rawloader decodes raw files into a common structure."
    // It does NOT do demosaicing. We would need `image` or another crate to demosaic.
    // This implies `rawloader` alone might be Insufficient for *thumbnails* unless
    // we just want the embedded preview, which `rawloader` might not expose easily.

    // Let's check if we can extract a thumbnail.
    // If not, we fall back to a crate that DOES extract previews, like `exif` or `nom-exif`?
    // But the user asked to test `rawloader`. Let's implement what we can.

    // Actually, for a *thumbnail*, doing full demosaicing of 24MP+ data in pure Rust
    // without optimization is extremely slow (likely slower than rsraw).
    // If rawloader doesn't have `extract_thumbnail`, this path might be a dead end for performance.

    // Let's look for `thumbnails` in `RawImage`.
    // (Assuming hypothetical API for now, since I can't check docs live without internet/docs tool)
    // If I can't find it, I'll return an error explaining we need a demosaicing step.

    /*
      Hypothetical structure:
      pub struct RawImage {
        pub make: String,
        pub model: String,
        pub width: usize,
        pub height: usize,
        pub data: Vec<u16>, // Sensor data
        ...
      }
    */

    // Since we don't have a demosaicing library ready (like `libraw` had built-in),
    // and `rawloader` just gives us the grayscale bayer pattern,
    // this might produce a grayscale "screen door" image if we just map it.
    // This is fine for a technical test of "can we read the file", but bad for "thumbnails".

    // Let's stick to the request: "Test with rawloader".
    // I will implementation a basic conversion (taking green channel or just converting to mono)
    // just to see if it READS the file faster than rsraw's 11s.

    let width = raw_image.width;
    let height = raw_image.height;

    // Perform a very naive "Superpixel" demosaic (just take one generic pixel per 2x2 block) to speed up
    // and get a smaller image immediately.
    // This reduces 6000x4000 -> 3000x2000, much faster for thumbs.

    let small_w = (width / 2) as u32;
    let small_h = (height / 2) as u32;

    let mut img_buffer = image::ImageBuffer::new(small_w, small_h);

    // Raw data usually u16. We need to normalize to u8.
    // The white level is usually defined per channel, picking the first one as reference.
    let white_level = if !raw_image.whitelevels.is_empty() {
        raw_image.whitelevels[0] as f32
    } else {
        65535.0
    };

    match &raw_image.data {
        rawloader::RawImageData::Integer(data) => {
             for y in 0..small_h {
                for x in 0..small_w {
                    // Map 2x2 block to 1 pixel. origin in raw is (x*2, y*2)
                    let raw_idx = (y * 2) as usize * width + (x * 2) as usize;
                    if raw_idx < data.len() {
                         let val = data[raw_idx] as f32;
                         // Normalize to 0..255
                         let p = ((val / white_level) * 255.0).min(255.0) as u8;
                         img_buffer.put_pixel(x, y, image::Rgb([p, p, p]));
                    }
                }
            }
        },
        rawloader::RawImageData::Float(data) => {
             for y in 0..small_h {
                for x in 0..small_w {
                    // Map 2x2 block to 1 pixel. origin in raw is (x*2, y*2)
                    let raw_idx = (y * 2) as usize * width + (x * 2) as usize;
                    if raw_idx < data.len() {
                         let val = data[raw_idx]; // Already float
                         // Normalize to 0..255 (Assumes data is raw float values, likely needing normalization against white_level too)
                         let p = ((val / white_level) * 255.0).min(255.0) as u8;
                         img_buffer.put_pixel(x, y, image::Rgb([p, p, p]));
                    }
                }
            }
        }
    }

    let dynamic_image = DynamicImage::ImageRgb8(img_buffer);

    // Resize and save
    process_image(dynamic_image, output_path, size_px)
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
