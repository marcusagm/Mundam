//! Adobe Illustrator (.ai) preview extractor.
//!
//! Ported from V1 backend.

use crate::processing::media::extractors::binary_jpeg;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn extract_ai_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    if let Ok(pdf_stream) = extract_ai_pdf_stream(path) {
        return Ok((pdf_stream, "application/pdf".to_string()));
    }
    if let Ok(preview_data) = binary_jpeg::extract_xmp_thumbnail(path) {
        return Ok((preview_data, "image/png".to_string()));
    }
    if let Ok(preview_result) = binary_jpeg::extract_any_embedded(path) {
        return Ok(preview_result);
    }
    Err("No preview found in AI file".into())
}

pub fn extract_ai_pdf_stream(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut file_buffer = Vec::new();
    file.read_to_end(&mut file_buffer)?;
    if let Some(start_index) = file_buffer.windows(5).position(|window| window == b"%PDF-") {
        if let Some(end_relative_index) = file_buffer[start_index..]
            .windows(5)
            .rposition(|window| window == b"%%EOF")
        {
            let end_index = start_index + end_relative_index + 5;
            return Ok(file_buffer[start_index..end_index].to_vec());
        }
        return Ok(file_buffer[start_index..].to_vec());
    }
    Err("Not a PDF-compatible AI file".into())
}

pub fn extract_ai_metadata(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut file_buffer = Vec::new();

    // Read up to 20MB for metadata parsing
    file.take(20 * 1024 * 1024).read_to_end(&mut file_buffer)?;

    let mut width = None;
    let mut height = None;
    let resolution_dpi = 72; // PostScript / PDF points per inch default resolution

    // 1. Search for /MediaBox in PDF-based files
    let mediabox_signature = b"/MediaBox";
    if let Some(mediabox_index) = file_buffer
        .windows(mediabox_signature.len())
        .position(|window| window == mediabox_signature)
    {
        let search_slice = &file_buffer[mediabox_index..];
        let end_bracket_index = search_slice.iter().position(|&byte| byte == b']');
        if let Some(bracket_index) = end_bracket_index {
            let box_slice = &search_slice[..bracket_index];
            if let Some(start_bracket_index) = box_slice.iter().position(|&byte| byte == b'[') {
                let numbers_slice = &box_slice[start_bracket_index + 1..];
                if let Ok(numbers_str) = std::str::from_utf8(numbers_slice) {
                    let coordinates: Vec<&str> = numbers_str.split_whitespace().collect();
                    if coordinates.len() >= 4 {
                        let lower_left_x_coordinate = coordinates[0].parse::<f64>().unwrap_or(0.0);
                        let lower_left_y_coordinate = coordinates[1].parse::<f64>().unwrap_or(0.0);
                        let upper_right_x_coordinate = coordinates[2].parse::<f64>().unwrap_or(0.0);
                        let upper_right_y_coordinate = coordinates[3].parse::<f64>().unwrap_or(0.0);
                        width = Some(
                            (upper_right_x_coordinate - lower_left_x_coordinate)
                                .abs()
                                .round() as u32,
                        );
                        height = Some(
                            (upper_right_y_coordinate - lower_left_y_coordinate)
                                .abs()
                                .round() as u32,
                        );
                    }
                }
            }
        }
    }

    // 2. Fallback to %%HiResBoundingBox or %%BoundingBox
    if width.is_none() || height.is_none() {
        let hi_res_boundingbox_signature = b"%%HiResBoundingBox:";
        if let Some(boundingbox_index) = file_buffer
            .windows(hi_res_boundingbox_signature.len())
            .position(|window| window == hi_res_boundingbox_signature)
        {
            let search_slice =
                &file_buffer[boundingbox_index + hi_res_boundingbox_signature.len()..];
            let end_line_index = search_slice
                .iter()
                .position(|&byte| byte == b'\n' || byte == b'\r');
            if let Some(line_index) = end_line_index {
                let numbers_slice = &search_slice[..line_index];
                if let Ok(numbers_str) = std::str::from_utf8(numbers_slice) {
                    let coordinates: Vec<&str> = numbers_str.split_whitespace().collect();
                    if coordinates.len() >= 4 {
                        let lower_left_x_coordinate = coordinates[0].parse::<f64>().unwrap_or(0.0);
                        let lower_left_y_coordinate = coordinates[1].parse::<f64>().unwrap_or(0.0);
                        let upper_right_x_coordinate = coordinates[2].parse::<f64>().unwrap_or(0.0);
                        let upper_right_y_coordinate = coordinates[3].parse::<f64>().unwrap_or(0.0);
                        width = Some(
                            (upper_right_x_coordinate - lower_left_x_coordinate)
                                .abs()
                                .round() as u32,
                        );
                        height = Some(
                            (upper_right_y_coordinate - lower_left_y_coordinate)
                                .abs()
                                .round() as u32,
                        );
                    }
                }
            }
        }
    }

    if width.is_none() || height.is_none() {
        let boundingbox_signature = b"%%BoundingBox:";
        if let Some(boundingbox_index) = file_buffer
            .windows(boundingbox_signature.len())
            .position(|window| window == boundingbox_signature)
        {
            let search_slice = &file_buffer[boundingbox_index + boundingbox_signature.len()..];
            let end_line_index = search_slice
                .iter()
                .position(|&byte| byte == b'\n' || byte == b'\r');
            if let Some(line_index) = end_line_index {
                let numbers_slice = &search_slice[..line_index];
                if let Ok(numbers_str) = std::str::from_utf8(numbers_slice) {
                    let coordinates: Vec<&str> = numbers_str.split_whitespace().collect();
                    if coordinates.len() >= 4 {
                        let lower_left_x_coordinate = coordinates[0].parse::<f64>().unwrap_or(0.0);
                        let lower_left_y_coordinate = coordinates[1].parse::<f64>().unwrap_or(0.0);
                        let upper_right_x_coordinate = coordinates[2].parse::<f64>().unwrap_or(0.0);
                        let upper_right_y_coordinate = coordinates[3].parse::<f64>().unwrap_or(0.0);
                        width = Some(
                            (upper_right_x_coordinate - lower_left_x_coordinate)
                                .abs()
                                .round() as u32,
                        );
                        height = Some(
                            (upper_right_y_coordinate - lower_left_y_coordinate)
                                .abs()
                                .round() as u32,
                        );
                    }
                }
            }
        }
    }

    // 3. Fallback to guess from XMP thumbnail image dimensions
    if width.is_none() || height.is_none() {
        if let Ok(preview_data) = binary_jpeg::extract_xmp_thumbnail(path) {
            if let Ok(reader) =
                image::ImageReader::new(std::io::Cursor::new(&preview_data)).with_guessed_format()
            {
                if let Ok((preview_width, preview_height)) = reader.into_dimensions() {
                    width = Some(preview_width);
                    height = Some(preview_height);
                }
            }
        }
    }

    let mut technical = serde_json::json!({
        "container": "Adobe Illustrator AI",
        "metadata_support": "Basic",
        "dpi": resolution_dpi,
    });

    if let Some(actual_width) = width {
        technical["width"] = serde_json::json!(actual_width);
    }
    if let Some(actual_height) = height {
        technical["height"] = serde_json::json!(actual_height);
    }

    Ok(serde_json::json!({
        "technical": technical,
        "semantic": {}
    }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DIRECTORY_PATH: &str = "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Project/ai";

    #[test]
    fn test_ai_metadata_sample() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new(TEST_DIRECTORY_PATH).join("sample.ai");
        if file_path.exists() {
            let metadata = extract_ai_metadata(&file_path)?;
            let technical_metadata = &metadata["technical"];
            assert_eq!(technical_metadata["width"], serde_json::json!(680));
            assert_eq!(technical_metadata["height"], serde_json::json!(680));
            assert_eq!(technical_metadata["dpi"], serde_json::json!(72));
        }

        Ok(())
    }

    #[test]
    fn test_ai_metadata_logo() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new(TEST_DIRECTORY_PATH).join("Logo.ai");
        if file_path.exists() {
            let metadata = extract_ai_metadata(&file_path)?;
            let technical_metadata = &metadata["technical"];
            assert_eq!(technical_metadata["width"], serde_json::json!(1920));
            assert_eq!(technical_metadata["height"], serde_json::json!(4259));
            assert_eq!(technical_metadata["dpi"], serde_json::json!(72));
        }

        Ok(())
    }

    #[test]
    fn test_ai_preview_sample() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new(TEST_DIRECTORY_PATH).join("sample.ai");
        if file_path.exists() {
            let (preview_data, mime_type) = extract_ai_preview(&file_path)?;
            assert!(!preview_data.is_empty(), "Preview data should not be empty");
            assert_eq!(mime_type, "application/pdf");
        }

        Ok(())
    }

    #[test]
    fn test_ai_metadata_nonexistent() {
        let nonexistent_path = Path::new("nonexistent_file_path.ai");
        let result = extract_ai_metadata(nonexistent_path);
        assert!(result.is_err());
    }
}
