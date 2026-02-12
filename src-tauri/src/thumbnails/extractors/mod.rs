pub mod binary_jpeg;

use std::path::Path;
use std::io::Read;
use image::ImageEncoder;
use tauri::{AppHandle, Runtime};

/// Central registry for on-the-fly preview extraction.
pub fn extract_preview<R: Runtime>(app_handle: Option<&AppHandle<R>>, path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let format = crate::formats::FileFormat::detect(path)
        .ok_or_else(|| "Unsupported format")?;

    match format.preview_strategy {
        crate::formats::PreviewStrategy::BrowserNative => {
             Err("Browser native format - serve directly".into())
        },

        crate::formats::PreviewStrategy::Raw => {
            if let Ok(data) = extract_raw_preview(path) {
                return Ok((data, "image/jpeg".to_string()));
            }
            if let Ok(data) = binary_jpeg::extract_embedded_jpeg(path) {
                return Ok((data, "image/jpeg".to_string()));
            }
            if let Ok(data) = extract_ffmpeg_frame(app_handle, path) {
                return Ok((data, "image/jpeg".to_string()));
            }
            Err("Failed all RAW preview extraction methods".into())
        },

        crate::formats::PreviewStrategy::Ffmpeg => {
            if let Ok(data) = extract_ffmpeg_frame(app_handle, path) {
                return Ok((data, "image/jpeg".to_string()));
            }
            let data = convert_to_png(path)?;
            Ok((data, "image/png".to_string()))
        },

        crate::formats::PreviewStrategy::NativeExtractor => {
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
                // Adobe Photoshop
                "psd" | "psb" => {
                    if let Ok(data) = extract_psd_composite(path) {
                        return Ok((data, "image/png".to_string()));
                    }
                    // Fallback to binary scanner
                    let (data, mime) = binary_jpeg::extract_any_embedded(path)?;
                    Ok((data, mime))
                },
                // Adobe Illustrator (PDF-based)
                "ai" => {
                    // Try PDF stream first (most common for modern AI)
                    if let Ok(data) = extract_ai_pdf(path) {
                         return Ok((data, "application/pdf".to_string()));
                    }
                    // Fallback to binary scanner for very old AI or those without PDF compat
                    let (data, mime) = binary_jpeg::extract_any_embedded(path)?;
                    if mime == "image/tiff" {
                        if let Ok(png) = convert_to_png_from_memory(&data) {
                            return Ok((png, "image/png".to_string()));
                        }
                    }
                    Ok((data, mime))
                },
                // Encapsulated PostScript
                "eps" => {
                    // Priority 0: Official Binary EPS Header (Pointer-based TIFF)
                    if let Ok((data, mime)) = binary_jpeg::extract_eps_binary_pointer(path) {
                        if mime == "image/tiff" {
                            if let Ok(png) = convert_to_png_from_memory(&data) {
                                return Ok((png, "image/png".to_string()));
                            }
                        }
                        return Ok((data, mime));
                    }
                    // Priority 1: Fast Binary Scanner & XMP
                    if let Ok((data, mime)) = binary_jpeg::extract_any_embedded(path) {
                        if mime == "image/tiff" {
                             if let Ok(png) = convert_to_png_from_memory(&data) {
                                 return Ok((png, "image/png".to_string()));
                             }
                        }
                        return Ok((data, mime));
                    }
                    // Priority 2: FFmpeg (Ghostscript) for rendering vector EPS
                    if let Ok(data) = extract_ffmpeg_frame(app_handle, path) {
                        return Ok((data, "image/jpeg".to_string()));
                    }
                    // Priority 3: Try to see if it's a PDF wrapper (rare but happens)
                    if let Ok(data) = extract_ai_pdf(path) {
                        return Ok((data, "application/pdf".to_string()));
                    }
                    Err("No preview found in EPS".into())
                },
                // ZIP-based Project Previews
                "clip" | "xmind" => {
                    let data = extract_zip_preview(path)?;
                    Ok((data, "image/png".to_string()))
                },
                "blend" | "xcf" => {
                    let (data, mime) = binary_jpeg::extract_any_embedded(path)?;
                    Ok((data, mime))
                },
                "hdr" | "exr" | "dds" => {
                    if let Ok(data) = convert_to_png(path) {
                        return Ok((data, "image/png".to_string()));
                    }
                    if let Ok(data) = extract_ffmpeg_frame(app_handle, path) {
                        return Ok((data, "image/jpeg".to_string()));
                    }
                    let (data, mime) = binary_jpeg::extract_any_embedded(path)?;
                    Ok((data, mime))
                },
                _ => Err("No native extractor for this extension".into()),
            }
        },

        crate::formats::PreviewStrategy::Convert => {
            let data = convert_to_png(path)?;
            Ok((data, "image/png".to_string()))
        },

        crate::formats::PreviewStrategy::None => Err("No preview strategy for this format".into()),
    }
}

/// Helper to extract a preview from a ZIP-based file.
fn extract_zip_preview(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let candidates = [
        "previews/preview.png",
        "Previews/preview.png",
        "Canvas/thumbnail.png",
        "Thumbnails/thumbnail.png",
        "Thumbnail/thumbnail.png",
        "QuickLook/Preview.png",
        "QuickLook/Thumbnail.png",
        "preview.png",
        "thumbnail.png",
        "icon.png",
    ];

    for name in candidates {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_lowercase();
        if entry_name.ends_with("preview.png") || entry_name.ends_with("thumbnail.png") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    Err("No preview found in zip archive".into())
}

fn convert_to_png(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let sdr_img = img.to_rgb8();
    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    sdr_img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(png_data)
}

fn extract_ai_pdf(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if let Some(start) = buffer.windows(5).position(|w| w == b"%PDF-") {
        if let Some(end_rel) = buffer[start..].windows(5).rposition(|w| w == b"%%EOF") {
            let end = start + end_rel + 5;
            return Ok(buffer[start..end].to_vec());
        }
        return Ok(buffer[start..].to_vec());
    }

    Err("Not a PDF-compatible AI file".into())
}

fn extract_psd_composite(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let psd = psd::Psd::from_bytes(&bytes).map_err(|e| format!("PSD parse error: {}", e))?;

    let rgba = psd.rgba();
    let width = psd.width() as u32;
    let height = psd.height() as u32;

    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    image::codecs::png::PngEncoder::new(&mut cursor)
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("PNG encode error: {}", e))?;

    Ok(png_data)
}

/// Helper to generate a thumbnail from extracted preview data.
pub fn generate_thumbnail_extracted<R: Runtime>(
    app_handle: Option<&AppHandle<R>>,
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut data, mime) = extract_preview(app_handle, input_path)?;

    // If it's a PDF (AI/EPS), we use PDFium for high-quality multiplatform rendering.
    if mime == "application/pdf" {
        if let Ok(rendered_data) = crate::media::pdf::render_pdf_data_to_image(app_handle, &data, size_px) {
            data = rendered_data;
        } else if let Ok((embedded_data, _)) = binary_jpeg::extract_any_embedded(input_path) {
            data = embedded_data;
        } else if let Ok(rendered_data) = extract_ffmpeg_frame(app_handle, input_path) {
            data = rendered_data;
        } else {
            return Err("No PDF rendering or raster preview available for this file".into());
        }
    }

    process_extracted_image(&data, output_path, size_px)
}

fn process_extracted_image(
    data: &[u8],
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use fast_image_resize as fr;

    let img = image::load_from_memory(data)?;
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

fn extract_raw_preview(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    crate::thumbnails::raw::extract_raw_preview_data(path)
}

fn extract_ffmpeg_frame<R: Runtime>(app_handle: Option<&AppHandle<R>>, path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    crate::media::ffmpeg::extract_frame_to_memory(app_handle, path)
        .map_err(|e| e.into())
}

fn convert_to_png_from_memory(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(data)?;
    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img.to_rgb8().write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(png_data)
}
