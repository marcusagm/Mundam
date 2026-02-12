use pdfium_render::prelude::*;
use image::DynamicImage;
use std::io::Cursor;
use tauri::Manager;

/// Renders a PDF (or AI with PDF stream) to a PNG image buffer.
/// Searches for PDFium in:
/// 1. Bundled resources (production/development)
/// 2. System library paths
pub fn render_pdf_data_to_image<R: tauri::Runtime>(
    app_handle: Option<&tauri::AppHandle<R>>,
    pdf_data: &[u8],
    size_px: u32
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {

    // 1. Try to find the bundled library
    let mut bindings = None;

    if let Some(handle) = app_handle {
        if let Ok(resource_dir) = handle.path().resource_dir() {
            let lib_name = Pdfium::pdfium_platform_library_name_at_path("./");
            let bundled_path = resource_dir
                .join("binaries")
                .join("pdfium")
                .join(&lib_name);

            if bundled_path.exists() {
                bindings = Pdfium::bind_to_library(bundled_path).ok();
            }
        }
    }

    // 2. Fallback to system library if not found in resources
    let bindings = match bindings {
        Some(b) => b,
        None => Pdfium::bind_to_system_library()
            .or_else(|_| Pdfium::bind_to_library("pdfium"))
            .map_err(|e| format!("PDFium library not found in resources or system: {}. Please ensure libpdfium is installed or bundled.", e))?
    };

    let pdfium = Pdfium::new(bindings);

    let document = pdfium.load_pdf_from_byte_vec(pdf_data.to_vec(), None)?;
    let pages = document.pages();
    if pages.is_empty() {
        return Err("PDF has no pages".into());
    }

    let first_page = pages.get(0)?;

    // Calculate dimensions maintaining aspect ratio
    let width = first_page.width().value;
    let height = first_page.height().value;
    let aspect = width / height;

    let (target_w, target_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect) as u32)
    } else {
        ((size_px as f32 * aspect) as u32, size_px)
    };

    // Render page to a bitmap
    let render_config = PdfRenderConfig::new()
        .set_target_width(target_w as i32)
        .set_maximum_height(target_h as i32)
        .rotate(PdfPageRenderRotation::None, false);

    let bitmap = first_page.render_with_config(&render_config)?;

    // pdfium-render returns BGRA8 bytes. We convert to RGBA for consistency.
    let mut rgba_data = bitmap.as_raw_bytes().to_vec();
    // Swap B and R channels (BGRA -> RGBA)
    for chunk in rgba_data.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let img = DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(target_w, target_h, rgba_data)
            .ok_or("Failed to create image from bitmap")?
    );

    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);
    img.write_to(&mut cursor, image::ImageFormat::Png)?;

    Ok(output)
}
