//! Aseprite file preview extractor.
//!
//! Uses `asefile` to parse .ase/.aseprite files and `image` to encode them.
//! Provides static PNG for single frames and Animated GIF for multi-frame animations.

use asefile::AsepriteFile;
use std::path::Path;
use std::time::Duration;
use image::{Frame, Delay, codecs::gif::{GifEncoder, Repeat}, DynamicImage};

/// Extracts a preview from an Aseprite file.
///
/// Returns a tuple of (data, mime_type).
/// - 1 frame: image/png
/// - >1 frame: image/gif (animated)
///
/// # Errors
/// Returns error if the file is corrupted or cannot be parsed.
pub fn extract_aseprite_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let aseprite_file = AsepriteFile::read_file(path)?;
    let total_frames = aseprite_file.num_frames();

    if total_frames == 0 {
        return Err("Aseprite file has no frames".into());
    }

    if total_frames == 1 {
        let frame_data = aseprite_file.frame(0);
        let image_rgba_v24 = frame_data.image();
        let (width, height) = (image_rgba_v24.width(), image_rgba_v24.height());

        // Convert from image 0.24 (asefile transitive) to image 0.25 (app primary)
        // We do this by moving the raw pixel buffer between types.
        let raw_pixels = image_rgba_v24.into_raw();
        let image_rgba_v25 = image::RgbaImage::from_raw(width, height, raw_pixels)
            .ok_or("Failed to re-wrap Aseprite frame into image 0.25 buffer")?;

        let mut output_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output_buffer);
        DynamicImage::ImageRgba8(image_rgba_v25).write_to(&mut cursor, image::ImageFormat::Png)?;

        Ok((output_buffer, "image/png".to_string()))
    } else {
        // Multi-frame: Generate an Animated GIF for the preview
        let mut output_buffer = Vec::new();
        {
            let mut gif_encoder = GifEncoder::new(&mut output_buffer);
            gif_encoder.set_repeat(Repeat::Infinite)?;
            for index in 0..total_frames {
                let frame_data = aseprite_file.frame(index);
                let image_rgba_v24 = frame_data.image();
                let (width, height) = (image_rgba_v24.width(), image_rgba_v24.height());

                let raw_pixels = image_rgba_v24.into_raw();
                let image_rgba_v25 = image::RgbaImage::from_raw(width, height, raw_pixels)
                    .ok_or("Failed to re-wrap Aseprite animation frame")?;

                let frame_duration_ms = frame_data.duration();
                let frame_delay = Delay::from_saturating_duration(Duration::from_millis(frame_duration_ms as u64));

                let gif_frame = Frame::from_parts(image_rgba_v25, 0, 0, frame_delay);
                gif_encoder.encode_frame(gif_frame)?;
            }
        }

        Ok((output_buffer, "image/gif".to_string()))
    }
}
