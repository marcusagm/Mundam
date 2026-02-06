use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PNG_SIGNATURE: &[u8; 8] = b"\x89\x50\x4e\x47\x0d\x0a\x1a\x0a";
const PNG_IEND: &[u8; 4] = b"IEND";

/// Extract the largest PNG preview from Affinity files (.afphoto, .afdesign, .afpub)
/// using binary signature scanning.
pub fn extract_largest_png(input_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(input_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    // Affinity files can be huge. We'll try to find the PNG at the end of the file first.
    // Most Affinity previews are stored in the last 15-20MB.
    let scan_size = 15 * 1024 * 1024; // 15MB
    let start_offset = if file_size > scan_size {
        file_size - scan_size
    } else {
        0
    };

    file.seek(SeekFrom::Start(start_offset))?;
    let mut buffer = Vec::with_capacity((file_size - start_offset) as usize);
    file.read_to_end(&mut buffer)?;

    let mut best_png: Option<(usize, usize)> = None; // (start, length)

    let mut i = 0;
    while i <= buffer.len().saturating_sub(8) {
        if &buffer[i..i + 8] == PNG_SIGNATURE {
            // Found a PNG signature!
            // Search limit: 50MB (like the JS script) or until end of buffer.
            let search_limit = (i + 50 * 1024 * 1024).min(buffer.len());
            
            if let Some(iend_rel_offset) = find_iend(&buffer[i + 8..search_limit]) {
                // The chunk ends after "IEND" (4 bytes) and CRC (4 bytes).
                let png_length = iend_rel_offset + 8 + 4 + 4; 
                
                // We want the largest PNG found (assuming it's the high-res one)
                let current_length = png_length;
                if best_png.map_or(true, |(_, best_len)| current_length > best_len) {
                    best_png = Some((i, current_length));
                }
                
                i += png_length;
                continue;
            }
        }
        i += 1;
    }

    if let Some((start, length)) = best_png {
        let end = (start + length).min(buffer.len());
        Ok(buffer[start..end].to_vec())
    } else {
        Err("No PNG preview found in Affinity file".into())
    }
}

fn find_iend(data: &[u8]) -> Option<usize> {
    // Search for "IEND" string
    data.windows(4).position(|window| window == PNG_IEND)
}

