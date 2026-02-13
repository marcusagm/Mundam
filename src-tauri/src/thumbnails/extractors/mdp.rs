//! Extractor for MediBang Paint / FireAlpaca (.mdp) project files.
//!
//! The MDP format is a proprietary container that stores metadata in XML
//! and image data (layers, tiles, thumbnails) in binary blocks called "PAC".
//!
//! This extractor implements two strategies:
//! 1. **Full Canvas Rendering** (primary): Reads all visible layers, decompresses
//!    tiles, and composites them with alpha blending for a high-resolution preview.
//! 2. **Thumbnail Extraction** (fallback): Reads the embedded low-res thumbnail.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use quick_xml::reader::Reader;
use flate2::read::ZlibDecoder;
use image::ImageEncoder;

/// Magic bytes at the start of every MDP file.
const MDP_MAGIC: &[u8; 7] = b"mdipack";

/// Magic bytes at the start of every PAC block.
const PAC_MAGIC: &[u8; 4] = b"PAC ";

/// Size of a PAC block header in bytes.
const PAC_HEADER_SIZE: u32 = 132;

/// Error type for MDP parsing.
#[derive(Debug, thiserror::Error)]
pub enum MdpError {
    /// The file does not start with known magic bytes.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The file does not have the expected structure.
    #[error("Invalid MDP format: missing or incorrect magic bytes")]
    InvalidFormat,

    /// No preview could be generated from the file.
    #[error("No preview available in MDP project")]
    NoPreview,

    /// Failed to parse the XML metadata section.
    #[error("XML parse error in MDP metadata: {0}")]
    Xml(String),
}

/// Metadata of a single layer parsed from the XML header.
struct MdpLayerInfo {
    /// The name of the binary PAC block containing tile data.
    binary_block_name: String,
    /// Width of the layer in pixels.
    width: u32,
    /// Height of the layer in pixels.
    height: u32,
    /// Horizontal offset on the canvas.
    offset_x: i32,
    /// Vertical offset on the canvas.
    offset_y: i32,
    /// Opacity value (0-255).
    alpha: u8,
    /// Layer type string (e.g., "32bpp", "8bpp", "1bpp").
    layer_type: String,
    /// ARGB color string for 8bpp/1bpp layers (e.g., "FF09151F").
    color_hex: Option<String>,
}

/// Metadata of the embedded thumbnail parsed from the XML header.
struct MdpThumbInfo {
    /// The name of the binary PAC block containing the thumbnail data.
    binary_block_name: String,
    /// Thumbnail width in pixels.
    width: u32,
    /// Thumbnail height in pixels.
    height: u32,
}

/// Parsed MDP file header containing all necessary metadata.
struct MdpHeader {
    /// The XML metadata string for potential future use.
    /// Canvas width and height are derived from the largest layer.
    canvas_width: u32,
    /// Canvas height.
    canvas_height: u32,
    /// Information about visible layers, ordered from bottom to top for compositing.
    visible_layers: Vec<MdpLayerInfo>,
    /// Optional thumbnail metadata.
    thumbnail: Option<MdpThumbInfo>,
}

/// Extracts a high-resolution preview from a MediBang Paint (.mdp) file.
///
/// The embedded thumbnail reflects the artist's intended composition (including
/// blend modes, group states, and visibility at save time), so it is used as the
/// primary source. If no thumbnail exists, the full canvas is rendered by
/// compositing visible layers with Normal blending.
///
/// # Arguments
/// * `path` - The path to the .mdp file.
///
/// # Errors
/// Returns error if the file is not a valid MDP or if both thumbnail extraction
/// and canvas rendering fail.
pub fn extract_mdp_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    // Prefer the embedded thumbnail — it matches what the artist sees in the app.
    match extract_mdp_thumbnail(path) {
        Ok(result) => return Ok(result),
        Err(thumbnail_error) => {
            println!("MDP: Thumbnail extraction failed ({thumbnail_error}), trying full render.");
        }
    }

    // Fallback: render the full canvas by compositing visible layers.
    render_mdp_canvas(path)
}

/// Renders the full canvas by compositing all visible layers.
///
/// # Errors
/// Returns error if layer data cannot be decoded or composited.
fn render_mdp_canvas(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    let header = parse_mdp_header(&mut reader)?;

    if header.visible_layers.is_empty() {
        return Err(MdpError::NoPreview.into());
    }

    // Collect the PAC block names we need to find.
    let needed_block_names: Vec<String> = header.visible_layers
        .iter()
        .map(|layer| layer.binary_block_name.clone())
        .collect();

    // Read all needed binary blocks from the file.
    let binary_blocks = read_pac_blocks(&mut reader, &needed_block_names)?;

    // Create transparent canvas (RGBA).
    let canvas_pixel_count = (header.canvas_width as usize) * (header.canvas_height as usize);
    let mut canvas_buffer = vec![0u8; canvas_pixel_count * 4];

    // Composite layers from bottom (last in XML) to top (first in XML).
    // The visible_layers vec is already in bottom-to-top order.
    for layer in &header.visible_layers {
        let block_data = match binary_blocks.get(&layer.binary_block_name) {
            Some(data) => data,
            None => continue,
        };

        composite_layer_onto_canvas(
            &mut canvas_buffer,
            header.canvas_width,
            header.canvas_height,
            layer,
            block_data,
        )?;
    }

    // Convert RGBA buffer to PNG.
    let mut png_output = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_output))
        .write_image(
            &canvas_buffer,
            header.canvas_width,
            header.canvas_height,
            image::ExtendedColorType::Rgba8,
        )?;

    Ok((png_output, "image/png".to_string()))
}

/// Extracts the embedded low-resolution thumbnail from the MDP file.
///
/// # Errors
/// Returns error if the thumbnail PAC block is missing or corrupted.
fn extract_mdp_thumbnail(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    let header = parse_mdp_header(&mut reader)?;

    let thumb_info = header.thumbnail
        .ok_or(MdpError::NoPreview)?;

    let blocks = read_pac_blocks(&mut reader, &[thumb_info.binary_block_name.clone()])?;
    let thumb_data = blocks.get(&thumb_info.binary_block_name)
        .ok_or(MdpError::NoPreview)?;

    // Thumbnail is raw BGRA pixels. Swap to RGBA.
    let mut rgba_pixels = thumb_data.clone();
    for pixel_chunk in rgba_pixels.chunks_exact_mut(4) {
        pixel_chunk.swap(0, 2);
    }

    let mut png_output = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_output))
        .write_image(
            &rgba_pixels,
            thumb_info.width,
            thumb_info.height,
            image::ExtendedColorType::Rgba8,
        )?;

    Ok((png_output, "image/png".to_string()))
}

/// Parses the MDP file header: validates magic bytes, reads XML metadata,
/// and extracts layer and thumbnail info.
///
/// After this function returns, the reader is positioned right at the start
/// of the PAC blocks section.
///
/// # Errors
/// Returns error if the file is not a valid MDP format.
fn parse_mdp_header<R: Read + Seek>(reader: &mut R) -> Result<MdpHeader, Box<dyn std::error::Error>> {
    // 1. Validate Header Magic: "mdipack" (7 bytes).
    let mut magic = [0u8; 7];
    reader.read_exact(&mut magic)?;
    if &magic != MDP_MAGIC {
        return Err(MdpError::InvalidFormat.into());
    }

    // 2. Skip Padding (5 bytes).
    reader.seek(SeekFrom::Current(5))?;

    // 3. Read XML size and Pack size (both u32 LE).
    let xml_length = reader.read_u32::<LittleEndian>()?;
    let _pack_size = reader.read_u32::<LittleEndian>()?;

    // 4. Extract XML Metadata.
    let mut xml_buffer = vec![0u8; xml_length as usize];
    reader.read_exact(&mut xml_buffer)?;
    let xml_string = String::from_utf8(xml_buffer)
        .map_err(|error| MdpError::Xml(error.to_string()))?;

    // 5. Parse XML to extract layer info and thumbnail info.
    let mut visible_layers = Vec::new();
    let mut thumbnail_info: Option<MdpThumbInfo> = None;
    let mut canvas_width = 0u32;
    let mut canvas_height = 0u32;

    let mut xml_parser = Reader::from_str(&xml_string);
    xml_parser.config_mut().trim_text(true);
    let mut event_buffer = Vec::new();

    loop {
        match xml_parser.read_event_into(&mut event_buffer) {
            Ok(quick_xml::events::Event::Start(element)) | Ok(quick_xml::events::Event::Empty(element)) => {
                match element.name().as_ref() {
                    b"Thumb" => {
                        let mut blob_name = String::new();
                        let mut width = 0u32;
                        let mut height = 0u32;
                        for attribute in element.attributes().flatten() {
                            match attribute.key.as_ref() {
                                b"bin" => blob_name = attribute.unescape_value().unwrap_or_default().into_owned(),
                                b"width" => width = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                b"height" => height = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                        if !blob_name.is_empty() && width > 0 && height > 0 {
                            thumbnail_info = Some(MdpThumbInfo { binary_block_name: blob_name, width, height });
                        }
                    }
                    b"Layer" => {
                        let mut blob_name = String::new();
                        let mut width = 0u32;
                        let mut height = 0u32;
                        let mut offset_x = 0i32;
                        let mut offset_y = 0i32;
                        let mut alpha = 255u8;
                        let mut visible = true;
                        let mut layer_type = String::new();
                        let mut color_hex: Option<String> = None;

                        for attribute in element.attributes().flatten() {
                            match attribute.key.as_ref() {
                                b"bin" => blob_name = attribute.unescape_value().unwrap_or_default().into_owned(),
                                b"width" => width = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                b"height" => height = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                b"ofsx" => offset_x = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                b"ofsy" => offset_y = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(0),
                                b"alpha" => alpha = attribute.unescape_value().unwrap_or_default().parse().unwrap_or(255),
                                b"visible" => visible = attribute.unescape_value().unwrap_or_default().as_ref() == "true",
                                b"type" => layer_type = attribute.unescape_value().unwrap_or_default().into_owned(),
                                b"color" => color_hex = Some(attribute.unescape_value().unwrap_or_default().into_owned()),
                                _ => {}
                            }
                        }

                        if visible && !blob_name.is_empty() && width > 0 && height > 0 {
                            // Track canvas dimensions as the max extent of any layer.
                            let extent_right = (offset_x.max(0) as u32).saturating_add(width);
                            let extent_bottom = (offset_y.max(0) as u32).saturating_add(height);
                            canvas_width = canvas_width.max(extent_right);
                            canvas_height = canvas_height.max(extent_bottom);

                            visible_layers.push(MdpLayerInfo {
                                binary_block_name: blob_name,
                                width,
                                height,
                                offset_x,
                                offset_y,
                                alpha,
                                layer_type,
                                color_hex,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(error) => return Err(MdpError::Xml(error.to_string()).into()),
            _ => (),
        }
        event_buffer.clear();
    }

    // MDP lists layers from top to bottom in XML. We need bottom-to-top for compositing.
    visible_layers.reverse();

    Ok(MdpHeader {
        canvas_width,
        canvas_height,
        visible_layers,
        thumbnail: thumbnail_info,
    })
}

/// Reads specified PAC blocks from the current reader position.
///
/// Returns a HashMap mapping block name to its decompressed data.
/// Only blocks whose names appear in `needed_names` are kept in memory.
///
/// # Errors
/// Returns error if the PAC block structure is malformed.
fn read_pac_blocks<R: Read + Seek>(
    reader: &mut R,
    needed_names: &[String],
) -> Result<HashMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut blocks = HashMap::new();

    loop {
        let mut pac_header = [0u8; PAC_HEADER_SIZE as usize];
        match reader.read_exact(&mut pac_header) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }

        if &pac_header[0..4] != PAC_MAGIC {
            break;
        }

        let item_total_size = u32::from_le_bytes(pac_header[4..8].try_into().unwrap());
        let item_type_flag = u32::from_le_bytes(pac_header[8..12].try_into().unwrap());

        let raw_name = &pac_header[68..132];
        let item_name = std::str::from_utf8(raw_name)
            .unwrap_or("")
            .trim_matches(char::from(0))
            .to_string();

        let data_length = item_total_size.saturating_sub(PAC_HEADER_SIZE);

        if needed_names.contains(&item_name) {
            let mut raw_data = vec![0u8; data_length as usize];
            reader.read_exact(&mut raw_data)?;

            // Decompress if the block is zlib-compressed (type flag 1).
            let decompressed_data = if item_type_flag == 1 {
                let mut zlib_decoder = ZlibDecoder::new(&raw_data[..]);
                let mut decompressed = Vec::new();
                zlib_decoder.read_to_end(&mut decompressed)?;
                decompressed
            } else {
                raw_data
            };

            blocks.insert(item_name, decompressed_data);

            // Early exit if we found all needed blocks.
            if blocks.len() == needed_names.len() {
                break;
            }
        } else {
            reader.seek(SeekFrom::Current(data_length as i64))?;
        }
    }

    Ok(blocks)
}

/// Composites a single layer's tile data onto the canvas buffer.
///
/// The layer's binary data contains a tile header followed by tile entries.
/// Each tile is a rectangular chunk of pixels that must be placed at the
/// correct position on the canvas.
///
/// # Errors
/// Returns error if tile data is corrupted or decompression fails.
fn composite_layer_onto_canvas(
    canvas_buffer: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    layer: &MdpLayerInfo,
    block_data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Empty layers may contain only 4 bytes (tile_count = 0) with no tile dimension.
    if block_data.len() < 4 {
        return Ok(());
    }

    let tile_count = u32::from_le_bytes(block_data[0..4].try_into().unwrap());
    if tile_count == 0 || block_data.len() < 8 {
        return Ok(());
    }

    let tile_dimension = u32::from_le_bytes(block_data[4..8].try_into().unwrap());

    let mut offset = 8usize;

    for _tile_index in 0..tile_count {
        if offset + 16 > block_data.len() {
            break;
        }

        let tile_column = u32::from_le_bytes(block_data[offset..offset + 4].try_into().unwrap());
        let tile_row = u32::from_le_bytes(block_data[offset + 4..offset + 8].try_into().unwrap());
        let compression_type = u32::from_le_bytes(block_data[offset + 8..offset + 12].try_into().unwrap());
        let compressed_data_size = u32::from_le_bytes(block_data[offset + 12..offset + 16].try_into().unwrap());

        offset += 16;

        if offset + compressed_data_size as usize > block_data.len() {
            break;
        }

        let tile_raw_data = &block_data[offset..offset + compressed_data_size as usize];
        offset += compressed_data_size as usize;

        // Align to 4-byte boundary.
        let padding = (4 - (compressed_data_size as usize) % 4) % 4;
        offset += padding;

        // Decompress tile data.
        let decompressed_tile = match compression_type {
            0 => {
                // Zlib compressed.
                let mut zlib_decoder = ZlibDecoder::new(tile_raw_data);
                let mut decompressed = Vec::new();
                zlib_decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            // Snappy (1) and FastLZ (2) are rare; skip unsupported tiles gracefully.
            _ => continue,
        };

        // Tiles are ALWAYS tile_dim × tile_dim in the binary data, even at canvas edges.
        // The blit function handles clipping at the canvas boundary.
        let tile_pixel_x = tile_column * tile_dimension;
        let tile_pixel_y = tile_row * tile_dimension;
        let total_tile_pixels = (tile_dimension * tile_dimension) as usize;

        // Decode tile pixels based on the layer type.
        let tile_rgba = decode_tile_to_rgba(
            &decompressed_tile,
            total_tile_pixels,
            &layer.layer_type,
            &layer.color_hex,
        );

        // Blit tile onto canvas with alpha blending (canvas-edge clipping is automatic).
        blit_tile_onto_canvas(
            canvas_buffer,
            canvas_width,
            canvas_height,
            &tile_rgba,
            tile_dimension,
            tile_dimension,
            layer.offset_x + tile_pixel_x as i32,
            layer.offset_y + tile_pixel_y as i32,
            layer.alpha,
        );
    }

    Ok(())
}

/// Decodes raw tile bytes into an RGBA pixel buffer based on layer type.
///
/// - **32bpp**: Each pixel is 4 bytes BGRA. Swapped to RGBA.
/// - **8bpp**: Each pixel is 1 byte (alpha mask). Combined with layer color.
/// - **1bpp**: Each pixel is 1 bit (binary mask). Combined with layer color.
fn decode_tile_to_rgba(
    decompressed_data: &[u8],
    total_pixels: usize,
    layer_type: &str,
    color_hex: &Option<String>,
) -> Vec<u8> {
    let mut rgba_buffer = vec![0u8; total_pixels * 4];

    match layer_type {
        "32bpp" => {
            let byte_count = total_pixels * 4;
            if decompressed_data.len() >= byte_count {
                rgba_buffer.copy_from_slice(&decompressed_data[..byte_count]);
                // Swap BGRA -> RGBA.
                for pixel_chunk in rgba_buffer.chunks_exact_mut(4) {
                    pixel_chunk.swap(0, 2);
                }
            }
        }
        "8bpp" => {
            // Each byte is an alpha value. The color is defined in XML.
            let (red, green, blue) = parse_layer_color(color_hex);
            for pixel_index in 0..total_pixels {
                if pixel_index < decompressed_data.len() {
                    let alpha_value = decompressed_data[pixel_index];
                    rgba_buffer[pixel_index * 4] = red;
                    rgba_buffer[pixel_index * 4 + 1] = green;
                    rgba_buffer[pixel_index * 4 + 2] = blue;
                    rgba_buffer[pixel_index * 4 + 3] = alpha_value;
                }
            }
        }
        "1bpp" => {
            // Each bit is on/off. The color is defined in XML.
            let (red, green, blue) = parse_layer_color(color_hex);
            for pixel_index in 0..total_pixels {
                let byte_index = pixel_index / 8;
                let bit_index = 7 - (pixel_index % 8);
                if byte_index < decompressed_data.len() {
                    let is_set = (decompressed_data[byte_index] >> bit_index) & 1;
                    rgba_buffer[pixel_index * 4] = red;
                    rgba_buffer[pixel_index * 4 + 1] = green;
                    rgba_buffer[pixel_index * 4 + 2] = blue;
                    rgba_buffer[pixel_index * 4 + 3] = if is_set == 1 { 255 } else { 0 };
                }
            }
        }
        _ => {
            // Unknown layer type - leave as transparent.
        }
    }

    rgba_buffer
}

/// Parses the ARGB color hex string from the XML (e.g., "FF09151F").
/// Returns (R, G, B) tuple.
fn parse_layer_color(color_hex: &Option<String>) -> (u8, u8, u8) {
    match color_hex {
        Some(hex) if hex.len() == 8 => {
            let red = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let green = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let blue = u8::from_str_radix(&hex[6..8], 16).unwrap_or(0);
            (red, green, blue)
        }
        _ => (0, 0, 0),
    }
}

/// Blits a tile's RGBA data onto the canvas using Porter-Duff Over compositing.
///
/// Applies the layer-level opacity (`layer_alpha`) as a multiplier on top of
/// each pixel's individual alpha channel.
fn blit_tile_onto_canvas(
    canvas_buffer: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    tile_rgba: &[u8],
    tile_width: u32,
    tile_height: u32,
    global_x: i32,
    global_y: i32,
    layer_alpha: u8,
) {
    for local_y in 0..tile_height {
        for local_x in 0..tile_width {
            let dest_x = global_x + local_x as i32;
            let dest_y = global_y + local_y as i32;

            if dest_x < 0 || dest_y < 0 || dest_x >= canvas_width as i32 || dest_y >= canvas_height as i32 {
                continue;
            }

            let tile_pixel_index = ((local_y * tile_width + local_x) * 4) as usize;
            let canvas_pixel_index = ((dest_y as u32 * canvas_width + dest_x as u32) * 4) as usize;

            if tile_pixel_index + 3 >= tile_rgba.len() || canvas_pixel_index + 3 >= canvas_buffer.len() {
                continue;
            }

            let source_red = tile_rgba[tile_pixel_index] as u32;
            let source_green = tile_rgba[tile_pixel_index + 1] as u32;
            let source_blue = tile_rgba[tile_pixel_index + 2] as u32;
            // Apply layer-level opacity to the pixel's own alpha.
            let source_alpha = ((tile_rgba[tile_pixel_index + 3] as u32) * (layer_alpha as u32)) / 255;

            if source_alpha == 0 {
                continue;
            }

            let dest_red = canvas_buffer[canvas_pixel_index] as u32;
            let dest_green = canvas_buffer[canvas_pixel_index + 1] as u32;
            let dest_blue = canvas_buffer[canvas_pixel_index + 2] as u32;
            let dest_alpha = canvas_buffer[canvas_pixel_index + 3] as u32;

            // Porter-Duff "Over" operator.
            let output_alpha = source_alpha + (dest_alpha * (255 - source_alpha)) / 255;
            if output_alpha > 0 {
                canvas_buffer[canvas_pixel_index] = ((source_red * source_alpha + dest_red * dest_alpha * (255 - source_alpha) / 255) / output_alpha) as u8;
                canvas_buffer[canvas_pixel_index + 1] = ((source_green * source_alpha + dest_green * dest_alpha * (255 - source_alpha) / 255) / output_alpha) as u8;
                canvas_buffer[canvas_pixel_index + 2] = ((source_blue * source_alpha + dest_blue * dest_alpha * (255 - source_alpha) / 255) / output_alpha) as u8;
                canvas_buffer[canvas_pixel_index + 3] = output_alpha as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper to resolve sample file paths relative to the project root.
    fn sample_path(relative: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // Go to project root.
        path.push(relative);
        path
    }

    #[test]
    fn test_extract_mdp_preview_full_render() {
        let path = sample_path("file-samples/Imagens/Design/MediBang Paint - Firealpaca/aula_silhueta.mdp");
        if !path.exists() {
            println!("Skipping: sample file not found at {:?}", path);
            return;
        }

        let result = extract_mdp_preview(&path);
        assert!(result.is_ok(), "Extraction failed: {:?}", result.err());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/png");
        assert!(!data.is_empty());
        assert_eq!(&data[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_extract_mdp_thumbnail_fallback() {
        let path = sample_path("file-samples/Imagens/Design/MediBang Paint - Firealpaca/aula_silhueta.mdp");
        if !path.exists() {
            println!("Skipping: sample file not found at {:?}", path);
            return;
        }

        let result = extract_mdp_thumbnail(&path);
        assert!(result.is_ok(), "Thumbnail extraction failed: {:?}", result.err());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/png");
        assert!(!data.is_empty());
    }

    #[test]
    fn test_render_small_mdp_file() {
        let path = sample_path("file-samples/Imagens/Design/MediBang Paint - Firealpaca/checkerboard5.mdp");
        if !path.exists() {
            println!("Skipping: sample file not found at {:?}", path);
            return;
        }

        let result = render_mdp_canvas(&path);
        assert!(result.is_ok(), "Render failed: {:?}", result.err());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/png");
        assert!(!data.is_empty());
    }

    #[test]
    fn test_render_yohaku_mdp_file() {
        let path = sample_path("file-samples/Imagens/Design/MediBang Paint - Firealpaca/yohaku_370x320.mdp");
        if !path.exists() {
            println!("Skipping: sample file not found at {:?}", path);
            return;
        }

        let result = render_mdp_canvas(&path);
        assert!(result.is_ok(), "Render failed: {:?}", result.err());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/png");
        assert!(!data.is_empty());
    }
}
