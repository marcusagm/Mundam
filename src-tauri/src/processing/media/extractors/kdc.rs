use crate::core::error::AppResult;
use crate::processing::media::extractors::image::{
    brute_force_extract_jpeg_bytes, extract_raw_metadata, process_and_encode_webp,
};
use std::path::Path;

/// Extracts technical metadata from a KDC file.
pub fn extract_kdc_metadata(path: &Path) -> AppResult<serde_json::Value> {
    extract_raw_metadata(path)
}

/// Extracts a high-quality embedded JPEG preview from a KDC file for previewing.
pub fn extract_kdc_preview(path: &Path) -> AppResult<Vec<u8>> {
    // For Kodak KDC files, FlashRaw demosaicing often results in different/flat colors
    // compared to the legendary "Kodak Color Science" applied in camera.
    // The embedded JPEG contains the exact camera rendering.

    if let Ok((jpeg_bytes, pixel_count)) = brute_force_extract_jpeg_bytes(path) {
        if pixel_count > 500_000 || jpeg_bytes.len() > 50_000 {
            // Validate that the extracted JPEG is actually valid (not just random bytes matching markers)
            if image::load_from_memory(&jpeg_bytes).is_ok() {
                return Ok(jpeg_bytes);
            }
        }
    }

    // Attempt macOS native extraction (CoreImage / sips) for old proprietary formats like DC120
    // Only accept if it produces a reasonably sized file (> 50KB) so we don't accidentally
    // use a low-quality sips thumbnail when FlashRaw could do a full decode (like for EasyShare)
    #[cfg(target_os = "macos")]
    if let Ok(sips_bytes) = extract_macos_sips_image(path, 2048) {
        if sips_bytes.len() > 50_000 {
            return Ok(sips_bytes);
        }
    }

    // Fallback to FlashRaw (which works for newer KDC like EasyShare, but fails on DC120)
    crate::processing::media::extractors::image::extract_raw_preview(path)
}

/// Generates a WebP thumbnail from a KDC file.
pub fn generate_kdc_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    // Use the exact camera rendering (embedded JPEG) for thumbnails
    if let Ok((jpeg_bytes, pixel_count)) = brute_force_extract_jpeg_bytes(path) {
        if pixel_count > 500_000 || jpeg_bytes.len() > 50_000 {
            if let Ok(decoded_image) = image::load_from_memory(&jpeg_bytes) {
                return process_and_encode_webp(decoded_image, size_hint);
            }
        }
    }

    // Attempt macOS native extraction for thumbnails
    #[cfg(target_os = "macos")]
    if let Ok(sips_bytes) = extract_macos_sips_image(path, size_hint.max(1024)) {
        if let Ok(decoded_image) = image::load_from_memory(&sips_bytes) {
            return process_and_encode_webp(decoded_image, size_hint);
        }
    }

    // Use the same quality-first path as the full preview. It attempts full sensor
    // decoding before falling back to a small embedded thumbnail.
    let preview_bytes = crate::processing::media::extractors::image::extract_raw_preview(path)?;
    let decoded_image = image::load_from_memory(&preview_bytes).map_err(|error| {
        crate::core::error::AppError::Generic(format!(
            "Failed to decode Kodak preview for thumbnail: {error}"
        ))
    })?;
    process_and_encode_webp(decoded_image, size_hint)
}

#[cfg(target_os = "macos")]
fn extract_macos_sips_image(path: &Path, max_dimension: u32) -> AppResult<Vec<u8>> {
    use crate::core::error::AppError;
    use std::process::Command;
    use uuid::Uuid;

    let temp_output_path = std::env::temp_dir().join(format!("mundam_kdc_{}.jpg", Uuid::new_v4()));

    let output = Command::new("sips")
        .args([
            "-s",
            "format",
            "jpeg",
            "-Z",
            &max_dimension.to_string(),
            &path.to_string_lossy(),
            "--out",
            &temp_output_path.to_string_lossy(),
        ])
        .output()
        .map_err(AppError::Io)?;

    if !output.status.success() {
        // Cleanup if possible
        let _ = std::fs::remove_file(&temp_output_path);
        return Err(AppError::Generic(format!(
            "sips failed to extract preview: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let bytes = std::fs::read(&temp_output_path).map_err(AppError::Io)?;
    // Cleanup
    let _ = std::fs::remove_file(&temp_output_path);
    Ok(bytes)
}
