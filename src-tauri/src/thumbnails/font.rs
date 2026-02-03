use std::path::Path;
use std::sync::Arc;
use resvg::usvg;
use tiny_skia::Pixmap;

const FONT_SVG_TEMPLATE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 500\">\
  <rect width=\"400\" height=\"500\" fill=\"#f8f9fa\"/>\
  <text x=\"200\" y=\"220\" font-family=\"{family}\" font-size=\"160\" text-anchor=\"middle\" fill=\"#1f2937\">Aa</text>\
  <text x=\"200\" y=\"330\" font-family=\"{family}\" font-size=\"32\" text-anchor=\"middle\" fill=\"#4b5563\">{family}</text>\
  <text x=\"200\" y=\"380\" font-family=\"{family}\" font-size=\"20\" text-anchor=\"middle\" fill=\"#9ca3af\">ABCDEFGHIJKLMNOPQRSTUVWXYZ</text>\
  <text x=\"200\" y=\"410\" font-family=\"{family}\" font-size=\"20\" text-anchor=\"middle\" fill=\"#9ca3af\">abcdefghijklmnopqrstuvwxyz</text>\
  <text x=\"200\" y=\"440\" font-family=\"{family}\" font-size=\"20\" text-anchor=\"middle\" fill=\"#9ca3af\">0123456789</text>\
</svg>";

/// Generates a thumbnail for a font file by rendering a sample SVG using the font itself.
pub fn generate_font_thumbnail(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup FontDB
    let mut fontdb = usvg::fontdb::Database::new();
    
    // Check if it's WOFF/WOFF2 and decode it using `wuff`
    let ext = input_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    
    if ext == "woff" {
        let data = std::fs::read(input_path)?;
        let decoded = wuff::decompress_woff1(&data)
            .map_err(|e| format!("WOFF1 decode failed: {:?}", e))?;
        fontdb.load_font_source(usvg::fontdb::Source::Binary(Arc::new(decoded)));
    } else if ext == "woff2" {
        let data = std::fs::read(input_path)?;
        let decoded = wuff::decompress_woff2(&data)
            .map_err(|e| format!("WOFF2 decode failed: {:?}", e))?;
        fontdb.load_font_source(usvg::fontdb::Source::Binary(Arc::new(decoded)));
    } else {
         fontdb.load_font_file(input_path).map_err(|e| format!("Failed to load font file: {}", e))?;
    }

    // 2. Identify the font family name
    // We take the last face added (or the first one found in the file).
    let face = fontdb.faces().last().ok_or("No font faces found in file")?;
    let family_name = face.families.first().map(|(name, _)| name.clone()).unwrap_or_else(|| face.post_script_name.clone());
    
    // 3. Prepare options with the custom fontdb
    let mut opt = usvg::Options::default();
    opt.fontdb = Arc::new(fontdb);

    // 4. Inject family name into SVG
    // Escape simple characters to avoid breaking SVG XML
    let safe_family = family_name.replace("\"", "&quot;").replace("'", "&apos;"); 
    let svg_content = FONT_SVG_TEMPLATE.replace("{family}", &safe_family);

    // 5. Parse SVG
    let tree = usvg::Tree::from_str(&svg_content, &opt)
        .map_err(|e| format!("SVG parse error: {}", e))?;

    // 6. Calculate scale and render
    let size = tree.size();
    let width = size.width();
    let height = size.height();
    
    if width == 0.0 || height == 0.0 {
        return Err("Invalid SVG dimensions".into());
    }

    // Scale to fit within size_px while maintaining aspect ratio (though our SVG is fixed aspect)
    let scale = if width > height {
        size_px as f32 / width
    } else {
        size_px as f32 / height
    };
    
    let target_width = (width * scale).ceil() as u32;
    let target_height = (height * scale).ceil() as u32;

    let mut pixmap = Pixmap::new(target_width, target_height)
        .ok_or("Failed to create pixmap")?;
    
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    
    resvg::render(
        &tree,
        transform,
        &mut pixmap.as_mut()
    );

    // 7. Encode to WebP
    let encoder = webp::Encoder::from_rgba(
        pixmap.data(),
        target_width,
        target_height,
    );
    
    // Use high quality for text
    let webp_data = encoder.encode(90.0); 
    std::fs::write(output_path, &*webp_data)?;

    Ok(())
}
