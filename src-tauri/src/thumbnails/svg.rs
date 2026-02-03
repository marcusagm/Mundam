use std::path::Path;
use std::fs;
use resvg::usvg;
use tiny_skia::Pixmap;

pub fn generate_thumbnail_svg(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load SVG data
    let svg_data = fs::read(input_path).map_err(|e| format!("Failed to read SVG: {}", e))?;
    
    // 2. Parse SVG options
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).map_err(|e| format!("SVG parse error: {}", e))?;
    
    // Note: convert_text is no longer needed/available on Tree directly in newer usvg
    // Text is converted during parsing or rendering depending on version.
    // For 0.44+, simple text is handled. Complex text needs explicit loading if separate.

    // 3. Calculate scale to fit size_px
    let size = tree.size(); // ViewBox size
    let width = size.width();
    let height = size.height();
    
    if width == 0.0 || height == 0.0 {
        return Err("Invalid SVG dimensions".into());
    }
    
    let scale = if width > height {
        size_px as f32 / width
    } else {
        size_px as f32 / height
    };
    
    // 4. Render
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    
    let target_width = (width * scale).ceil() as u32;
    let target_height = (height * scale).ceil() as u32;

    let mut pixmap = Pixmap::new(target_width, target_height)
        .ok_or("Failed to create pixmap buffer")?;
        
    resvg::render(
        &tree,
        transform,
        &mut pixmap.as_mut()
    );

    // 5. Encode to WebP
    // tiny-skia produces RGBA8 (premultiplied?). resvg docs say standard RGBA8 usually.
    // The webp encoder expects [u8] RGBA.
    
    // We can use the webp crate directly.
    // Safety: pixmap.data() is guaranteed to be correct size.
    
    let encoder = webp::Encoder::from_rgba(
        pixmap.data(),
        target_width,
        target_height,
    );
    
    let webp_data = encoder.encode(80.0);
    std::fs::write(output_path, &*webp_data)?;

    Ok(())
}
