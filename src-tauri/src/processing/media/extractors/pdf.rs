use pdfium_render::prelude::*;
use std::path::PathBuf;

/// Renders a PDF dynamic byte slice into PNG image bytes using the bundled pdfium library.
///
/// # Arguments
///
/// * `pdf_data` - The PDF bytes slice to render.
/// * `size_hint` - Target maximum width or height.
///
/// # Returns
///
/// `Result<Vec<u8>, Box<dyn std::error::Error>>` - The PNG formatted image bytes on success.
pub fn render_pdf_to_png(
    pdf_data: &[u8],
    size_hint: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let library_path = find_pdfium_library_path().ok_or("Could not locate libpdfium binary")?;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(library_path).or_else(|binding_error| {
            tracing::warn!(
                "Failed to bind to pdfium library, trying system library: {:?}",
                binding_error
            );
            Pdfium::bind_to_system_library()
        })?,
    );

    let document = pdfium.load_pdf_from_byte_slice(pdf_data, None)?;

    if document.pages().is_empty() {
        return Err("PDF document contains no pages".into());
    }
    let page = document.pages().get(0)?;

    let size_hint_i32 = size_hint as i32;
    let render_config = PdfRenderConfig::new()
        .set_target_width(size_hint_i32)
        .set_maximum_height(size_hint_i32);

    let dynamic_image = page.render_with_config(&render_config)?.as_image();

    let mut byte_buffer = std::io::Cursor::new(Vec::new());
    dynamic_image.write_to(&mut byte_buffer, image::ImageFormat::Png)?;
    Ok(byte_buffer.into_inner())
}

/// Extracts technical and semantic metadata from a PDF file using pdfium-render.
///
/// # Arguments
///
/// * `pdf_data` - The PDF bytes slice.
///
/// # Returns
///
/// A `Result` wrapping a `serde_json::Value` with technical and semantic metadata.
pub fn extract_pdf_metadata(
    pdf_data: &[u8],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let library_path = find_pdfium_library_path().ok_or("Could not locate libpdfium binary")?;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(library_path).or_else(|binding_error| {
            tracing::warn!(
                "Failed to bind to pdfium library, trying system library: {:?}",
                binding_error
            );
            Pdfium::bind_to_system_library()
        })?,
    );

    let document = pdfium.load_pdf_from_byte_slice(pdf_data, None)?;

    let pages_count = document.pages().len() as u32;
    let mut width = None;
    let mut height = None;

    if pages_count > 0 {
        if let Ok(first_page) = document.pages().get(0) {
            width = Some(first_page.width().value.round() as u32);
            height = Some(first_page.height().value.round() as u32);
        }
    }

    let metadata = document.metadata();

    let title = metadata
        .get(PdfDocumentMetadataTagType::Title)
        .map(|tag| tag.value().to_string());
    let author = metadata
        .get(PdfDocumentMetadataTagType::Author)
        .map(|tag| tag.value().to_string());
    let subject = metadata
        .get(PdfDocumentMetadataTagType::Subject)
        .map(|tag| tag.value().to_string());
    let keywords = metadata
        .get(PdfDocumentMetadataTagType::Keywords)
        .map(|tag| tag.value().to_string());
    let creator = metadata
        .get(PdfDocumentMetadataTagType::Creator)
        .map(|tag| tag.value().to_string());
    let producer = metadata
        .get(PdfDocumentMetadataTagType::Producer)
        .map(|tag| tag.value().to_string());
    let creation_date = metadata
        .get(PdfDocumentMetadataTagType::CreationDate)
        .map(|tag| tag.value().to_string());
    let modification_date = metadata
        .get(PdfDocumentMetadataTagType::ModificationDate)
        .map(|tag| tag.value().to_string());

    let mut technical_metadata = serde_json::json!({
        "container": "PDF",
        "metadata_support": "Standard",
        "pages_count": pages_count,
        "dpi": 72,
    });

    if let Some(actual_width) = width {
        technical_metadata["width"] = serde_json::json!(actual_width);
    }
    if let Some(actual_height) = height {
        technical_metadata["height"] = serde_json::json!(actual_height);
    }

    let mut semantic_metadata = serde_json::json!({});
    if let Some(actual_title) = title {
        semantic_metadata["title"] = serde_json::json!(actual_title);
    }
    if let Some(actual_author) = author {
        semantic_metadata["author"] = serde_json::json!(actual_author);
    }
    if let Some(actual_subject) = subject {
        semantic_metadata["subject"] = serde_json::json!(actual_subject);
    }
    if let Some(actual_keywords) = keywords {
        semantic_metadata["keywords"] = serde_json::json!(actual_keywords);
    }
    if let Some(actual_creator) = creator {
        semantic_metadata["creator"] = serde_json::json!(actual_creator);
    }
    if let Some(actual_producer) = producer {
        semantic_metadata["producer"] = serde_json::json!(actual_producer);
    }
    if let Some(actual_creation_date) = creation_date {
        semantic_metadata["creation_date"] = serde_json::json!(actual_creation_date);
    }
    if let Some(actual_modification_date) = modification_date {
        semantic_metadata["modification_date"] = serde_json::json!(actual_modification_date);
    }

    Ok(serde_json::json!({
        "technical": technical_metadata,
        "semantic": semantic_metadata
    }))
}

/// Locates PDFium in the Tauri resource bundle or in the development resources tree.
fn find_pdfium_library_path() -> Option<PathBuf> {
    let library_filename = if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    };

    if let Some(resource_dir) = std::env::var_os("MUNDAM_RESOURCE_DIR") {
        let path = PathBuf::from(resource_dir)
            .join("resources")
            .join("pdfium")
            .join(library_filename);
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(debug_assertions)]
    {
        let development_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("pdfium")
            .join(library_filename);
        if development_path.exists() {
            return Some(development_path);
        }
    }

    if let Ok(executable_path) = std::env::current_exe() {
        let mut current_directory = executable_path.parent();
        while let Some(path) = current_directory {
            let macos_resources_path = path
                .join("Resources")
                .join("resources")
                .join("pdfium")
                .join(library_filename);
            if macos_resources_path.exists() {
                return Some(macos_resources_path);
            }

            let bundled_path = path
                .join("resources")
                .join("pdfium")
                .join(library_filename);
            if bundled_path.exists() {
                return Some(bundled_path);
            }

            let check_path_without_binaries = path.join("pdfium").join(library_filename);
            if check_path_without_binaries.exists() {
                return Some(check_path_without_binaries);
            }

            current_directory = path.parent();
        }
    }

    if let Ok(current_working_directory) = std::env::current_dir() {
        let mut current_directory = Some(current_working_directory.as_path());
        while let Some(path) = current_directory {
            let bundled_path = path
                .join("resources")
                .join("pdfium")
                .join(library_filename);
            if bundled_path.exists() {
                return Some(bundled_path);
            }
            let check_path_without_binaries = path.join("pdfium").join(library_filename);
            if check_path_without_binaries.exists() {
                return Some(check_path_without_binaries);
            }
            current_directory = path.parent();
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const TEST_AI_PATH: &str = "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Project/ai";

    #[test]
    fn test_render_pdf_from_ai() {
        let file_path = Path::new(TEST_AI_PATH).join("sample.ai");
        if file_path.exists() {
            let pdf_data =
                crate::processing::media::extractors::ai::extract_ai_pdf_stream(&file_path)
                    .expect("Failed to extract PDF stream from AI file");
            assert!(!pdf_data.is_empty(), "PDF stream data should not be empty");

            let png_bytes = render_pdf_to_png(&pdf_data, 128).expect("Failed to render PDF to PNG");
            assert!(
                !png_bytes.is_empty(),
                "Rendered PNG bytes should not be empty"
            );
            assert_eq!(
                &png_bytes[0..8],
                b"\x89PNG\r\n\x1a\n",
                "Rendered data must have PNG signature"
            );
        }
    }

    #[test]
    fn test_extract_pdf_metadata_from_ai() {
        let file_path = Path::new(TEST_AI_PATH).join("sample.ai");
        if file_path.exists() {
            let pdf_data =
                crate::processing::media::extractors::ai::extract_ai_pdf_stream(&file_path)
                    .expect("Failed to extract PDF stream from AI file");
            assert!(!pdf_data.is_empty(), "PDF stream data should not be empty");

            let metadata = extract_pdf_metadata(&pdf_data).expect("Failed to extract PDF metadata");
            let technical = &metadata["technical"];
            assert_eq!(technical["container"], "PDF");
            assert_eq!(technical["pages_count"], 1);
            assert!(technical["width"].as_u64().is_some());
            assert!(technical["height"].as_u64().is_some());
        }
    }
}
