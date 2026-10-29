//! Dedicated extractor for GoPro RAW (.gpr) files.
//!
//! GPR files are based on Adobe DNG but use VC-5 (CineForm wavelet) compression
//! for the sensor data. This makes them incompatible with most standard RAW
//! decoders:
//!
//! - **FlashRaw/FlashRaw**: The bundled FlashRaw build does not decode GoPro VC-5 data.
//! - **FFmpeg**: Its still-image path does not decode this GPR compression mode.
//!
//! # Processing pipeline
//!
//! Metadata is read from the DNG/TIFF container and EXIF tags. FlashRaw decodes
//! the GoPro VC-5 sensor data and develops it into display-ready pixels in Rust.

use crate::core::error::{AppError, AppResult};
use flashraw::{DecodeOptions, FlashRaw, Limits, OutputFormat, RawPixelData};

use crate::processing::media::extractors::image;
use std::path::Path;

const MAX_GPR_DECODE_SAMPLES: u64 = 120_000_000;
const MAX_GPR_DIMENSION: u32 = 12_000;

fn decode_gpr_frame(decoder: &FlashRaw<'_>) -> AppResult<::image::DynamicImage> {
    let decoded_frame = decoder
        .decode(DecodeOptions {
            output: OutputFormat::Rgba8,
            limits: Limits::default()
                .with_max_samples(MAX_GPR_DECODE_SAMPLES)
                .with_max_dimension(MAX_GPR_DIMENSION),
            ..DecodeOptions::default()
        })
        .map_err(|error| AppError::Generic(format!("Failed to decode GPR image: {error}")))?;
    let pixel_bytes = match decoded_frame.data {
        RawPixelData::U8(pixel_bytes) => pixel_bytes,
        _ => {
            return Err(AppError::Generic(
                "GPR decoder returned an unexpected pixel format".to_string(),
            ));
        }
    };
    let rgba_image =
        ::image::RgbaImage::from_raw(decoded_frame.width, decoded_frame.height, pixel_bytes)
            .ok_or_else(|| {
                AppError::Generic("GPR decoder returned an invalid image buffer".to_string())
            })?;

    Ok(::image::DynamicImage::ImageRgba8(rgba_image))
}

fn open_gpr_image(path: &Path) -> AppResult<FlashRaw<'static>> {
    FlashRaw::open_path(path)
        .map_err(|error| AppError::Generic(format!("Failed to open GPR image: {error}")))
}

fn decode_gpr_image(path: &Path) -> AppResult<::image::DynamicImage> {
    let decoder = open_gpr_image(path)?;
    decode_gpr_frame(&decoder)
}

fn decode_gpr_thumbnail(path: &Path) -> AppResult<::image::DynamicImage> {
    let decoder = open_gpr_image(path)?;
    if let Some(thumbnail) = decoder.thumbnail() {
        if thumbnail.orientation.exif() == 1 {
            if let Ok(thumbnail_bytes) = decoder.preview_bytes(thumbnail) {
                if let Ok(thumbnail_image) = ::image::load_from_memory(thumbnail_bytes) {
                    return Ok(thumbnail_image);
                }
            }
        }
    }

    decode_gpr_frame(&decoder)
}

// ─── Public API ────────────────────────────────────────────────────────────

/// Extracts technical metadata from a GoPro RAW (.gpr) file.
///
/// Reads normalized DNG metadata through FlashRaw and supplements it with EXIF tags
/// (make, model, exposure, GPS, and other available fields).
///
/// This avoids relying on FlashRaw, which doesn't support the VC-5 codec.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
///
/// # Returns
///
/// A JSON object containing image dimensions, camera details, capture settings, and EXIF fields.
pub fn extract_gpr_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let decoder = open_gpr_image(path)?;
    let raw_metadata = decoder.metadata();
    let exposure = &raw_metadata.exposure;
    let (image_width, image_height) =
        crate::processing::media::extractors::image::dimensions_for_orientation(
            raw_metadata.width,
            raw_metadata.height,
            raw_metadata.orientation.exif(),
        );

    let mut metadata = serde_json::json!({
        "width": image_width,
        "height": image_height,
        "rawWidth": raw_metadata.raw_width,
        "rawHeight": raw_metadata.raw_height,
        "Make": raw_metadata.make,
        "Model": raw_metadata.model,
        "Orientation": raw_metadata.orientation.exif(),
        "BitsPerSample": raw_metadata.bits_per_sample,
        "ISO": exposure.iso,
        "ExposureTime": exposure.exposure_time,
        "FNumber": exposure.f_number,
        "FocalLength": exposure.focal_length,
        "FocalLengthIn35mmFilm": exposure.focal_length_35mm,
        "DateTimeOriginal": raw_metadata.capture_time,
        "LensModel": raw_metadata.lens.model,
        "FirmwareVersion": raw_metadata.firmware,
        "SerialNumber": raw_metadata.serial,
        "Artist": raw_metadata.artist,
        "Copyright": raw_metadata.copyright,
    });

    if let Some(gps) = &raw_metadata.gps {
        metadata["GPS"] = serde_json::json!({
            "latitude": gps.latitude,
            "longitude": gps.longitude,
            "altitude": gps.altitude,
            "timestamp": gps.timestamp,
        });
    }

    if let Some(compression) = raw_metadata.compression {
        metadata["compression"] = serde_json::Value::String(format!("{compression:?}"));
    }

    if let Some(metadata_object) = metadata.as_object_mut() {
        metadata_object.retain(|_, value| !value.is_null());
        for (key, value) in &raw_metadata.extra {
            metadata_object.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
    }

    // Preserve EXIF fields not covered by FlashRaw's normalized metadata.
    if let Ok(exif_data) = rexif::parse_file(path) {
        for entry in exif_data.entries {
            let tag_name = entry.tag.to_string();
            let tag_value = entry.value_more_readable.to_string();
            if !tag_value.trim().is_empty() {
                metadata[tag_name] = serde_json::Value::String(tag_value);
            }
        }
    }

    Ok(metadata)
}

/// Generates a WebP thumbnail from a GoPro RAW (.gpr) file using FlashRaw.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Returns
///
/// WebP-encoded thumbnail bytes.
pub fn generate_gpr_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    tracing::debug!("GPR thumbnail: decoding with FlashRaw for {:?}", path);
    image::process_and_encode_webp(decode_gpr_thumbnail(path)?, size_hint)
}

/// Extracts a high-quality JPEG preview from a GoPro RAW (.gpr) file.
///
/// Uses FlashRaw to decode and develop the VC-5 sensor data.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
///
/// # Returns
///
/// JPEG bytes suitable for display.
pub fn extract_gpr_preview(path: &Path) -> AppResult<Vec<u8>> {
    tracing::debug!("GPR preview: decoding with FlashRaw for {:?}", path);
    let decoded_image = decode_gpr_image(path)?.to_rgb8();

    let mut jpeg_bytes = Vec::new();
    let mut encoder = ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 92);
    encoder
        .encode_image(&decoded_image)
        .map_err(|e| AppError::Generic(format!("Failed to encode GPR preview as JPEG: {}", e)))?;

    Ok(jpeg_bytes)
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn gpr_sample_directory() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("file-samples")
            .join("Arquivo para testes")
            .join("Image")
            .join("gpr")
    }

    #[test]
    fn reads_sensor_dimensions_from_gpr_samples() {
        let gpr_directory = gpr_sample_directory();
        if !gpr_directory.exists() {
            return;
        }

        let expected_dimensions: Vec<(&str, u32, u32)> = vec![
            ("GOPR0002.GPR", 5568, 4176),
            ("GOPR0024.GPR", 4000, 3000),
            ("GOPR2657.GPR", 4000, 3000),
            ("GOPR9231.GPR", 4000, 3000),
            ("GPBK7066.GPR", 3104, 3000),
            ("GPFR7066.GPR", 3104, 3000),
        ];

        for (filename, expected_width, expected_height) in &expected_dimensions {
            let file_path = gpr_directory.join(filename);
            if !file_path.exists() {
                continue;
            }
            let decoder = FlashRaw::open_path(&file_path)
                .unwrap_or_else(|error| panic!("Failed to open {}: {}", filename, error));
            let width = decoder.metadata().width;
            let height = decoder.metadata().height;
            assert_eq!(
                width, *expected_width,
                "{}: expected width {}, got {}",
                filename, expected_width, width
            );
            assert_eq!(
                height, *expected_height,
                "{}: expected height {}, got {}",
                filename, expected_height, height
            );
        }
    }

    #[test]
    fn test_gpr_metadata_extraction() {
        let sample_path = gpr_sample_directory().join("GOPR0002.GPR");
        if !sample_path.exists() {
            return;
        }

        let metadata = extract_gpr_metadata(&sample_path).unwrap();
        assert_eq!(metadata["width"], 5568);
        assert_eq!(metadata["height"], 4176);
        // GoPro HERO9 Black should expose model info via EXIF
        assert!(
            metadata.get("Model").is_some(),
            "Metadata should contain camera Model"
        );
    }

    #[tokio::test]
    async fn test_gpr_thumbnail_generation() {
        let gpr_directory = gpr_sample_directory();
        if !gpr_directory.exists() {
            return;
        }

        for entry in std::fs::read_dir(gpr_directory).unwrap() {
            let entry = entry.unwrap();
            let file_path = entry.path();
            let extension = file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_uppercase();
            if extension != "GPR" {
                continue;
            }

            let filename = file_path.file_name().unwrap().to_string_lossy();
            println!("Testing thumbnail for: {}", filename);

            let thumbnail_result = generate_gpr_thumbnail(&file_path, 300);
            assert!(
                thumbnail_result.is_ok(),
                "Thumbnail generation failed for {}: {:?}",
                filename,
                thumbnail_result.err()
            );
            let thumbnail_bytes = thumbnail_result.unwrap();
            assert!(
                !thumbnail_bytes.is_empty(),
                "Thumbnail is empty for {}",
                filename
            );
        }
    }

    #[tokio::test]
    async fn test_gpr_preview_extraction() {
        let sample_path = gpr_sample_directory().join("GOPR0002.GPR");
        if !sample_path.exists() {
            return;
        }

        let preview_result = extract_gpr_preview(&sample_path);
        assert!(
            preview_result.is_ok(),
            "Preview extraction failed: {:?}",
            preview_result.err()
        );
        let preview_bytes = preview_result.unwrap();
        assert!(
            preview_bytes.starts_with(&[0xFF, 0xD8]),
            "Preview is not a valid JPEG"
        );
        assert!(
            preview_bytes.len() > 10_000,
            "Preview seems too small: {} bytes",
            preview_bytes.len()
        );
    }
}
