//! Extractor for MediBang Paint / FireAlpaca (.mdp) project files.
//!
//! The MDP format is a proprietary container formats that stores metadata in XML
//! and image data (layers, tiles, thumbnails) in binary blocks called "PAC".
//! This extractor parses the file to locate the thumbnail block and decodes it.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use quick_xml::reader::Reader;
use flate2::read::ZlibDecoder;
use image::ImageEncoder;

/// Error type for MDP parsing.
#[derive(Debug, thiserror::Error)]
pub enum MdpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid MDP format: missing or incorrect magic bytes")]
    InvalidFormat,
    #[error("Missing thumbnail in MDP project")]
    MissingThumbnail,
    #[error("XML parse error in MDP metadata: {0}")]
    Xml(String),
}

/// Extracts the thumbnail from a MediBang Paint (.mdp) file.
///
/// # Arguments
/// * `path` - The path to the .mdp file.
///
/// # Errors
/// Returns error if the file is not a valid MDP, if the thumbnail is missing,
/// or if decompression/decoding fails.
pub fn extract_mdp_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    // 1. Validate Header Magic: "mdipack" (7 bytes)
    let mut magic = [0u8; 7];
    reader.read_exact(&mut magic)?;
    if &magic != b"mdipack" {
        return Err(MdpError::InvalidFormat.into());
    }

    // 2. Skip Padding (5 bytes)
    reader.seek(SeekFrom::Current(5))?;

    // 3. Read Metadata Size and Total Pack Size (u32, Little Endian)
    let xml_length = reader.read_u32::<LittleEndian>()?;
    let pack_size = reader.read_u32::<LittleEndian>()?;

    // 4. Extract XML Metadata
    let mut xml_buffer = vec![0u8; xml_length as usize];
    reader.read_exact(&mut xml_buffer)?;
    let xml_string = String::from_utf8(xml_buffer)
        .map_err(|e| MdpError::Xml(e.to_string()))?;

    // Parse XML to identify the thumbnail's binary blob name and its dimensions.
    // The thumbnail is defined in a <Thumb> tag, e.g., <Thumb bin="thum" width="160" height="120" />.
    let mut thumb_blob_name = String::new();
    let mut thumb_width = 0u32;
    let mut thumb_height = 0u32;

    let mut xml_parser = Reader::from_str(&xml_string);
    xml_parser.config_mut().trim_text(true);
    let mut buffer = Vec::new();

    loop {
        match xml_parser.read_event_into(&mut buffer) {
            Ok(quick_xml::events::Event::Start(element)) | Ok(quick_xml::events::Event::Empty(element))
            if element.name().as_ref() == b"Thumb" => {
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    match attribute.key.as_ref() {
                        b"bin" => thumb_blob_name = attribute.unescape_value()?.into_owned(),
                        b"width" => thumb_width = attribute.unescape_value()?.parse().unwrap_or(0),
                        b"height" => thumb_height = attribute.unescape_value()?.parse().unwrap_or(0),
                        _ => {}
                    }
                }
                break;
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(MdpError::Xml(e.to_string()).into()),
            _ => (),
        }
        buffer.clear();
    }

    if thumb_blob_name.is_empty() || thumb_width == 0 || thumb_height == 0 {
        return Err(MdpError::MissingThumbnail.into());
    }

    // 5. Iterate through PAC blocks to find the thumbnail data.
    let mut bytes_parsed_in_pack = 0u32;
    while bytes_parsed_in_pack < pack_size {
        // PAC Header is consistently 132 bytes.
        let mut pac_header = [0u8; 132];
        reader.read_exact(&mut pac_header)?;

        if &pac_header[0..4] != b"PAC " {
            return Err(MdpError::InvalidFormat.into());
        }

        let item_total_size = u32::from_le_bytes(pac_header[4..8].try_into().unwrap());
        let item_type_flag = u32::from_le_bytes(pac_header[8..12].try_into().unwrap());
        // item_type_flag: 0 = Uncompressed, 1 = Zlib Compressed.

        // The item name is at offset 68, null-terminated.
        let raw_name = &pac_header[68..132];
        let item_name = std::str::from_utf8(raw_name)?
            .trim_matches(char::from(0))
            .to_string();

        if item_name == thumb_blob_name {
            // Found the thumbnail block!
            let data_length = item_total_size - 132;
            let mut binary_data = vec![0u8; data_length as usize];
            reader.read_exact(&mut binary_data)?;

            // Decompress if needed.
            let pixel_data = if item_type_flag == 1 {
                let mut zlib_decoder = ZlibDecoder::new(&binary_data[..]);
                let mut decompressed_buffer = Vec::new();
                zlib_decoder.read_to_end(&mut decompressed_buffer)?;
                decompressed_buffer
            } else {
                binary_data
            };

            // MediBang stores thumbnail pixels in BGRA format.
            // We need to convert it to RGBA by swapping the Red and Blue channels.
            let mut final_rgba_pixels = pixel_data;
            for pixel_chunk in final_rgba_pixels.chunks_exact_mut(4) {
                pixel_chunk.swap(0, 2);
            }

            // Encode the RGBA buffer into PNG format for the Mundam preview system.
            let mut png_output = Vec::new();
            image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_output))
                .write_image(
                    &final_rgba_pixels,
                    thumb_width,
                    thumb_height,
                    image::ExtendedColorType::Rgba8
                )?;

            return Ok((png_output, "image/png".to_string()));
        } else {
            // Skip this block's data and move to the next PAC header.
            reader.seek(SeekFrom::Current((item_total_size - 132) as i64))?;
        }
        bytes_parsed_in_pack += item_total_size;
    }

    Err(MdpError::MissingThumbnail.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_mdp_preview() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // Go to project root
        path.push("file-samples/Imagens/Design/MediBang Paint - Firealpaca/aula_silhueta.mdp");

        if path.exists() {
            let result = extract_mdp_preview(&path);
            assert!(result.is_ok(), "Extraction failed: {:?}", result.err());
            let (data, mime) = result.unwrap();
            assert_eq!(mime, "image/png");
            assert!(!data.is_empty());

            // Basic PNG magic number check
            assert_eq!(&data[0..8], b"\x89PNG\r\n\x1a\n");
        } else {
            println!("Skipping test: sample file not found at {:?}", path);
        }
    }
}
