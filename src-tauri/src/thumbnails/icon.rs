use std::path::Path;
use crate::thumbnails::native::encode_webp_native;

/// Icon category for unsupported file types
#[derive(Debug, Clone, Copy)]
pub enum IconCategory {
    File3D,
    Font,
    Design,
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
        "skp" | "dwg" | "dxf" | "max" | "lwo" | "lws" | "ma" | "mb" => {
            IconCategory::File3D
        }
        
        // Font formats
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "fon" | "fnt" => {
            IconCategory::Font
        }
        
        // Design formats (non-ZIP based)
        "cdr" | "indd" | "xd" | "fig" | "sketch" => {
            IconCategory::Design
        }
        
        _ => IconCategory::Generic,
    }
}

/// Generate icon-based thumbnail for unsupported formats
/// Creates a simple colored placeholder with file extension
pub fn generate_thumbnail_icon(
    input_path: &Path,
    output_path: &Path,
    size_px: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let category = get_icon_category(input_path);
    // let _ext = input_path
    //     .extension()
    //     .and_then(|e| e.to_str())
    //     .unwrap_or("?")
    //     .to_uppercase();
    
    // Create a simple colored thumbnail based on category
    // Using a solid color background with extension text
    let (bg_color, text_color) = match category {
        IconCategory::File3D => ([0x1a, 0x3a, 0x4e, 0xff], [0x6a, 0xaa, 0xee, 0xff]),
        IconCategory::Font => ([0x3e, 0x3a, 0x2a, 0xff], [0xca, 0xba, 0x8a, 0xff]),
        IconCategory::Design => ([0x3a, 0x2a, 0x4e, 0xff], [0xba, 0x9a, 0xde, 0xff]),
        IconCategory::Generic => ([0x2a, 0x2a, 0x3e, 0xff], [0x88, 0x88, 0xaa, 0xff]),
    };
    
    // Create image buffer (RGBA)
    let mut pixels = vec![0u8; (size_px * size_px * 4) as usize];
    
    // Fill with background color and rounded corners effect
    for y in 0..size_px {
        for x in 0..size_px {
            let idx = ((y * size_px + x) * 4) as usize;
            
            // Simple rounded corner check (corner radius ~10% of size)
            let corner_radius = (size_px / 10) as i32;
            let in_corner = is_in_rounded_corner(x as i32, y as i32, size_px as i32, corner_radius);
            
            if in_corner {
                // Transparent for corners
                pixels[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                pixels[idx..idx + 4].copy_from_slice(&bg_color);
            }
        }
    }
    
    // Draw a simple file icon shape in the center
    let icon_size = size_px * 60 / 100;
    let icon_x = (size_px - icon_size) / 2;
    let icon_y = (size_px - icon_size) / 2 - size_px / 10;
    draw_file_icon(&mut pixels, size_px, icon_x, icon_y, icon_size, &text_color);
    
    // Encode to WebP
    encode_webp_native(&pixels, size_px, size_px, output_path)?;
    
    Ok(())
}

/// Check if pixel is in a rounded corner region
fn is_in_rounded_corner(x: i32, y: i32, size: i32, radius: i32) -> bool {
    // Top-left corner
    if x < radius && y < radius {
        let dx = radius - x;
        let dy = radius - y;
        return dx * dx + dy * dy > radius * radius;
    }
    // Top-right corner
    if x >= size - radius && y < radius {
        let dx = x - (size - radius - 1);
        let dy = radius - y;
        return dx * dx + dy * dy > radius * radius;
    }
    // Bottom-left corner
    if x < radius && y >= size - radius {
        let dx = radius - x;
        let dy = y - (size - radius - 1);
        return dx * dx + dy * dy > radius * radius;
    }
    // Bottom-right corner
    if x >= size - radius && y >= size - radius {
        let dx = x - (size - radius - 1);
        let dy = y - (size - radius - 1);
        return dx * dx + dy * dy > radius * radius;
    }
    false
}

/// Draw a simple file icon shape
fn draw_file_icon(pixels: &mut [u8], img_size: u32, x: u32, y: u32, size: u32, color: &[u8; 4]) {
    let fold_size = size / 4;
    
    for py in 0..size {
        for px in 0..size {
            let abs_x = x + px;
            let abs_y = y + py;
            
            if abs_x >= img_size || abs_y >= img_size {
                continue;
            }
            
            // File body (simple rectangle with folded corner)
            let in_body = px < size && py < size;
            let in_fold = px >= size - fold_size && py < fold_size;
            
            if in_body && !in_fold {
                // Draw border (2px)
                let is_border = px < 2 || px >= size - 2 || py < 2 || py >= size - 2
                    || (px >= size - fold_size - 2 && py < fold_size);
                
                if is_border {
                    let idx = ((abs_y * img_size + abs_x) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(color);
                }
            }
        }
    }
}
