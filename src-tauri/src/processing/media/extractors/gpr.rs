//! Dedicated extractor for GoPro RAW (.gpr) files.
//!
//! GPR files are based on Adobe DNG but use VC-5 (CineForm wavelet) compression
//! for the sensor data. This makes them incompatible with most standard RAW
//! decoders:
//!
//! - **rsraw/LibRaw**: Returns `FileUnsupported` (bundled LibRaw lacks VC-5 codec)
//! - **FFmpeg**: Fails with "Unknown compression method 9" (no VC-5 support for stills)
//! - **quickraw**: Silent failure
//! - **Embedded JPEG**: Most GPR files contain **no** embedded JPEG previews at all
//!
//! # Strategy
//!
//! **Metadata**: Parse the TIFF/DNG container directly to read IFD0 image dimensions
//! (tags 0x0100/0x0101), then supplement with EXIF camera data via `rexif`.
//!
//! **Thumbnails & Previews**: Use macOS `sips` (Scriptable Image Processing System),
//! which leverages Apple's CoreImage framework and natively recognizes GPR as DNG,
//! successfully decoding the VC-5 compressed sensor data.
//!
//! # Fallback
//!
//! If `sips` is unavailable (non-macOS), falls back to the generic RAW extraction
//! pipeline (quickraw → LibRaw → brute-force JPEG scan → FFmpeg).

use crate::core::error::{AppError, AppResult};
#[cfg(not(target_os = "windows"))]
use crate::processing::media::extractors::image;
use std::path::Path;

// ─── TIFF IFD0 dimension reader ────────────────────────────────────────────

/// Reads a `u16` from `data` at `offset` using the specified endianness.
fn read_tiff_u16(data: &[u8], offset: usize, is_little_endian: bool) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    if is_little_endian {
        Some(u16::from_le_bytes(bytes))
    } else {
        Some(u16::from_be_bytes(bytes))
    }
}

/// Reads a `u32` from `data` at `offset` using the specified endianness.
fn read_tiff_u32(data: &[u8], offset: usize, is_little_endian: bool) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    if is_little_endian {
        Some(u32::from_le_bytes(bytes))
    } else {
        Some(u32::from_be_bytes(bytes))
    }
}

/// Extracts image dimensions from the IFD0 of a TIFF/DNG-based GPR file.
///
/// Reads the TIFF header to determine endianness, then iterates through the
/// first IFD entries looking for `ImageWidth` (0x0100) and `ImageLength` (0x0101).
///
/// # Arguments
///
/// * `file_bytes` - The raw bytes of the GPR file.
///
/// # Returns
///
/// A tuple `(width, height)` if both dimensions are found, or `None`.
fn extract_tiff_ifd0_dimensions(file_bytes: &[u8]) -> Option<(u32, u32)> {
    if file_bytes.len() < 8 {
        return None;
    }

    let is_little_endian = match &file_bytes[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };

    let magic_number = read_tiff_u16(file_bytes, 2, is_little_endian)?;
    if magic_number != 42 {
        return None;
    }

    let first_ifd_offset = read_tiff_u32(file_bytes, 4, is_little_endian)? as usize;
    let entry_count = read_tiff_u16(file_bytes, first_ifd_offset, is_little_endian)?;

    let mut image_width: Option<u32> = None;
    let mut image_height: Option<u32> = None;

    for entry_index in 0..entry_count {
        let entry_offset = first_ifd_offset + 2 + entry_index as usize * 12;
        let tag_id = read_tiff_u16(file_bytes, entry_offset, is_little_endian)?;
        let field_type = read_tiff_u16(file_bytes, entry_offset + 2, is_little_endian)?;

        match tag_id {
            // ImageWidth (0x0100)
            0x0100 => {
                image_width = if field_type == 3 {
                    // SHORT
                    read_tiff_u16(file_bytes, entry_offset + 8, is_little_endian)
                        .map(|value| value as u32)
                } else {
                    // LONG
                    read_tiff_u32(file_bytes, entry_offset + 8, is_little_endian)
                };
            }
            // ImageLength (0x0101)
            0x0101 => {
                image_height = if field_type == 3 {
                    read_tiff_u16(file_bytes, entry_offset + 8, is_little_endian)
                        .map(|value| value as u32)
                } else {
                    read_tiff_u32(file_bytes, entry_offset + 8, is_little_endian)
                };
            }
            _ => {}
        }

        // Early exit once both dimensions are found
        if image_width.is_some() && image_height.is_some() {
            break;
        }
    }

    match (image_width, image_height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
        _ => None,
    }
}

// ─── Native GoPro SDK (C FFI) ──────────────────────────────────────────────
// The GPR SDK is not compiled on Windows (pthread conflicts with MinGW).
// All FFI symbols and the native decode path are guarded accordingly.

#[cfg(not(target_os = "windows"))]
#[repr(C)]
struct gpr_allocator {
    alloc: Option<extern "C" fn(usize) -> *mut std::ffi::c_void>,
    free: Option<extern "C" fn(*mut std::ffi::c_void)>,
}

#[cfg(not(target_os = "windows"))]
#[repr(C)]
struct gpr_buffer {
    buffer: *mut std::ffi::c_void,
    size: usize,
}

#[cfg(not(target_os = "windows"))]
#[repr(C)]
struct gpr_rgb_buffer {
    buffer: *mut std::ffi::c_void,
    size: usize,
    width: usize,
    height: usize,
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
#[repr(C)]
enum GPR_RGB_RESOLUTION {
    SIXTEENTH = 1,
    EIGHTH = 2,
    QUARTER = 3,
    HALF = 4,
    FULL = 5,
}

#[cfg(not(target_os = "windows"))]
extern "C" {
    fn gpr_convert_gpr_to_rgb(
        allocator: *const gpr_allocator,
        rgb_resolution: GPR_RGB_RESOLUTION,
        rgb_bits: std::os::raw::c_int,
        inp_gpr_buffer: *mut gpr_buffer,
        out_rgb_buffer: *mut gpr_rgb_buffer,
    ) -> bool;
}

#[cfg(not(target_os = "windows"))]
extern "C" fn gpr_malloc(size: usize) -> *mut std::ffi::c_void {
    unsafe { libc::malloc(size) }
}

#[cfg(not(target_os = "windows"))]
extern "C" fn gpr_free_func(ptr: *mut std::ffi::c_void) {
    unsafe { libc::free(ptr) }
}

/// Decodes VC-5 payload natively using the GoPro GPR SDK.
/// Returns an RGB8 image buffer.
#[cfg(not(target_os = "windows"))]
fn decode_gpr_natively(
    gpr_data: &[u8],
    resolution: GPR_RGB_RESOLUTION,
) -> AppResult<::image::RgbImage> {
    let allocator = gpr_allocator {
        alloc: Some(gpr_malloc),
        free: Some(gpr_free_func),
    };

    let mut inp_buffer = gpr_buffer {
        buffer: gpr_data.as_ptr() as *mut std::ffi::c_void,
        size: gpr_data.len(),
    };

    let mut out_buffer = gpr_rgb_buffer {
        buffer: std::ptr::null_mut(),
        size: 0,
        width: 0,
        height: 0,
    };

    let success = unsafe {
        gpr_convert_gpr_to_rgb(
            &allocator,
            resolution,
            8, // 8 bits per channel (RGB24)
            &mut inp_buffer,
            &mut out_buffer,
        )
    };

    if !success || out_buffer.buffer.is_null() {
        if !out_buffer.buffer.is_null() {
            unsafe { libc::free(out_buffer.buffer) };
        }
        return Err(AppError::Generic(
            "GoPro SDK failed to decode VC-5 payload".to_string(),
        ));
    }

    let pixels_len = (out_buffer.width * out_buffer.height * 3) as usize;

    if out_buffer.size < pixels_len {
        unsafe { libc::free(out_buffer.buffer) };
        return Err(AppError::Generic(
            "GoPro SDK returned truncated RGB buffer".to_string(),
        ));
    }

    let slice = unsafe { std::slice::from_raw_parts(out_buffer.buffer as *const u8, pixels_len) };

    let img = ::image::RgbImage::from_raw(
        out_buffer.width as u32,
        out_buffer.height as u32,
        slice.to_vec(),
    )
    .ok_or_else(|| {
        unsafe { libc::free(out_buffer.buffer) };
        AppError::Generic("Failed to construct RGB image from decoded buffer".to_string())
    })?;

    unsafe { libc::free(out_buffer.buffer) };

    Ok(img)
}

// ─── Public API ────────────────────────────────────────────────────────────

/// Extracts technical metadata from a GoPro RAW (.gpr) file.
///
/// Uses a two-tier approach:
/// 1. Parse the TIFF/DNG container directly for image dimensions from IFD0
/// 2. Extract camera EXIF data (make, model, exposure, GPS, etc.) via `rexif`
///
/// This avoids relying on LibRaw, which doesn't support the VC-5 codec.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
///
/// # Returns
///
/// A JSON object containing `width`, `height`, and any available EXIF fields.
pub fn extract_gpr_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let file_bytes = std::fs::read(path).map_err(AppError::Io)?;

    let (image_width, image_height) = extract_tiff_ifd0_dimensions(&file_bytes).unwrap_or((0, 0));

    // Extract EXIF data via rexif (works well on TIFF-based containers)
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

    let mut metadata = serde_json::json!({
        "width": image_width,
        "height": image_height,
    });

    if let Some(metadata_object) = metadata.as_object_mut() {
        for (key, value) in exif_map {
            metadata_object.insert(key, value);
        }
    }

    Ok(metadata)
}

/// Generates a WebP thumbnail from a GoPro RAW (.gpr) file.
///
/// Uses the native GoPro SDK via C FFI to decode the low-resolution
/// uncompressed wavelets directly, bypassing LibRaw and external tools entirely.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
/// * `size_hint` - Maximum dimension (width or height) in pixels.
///
/// # Returns
///
/// WebP-encoded thumbnail bytes.
#[cfg(not(target_os = "windows"))]
pub fn generate_gpr_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    tracing::debug!("GPR thumbnail: using native SDK pipeline for {:?}", path);
    let file_bytes = std::fs::read(path).map_err(AppError::Io)?;

    // Use SIXTEENTH resolution which is uncompressed in VC-5 and very fast (~6ms)
    let decoded_image = decode_gpr_natively(&file_bytes, GPR_RGB_RESOLUTION::SIXTEENTH)?;
    let dynamic_image = ::image::DynamicImage::ImageRgb8(decoded_image);

    image::process_and_encode_webp(dynamic_image, size_hint)
}

/// Windows stub — GPR SDK not available on Windows (DNG SDK pthread conflict with MinGW).
#[cfg(target_os = "windows")]
pub fn generate_gpr_thumbnail(_path: &Path, _size_hint: u32) -> AppResult<Vec<u8>> {
    Err(AppError::Generic(
        "GPR/GoPro RAW thumbnail is not supported on Windows builds (GPR SDK requires non-MinGW environment)".to_string()
    ))
}

/// Extracts a high-quality JPEG preview from a GoPro RAW (.gpr) file.
///
/// Uses the native GoPro SDK via C FFI to decode the half-resolution
/// VC-5 wavelets.
///
/// # Arguments
///
/// * `path` - Path to the GPR file.
///
/// # Returns
///
/// JPEG bytes suitable for display.
#[cfg(not(target_os = "windows"))]
pub fn extract_gpr_preview(path: &Path) -> AppResult<Vec<u8>> {
    tracing::debug!("GPR preview: using native SDK pipeline for {:?}", path);
    let file_bytes = std::fs::read(path).map_err(AppError::Io)?;

    // Use HALF resolution for previews (good balance of quality and speed)
    let decoded_image = decode_gpr_natively(&file_bytes, GPR_RGB_RESOLUTION::HALF)?;

    let mut jpeg_bytes = Vec::new();
    let mut encoder = ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 92);
    encoder
        .encode_image(&decoded_image)
        .map_err(|e| AppError::Generic(format!("Failed to encode GPR preview as JPEG: {}", e)))?;

    Ok(jpeg_bytes)
}

/// Windows stub — GPR SDK not available on Windows.
#[cfg(target_os = "windows")]
pub fn extract_gpr_preview(_path: &Path) -> AppResult<Vec<u8>> {
    Err(AppError::Generic(
        "GPR/GoPro RAW preview is not supported on Windows builds (GPR SDK requires non-MinGW environment)".to_string()
    ))
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_tiff_ifd0_dimension_extraction() {
        let gpr_directory = Path::new(
            "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/gpr",
        );
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
            let file_bytes = std::fs::read(&file_path).unwrap();
            let dimensions = extract_tiff_ifd0_dimensions(&file_bytes);
            assert!(
                dimensions.is_some(),
                "Failed to extract dimensions from {}",
                filename
            );
            let (width, height) = dimensions.unwrap();
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
        let sample_path = Path::new(
            "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/gpr/GOPR0002.GPR",
        );
        if !sample_path.exists() {
            return;
        }

        let metadata = extract_gpr_metadata(sample_path).unwrap();
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
        let gpr_directory = Path::new(
            "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/gpr",
        );
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
        let sample_path = Path::new(
            "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/gpr/GOPR0002.GPR",
        );
        if !sample_path.exists() {
            return;
        }

        let preview_result = extract_gpr_preview(sample_path);
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
