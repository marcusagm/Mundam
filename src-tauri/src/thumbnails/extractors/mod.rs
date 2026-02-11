pub mod binary_jpeg;

use std::path::Path;
use std::io::Read;
use image::ImageEncoder;

/// Central registry for on-the-fly preview extraction.
/// Used by both the thumbnail worker and the custom protocol handler.
pub fn extract_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Affinity Suite
        "afphoto" | "afdesign" | "afpub" => {
            let data = super::affinity::extract_largest_png(path)?;
            Ok((data, "image/png".to_string()))
        },

        // ZIP-based Project Previews (Clip Studio, XMind)
        "clip" | "xmind" => {
            let data = extract_zip_preview(path)?;
            Ok((data, "image/png".to_string()))
        },

        // RAW Photos - Delegated to rsraw (via ThumbnailStrategy::Raw)
        // Kept here only if NativeExtractor strategy is explicitly requested, but definitions.rs now uses Raw strategy.
        // We remove them to avoid confusion if NativeExtractor is accidentally used.

        // Adobe Photoshop
        "psd" | "psb" => {
            if let Ok(data) = extract_psd_composite(path) {
                return Ok((data, "image/png".to_string()));
            }
            // Fallback to binary scanner
            let data = binary_jpeg::extract_embedded_jpeg(path)?;
            Ok((data, "image/jpeg".to_string()))
        },

        // Adobe Illustrator & EPS (version 9+ are PDFs)
        "ai" | "eps" => {
            let data = extract_ai_pdf(path)?;
            Ok((data, "application/pdf".to_string()))
        },

        // 3D Models with embedded previews
        "blend" => {
            let data = binary_jpeg::extract_embedded_jpeg(path)?;
            Ok((data, "image/jpeg".to_string()))
        },

        // Specialized formats that browsers can't render but 'image' crate can
        "tga" | "tiff" | "tif" | "exr" | "hdr" | "dds" | "pbm" | "pgm" | "ppm" | "pnm" | "pam" => {
            let data = convert_to_png(path)?;
            Ok((data, "image/png".to_string()))
        },

        _ => Err("No native extractor for this format".into()),
    }
}

/// Helper to extract a preview from a ZIP-based file.
fn extract_zip_preview(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let candidates = [
        "preview.png",
        "Thumbnails/thumbnail.png",
        "Thumbnail/thumbnail.png",
        "QuickLook/Preview.png",
        "QuickLook/Thumbnail.png",
        "icon.png",
    ];

    for name in candidates {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    Err("No preview found in zip archive".into())
}

/// Helper to convert a specialized image to PNG for browser display.
fn convert_to_png(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(png_data)
}

/// Extract the PDF stream from an Adobe Illustrator file.
fn extract_ai_pdf(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Search for "%PDF-" signature
    if let Some(start) = buffer.windows(5).position(|w| w == b"%PDF-") {
        // AI files often have XML metadata AFTER the PDF.
        // We can try to finding the "%%EOF" which marks the end of PDF.
        if let Some(end_rel) = buffer[start..].windows(5).rposition(|w| w == b"%%EOF") {
            let end = start + end_rel + 5;
            return Ok(buffer[start..end].to_vec());
        }
        // Fallback: just return everything from start
        return Ok(buffer[start..].to_vec());
    }

    Err("Not a PDF-compatible AI file".into())
}

/// Extract the composite image from a PSD file using the psd crate.
fn extract_psd_composite(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let psd = psd::Psd::from_bytes(&bytes).map_err(|e| format!("PSD parse error: {}", e))?;

    let rgba = psd.rgba();
    let width = psd.width() as u32;
    let height = psd.height() as u32;

    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);

    // Use image crate to encode RGBA to PNG
    image::codecs::png::PngEncoder::new(&mut cursor)
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("PNG encode error: {}", e))?;

    Ok(png_data)
}

/// Helper to generate a thumbnail from extracted preview data.
pub fn generate_thumbnail_extracted(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (data, mime) = extract_preview(input_path)?;

    // If it's a PDF, we cannot generate a thumbnail easily in native Rust without poppler/resvg-pdf.
    // For now, we return Error so it falls back to Icon, but the ItemView WILL work because it uses orig://
    if mime == "application/pdf" {
        return Err("Thumbnail generation for PDF not supported natively".into());
    }

    process_extracted_image(&data, output_path, size_px)
}

fn process_extracted_image(
    data: &[u8],
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use fast_image_resize as fr;

    let img = match image::load_from_memory(data) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("THUMB Error: Failed to decode extracted image data for {:?}: {}", output_path.file_name(), e);
            return Err(e.into());
        }
    };
    let width = img.width();
    let height = img.height();

    let aspect = width as f32 / height as f32;
    let (new_w, new_h) = if aspect > 1.0 {
        (size_px, (size_px as f32 / aspect).max(1.0) as u32)
    } else {
        (((size_px as f32 * aspect).max(1.0)) as u32, size_px)
    };

    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| e.to_string())?;

    let mut dst_image = fr::images::Image::new(new_w, new_h, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, None)
        .map_err(|e| e.to_string())?;

    let buffer = dst_image.buffer();
    crate::thumbnails::native::encode_webp_native(buffer, new_w, new_h, output_path)?;

    Ok(())
}
