//! GIMP XCF (.xcf) preview and metadata extractor.
//!
//! This module provides functionality to extract thumbnails and dimensions from GIMP XCF files.
//! It supports versions from v001 to v013, handling both 32-bit and 64-bit offsets.
//!
//! Ported and enhanced from the V1 backend with full variable naming compliance
//! and improved extraction logic.

use byteorder::{BigEndian, ReadBytesExt};
use image::{ExtendedColorType, ImageEncoder};
use std::cmp;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

/// Granular error types for XCF parsing to ensure parity and debuggability.
#[derive(Debug, Error)]
pub enum XcfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid XCF format: missing magic signature")]
    InvalidFormat,
    #[error("Unsupported XCF version: {0}")]
    UnsupportedVersion(String),
    #[error("No layers found in XCF file")]
    NoLayers,
    #[error("Failed to parse UTF-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Generic extraction error: {0}")]
    Generic(String),
}

/// Metadata about a single layer in the XCF file.
struct LayerInfo {
    pointer: u64,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
}

/// Extracts canvas dimensions from an XCF file header.
///
/// # Arguments
/// * `path` - The path to the .xcf file.
///
/// # Errors
/// Returns `XcfError::Io` if file cannot be read, or `XcfError::InvalidFormat` if header is invalid.
pub fn extract_xcf_metadata(path: &Path) -> Result<serde_json::Value, XcfError> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    // 1. Validate Header Magic
    let mut magic = [0u8; 9];
    reader.read_exact(&mut magic)?;
    if &magic != b"gimp xcf " {
        return Err(XcfError::InvalidFormat);
    }

    // 2. Read Version (vXXX)
    let mut version_bytes = [0u8; 4];
    reader.read_exact(&mut version_bytes)?;
    let version_str = std::str::from_utf8(&version_bytes).unwrap_or("file");
    let version = if version_str == "file" {
        0
    } else if version_bytes[0] == b'v' {
        version_str[1..].parse::<u16>().unwrap_or(0)
    } else {
        0
    };

    // 3. Skip Null Terminator
    let mut null_byte = [0u8; 1];
    reader.read_exact(&mut null_byte)?;

    // 4. Read Canvas Dimensions (Big Endian)
    let canvas_width = reader.read_u32::<BigEndian>()?;
    let canvas_height = reader.read_u32::<BigEndian>()?;
    let _base_type = reader.read_u32::<BigEndian>()?;

    if version >= 4 {
        let _precision = reader.read_u32::<BigEndian>()?;
    }

    let mut dpi_x = 72.0;
    let mut dpi_y = 72.0;

    // 5. Read Properties for Resolution
    loop {
        let property_type = reader.read_u32::<BigEndian>().unwrap_or(0);
        if property_type == 0 {
            break;
        }
        let property_length = reader.read_u32::<BigEndian>().unwrap_or(0);

        let start_pos = reader.stream_position().unwrap_or(0);
        if property_type == 19 && property_length == 8 {
            // PROP_RESOLUTION
            if let Ok(x_res) = reader.read_f32::<BigEndian>() {
                if let Ok(y_res) = reader.read_f32::<BigEndian>() {
                    dpi_x = x_res;
                    dpi_y = y_res;
                }
            }
        }

        // Seek to next property
        let _ = reader.seek(SeekFrom::Start(start_pos + property_length as u64));
    }

    let mut technical = serde_json::json!({
        "container": "GIMP XCF",
        "metadata_support": "Full"
    });

    technical["width"] = serde_json::json!(canvas_width);
    technical["height"] = serde_json::json!(canvas_height);
    technical["dpi"] = serde_json::json!(dpi_x as u32);
    technical["dpi_y"] = serde_json::json!(dpi_y as u32);
    technical["version"] = serde_json::json!(version);

    Ok(serde_json::json!({
        "technical": technical,
        "semantic": {}
    }))
}

/// Main entry point for extracting a preview/thumbnail from an XCF file.
///
/// It attempts to use layer compositing to generate a high-fidelity preview.
///
/// # Arguments
/// * `path` - The path to the .xcf file.
///
/// # Returns
/// A tuple containing the PNG bytes and the MIME type "image/png".
pub fn extract_xcf_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    // 1. Validate Header
    let mut magic = [0u8; 9];
    reader.read_exact(&mut magic)?;
    if &magic != b"gimp xcf " {
        return Err(XcfError::InvalidFormat.into());
    }

    // 2. Read Version
    let mut version_bytes = [0u8; 4];
    reader.read_exact(&mut version_bytes)?;
    let version_str = std::str::from_utf8(&version_bytes)?;
    let version = if version_str == "file" {
        0
    } else if version_bytes[0] == b'v' {
        version_str[1..].parse::<u16>().unwrap_or(0)
    } else {
        0
    };

    // 3. Metadata Header
    reader.read_exact(&mut [0u8])?; // Null terminator
    let canvas_width = reader.read_u32::<BigEndian>()?;
    let canvas_height = reader.read_u32::<BigEndian>()?;
    let _base_type = reader.read_u32::<BigEndian>()?;

    // Version 4+ has precision field
    if version >= 4 {
        let _precision = reader.read_u32::<BigEndian>()?;
    }

    // 4. Handle Global Properties (Search for Thumbnail Property if possible)
    let embedded_thumbnail = find_embedded_thumbnail(&mut reader)?;
    if let Some(thumbnail_bytes) = embedded_thumbnail {
        return Ok((thumbnail_bytes, "image/png".to_string()));
    }

    // 5. Determine Pointer Size (v11+ uses 8 bytes)
    let bytes_per_offset = if version >= 11 { 8 } else { 4 };
    let mut layer_pointers = Vec::new();

    // 6. Collect Layer Pointers
    loop {
        let pointer = if bytes_per_offset == 8 {
            reader.read_u64::<BigEndian>()?
        } else {
            reader.read_u32::<BigEndian>()? as u64
        };
        if pointer == 0 {
            break;
        }
        layer_pointers.push(pointer);
    }

    if layer_pointers.is_empty() {
        return Err(XcfError::NoLayers.into());
    }

    // 7. Inspect Layers for Visibility and Composition
    let mut visible_layers = Vec::new();
    for &pointer in &layer_pointers {
        reader.seek(SeekFrom::Start(pointer))?;
        let width = reader.read_u32::<BigEndian>()?;
        let height = reader.read_u32::<BigEndian>()?;
        let _layer_type = reader.read_u32::<BigEndian>()?;
        let _layer_name = read_gimp_string(&mut reader)?;

        let mut is_visible = true;
        let mut offset_x = 0i32;
        let mut offset_y = 0i32;

        // Layer Properties
        loop {
            let property_type = reader.read_u32::<BigEndian>()?;
            let property_length = reader.read_u32::<BigEndian>()?;
            if property_type == 0 {
                break;
            }
            match property_type {
                8 => {
                    // PROP_VISIBLE
                    is_visible = reader.read_u32::<BigEndian>()? != 0;
                }
                15 => {
                    // PROP_OFFSETS
                    offset_x = reader.read_i32::<BigEndian>()?;
                    offset_y = reader.read_i32::<BigEndian>()?;
                }
                _ => {
                    reader.seek(SeekFrom::Current(property_length as i64))?;
                }
            }
        }

        if is_visible {
            let hierarchy_pointer = if bytes_per_offset == 8 {
                reader.read_u64::<BigEndian>()?
            } else {
                reader.read_u32::<BigEndian>()? as u64
            };

            visible_layers.push(LayerInfo {
                pointer: hierarchy_pointer,
                width,
                height,
                offset_x,
                offset_y,
            });
        }
    }

    // 8. Create Canvas Buffer (Transparent RGBA)
    let mut canvas_data = vec![0u8; (canvas_width * canvas_height * 4) as usize];

    // 9. Composite Layers from Bottom to Top
    visible_layers.reverse();

    for layer in visible_layers {
        if layer.pointer == 0 {
            continue;
        }
        reader.seek(SeekFrom::Start(layer.pointer))?;
        let _hierarchy_width = reader.read_u32::<BigEndian>()?;
        let _hierarchy_height = reader.read_u32::<BigEndian>()?;
        let bytes_per_pixel = reader.read_u32::<BigEndian>()?;

        // We only support 3 (RGB) or 4 (RGBA) bytes per pixel for this extractor
        if bytes_per_pixel != 3 && bytes_per_pixel != 4 {
            continue;
        }

        let level_pointer = if bytes_per_offset == 8 {
            reader.read_u64::<BigEndian>()?
        } else {
            reader.read_u32::<BigEndian>()? as u64
        };
        if level_pointer == 0 {
            continue;
        }

        reader.seek(SeekFrom::Start(level_pointer))?;
        let _level_width = reader.read_u32::<BigEndian>()?;
        let _level_height = reader.read_u32::<BigEndian>()?;

        let tiles_x = layer.width.div_ceil(64);
        let tiles_y = layer.height.div_ceil(64);

        for tile_y in 0..tiles_y {
            for tile_x in 0..tiles_x {
                // Seek to the pointer for this specific tile
                let tile_pointer_offset = level_pointer
                    + 8
                    + ((tile_y * tiles_x + tile_x) * bytes_per_offset as u32) as u64;
                reader.seek(SeekFrom::Start(tile_pointer_offset))?;

                let tile_pointer = if bytes_per_offset == 8 {
                    reader.read_u64::<BigEndian>()?
                } else {
                    reader.read_u32::<BigEndian>()? as u64
                };

                if tile_pointer == 0 {
                    continue;
                }

                let next_pointer_position = reader.stream_position()?;
                reader.seek(SeekFrom::Start(tile_pointer))?;

                decode_and_composite_tile(
                    &mut reader,
                    &mut canvas_data,
                    tile_x,
                    tile_y,
                    layer.width,
                    layer.height,
                    canvas_width,
                    canvas_height,
                    layer.offset_x,
                    layer.offset_y,
                    bytes_per_pixel,
                )?;

                reader.seek(SeekFrom::Start(next_pointer_position))?;
            }
        }
    }

    // 10. Encode Result as PNG
    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_bytes)).write_image(
        &canvas_data,
        canvas_width,
        canvas_height,
        ExtendedColorType::Rgba8,
    )?;

    Ok((png_bytes, "image/png".to_string()))
}

/// Attempts to find and extract an embedded thumbnail from the global property list.
///
/// # Returns
/// `Ok(Some(Vec<u8>))` if a thumbnail is found, `Ok(None)` otherwise.
fn find_embedded_thumbnail<R: Read + Seek>(reader: &mut R) -> Result<Option<Vec<u8>>, XcfError> {
    loop {
        let property_type = reader.read_u32::<BigEndian>()?;
        let property_length = reader.read_u32::<BigEndian>()?;

        if property_type == 0 {
            break;
        }

        if property_type == 25 {
            // PROP_THUMBNAIL
            let thumbnail_width = reader.read_u32::<BigEndian>()?;
            let thumbnail_height = reader.read_u32::<BigEndian>()?;
            let _thumbnail_type = reader.read_u32::<BigEndian>()?; // Usually 0 (RGB) or 1 (RGBA)
            let data_length = reader.read_u32::<BigEndian>()?;

            let mut thumbnail_data = vec![0u8; data_length as usize];
            reader.read_exact(&mut thumbnail_data)?;

            // GIMP typically saves this as a RAW RGB block.
            // We need to encode it to PNG for our purposes.
            let mut png_bytes = Vec::new();

            // Heuristic: if data_length matches w*h*3, it's RGB8. If w*h*4, it's RGBA8.
            let color_type = if data_length == thumbnail_width * thumbnail_height * 3 {
                ExtendedColorType::Rgb8
            } else if data_length == thumbnail_width * thumbnail_height * 4 {
                ExtendedColorType::Rgba8
            } else {
                // If it doesn't match raw pixels, it might be a PNG already?
                // Let's check for PNG magic
                if thumbnail_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                    return Ok(Some(thumbnail_data));
                }
                return Ok(None);
            };

            image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_bytes)).write_image(
                &thumbnail_data,
                thumbnail_width,
                thumbnail_height,
                color_type,
            )?;

            return Ok(Some(png_bytes));
        }

        reader.seek(SeekFrom::Current(property_length as i64))?;
    }
    Ok(None)
}

/// Reads a GIMP-style Pascal string (UInt32 length + bytes + potential null).
fn read_gimp_string<R: Read>(reader: &mut R) -> Result<String, Box<dyn std::error::Error>> {
    let length = reader.read_u32::<BigEndian>()?;
    if length == 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u8; length as usize];
    reader.read_exact(&mut buffer)?;

    // Remove null terminator if present
    let actual_length = buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(length as usize);
    Ok(String::from_utf8_lossy(&buffer[..actual_length]).to_string())
}

/// Decodes an RLE-compressed tile and composites it onto the canvas.
#[allow(clippy::too_many_arguments)]
fn decode_and_composite_tile<R: Read>(
    reader: &mut R,
    canvas_data: &mut [u8],
    tile_x: u32,
    tile_y: u32,
    layer_width: u32,
    layer_height: u32,
    canvas_width: u32,
    canvas_height: u32,
    offset_x: i32,
    offset_y: i32,
    bytes_per_pixel: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_start = tile_x * 64;
    let y_start = tile_y * 64;
    let tile_width = cmp::min(64, layer_width - x_start);
    let tile_height = cmp::min(64, layer_height - y_start);
    let total_pixels = tile_width * tile_height;

    // Temporary buffer for the tile RGBA data
    let mut tile_rgba = vec![0u8; (total_pixels * 4) as usize];

    // If it's RGB (3 bytes), set alpha to fully opaque
    if bytes_per_pixel == 3 {
        for i in 0..total_pixels {
            tile_rgba[(i * 4 + 3) as usize] = 255;
        }
    }

    // Decode RLE for each channel
    for channel in 0..bytes_per_pixel {
        let mut pixels_read = 0;
        while pixels_read < total_pixels {
            let determinant = reader.read_u8()?;
            if determinant < 127 {
                let count = (determinant as u32) + 1;
                let value = reader.read_u8()?;
                for i in 0..count {
                    if pixels_read + i < total_pixels {
                        tile_rgba[((pixels_read + i) * 4 + channel) as usize] = value;
                    }
                }
                pixels_read += count;
            } else if determinant == 127 {
                let count = reader.read_u16::<BigEndian>()? as u32;
                let value = reader.read_u8()?;
                for i in 0..count {
                    if pixels_read + i < total_pixels {
                        tile_rgba[((pixels_read + i) * 4 + channel) as usize] = value;
                    }
                }
                pixels_read += count;
            } else if determinant == 128 {
                let count = reader.read_u16::<BigEndian>()? as u32;
                for i in 0..count {
                    let value = reader.read_u8()?;
                    if pixels_read + i < total_pixels {
                        tile_rgba[((pixels_read + i) * 4 + channel) as usize] = value;
                    }
                }
                pixels_read += count;
            } else {
                let count = 256 - determinant as u32;
                for i in 0..count {
                    let value = reader.read_u8()?;
                    if pixels_read + i < total_pixels {
                        tile_rgba[((pixels_read + i) * 4 + channel) as usize] = value;
                    }
                }
                pixels_read += count;
            }
        }
    }

    // Composite tile onto canvas using Porter-Duff Over
    for local_y in 0..tile_height {
        for local_x in 0..tile_width {
            let global_x = offset_x + (x_start + local_x) as i32;
            let global_y = offset_y + (y_start + local_y) as i32;

            if global_x < 0
                || global_y < 0
                || global_x >= canvas_width as i32
                || global_y >= canvas_height as i32
            {
                continue;
            }

            let canvas_index = ((global_y as u32 * canvas_width + global_x as u32) * 4) as usize;
            let tile_index = ((local_y * tile_width + local_x) * 4) as usize;

            let source_alpha = tile_rgba[tile_index + 3] as u32;
            if source_alpha == 0 {
                continue;
            }

            let source_red = tile_rgba[tile_index] as u32;
            let source_green = tile_rgba[tile_index + 1] as u32;
            let source_blue = tile_rgba[tile_index + 2] as u32;

            let dest_red = canvas_data[canvas_index] as u32;
            let dest_green = canvas_data[canvas_index + 1] as u32;
            let dest_blue = canvas_data[canvas_index + 2] as u32;
            let dest_alpha = canvas_data[canvas_index + 3] as u32;

            // Porter-Duff "Over" blending
            let out_alpha = source_alpha + (dest_alpha * (255 - source_alpha) / 255);
            let blend_channel = |source_channel: u32, destination_channel: u32| {
                (source_channel * source_alpha
                    + destination_channel * dest_alpha * (255 - source_alpha) / 255)
                    .checked_div(out_alpha)
                    .map(|channel| channel as u8)
            };
            if let (Some(red), Some(green), Some(blue)) = (
                blend_channel(source_red, dest_red),
                blend_channel(source_green, dest_green),
                blend_channel(source_blue, dest_blue),
            ) {
                canvas_data[canvas_index] = red;
                canvas_data[canvas_index + 1] = green;
                canvas_data[canvas_index + 2] = blue;
                canvas_data[canvas_index + 3] = out_alpha as u8;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::decode_and_composite_tile;

    #[test]
    fn tile_compositing_preserves_transparency_opacity_and_integer_rounding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (source, destination, expected) in [
            ([200, 100, 50, 0], [10, 20, 30, 200], [10, 20, 30, 200]),
            ([200, 100, 50, 255], [10, 20, 30, 200], [200, 100, 50, 255]),
            ([200, 100, 50, 128], [10, 20, 30, 0], [200, 100, 50, 128]),
            ([200, 0, 0, 128], [0, 0, 200, 255], [100, 0, 99, 255]),
        ] {
            let encoded: Vec<u8> = source
                .into_iter()
                .flat_map(|channel| [0, channel])
                .collect();
            let mut reader = std::io::Cursor::new(encoded);
            let mut canvas = destination;
            decode_and_composite_tile(&mut reader, &mut canvas, 0, 0, 1, 1, 1, 1, 0, 0, 4)?;
            assert_eq!(canvas, expected);
        }
        Ok(())
    }
}
