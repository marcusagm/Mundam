use crate::core::error::{AppError, AppResult};
use resvg::usvg;
use std::path::Path;
use std::sync::Arc;
use tiny_skia::Pixmap;

const FONT_SVG_TEMPLATE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 500\">\
  <rect width=\"400\" height=\"500\" fill=\"#f8f9fa\"/>\
  <text x=\"200\" y=\"220\" font-family=\"{family}\" font-size=\"160\" text-anchor=\"middle\" fill=\"#1f2937\">{main_preview}</text>\
  <text x=\"200\" y=\"330\" font-family=\"{family}\" font-size=\"24\" text-anchor=\"middle\" fill=\"#4b5563\">{family}</text>\
  <text x=\"200\" y=\"380\" font-family=\"{family}\" font-size=\"16\" text-anchor=\"middle\" fill=\"#9ca3af\">{row1}</text>\
  <text x=\"200\" y=\"410\" font-family=\"{family}\" font-size=\"16\" text-anchor=\"middle\" fill=\"#9ca3af\">{row2}</text>\
  <text x=\"200\" y=\"440\" font-family=\"{family}\" font-size=\"16\" text-anchor=\"middle\" fill=\"#9ca3af\">{row3}</text>\
</svg>";

/// Decompresses a font if it is compressed in WOFF or WOFF2 format.
///
/// # Arguments
///
/// * `file_data` - The raw bytes of the font file.
/// * `extension` - The file extension of the font.
///
/// # Returns
///
/// `AppResult<Vec<u8>>` - The decompressed raw font bytes.
pub fn decompress_font(file_data: &[u8], extension: &str) -> AppResult<Vec<u8>> {
    let extension_lowercase = extension.to_lowercase();
    if extension_lowercase == "woff" {
        wuff::decompress_woff1(file_data)
            .map_err(|error| AppError::Generic(format!("WOFF1 decompression failed: {:?}", error)))
    } else if extension_lowercase == "woff2" {
        wuff::decompress_woff2(file_data)
            .map_err(|error| AppError::Generic(format!("WOFF2 decompression failed: {:?}", error)))
    } else {
        Ok(file_data.to_vec())
    }
}

/// Helper to check if a font face has a specific glyph mapped.
fn has_glyph_defined(face: &ttf_parser::Face, character: char) -> bool {
    match face.glyph_index(character) {
        Some(glyph_id) => glyph_id.0 > 0,
        None => false,
    }
}

/// The fixed width of the font thumbnail canvas in pixels.
const THUMBNAIL_CANVAS_WIDTH: i64 = 400;

/// The fixed height of the font thumbnail canvas in pixels.
const THUMBNAIL_CANVAS_HEIGHT: i64 = 500;

/// Extracts comprehensive technical metadata from a font file.
///
/// The `width` and `height` fields in the returned JSON always correspond to
/// the **thumbnail canvas dimensions** (`400 × 500` px). This ensures the
/// backend thumbnail worker stores correct values in the database, enabling
/// the frontend to compute an accurate aspect ratio for masonry layout without
/// any client-side heuristics.
///
/// Typographic metrics (ascender, descender, em-height) are stored separately
/// under `typographic_*` keys.
///
/// # Arguments
///
/// * `path` - The filesystem path to the font file.
///
/// # Returns
///
/// `AppResult<serde_json::Value>` - A JSON structure containing the extracted metadata.
pub fn extract_font_metadata(path: &Path) -> AppResult<serde_json::Value> {
    let file_data = std::fs::read(path).map_err(AppError::Io)?;
    let extension = path
        .extension()
        .and_then(|extension_str| extension_str.to_str())
        .unwrap_or("");
    let decompressed_font_data = decompress_font(&file_data, extension)?;

    let number_of_faces = ttf_parser::fonts_in_collection(&decompressed_font_data).unwrap_or(1);
    let mut faces_metadata = Vec::new();

    for face_index in 0..number_of_faces {
        if let Ok(face) = ttf_parser::Face::parse(&decompressed_font_data, face_index) {
            let mut font_family = None;
            let mut subfamily = None;
            let mut full_name = None;
            let mut postscript_name = None;

            for name in face.names() {
                if let Some(name_string) = name.to_string() {
                    if name.name_id == ttf_parser::name_id::FAMILY {
                        font_family = Some(name_string);
                    } else if name.name_id == ttf_parser::name_id::SUBFAMILY {
                        subfamily = Some(name_string);
                    } else if name.name_id == ttf_parser::name_id::FULL_NAME {
                        full_name = Some(name_string);
                    } else if name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME {
                        postscript_name = Some(name_string);
                    }
                }
            }

            let family = font_family
                .or(full_name.clone())
                .unwrap_or_else(|| "Unknown Font".to_string());

            faces_metadata.push(serde_json::json!({
                "face_index": face_index,
                "family": family,
                "subfamily": subfamily,
                "full_name": full_name,
                "postscript_name": postscript_name,
                "is_regular": face.is_regular(),
                "is_bold": face.is_bold(),
                "is_italic": face.is_italic(),
                "is_oblique": face.is_oblique(),
                "weight": format!("{:?}", face.weight()),
                "number_of_glyphs": face.number_of_glyphs(),
                "units_per_em": face.units_per_em(),
                "typographic_ascender": face.ascender(),
                "typographic_descender": face.descender(),
                "typographic_height": face.height(),
            }));
        }
    }

    if faces_metadata.is_empty() {
        return Err(AppError::Generic(
            "No valid font faces found in file".into(),
        ));
    }

    let mut main_metadata = faces_metadata[0].clone();
    if number_of_faces > 1 {
        main_metadata["faces"] = serde_json::json!(faces_metadata);
        main_metadata["face_count"] = serde_json::json!(number_of_faces);
    }

    // Expose thumbnail canvas dimensions as the canonical width/height so the
    // backend worker stores them in the DB and the frontend can derive the
    // correct aspect ratio for masonry layout without client-side heuristics.
    main_metadata["width"] = serde_json::json!(THUMBNAIL_CANVAS_WIDTH);
    main_metadata["height"] = serde_json::json!(THUMBNAIL_CANVAS_HEIGHT);

    Ok(main_metadata)
}

/// Generates a WebP thumbnail from the font file.
///
/// # Arguments
///
/// * `path` - The filesystem path to the font file.
/// * `size_hint` - Requested pixel dimension for the thumbnail.
///
/// # Returns
///
/// `AppResult<Vec<u8>>` - The encoded WebP thumbnail bytes.
pub fn generate_font_thumbnail(path: &Path, size_hint: u32) -> AppResult<Vec<u8>> {
    let file_data = std::fs::read(path).map_err(AppError::Io)?;
    let extension = path
        .extension()
        .and_then(|extension_str| extension_str.to_str())
        .unwrap_or("");
    let decompressed_font_data = decompress_font(&file_data, extension)?;

    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_source(usvg::fontdb::Source::Binary(Arc::new(
        decompressed_font_data.clone(),
    )));

    let face = fontdb
        .faces()
        .next()
        .ok_or_else(|| AppError::Generic("No font faces found in database".into()))?;

    let family_name = face
        .families
        .first()
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| face.post_script_name.clone());

    let parser_face = ttf_parser::Face::parse(&decompressed_font_data, 0)
        .map_err(|error| AppError::Generic(format!("Font parse error: {:?}", error)))?;

    let has_latin_lowercase_a = has_glyph_defined(&parser_face, 'a');
    let has_latin_uppercase_a = has_glyph_defined(&parser_face, 'A');

    let (main_preview, row_one, row_two, row_three) =
        if has_latin_lowercase_a || has_latin_uppercase_a {
            (
                "Aa".to_string(),
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
                "abcdefghijklmnopqrstuvwxyz".to_string(),
                "0123456789".to_string(),
            )
        } else {
            let mut symbol_characters = Vec::new();
            if let Some(cmap) = parser_face.tables().cmap {
                for subtable in cmap.subtables {
                    if subtable.is_unicode() {
                        // Try Private Use Area (PUA) first
                        for codepoint in 0xE000..=0xF8FF {
                            if let Some(glyph_id) = subtable.glyph_index(codepoint) {
                                if glyph_id.0 > 0 {
                                    if let Some(character) = std::char::from_u32(codepoint) {
                                        symbol_characters.push(character);
                                        if symbol_characters.len() >= 26 {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        // Try Dingbats / Symbols if PUA was insufficient
                        if symbol_characters.len() < 10 {
                            for codepoint in 0x2600..=0x27BF {
                                if let Some(glyph_id) = subtable.glyph_index(codepoint) {
                                    if glyph_id.0 > 0 {
                                        if let Some(character) = std::char::from_u32(codepoint) {
                                            symbol_characters.push(character);
                                            if symbol_characters.len() >= 26 {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Try printable ASCII if still insufficient
                        if symbol_characters.len() < 10 {
                            for codepoint in 0x21..=0x7E {
                                if let Some(glyph_id) = subtable.glyph_index(codepoint) {
                                    if glyph_id.0 > 0 {
                                        if let Some(character) = std::char::from_u32(codepoint) {
                                            symbol_characters.push(character);
                                            if symbol_characters.len() >= 26 {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !symbol_characters.is_empty() {
                        break;
                    }
                }
            }

            if symbol_characters.is_empty() {
                (
                    "Aa".to_string(),
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
                    "abcdefghijklmnopqrstuvwxyz".to_string(),
                    "0123456789".to_string(),
                )
            } else {
                let main_char = symbol_characters.iter().take(2).collect::<String>();

                let mut characters_chunk_one = String::new();
                let mut characters_chunk_two = String::new();
                let mut characters_chunk_three = String::new();

                for (index, symbol) in symbol_characters.iter().enumerate() {
                    if index < 8 {
                        characters_chunk_one.push(*symbol);
                        characters_chunk_one.push(' ');
                    } else if index < 16 {
                        characters_chunk_two.push(*symbol);
                        characters_chunk_two.push(' ');
                    } else {
                        characters_chunk_three.push(*symbol);
                        characters_chunk_three.push(' ');
                    }
                }
                if characters_chunk_one.is_empty() {
                    characters_chunk_one = " ".to_string();
                }
                if characters_chunk_two.is_empty() {
                    characters_chunk_two = " ".to_string();
                }
                if characters_chunk_three.is_empty() {
                    characters_chunk_three = " ".to_string();
                }

                (
                    main_char,
                    characters_chunk_one.trim().to_string(),
                    characters_chunk_two.trim().to_string(),
                    characters_chunk_three.trim().to_string(),
                )
            }
        };

    let options = usvg::Options {
        fontdb: Arc::new(fontdb),
        ..Default::default()
    };

    let safe_family = family_name
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;");
    let safe_main = main_preview
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;");
    let safe_row_one = row_one
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;");
    let safe_row_two = row_two
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;");
    let safe_row_three = row_three
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;");

    let svg_content = FONT_SVG_TEMPLATE
        .replace("{family}", &safe_family)
        .replace("{main_preview}", &safe_main)
        .replace("{row1}", &safe_row_one)
        .replace("{row2}", &safe_row_two)
        .replace("{row3}", &safe_row_three);

    let tree = usvg::Tree::from_str(&svg_content, &options)
        .map_err(|error| AppError::Generic(format!("SVG parse error: {}", error)))?;

    let size = tree.size();
    let width = size.width();
    let height = size.height();

    if width == 0.0 || height == 0.0 {
        return Err(AppError::Generic("Invalid SVG dimensions".into()));
    }

    let scale = if width > height {
        size_hint as f32 / width
    } else {
        size_hint as f32 / height
    };

    let target_width = (width * scale).ceil() as u32;
    let target_height = (height * scale).ceil() as u32;

    let mut pixmap = Pixmap::new(target_width, target_height)
        .ok_or_else(|| AppError::Generic("Failed to create pixmap".into()))?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let encoder = webp::Encoder::from_rgba(pixmap.data(), target_width, target_height);
    let webp_data = encoder.encode(90.0);
    Ok(webp_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_decompress_font_non_compressed() -> Result<(), Box<dyn std::error::Error>> {
        let dummy_data = vec![1, 2, 3, 4];
        let result_data = decompress_font(&dummy_data, "ttf")?;
        assert_eq!(result_data, dummy_data);

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_ttf() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/ttf/Lato-Regular.ttf");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_ttf")?,
                "Lato"
            );
            assert!(!extracted_metadata["is_bold"]
                .as_bool()
                .ok_or("Expected a value in test_font_metadata_and_thumbnail_ttf")?);
            assert!(!extracted_metadata["is_italic"]
                .as_bool()
                .ok_or("Expected a value in test_font_metadata_and_thumbnail_ttf")?);

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_symbol_ttf() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/ttf/fontawesome-webfont.ttf");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_symbol_ttf")?,
                "FontAwesome"
            );

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_otf() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/otf/Lato-Regular.otf");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_otf")?,
                "Lato"
            );

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_ttc() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/ttc/SuperClarendon.ttc");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_ttc")?,
                "Superclarendon"
            );
            assert!(
                extracted_metadata["face_count"]
                    .as_u64()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_ttc")?
                    > 1
            );

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_woff() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/woff/Lato-Regular.woff");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_woff")?,
                "Lato"
            );

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_font_metadata_and_thumbnail_woff2() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Font/woff2/OpenSans-BoldItalic.woff2");
        if file_path.exists() {
            let extracted_metadata = extract_font_metadata(file_path)?;
            assert_eq!(
                extracted_metadata["family"]
                    .as_str()
                    .ok_or("Expected a value in test_font_metadata_and_thumbnail_woff2")?,
                "Open Sans"
            );

            let generated_thumbnail = generate_font_thumbnail(file_path, 256)?;
            assert!(!generated_thumbnail.is_empty());
        }

        Ok(())
    }
}
