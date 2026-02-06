use std::fs::File;
use std::io::Read;
use std::path::Path;

const JPEG_SOI: &[u8; 2] = b"\xff\xd8";
const JPEG_EOI: &[u8; 2] = b"\xff\xd9";

/// Scans a file for the largest embedded JPEG.
/// This is highly effective for RAW files (ARW, CR2, NEF, DNG, etc).
pub fn extract_embedded_jpeg(input_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(input_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    // Limit scan to 20MB for performance on huge files.
    let scan_limit = 20 * 1024 * 1024; // 20MB
    let buffer_size = file_size.min(scan_limit) as usize;
    let mut buffer = vec![0u8; buffer_size];
    file.read_exact(&mut buffer)?;

    let mut best_jpeg: Option<(usize, usize)> = None; // (start, length)

    let mut i = 0;
    while i < buffer.len().saturating_sub(2) {
        // Find next SOI
        if let Some(soi_rel_pos) = buffer[i..].windows(2).position(|w| w == JPEG_SOI) {
            let start = i + soi_rel_pos;
            
            // Search for EOI starting from after SOI
            let j = start + 2;
            let eoi_search_limit = (j + 15 * 1024 * 1024).min(buffer.len());
            
            if let Some(eoi_rel_pos) = buffer[j..eoi_search_limit].windows(2).position(|w| w == JPEG_EOI) {
                let end = j + eoi_rel_pos + 2;
                let length = end - start;
                
                if best_jpeg.map_or(true, |(_, best_len)| length > best_len) {
                    best_jpeg = Some((start, length));
                }
                i = end;
                continue;
            }
            i = start + 2;
        } else {
            break;
        }
    }

    if let Some((start, length)) = best_jpeg {
        Ok(buffer[start..start + length].to_vec())
    } else {
        Err("No embedded JPEG found".into())
    }
}
