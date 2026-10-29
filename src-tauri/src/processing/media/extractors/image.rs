//! Shared image extraction logic for all raster, RAW and modern image providers.
//!
//! This module centralises thumbnail generation and metadata extraction so every
//! per-format provider in `providers/image/` can delegate to a single, well-tested
//! implementation rather than duplicating logic.
//!
//! # Extraction Tiers (RAW)
//!
//! 1. **FlashRaw** — extracts the largest embedded preview.
//! 2. **Brute-force JPEG scan** — walks the file looking for `FF D8 FF` markers.
//! 3. **FFmpeg fallback** — last resort for formats that embed no preview.

use crate::core::error::{AppError, AppResult};
use std::path::Path;

// ─── Shared helpers ────────────────────────────────────────────────────────

/// Resizes a [`image::DynamicImage`] to fit within `size_hint` and encodes the
/// result as WebP at 80 % quality.
///
/// # Arguments
///
/// * `source_image` - The decoded source image.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Errors
///
/// * `AppError::Generic` - If resize or WebP encoding fails.
pub fn process_and_encode_webp(
    source_image: image::DynamicImage,
    size_hint: u32,
) -> AppResult<Vec<u8>> {
    use fast_image_resize as fr;

    let source_width = source_image.width();
    let source_height = source_image.height();

    if source_width == 0 || source_height == 0 {
        return Err(AppError::Generic(
            "Image dimensions are zero; cannot generate thumbnail".into(),
        ));
    }

    let aspect_ratio = source_width as f32 / source_height as f32;
    let (target_width, target_height) = if aspect_ratio > 1.0 {
        (size_hint, (size_hint as f32 / aspect_ratio).max(1.0) as u32)
    } else {
        ((size_hint as f32 * aspect_ratio).max(1.0) as u32, size_hint)
    };

    let source_fr_image = fr::images::Image::from_vec_u8(
        source_width,
        source_height,
        source_image.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|error| AppError::Generic(error.to_string()))?;

    let mut destination_fr_image =
        fr::images::Image::new(target_width, target_height, fr::PixelType::U8x4);

    let mut resizer = fr::Resizer::new();
    let resize_options =
        fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Bilinear));

    resizer
        .resize(
            &source_fr_image,
            &mut destination_fr_image,
            Some(&resize_options),
        )
        .map_err(|error| AppError::Generic(error.to_string()))?;

    let webp_encoder =
        webp::Encoder::from_rgba(destination_fr_image.buffer(), target_width, target_height);
    let webp_data = webp_encoder.encode(80.0);

    Ok(webp_data.to_vec())
}

// ─── EXIF helpers ──────────────────────────────────────────────────────────

/// Extracts EXIF metadata from any image file that embeds it (JPEG, TIFF, RAW, etc.).
///
/// Returns a `serde_json::Map` with human-readable keys and string values. An
/// empty map is returned if no EXIF data is found or parsing fails.
///
/// # Arguments
///
/// * `path` - Path to the image file.
fn extract_exif_data(path: &Path) -> serde_json::Map<String, serde_json::Value> {
    let mut exif_map = serde_json::Map::new();

    if let Ok(exif_data) = rexif::parse_file(path) {
        for entry in exif_data.entries {
            let tag_name = entry.tag.to_string();
            let tag_value = entry.value_more_readable.to_string();
            if !tag_value.trim().is_empty() {
                exif_map.insert(tag_name, serde_json::Value::String(tag_value));
            }
        }
    }

    exif_map
}

/// Parses a resolution value string from EXIF into an `f64`.
///
/// EXIF resolution values are often expressed as rational numbers like `"72/1"`.
fn parse_exif_resolution_value(resolution_string: &str) -> Option<f64> {
    let trimmed = resolution_string.trim();
    if let Some(slash_position) = trimmed.find('/') {
        let numerator: f64 = trimmed[..slash_position].trim().parse().ok()?;
        let denominator: f64 = trimmed[slash_position + 1..].trim().parse().ok()?;
        if denominator != 0.0 {
            return Some(numerator / denominator);
        }
    }
    trimmed.parse::<f64>().ok()
}

// ─── Standard raster ───────────────────────────────────────────────────────

/// Extracts technical metadata from a standard raster image (JPEG, PNG, GIF,
/// BMP, TIFF, DDS, NetPBM, etc.) using the `image` crate for dimensions and
/// `rexif` for EXIF data including resolution (DPI).
///
/// # Arguments
///
/// * `path` - Path to the image file.
///
/// # Errors
///
/// * `AppError::Io` - If the file cannot be opened.
/// * `AppError::Generic` - If dimension extraction fails.
/// Helper to open a raster image with limits disabled and custom extension handling (e.g. .cur).
fn open_raster_image(path: &Path) -> AppResult<image::DynamicImage> {
    let file_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut image_reader = image::ImageReader::open(path).map_err(AppError::Io)?;
    if file_extension == "cur" {
        image_reader.set_format(image::ImageFormat::Ico);
    } else {
        image_reader = image_reader.with_guessed_format().map_err(AppError::Io)?;
    }

    image_reader.limits(image::Limits::no_limits());

    image_reader
        .decode()
        .map_err(|decode_error| AppError::Generic(format!("Image decode error: {}", decode_error)))
}

pub fn extract_raster_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let file_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut image_reader = image::ImageReader::open(path).map_err(AppError::Io)?;

    if file_extension == "cur" {
        image_reader.set_format(image::ImageFormat::Ico);
    } else {
        image_reader = image_reader.with_guessed_format().map_err(AppError::Io)?;
    }

    image_reader.limits(image::Limits::no_limits());

    let detected_format = image_reader.format();
    let image_dimensions = match image_reader.into_dimensions() {
        Ok(dimensions) => dimensions,
        Err(dimension_error) => {
            tracing::warn!(
                "ImageReader::into_dimensions failed ({}), falling back to full decode: {:?}",
                dimension_error,
                path
            );
            let decoded_image = open_raster_image(path)?;
            (decoded_image.width(), decoded_image.height())
        }
    };

    let exif_map = extract_exif_data(path);

    let x_resolution = exif_map
        .get("XResolution")
        .and_then(|value| value.as_str())
        .and_then(parse_exif_resolution_value);

    let y_resolution = exif_map
        .get("YResolution")
        .and_then(|value| value.as_str())
        .and_then(parse_exif_resolution_value);

    let resolution_unit = exif_map
        .get("ResolutionUnit")
        .and_then(|value| value.as_str())
        .map(|unit| unit.to_string());

    let mut metadata = serde_json::json!({
        "width": image_dimensions.0,
        "height": image_dimensions.1,
        "format": detected_format.map(|format| format!("{:?}", format)).unwrap_or_default(),
    });

    if let Some(obj) = metadata.as_object_mut() {
        for (k, v) in exif_map {
            obj.insert(k, v);
        }
    }

    if let Some(dpi_horizontal) = x_resolution {
        metadata["x_resolution"] = serde_json::json!(dpi_horizontal);
    }
    if let Some(dpi_vertical) = y_resolution {
        metadata["y_resolution"] = serde_json::json!(dpi_vertical);
    }
    if let Some(unit) = resolution_unit {
        metadata["resolution_unit"] = serde_json::json!(unit);
    }

    Ok(metadata)
}

/// Generates a WebP thumbnail from a standard raster image.
///
/// Uses `zune-jpeg` for fast JPEG decoding and `image-rs` for all other formats,
/// then resizes with `fast-image-resize` and encodes to WebP.
///
/// # Arguments
///
/// * `path` - Path to the image file.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Errors
///
/// * `AppError::Io` - If the file cannot be read.
/// * `AppError::Generic` - If decoding, resizing or encoding fails.
pub fn generate_raster_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    use zune_jpeg::JpegDecoder;

    let file_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_lowercase();

    let decoded_image = match file_extension.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => {
            let jpeg_bytes = std::fs::read(path).map_err(AppError::Io)?;
            let mut jpeg_decoder = JpegDecoder::new(&jpeg_bytes);
            let decoded_pixels = jpeg_decoder
                .decode()
                .map_err(|error| AppError::Generic(format!("JPEG decode error: {:?}", error)))?;
            let jpeg_info = jpeg_decoder
                .info()
                .ok_or_else(|| AppError::Generic("Failed to retrieve JPEG image info".into()))?;

            let mut rgba_pixels = Vec::with_capacity(decoded_pixels.len() / 3 * 4);
            for rgb_chunk in decoded_pixels.chunks_exact(3) {
                rgba_pixels.push(rgb_chunk[0]);
                rgba_pixels.push(rgb_chunk[1]);
                rgba_pixels.push(rgb_chunk[2]);
                rgba_pixels.push(255);
            }

            let rgba_buffer = image::RgbaImage::from_raw(
                jpeg_info.width as u32,
                jpeg_info.height as u32,
                rgba_pixels,
            )
            .ok_or_else(|| {
                AppError::Generic("Failed to build RGBA buffer from JPEG data".into())
            })?;
            image::DynamicImage::ImageRgba8(rgba_buffer)
        }
        _ => open_raster_image(path)?,
    };

    process_and_encode_webp(decoded_image, size_hint)
}

// ─── RAW photography ───────────────────────────────────────────────────────

/// Extracts technical metadata from a RAW photography file.
///
/// Uses FlashRaw for canonical image dimensions and then `rexif` to
/// surface camera-specific EXIF data (make, model, exposure settings, etc.).
///
/// # Arguments
///
/// * `path` - Path to the RAW file.
///
/// # Errors
///
/// * `AppError::Io` - If the file cannot be opened.
/// * `AppError::Generic` - If RAW parsing fails.
pub fn extract_raw_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let mut image_width = 0u32;
    let mut image_height = 0u32;
    let mut flashraw_metadata = None;

    // FlashRaw reads metadata without decoding sensor pixels.
    if let Ok(raw_image) = flashraw::FlashRaw::open_path(path) {
        let metadata = raw_image.metadata().clone();
        (image_width, image_height) = dimensions_for_orientation(
            metadata.width,
            metadata.height,
            metadata.orientation.exif(),
        );
        flashraw_metadata = Some(metadata);
    }

    // If FlashRaw fails, fallback to brute force dimension extraction
    if image_width == 0 || image_height == 0 {
        if let Ok((bytes, _)) = brute_force_extract_jpeg_bytes(path) {
            if let Ok(decoded_image) = image::load_from_memory(&bytes) {
                image_width = decoded_image.width();
                image_height = decoded_image.height();
            }
        }
    }

    // Leaf MOS files can contain large maker notes that make generic EXIF parsing
    // unacceptably slow. FlashRaw already reads their normalized metadata safely.
    let is_leaf_mos = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mos"));
    let mut exif_map = if is_leaf_mos {
        serde_json::Map::new()
    } else {
        extract_exif_data(path)
    };

    if let Some(raw_metadata) = &flashraw_metadata {
        insert_metadata_if_missing(&mut exif_map, "Make", &raw_metadata.make);
        insert_metadata_if_missing(&mut exif_map, "Model", &raw_metadata.model);
        exif_map.insert(
            "Orientation".into(),
            serde_json::json!(raw_metadata.orientation.exif()),
        );
        insert_metadata_if_missing(
            &mut exif_map,
            "DateTimeOriginal",
            raw_metadata.capture_time.as_deref().unwrap_or_default(),
        );
        insert_metadata_if_missing(
            &mut exif_map,
            "LensModel",
            raw_metadata.lens.model.as_deref().unwrap_or_default(),
        );
        if let Some(iso) = raw_metadata.exposure.iso {
            exif_map
                .entry("ISO")
                .or_insert_with(|| serde_json::json!(iso));
        }
        if let Some(exposure_time) = raw_metadata.exposure.exposure_time {
            exif_map
                .entry("ExposureTime")
                .or_insert_with(|| serde_json::json!(exposure_time));
        }
        if let Some(f_number) = raw_metadata.exposure.f_number {
            exif_map
                .entry("FNumber")
                .or_insert_with(|| serde_json::json!(f_number));
        }
        if let Some(focal_length) = raw_metadata.exposure.focal_length {
            exif_map
                .entry("FocalLength")
                .or_insert_with(|| serde_json::json!(focal_length));
        }
    }

    let x_resolution = exif_map
        .get("XResolution")
        .and_then(|value| value.as_str())
        .and_then(parse_exif_resolution_value);

    let y_resolution = exif_map
        .get("YResolution")
        .and_then(|value| value.as_str())
        .and_then(parse_exif_resolution_value);

    let mut metadata = serde_json::json!({
        "width": image_width,
        "height": image_height,
    });

    if let Some(obj) = metadata.as_object_mut() {
        for (k, v) in exif_map {
            obj.insert(k, v);
        }
    }

    if let Some(dpi_horizontal) = x_resolution {
        metadata["x_resolution"] = serde_json::json!(dpi_horizontal);
    }
    if let Some(dpi_vertical) = y_resolution {
        metadata["y_resolution"] = serde_json::json!(dpi_vertical);
    }

    Ok(metadata)
}

fn insert_metadata_if_missing(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: &str,
) {
    if !value.is_empty() {
        metadata
            .entry(key.to_string())
            .or_insert_with(|| serde_json::Value::String(value.to_string()));
    }
}

/// Returns displayed image dimensions after applying an EXIF orientation value.
pub fn dimensions_for_orientation(
    image_width: u32,
    image_height: u32,
    exif_orientation: u32,
) -> (u32, u32) {
    if matches!(exif_orientation, 5..=8) {
        (image_height, image_width)
    } else {
        (image_width, image_height)
    }
}

fn ensure_jpeg_bytes(raw_bytes: Vec<u8>) -> AppResult<Vec<u8>> {
    if raw_bytes.starts_with(&[0xFF, 0xD8]) {
        // Already a JPEG
        Ok(raw_bytes)
    } else if raw_bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || raw_bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
        || raw_bytes.starts_with(&[0x89, b'P', b'N', b'G'])
    {
        // TIFF or PNG, convert to JPEG
        let decoded_image = image::load_from_memory(&raw_bytes).map_err(|decode_error| {
            AppError::Generic(format!(
                "Failed to decode embedded raw preview image: {}",
                decode_error
            ))
        })?;
        let mut jpeg_data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut jpeg_data);
        decoded_image
            .to_rgb8()
            .write_to(&mut cursor, image::ImageFormat::Jpeg)
            .map_err(|encode_error| {
                AppError::Generic(format!(
                    "Failed to encode raw preview to JPEG: {}",
                    encode_error
                ))
            })?;
        Ok(jpeg_data)
    } else {
        // Fallback: try decoding anyway, if it works convert to JPEG
        if let Ok(decoded_image) = image::load_from_memory(&raw_bytes) {
            let mut jpeg_data = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut jpeg_data);
            if decoded_image
                .to_rgb8()
                .write_to(&mut cursor, image::ImageFormat::Jpeg)
                .is_ok()
            {
                return Ok(jpeg_data);
            }
        }
        Ok(raw_bytes)
    }
}

/// Generates a WebP thumbnail from a RAW photography file using a hybrid strategy.
///
/// Uses embedded previews first, then FlashRaw sensor decoding, JPEG scanning, and FFmpeg.
///
/// # Arguments
///
/// * `path` - Path to the RAW file.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Errors
///
/// * `AppError::Generic` - If all extraction tiers fail.
pub fn generate_raw_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    // Tier 1: FlashRaw embedded preview
    if let Ok(embedded_preview_bytes) = extract_flashraw_preview(path) {
        if let Ok(decoded_preview) = image::load_from_memory(&embedded_preview_bytes) {
            return process_and_encode_webp(decoded_preview, size_hint);
        }
    }

    // Tier 2: Brute-force JPEG scan
    if let Ok((embedded_preview_bytes, _)) = brute_force_extract_jpeg_bytes(path) {
        if let Ok(decoded_preview) = image::load_from_memory(&embedded_preview_bytes) {
            return process_and_encode_webp(decoded_preview, size_hint);
        }
    }

    // Tier 3: Develop sensor data when a RAW file has no usable embedded preview.
    if let Ok(decoded_bytes) = extract_flashraw_full_decode(path) {
        if let Ok(decoded_image) = image::load_from_memory(&decoded_bytes) {
            return process_and_encode_webp(decoded_image, size_hint);
        }
    }

    // Tier 4: FFmpeg fallback (handles formats like x3f, gpr, iiq that lack embedded JPEGs)
    if let Ok(ffmpeg_bytes) = generate_ffmpeg_image_thumbnail(path, size_hint) {
        return Ok(ffmpeg_bytes);
    }

    Err(AppError::Generic(format!(
        "All RAW extraction tiers failed for {:?}",
        path
    )))
}

/// Extracts a high-quality embedded JPEG preview from a RAW file for previewing.
/// It returns the raw bytes directly without WebP re-encoding to preserve quality and speed.
pub fn extract_raw_preview(path: &Path) -> AppResult<Vec<u8>> {
    let mut small_embedded_preview = None;

    // Tier 1: FlashRaw embedded preview
    if let Ok(embedded_preview_bytes) = extract_flashraw_preview(path) {
        if let Ok(jpeg_bytes) = ensure_jpeg_bytes(embedded_preview_bytes) {
            if jpeg_bytes.len() > 50_000 {
                return Ok(jpeg_bytes);
            }
            if image::load_from_memory(&jpeg_bytes).is_ok() {
                small_embedded_preview = Some(jpeg_bytes);
            }
        }
    }

    // Tier 2: Brute-force JPEG scan
    if let Ok((embedded_preview_bytes, pixel_count)) = brute_force_extract_jpeg_bytes(path) {
        if let Ok(jpeg_bytes) = ensure_jpeg_bytes(embedded_preview_bytes) {
            if pixel_count > 500_000 || jpeg_bytes.len() > 50_000 {
                // Validate that the extracted JPEG is actually valid
                if image::load_from_memory(&jpeg_bytes).is_ok() {
                    return Ok(jpeg_bytes);
                }
            } else if image::load_from_memory(&jpeg_bytes).is_ok() {
                small_embedded_preview = Some(jpeg_bytes);
            }
        }
    }

    // Tier 2.5: Full FlashRaw decode (slow but useful for files without large embedded previews).
    if let Ok(decoded_bytes) = extract_flashraw_full_decode(path) {
        return Ok(decoded_bytes);
    }

    // Keep a small camera-rendered preview if the RAW sensor compression is unsupported.
    if let Some(preview_bytes) = small_embedded_preview {
        return Ok(preview_bytes);
    }

    // Tier 3: FFmpeg fallback (extracts full size or max 2048px JPEG)
    if let Ok((ffmpeg_bytes, _)) = generate_ffmpeg_image_preview(path) {
        return Ok(ffmpeg_bytes);
    }

    Err(AppError::Generic(format!(
        "Could not extract preview from RAW file {:?}",
        path
    )))
}

/// Generates a WebP thumbnail for HDR/EXR/DDS files using a multi-tier fallback strategy.
pub fn generate_hdr_exr_dds_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    // Tier 1: Try raster image decoding first (native image-rs)
    if let Ok(thumbnail_bytes) = generate_raster_thumbnail(path, size_hint) {
        return Ok(thumbnail_bytes);
    }
    // Tier 2: Try FFmpeg extraction fallback
    if let Ok(thumbnail_bytes) = generate_ffmpeg_image_thumbnail(path, size_hint) {
        return Ok(thumbnail_bytes);
    }
    // Tier 3: Try brute force JPEG scanner
    if let Ok((image_bytes, _)) = brute_force_extract_jpeg_bytes(path) {
        if let Ok(decoded_image) = image::load_from_memory(&image_bytes) {
            return process_and_encode_webp(decoded_image, size_hint);
        }
    }
    Err(AppError::Generic(format!(
        "Failed to generate thumbnail for HDR/EXR/DDS file: {:?}",
        path
    )))
}

/// Generates a preview for HDR/EXR/DDS files using a multi-tier fallback strategy.
pub fn generate_hdr_exr_dds_preview(path: &Path) -> AppResult<(Vec<u8>, String)> {
    // Tier 1: Try raster preview (native image-rs)
    if let Ok(preview_result) = generate_raster_preview(path) {
        return Ok(preview_result);
    }
    // Tier 2: Try FFmpeg preview fallback
    if let Ok(preview_result) = generate_ffmpeg_image_preview(path) {
        return Ok(preview_result);
    }
    // Tier 3: Try brute force JPEG scanner
    if let Ok((jpeg_bytes, _)) = brute_force_extract_jpeg_bytes(path) {
        return Ok((jpeg_bytes, "image/jpeg".to_string()));
    }
    Err(AppError::Generic(format!(
        "Failed to generate preview for HDR/EXR/DDS file: {:?}",
        path
    )))
}

/// Extracts the largest embedded preview using FlashRaw. JPEG previews are returned
/// byte-for-byte; RGB previews are encoded for the existing image pipeline.
pub(crate) fn extract_flashraw_preview(path: &Path) -> AppResult<Vec<u8>> {
    extract_flashraw_preview_candidates(path)?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Generic("No supported previews found in RAW file".into()))
}

pub(crate) fn extract_flashraw_preview_candidates(path: &Path) -> AppResult<Vec<Vec<u8>>> {
    let raw = flashraw::FlashRaw::open_path(path)
        .map_err(|error| AppError::Generic(format!("FlashRaw open error: {error}")))?;
    let mut previews: Vec<_> = raw
        .previews()
        .iter()
        .filter(|preview| {
            matches!(
                preview.encoding,
                flashraw::PreviewEncoding::Jpeg | flashraw::PreviewEncoding::Rgb8
            )
        })
        .collect();
    previews.sort_by_key(|preview| {
        (
            std::cmp::Reverse(preview.width as u64 * preview.height as u64),
            std::cmp::Reverse(preview.length),
        )
    });

    let mut candidates = Vec::with_capacity(previews.len());
    for preview in previews {
        let preview_orientation =
            resolve_leaf_mos_preview_orientation(path, raw.metadata(), preview);

        let candidate_bytes = if matches!(preview.encoding, flashraw::PreviewEncoding::Jpeg) {
            let Ok(preview_bytes) = raw.preview_bytes(preview) else {
                continue;
            };
            if preview_orientation == flashraw::Orientation::Normal {
                preview_bytes.to_vec()
            } else {
                let Ok(decoded_image) = image::load_from_memory(preview_bytes) else {
                    continue;
                };
                let oriented_image =
                    apply_raw_preview_orientation(decoded_image, preview_orientation);
                let Ok(encoded_image) =
                    encode_dynamic_image_jpeg(oriented_image, "oriented FlashRaw preview")
                else {
                    continue;
                };
                encoded_image
            }
        } else {
            let Ok(decoded) = raw.decode_preview_rgb(preview) else {
                continue;
            };
            let Some(rgb) = image::RgbImage::from_raw(decoded.width, decoded.height, decoded.rgb)
            else {
                continue;
            };
            let oriented_image = apply_raw_preview_orientation(
                image::DynamicImage::ImageRgb8(rgb),
                preview_orientation,
            );
            let Ok(encoded_image) = encode_dynamic_image_jpeg(oriented_image, "FlashRaw preview")
            else {
                continue;
            };
            encoded_image
        };

        if image::load_from_memory(&candidate_bytes).is_ok() {
            candidates.push(candidate_bytes);
        }
    }

    Ok(candidates)
}

fn resolve_leaf_mos_preview_orientation(
    path: &Path,
    metadata: &flashraw::RawMetadata,
    preview: &flashraw::PreviewInfo,
) -> flashraw::Orientation {
    let is_leaf_mos = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mos"));
    if !is_leaf_mos || preview.width == 0 || preview.height == 0 {
        return preview.orientation;
    }

    let file_orientation = metadata.orientation;
    if !matches!(
        file_orientation,
        flashraw::Orientation::Transpose
            | flashraw::Orientation::Rotate90
            | flashraw::Orientation::Transverse
            | flashraw::Orientation::Rotate270
    ) {
        return preview.orientation;
    }

    let preview_ratio = preview.width as f64 / preview.height as f64;
    let raw_ratio = metadata.width as f64 / metadata.height.max(1) as f64;
    let display_ratio = metadata.height as f64 / metadata.width.max(1) as f64;

    if ratios_are_close(preview_ratio, display_ratio) {
        flashraw::Orientation::Normal
    } else if ratios_are_close(preview_ratio, raw_ratio) {
        file_orientation
    } else {
        preview.orientation
    }
}

fn ratios_are_close(first_ratio: f64, second_ratio: f64) -> bool {
    (first_ratio - second_ratio).abs() <= second_ratio.abs().max(0.01) * 0.03
}

fn apply_raw_preview_orientation(
    image: image::DynamicImage,
    orientation: flashraw::Orientation,
) -> image::DynamicImage {
    use flashraw::Orientation;

    match orientation {
        Orientation::Normal => image,
        Orientation::FlipH => image.fliph(),
        Orientation::Rotate180 => image.rotate180(),
        Orientation::FlipV => image.flipv(),
        Orientation::Transpose => image.rotate90().fliph(),
        Orientation::Rotate90 => image.rotate90(),
        Orientation::Transverse => image.rotate270().fliph(),
        Orientation::Rotate270 => image.rotate270(),
    }
}

/// Demosaics sensor pixels through FlashRaw for files without a usable large preview.
fn extract_flashraw_full_decode(path: &Path) -> AppResult<Vec<u8>> {
    let raw = flashraw::FlashRaw::open_path(path)
        .map_err(|error| AppError::Generic(format!("FlashRaw open error: {error}")))?;
    let frame = raw
        .decode(flashraw::DecodeOptions {
            output: flashraw::OutputFormat::Rgba8,
            preview_fallback: false,
            ..Default::default()
        })
        .map_err(|error| AppError::Generic(format!("FlashRaw decode error: {error}")))?;
    let rgba = image::RgbaImage::from_raw(
        frame.width,
        frame.height,
        frame
            .as_u8()
            .ok_or_else(|| AppError::Generic("FlashRaw returned a non-RGBA8 frame".into()))?
            .to_vec(),
    )
    .ok_or_else(|| AppError::Generic("Invalid RGBA8 frame returned by FlashRaw".into()))?;
    encode_dynamic_image_jpeg(image::DynamicImage::ImageRgba8(rgba), "FlashRaw decode")
}

pub(crate) fn encode_dynamic_image_jpeg(
    image: image::DynamicImage,
    context: &str,
) -> AppResult<Vec<u8>> {
    let mut jpeg_buffer = Vec::new();
    image
        .to_rgb8()
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg_buffer),
            image::ImageFormat::Jpeg,
        )
        .map_err(|error| {
            AppError::Generic(format!("Failed to encode {context} as JPEG: {error}"))
        })?;
    Ok(jpeg_buffer)
}

/// Scans the first 8 MB of a file looking for JPEG start-of-image markers and
/// returns the largest valid JPEG image found.
pub fn brute_force_extract_jpeg_bytes(path: &Path) -> AppResult<(Vec<u8>, u32)> {
    let file_handle = std::fs::File::open(path).map_err(AppError::Io)?;
    let memory_map = unsafe {
        memmap2::MmapOptions::new()
            .map(&file_handle)
            .map_err(AppError::Io)?
    };

    // Scan up to 30MB like in V1
    let scan_limit = memory_map.len().min(30 * 1024 * 1024);
    let mut best_bytes: Option<Vec<u8>> = None;
    let mut best_pixel_count = 0u32;
    let mut scan_offset = 0usize;

    while scan_offset < scan_limit.saturating_sub(4) {
        let is_jpeg_marker = memory_map[scan_offset] == 0xFF
            && memory_map[scan_offset + 1] == 0xD8
            && memory_map[scan_offset + 2] == 0xFF;

        if is_jpeg_marker {
            if let Ok(decoded_image) = image::load_from_memory(&memory_map[scan_offset..]) {
                let pixel_count = decoded_image.width() * decoded_image.height();
                if pixel_count > best_pixel_count {
                    best_pixel_count = pixel_count;
                    // Copy the slice correctly by finding EOI, or just storing the slice
                    // Since image::load_from_memory stops at the end of the image, we don't know the exact byte length.
                    // But we can scan forward for EOI (FF D9).
                    let eoi_limit = (scan_offset + 20 * 1024 * 1024).min(memory_map.len());
                    let mut end_offset = scan_offset + 2;
                    let mut found_eoi = false;
                    while end_offset < eoi_limit.saturating_sub(1) {
                        if memory_map[end_offset] == 0xFF && memory_map[end_offset + 1] == 0xD9 {
                            end_offset += 2;
                            found_eoi = true;
                            break;
                        }
                        end_offset += 1;
                    }

                    if found_eoi {
                        best_bytes = Some(memory_map[scan_offset..end_offset].to_vec());
                    } else {
                        // Fallback: just save it as JPEG again
                        let mut jpeg_data = Vec::new();
                        let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                        if decoded_image
                            .to_rgb8()
                            .write_to(&mut cursor, image::ImageFormat::Jpeg)
                            .is_ok()
                        {
                            best_bytes = Some(jpeg_data);
                        }
                    }
                }
                scan_offset += 2048;
                continue;
            }
        }
        scan_offset += 1;
    }

    if let Some(bytes) = best_bytes {
        Ok((bytes, best_pixel_count))
    } else {
        Err(AppError::Generic(
            "Brute-force JPEG scan found no valid images".into(),
        ))
    }
}

// ─── FFmpeg-based (modern formats) ─────────────────────────────────────────

/// Extracts technical metadata from a modern image format (HEIC, AVIF, JXL)
/// using `ffprobe`.
///
/// # Arguments
///
/// * `path` - Path to the image file.
///
/// # Errors
///
/// * `AppError::Generic` - If `ffprobe` is not found or returns an error.
/// * `AppError::Transcoding` - If JSON parsing of `ffprobe` output fails.
pub fn extract_ffmpeg_image_metadata(path: &Path) -> AppResult<serde_json::Value> {
    use crate::processing::transcoding::{resolve_transcoding_tools, run_command_with_timeout};
    use std::process::Command;

    let transcoding_tools = resolve_transcoding_tools::<tauri::Wry>(None)?;

    let mut ffprobe_command = Command::new(transcoding_tools.ffprobe);
    ffprobe_command.args([
        "-v",                    // Log level
        "error",                 // Only show errors
        "-show_format",          // Show format information
        "-show_streams",         // Show stream information
        "-of",                   // Output format
        "json",                  // JSON format
        &path.to_string_lossy(), // Path to the image file
    ]);

    let ffprobe_output = run_command_with_timeout(ffprobe_command, 10)?;

    if !ffprobe_output.status.success() {
        let error_message = String::from_utf8_lossy(&ffprobe_output.stderr);
        return Err(AppError::Transcoding(format!(
            "FFprobe failed: {}",
            error_message
        )));
    }

    let probe_json: serde_json::Value =
        serde_json::from_slice(&ffprobe_output.stdout).map_err(|error| {
            AppError::Transcoding(format!("Failed to parse FFprobe JSON output: {}", error))
        })?;

    let mut technical_metadata = serde_json::Map::new();

    if let Some(streams) = probe_json
        .get("streams")
        .and_then(|streams| streams.as_array())
    {
        if let Some(first_stream) = streams.first() {
            if let Some(stream_width) = first_stream.get("width") {
                technical_metadata.insert("width".into(), stream_width.clone());
            }
            if let Some(stream_height) = first_stream.get("height") {
                technical_metadata.insert("height".into(), stream_height.clone());
            }
            if let Some(codec_name) = first_stream.get("codec_name") {
                technical_metadata.insert("codec".into(), codec_name.clone());
            }
            if let Some(color_space) = first_stream.get("color_space") {
                technical_metadata.insert("color_space".into(), color_space.clone());
            }
            if let Some(color_transfer) = first_stream.get("color_transfer") {
                technical_metadata.insert("color_transfer".into(), color_transfer.clone());
            }
        }
    }

    Ok(serde_json::Value::Object(technical_metadata))
}

/// Generates a WebP thumbnail from a modern image format using FFmpeg.
///
/// # Arguments
///
/// * `path` - Path to the image file.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Errors
///
/// * `AppError::Transcoding` - If FFmpeg fails or times out.
pub fn generate_ffmpeg_image_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    use crate::processing::transcoding::{resolve_transcoding_tools, run_command_with_timeout};
    use std::process::Command;

    let transcoding_tools = resolve_transcoding_tools::<tauri::Wry>(None)?;

    let mut ffmpeg_command = Command::new(transcoding_tools.ffmpeg);
    ffmpeg_command.args([
        "-hide_banner",          // Hides the ffmpeg banner
        "-loglevel",             // Set the log level
        "error",                 // Only show errors
        "-strict",               // Set the strictness level
        "unofficial",            // Set the strictness level to unofficial
        "-i",                    // Input file
        &path.to_string_lossy(), // Path to the image file
        "-vf",                   // Video filters
        &format!(
            "format=yuv420p,scale='min({},iw)':'if(gt(ih,iw),-1,-2)':flags=lanczos", // Format the image to yuv420p and scale it to the hint
            size_hint // Scale the image to the hint
        ),
        "-vframes", // Number of frames to extract
        "1",        // Extract 1 frame
        "-f",       // Output format
        "image2",   // Image2 format
        "-c:v",     // Video codec
        "mjpeg",    // Motion JPEG
        "-",        // Output to stdout
    ]);

    // 60 seconds for heavy images like HEIC/AVIF
    let ffmpeg_output = run_command_with_timeout(ffmpeg_command, 60)?;

    if ffmpeg_output.status.success() {
        Ok(ffmpeg_output.stdout)
    } else {
        let error_message = String::from_utf8_lossy(&ffmpeg_output.stderr);
        Err(AppError::Transcoding(format!(
            "FFmpeg image thumbnail extraction failed: {}",
            error_message
        )))
    }
}

/// Generates a WebP preview from a generic raster image.
///
/// # Arguments
///
/// * `path` - Path to the image file.
///
/// # Errors
///
/// * `AppError::Generic` - If decoding or encoding fails.
pub fn generate_raster_preview(path: &Path) -> AppResult<(Vec<u8>, String)> {
    let decoded_image = open_raster_image(path).map_err(|preview_decode_error| {
        AppError::Generic(format!(
            "Raster preview extraction failed: {}",
            preview_decode_error
        ))
    })?;

    let bytes = process_and_encode_webp(decoded_image, 2048)?;
    Ok((bytes, "image/webp".to_string()))
}

pub fn generate_ffmpeg_image_preview(path: &Path) -> AppResult<(Vec<u8>, String)> {
    use crate::processing::transcoding::{resolve_transcoding_tools, run_command_with_timeout};
    use std::process::Command;

    let transcoding_tools = resolve_transcoding_tools::<tauri::Wry>(None)?;

    let mut ffmpeg_command = Command::new(transcoding_tools.ffmpeg);
    ffmpeg_command.args([
        "-hide_banner",          // Hides the ffmpeg banner
        "-loglevel",             // Log level
        "error",                 // Only show errors
        "-strict",               // Strictness level
        "unofficial",            // Unofficial strictness level
        "-i",                    // Input file
        &path.to_string_lossy(), // Path to the image file
        "-vf",                   // Video filters
        "format=yuv420p,scale='min(2048,iw)':'if(gt(ih,iw),-1,-2)':flags=lanczos", // Format the image to yuv420p and scale it to the hint
        "-vframes", // Number of frames to extract
        "1",        // Extract 1 frame
        "-f",       // Output format
        "image2",   // Image2 format
        "-c:v",     // Video codec
        "mjpeg",    // Motion JPEG
        "-q:v",     // Quality
        "2",        // Quality
        "-",        // Output to stdout
    ]);

    // 60 seconds for heavy images like HEIC/AVIF
    let ffmpeg_output = run_command_with_timeout(ffmpeg_command, 60)?;

    if ffmpeg_output.status.success() {
        Ok((ffmpeg_output.stdout, "image/jpeg".to_string()))
    } else {
        let error_message = String::from_utf8_lossy(&ffmpeg_output.stderr);
        Err(AppError::Transcoding(format!(
            "FFmpeg image preview extraction failed: {}",
            error_message
        )))
    }
}

// ─── EXR ───────────────────────────────────────────────────────────────────

/// Extracts technical metadata from an OpenEXR file using the `image` crate.
///
/// # Arguments
///
/// * `path` - Path to the `.exr` file.
///
/// # Errors
///
/// * `AppError::Generic` - If the EXR file cannot be decoded.
pub fn extract_exr_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let decoded_image = image::open(path)
        .map_err(|error| AppError::Generic(format!("EXR decode error: {}", error)))?;

    Ok(serde_json::json!({
        "width": decoded_image.width(),
        "height": decoded_image.height(),
        "color_mode": format!("{:?}", decoded_image.color()),
        "format": "OpenEXR",
    }))
}
