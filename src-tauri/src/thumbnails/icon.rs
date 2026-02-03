use std::path::Path;
use resvg::usvg;
use tiny_skia::Pixmap;

/// Icon category for unsupported file types
#[derive(Debug, Clone, Copy)]
pub enum IconCategory {
    File3D,
    Font,
    Design,
    Code,
    Video,
    Audio,
    Image,
    Archive,
    Generic,
}

/// Determine icon category based on file extension
fn get_icon_category(path: &Path) -> IconCategory {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        // 3D formats
        "c4d" | "3ds" | "obj" | "fbx" | "blend" | "stl" | "dae" | 
        "skp" | "dwg" | "dxf" | "max" | "lwo" | "lws" | "ma" | "mb" | "gltf" | "glb" => {
            IconCategory::File3D
        }
        
        // Font formats
        "ttf" | "otf" | "ttc" | "woff" | "woff2" | "eot" | "fon" | "fnt" => {
            IconCategory::Font
        }
        
        // Design formats
        "cdr" | "indd" | "xd" | "fig" | "sketch" | "ai" | "eps" | "psd" | "afdesign" | "afphoto" | "afpub" => {
            IconCategory::Design
        }

        // Code/Web
        "html" | "css" | "js" | "ts" | "jsx" | "tsx" | "json" | "xml" | "svg" | "py" | "rs" | "go" | "c" | "cpp" | "java" => {
            IconCategory::Code
        }

        // Video
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "flv" | "wmv" => {
            IconCategory::Video
        }

        // Audio
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => {
            IconCategory::Audio
        }

        // Archive
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => {
            IconCategory::Archive
        }

        // Images (that fell back to icon)
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw" => {
            IconCategory::Image
        }
        
        _ => IconCategory::Generic,
    }
}

fn get_category_color(category: IconCategory) -> String {
    match category {
        IconCategory::Design => "rgb(234, 76, 137)".to_string(), // Dribbble pinkish
        IconCategory::File3D => "rgb(50, 50, 180)".to_string(), // Deep Blue-Purple
        IconCategory::Code => "rgb(44, 62, 80)".to_string(),    // Dark Slate
        IconCategory::Video => "rgb(229, 9, 20)".to_string(),   // Netflix Red-ish
        IconCategory::Audio => "rgb(30, 215, 96)".to_string(),  // Spotify Green-ish
        IconCategory::Image => "rgb(0, 160, 169)".to_string(),  // Teal (Default)
        IconCategory::Font => "rgb(80, 80, 80)".to_string(),    // Dark Gray
        IconCategory::Archive => "rgb(255, 165, 0)".to_string(), // Orange
        IconCategory::Generic => "rgb(120, 120, 120)".to_string(), // Grey
    }
}

// SVG Template from docs/idea/logo/file-icon.svg
const SVG_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 500" xmlns:bx="https://boxy-svg.com">
  <path d="M 382.426 17.574 C 392.734 27.882 399.563 44.15 400 60 L 400 440 C 399.564 455.85 392.735 472.118 382.426 482.426 C 372.118 492.734 355.85 499.563 340 500 L 60 500 C 44.15 499.564 27.882 492.735 17.574 482.426 C 7.266 472.118 0.437 455.85 0 440 L 0 60 C 0.436 44.15 7.265 27.882 17.574 17.574 C 27.882 7.266 44.15 0.437 60 0 L 340 0 C 355.85 0.436 372.118 7.265 382.426 17.574 Z" style="stroke-linecap: round; stroke-linejoin: round; stroke-width: 20px; paint-order: fill; fill: rgb(0, 160, 169);" id="background"/>
  <path d="M 60 0 L 340 0 C 355.85 0.436 372.118 7.265 382.426 17.574 C 392.734 27.882 399.563 44.15 400 60 L 400 440 C 399.564 455.85 392.735 472.118 382.426 482.426 C 372.118 492.734 355.85 499.563 340 500 L 60 500 C 44.15 499.564 27.882 492.735 17.574 482.426 C 7.266 472.118 0.437 455.85 0 440 L 0 60 C 0.436 44.15 7.265 27.882 17.574 17.574 C 27.882 7.266 44.15 0.437 60 0 Z M 31.716 31.716 C 23.928 39.504 19.564 48.236 20 60 L 20 440 C 19.564 451.764 23.928 460.496 31.716 468.284 C 39.504 476.072 48.236 480.436 60 480 L 340 480 C 351.764 480.436 360.496 476.072 368.284 468.284 C 376.072 460.496 380.436 451.764 380 440 L 380 60 C 380.436 48.236 376.072 39.504 368.284 31.716 C 360.496 23.928 351.764 19.564 340 20 L 60 20 C 48.236 19.564 39.504 23.928 31.716 31.716 Z" style="mix-blend-mode: color-burn; fill: rgb(128, 128, 128);"/>
  <g id="content" style="mix-blend-mode: color-burn;">
    <text style="fill: rgb(128, 128, 128); font-family: Poppins, sans-serif; font-weight: bold; font-size: 80px; text-anchor: middle;" id="extension" x="200" y="440">.generic</text>
    <g transform="matrix(0.802483, -0.463314, 0.463314, 0.802483, -41.736465, -2.234888)" style="filter: none; transform-origin: 244.285px 216.65px;" id="logo">
      <path d="M 81.325 172.932 L 155.054 130.364 L 155.054 248.022 C 155.49 258.538 160.157 269.976 166.969 276.788 C 173.782 283.6 185.22 288.267 195.735 288.703 L 214.26 288.703 L 164.689 317.323 C 159.34 320.916 154.859 320.96 148.996 319.389 C 143.133 317.818 139.275 315.539 136.438 309.753 L 73.755 201.183 C 70.162 195.834 70.118 191.353 71.689 185.49 C 73.26 179.627 75.539 175.769 81.325 172.932 Z" style="fill-rule: nonzero; stroke-width: 20; stroke-linejoin: round; stroke-linecap: round; fill: rgb(128, 128, 128);"/>
      <path d="M 195.735 101.975 L 283.755 101.975 C 290.184 101.538 294.087 103.74 298.379 108.032 C 299.49 109.143 300.46 110.227 301.281 111.336 C 295.053 114.98 289.29 120.43 285.447 126.493 L 222.764 235.063 C 217.884 244.387 216.207 256.626 218.7 265.933 C 218.947 266.853 219.243 267.778 219.585 268.703 L 195.735 268.703 C 189.306 269.14 185.403 266.938 181.111 262.646 C 176.819 258.354 174.618 254.451 175.054 248.022 L 175.054 122.656 C 174.617 116.227 176.819 112.324 181.111 108.032 C 185.403 103.74 189.306 101.539 195.735 101.975 Z" style="fill-rule: nonzero; stroke-width: 20px; stroke-linejoin: round; stroke-linecap: round; fill: rgb(128, 128, 128);"/>
      <path d="M 416.882 185.491 C 418.453 191.354 418.409 195.835 414.816 201.184 L 352.133 309.754 C 349.296 315.54 345.438 317.819 339.575 319.39 C 333.712 320.961 329.231 320.917 323.882 317.324 L 247.654 273.314 C 241.869 270.477 239.589 266.619 238.018 260.756 C 236.447 254.893 236.492 250.412 240.085 245.063 L 302.768 136.493 C 305.604 130.707 309.462 128.428 315.325 126.857 C 321.188 125.286 325.669 125.33 331.018 128.923 L 407.246 172.933 C 413.032 175.77 415.311 179.628 416.882 185.491 Z" style="fill-rule: nonzero; stroke-width: 20; stroke-linejoin: round; stroke-linecap: round; fill: rgb(128, 128, 128);"/>
    </g>
  </g>
</svg>"#;

/// Get or generate a shared generic file icon
///
/// Returns the relative path to the icon file (extensions/icon_<ext>.webp)
pub fn get_or_generate_icon(
    input_path: &Path,
    thumbnails_dir: &Path,
    size_px: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("generic")
        .to_lowercase();
    
    // We store icons in a subfolder "extensions" to separate them from file hashes
    let icons_dir = thumbnails_dir.join("extensions");
    if !icons_dir.exists() {
        std::fs::create_dir_all(&icons_dir)?;
    }

    let icon_filename = format!("icon_{}.webp", ext);
    let icon_path = icons_dir.join(&icon_filename);
    
    // Return the relative path string that will be stored in DB
    // e.g., "extensions/icon_pdf.webp"
    // Since Windows uses backslash, we should ensure forward slash for DB consistency if possible,
    // but the app handles PathBuf join, so reusing the returned string is tricky if it has separators.
    // The DB stores whatever we return here. The frontend thumb:// protocol just needs valid relative or absolute.
    // Actually, `thumb_handler` probably joins `thumbnails_dir` with the DB string.
    let relative_path_string = format!("extensions/{}", icon_filename);

    if icon_path.exists() {
        return Ok(relative_path_string);
    }
    
    // Generate logical (Code from previous generate_thumbnail_icon)
    let category = get_icon_category(input_path);
    let color = get_category_color(category);
    
    // Customize the SVG template
    // 1. Replace background color
    let mut svg_content = SVG_TEMPLATE.replace("rgb(0, 160, 169)", &color);
    
    // 2. Replace extension text
    let display_ext = if ext.len() > 4 {
        format!(".{}", &ext[0..4])
    } else {
        format!(".{}", ext)
    };
    svg_content = svg_content.replace(".generic", &display_ext);

    // Render using resvg
    let opt = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    
    let tree = usvg::Tree::from_str(&svg_content, &opt)
        .map_err(|e| format!("SVG template parse error: {}", e))?;
        
    let size = tree.size();
    let width = size.width();
    let height = size.height();
    
    if width == 0.0 || height == 0.0 {
        return Err("Invalid SVG dimensions in template".into());
    }
    
    let scale = if width > height {
        size_px as f32 / width
    } else {
        size_px as f32 / height
    };
    
    let mut pixmap = Pixmap::new(size_px, size_px)
        .ok_or("Failed to create pixmap buffer")?;
        
    let scaled_width = width * scale;
    let scaled_height = height * scale;
    let x_offset = (size_px as f32 - scaled_width) / 2.0;
    let y_offset = (size_px as f32 - scaled_height) / 2.0;
    
    let transform = tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(x_offset, y_offset);

    resvg::render(
        &tree,
        transform,
        &mut pixmap.as_mut()
    );
    
    // Encode to WebP
    let encoder = webp::Encoder::from_rgba(
        pixmap.data(),
        size_px,
        size_px,
    );
    let webp_data = encoder.encode(85.0);
    std::fs::write(&icon_path, &*webp_data)?;
    
    Ok(relative_path_string)
}
