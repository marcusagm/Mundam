use crate::core::error::AppResult;
use crate::processing::media::extractors::image::{
    brute_force_extract_jpeg_bytes, encode_dynamic_image_jpeg, extract_flashraw_preview_candidates,
    extract_raw_metadata, process_and_encode_webp,
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
            if is_usable_kdc_image(&jpeg_bytes) {
                return Ok(jpeg_bytes);
            }
        }
    }

    // Older Kodak files may only contain a small RGB preview. Prefer it to a
    // sensor decode: some DC120 variants decode successfully but produce black.
    if let Ok(preview_candidates) = extract_flashraw_preview_candidates(path) {
        if let Some(preview_bytes) = preview_candidates
            .into_iter()
            .find(|preview_bytes| is_usable_kdc_image(preview_bytes))
        {
            return Ok(preview_bytes);
        }
    }

    // Attempt macOS native extraction (CoreImage / sips) for old proprietary formats like DC120
    // Only accept if it produces a reasonably sized file (> 50KB) so we don't accidentally
    // use a low-quality sips thumbnail when FlashRaw could do a full decode (like for EasyShare)
    #[cfg(target_os = "macos")]
    if let Ok(sips_bytes) = extract_macos_sips_image(path, 2048) {
        if sips_bytes.len() > 50_000 && is_usable_kdc_image(&sips_bytes) {
            return Ok(sips_bytes);
        }
    }

    // Fallback to FlashRaw (which works for newer KDC like EasyShare, but fails on DC120)
    if let Ok(preview_bytes) =
        crate::processing::media::extractors::image::extract_raw_preview(path)
    {
        if is_usable_kdc_image(&preview_bytes) {
            return Ok(preview_bytes);
        }
    }

    extract_kdc_embedded_rgb_thumbnail(path)
}

/// Generates a WebP thumbnail from a KDC file.
pub fn generate_kdc_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    // Use the exact camera rendering (embedded JPEG) for thumbnails
    if let Ok((jpeg_bytes, pixel_count)) = brute_force_extract_jpeg_bytes(path) {
        if pixel_count > 500_000 || jpeg_bytes.len() > 50_000 {
            if is_usable_kdc_image(&jpeg_bytes) {
                let decoded_image = image::load_from_memory(&jpeg_bytes).map_err(|error| {
                    crate::core::error::AppError::Generic(format!(
                        "Failed to decode Kodak embedded JPEG: {error}"
                    ))
                })?;
                return process_and_encode_webp(decoded_image, size_hint);
            }
        }
    }

    if let Ok(preview_candidates) = extract_flashraw_preview_candidates(path) {
        if let Some(preview_bytes) = preview_candidates
            .into_iter()
            .find(|preview_bytes| is_usable_kdc_image(preview_bytes))
        {
            if let Ok(decoded_image) = image::load_from_memory(&preview_bytes) {
                return process_and_encode_webp(decoded_image, size_hint);
            }
        }
    }

    // Attempt macOS native extraction for thumbnails
    #[cfg(target_os = "macos")]
    if let Ok(sips_bytes) = extract_macos_sips_image(path, size_hint.max(1024)) {
        if is_usable_kdc_image(&sips_bytes) {
            if let Ok(decoded_image) = image::load_from_memory(&sips_bytes) {
                return process_and_encode_webp(decoded_image, size_hint);
            }
        }
    }

    // Use the same quality-first path as the full preview. It attempts full sensor
    // decoding before using the small TIFF thumbnail as a last resort.
    let preview_bytes = extract_kdc_preview(path)?;
    let decoded_image = image::load_from_memory(&preview_bytes).map_err(|error| {
        crate::core::error::AppError::Generic(format!(
            "Failed to decode Kodak preview for thumbnail: {error}"
        ))
    })?;
    process_and_encode_webp(decoded_image, size_hint)
}

fn is_usable_kdc_image(bytes: &[u8]) -> bool {
    let Ok(decoded_image) = image::load_from_memory(bytes) else {
        return false;
    };

    let rgb = decoded_image.to_rgb8();
    let mut maximum_channel = 0u8;
    let mut channel_sum = 0u64;
    let mut channel_count = 0u64;

    for pixel in rgb.pixels() {
        for channel in pixel.0 {
            maximum_channel = maximum_channel.max(channel);
            channel_sum += u64::from(channel);
            channel_count += 1;
        }
    }

    maximum_channel > 8 && channel_sum / channel_count.max(1) > 1
}

fn extract_kdc_embedded_rgb_thumbnail(path: &Path) -> AppResult<Vec<u8>> {
    use crate::core::error::AppError;

    let data = std::fs::read(path).map_err(AppError::Io)?;
    let byte_order = match data.get(0..2) {
        Some(b"II") => TiffByteOrder::LittleEndian,
        Some(b"MM") => TiffByteOrder::BigEndian,
        _ => return Err(AppError::Generic("Invalid KDC TIFF byte order".into())),
    };
    if byte_order.read_u16(&data, 2)? != 42 {
        return Err(AppError::Generic("Invalid KDC TIFF header".into()));
    }

    let mut ifd_offset = byte_order.read_u32(&data, 4)? as usize;
    for _ in 0..16 {
        if ifd_offset == 0 {
            break;
        }
        let (entries, next_ifd) = read_tiff_ifd(&data, ifd_offset, byte_order)?;
        if let Some(image) = decode_kdc_rgb_ifd(&data, &entries)? {
            return encode_dynamic_image_jpeg(image, "Kodak embedded RGB thumbnail");
        }
        ifd_offset = next_ifd;
    }

    Err(AppError::Generic(
        "No uncompressed RGB thumbnail found in KDC TIFF directories".into(),
    ))
}

#[derive(Clone, Copy)]
enum TiffByteOrder {
    LittleEndian,
    BigEndian,
}

impl TiffByteOrder {
    fn read_u16(self, data: &[u8], offset: usize) -> AppResult<u16> {
        let bytes = data.get(offset..offset + 2).ok_or_else(|| {
            crate::core::error::AppError::Generic("Truncated KDC TIFF value".into())
        })?;
        let bytes = [bytes[0], bytes[1]];
        Ok(match self {
            Self::LittleEndian => u16::from_le_bytes(bytes),
            Self::BigEndian => u16::from_be_bytes(bytes),
        })
    }

    fn read_u32(self, data: &[u8], offset: usize) -> AppResult<u32> {
        let bytes = data.get(offset..offset + 4).ok_or_else(|| {
            crate::core::error::AppError::Generic("Truncated KDC TIFF value".into())
        })?;
        let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
        Ok(match self {
            Self::LittleEndian => u32::from_le_bytes(bytes),
            Self::BigEndian => u32::from_be_bytes(bytes),
        })
    }
}

fn read_tiff_ifd(
    data: &[u8],
    ifd_offset: usize,
    byte_order: TiffByteOrder,
) -> AppResult<(std::collections::HashMap<u16, Vec<u32>>, usize)> {
    use crate::core::error::AppError;

    let entry_count = byte_order.read_u16(data, ifd_offset)? as usize;
    if entry_count > 4096 {
        return Err(AppError::Generic(
            "KDC TIFF IFD contains too many entries".into(),
        ));
    }
    let entries_offset = ifd_offset + 2;
    let next_offset_position = entries_offset + entry_count * 12;
    let next_ifd = byte_order.read_u32(data, next_offset_position)? as usize;
    let mut entries = std::collections::HashMap::new();

    for entry_index in 0..entry_count {
        let entry_offset = entries_offset + entry_index * 12;
        let tag = byte_order.read_u16(data, entry_offset)?;
        let field_type = byte_order.read_u16(data, entry_offset + 2)?;
        let value_count = byte_order.read_u32(data, entry_offset + 4)? as usize;
        if value_count == 0 || value_count > 4096 || !matches!(field_type, 1 | 3 | 4) {
            continue;
        }

        let value_size = match field_type {
            1 => 1,
            3 => 2,
            4 => 4,
            _ => unreachable!(),
        };
        let values_size = value_size * value_count;
        let value_offset = if values_size <= 4 {
            entry_offset + 8
        } else {
            byte_order.read_u32(data, entry_offset + 8)? as usize
        };
        if value_offset
            .checked_add(values_size)
            .is_none_or(|end| end > data.len())
        {
            continue;
        }

        let mut values = Vec::with_capacity(value_count);
        for value_index in 0..value_count {
            let offset = value_offset + value_index * value_size;
            let value = match field_type {
                1 => u32::from(data[offset]),
                3 => u32::from(byte_order.read_u16(data, offset)?),
                4 => byte_order.read_u32(data, offset)?,
                _ => unreachable!(),
            };
            values.push(value);
        }
        entries.insert(tag, values);
    }

    Ok((entries, next_ifd))
}

fn decode_kdc_rgb_ifd(
    data: &[u8],
    entries: &std::collections::HashMap<u16, Vec<u32>>,
) -> AppResult<Option<image::DynamicImage>> {
    let get = |tag: u16| entries.get(&tag).and_then(|values| values.first()).copied();
    let (Some(width), Some(height)) = (get(0x0100), get(0x0101)) else {
        return Ok(None);
    };
    if get(0x0103) != Some(1)
        || get(0x0106) != Some(2)
        || get(0x0115) != Some(3)
        || get(0x011c).unwrap_or(1) != 1
        || entries
            .get(&0x0102)
            .is_none_or(|bits| bits.len() < 3 || bits[..3] != [8, 8, 8])
        || width == 0
        || height == 0
        || width > 1024
        || height > 1024
    {
        return Ok(None);
    }

    let Some(strip_offsets) = entries.get(&0x0111) else {
        return Ok(None);
    };
    let Some(strip_lengths) = entries.get(&0x0117) else {
        return Ok(None);
    };
    if strip_offsets.len() != strip_lengths.len() {
        return Ok(None);
    }

    let expected_byte_count = width as usize * height as usize * 3;
    let mut rgb_bytes = Vec::with_capacity(expected_byte_count);
    for (&offset, &length) in strip_offsets.iter().zip(strip_lengths) {
        let start = offset as usize;
        let end = start.checked_add(length as usize).ok_or_else(|| {
            crate::core::error::AppError::Generic("Invalid KDC thumbnail range".into())
        })?;
        let strip = data.get(start..end).ok_or_else(|| {
            crate::core::error::AppError::Generic("Truncated KDC thumbnail pixels".into())
        })?;
        let remaining = expected_byte_count.saturating_sub(rgb_bytes.len());
        rgb_bytes.extend_from_slice(&strip[..strip.len().min(remaining)]);
    }
    if rgb_bytes.len() < expected_byte_count {
        return Ok(None);
    }

    let rgb = image::RgbImage::from_raw(width, height, rgb_bytes)
        .ok_or_else(|| crate::core::error::AppError::Generic("Invalid KDC RGB thumbnail".into()))?;
    let thumbnail = image::DynamicImage::ImageRgb8(rgb);
    let orientation = get(0x0112).unwrap_or(1);
    let image = match orientation {
        2 => thumbnail.fliph(),
        3 => thumbnail.rotate180(),
        4 => thumbnail.flipv(),
        5 => thumbnail.rotate90().fliph(),
        6 => thumbnail.rotate90(),
        7 => thumbnail.rotate270().fliph(),
        8 => thumbnail.rotate270(),
        _ => thumbnail,
    };
    Ok(Some(image))
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
