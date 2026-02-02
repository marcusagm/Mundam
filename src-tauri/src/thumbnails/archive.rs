use std::fs::File;
use std::io::Read;
use std::path::Path;
use fast_image_resize as fr;
use crate::thumbnails::native::encode_webp_native;

/// Extract preview from ZIP-based formats (Affinity, XMind, etc.)
pub fn generate_thumbnail_zip_preview(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    // Common preview paths in ZIP-based design files
    let preview_paths = [
        "preview.png",
        "Thumbnails/thumbnail.png", 
        "QuickLook/Preview.png",
        "QuickLook/Thumbnail.png",
        "icon.png",
    ];
    
    for preview_path in &preview_paths {
        if let Ok(mut entry) = archive.by_name(preview_path) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            
            // Decode the preview image
            let img = image::load_from_memory(&buf)?;
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
            encode_webp_native(buffer, new_w, new_h, output_path)?;
            
            return Ok(());
        }
    }
    
    Err("No preview found in archive".into())
}
