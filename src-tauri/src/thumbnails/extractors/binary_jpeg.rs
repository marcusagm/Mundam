use std::fs::File;
use std::io::Read;
use std::path::Path;
use memmap2::Mmap;

const JPEG_SOI: &[u8; 2] = b"\xff\xd8";
const JPEG_EOI: &[u8; 2] = b"\xff\xd9";
const PNG_HEADER: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const PNG_FOOTER: &[u8; 4] = b"IEND";

const TIFF_LE: &[u8; 4] = b"II\x2a\x00";
const TIFF_BE: &[u8; 4] = b"MM\x00\x2a";

/// Scans for any embedded image (JPEG, PNG or TIFF), returning the largest one.
pub fn extract_any_embedded(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };

    let mut best: Option<(Vec<u8>, String)> = None;

    if let Ok(data) = scan_mmap_for_jpeg(&mmap) {
        best = Some((data, "image/jpeg".to_string()));
    }

    if let Ok(data) = scan_mmap_for_png(&mmap) {
         if best.as_ref().map_or(true, |(old_data, _)| data.len() > old_data.len()) {
             best = Some((data, "image/png".to_string()));
         }
    }

    if let Ok(data) = scan_mmap_for_tiff(&mmap) {
         if best.as_ref().map_or(true, |(old_data, _)| data.len() > old_data.len()) {
             best = Some((data, "image/tiff".to_string()));
         }
    }

    if let Ok(data) = extract_xmp_thumbnail(path) {
         if best.as_ref().map_or(true, |(old_data, _)| data.len() > old_data.len()) {
             // XMP thumbnails are almost always JPEG
             best = Some((data, "image/jpeg".to_string()));
         }
    }

    best.ok_or_else(|| "No embedded image found".into())
}

pub fn extract_embedded_jpeg(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    scan_mmap_for_jpeg(&mmap)
}

#[allow(dead_code)]
pub fn extract_embedded_png(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    scan_mmap_for_png(&mmap)
}

fn scan_mmap_for_jpeg(mmap: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut best_jpeg: Option<(usize, usize)> = None;
    let mut i = 0;

    // Limit scan to 30MB for performance on huge files
    let scan_limit = mmap.len().min(30 * 1024 * 1024);

    while i < scan_limit.saturating_sub(2) {
        if let Some(pos) = mmap[i..scan_limit].windows(2).position(|w| w == JPEG_SOI) {
            let start = i + pos;
            let j = start + 2;
            let eoi_limit = (j + 20 * 1024 * 1024).min(mmap.len());
            if let Some(eoi_pos) = mmap[j..eoi_limit].windows(2).position(|w| w == JPEG_EOI) {
                let end = j + eoi_pos + 2;
                let length = end - start;
                if best_jpeg.map_or(true, |(_, bl)| length > bl) {
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
        Ok(mmap[start..start + length].to_vec())
    } else {
        Err("No JPEG found".into())
    }
}

fn scan_mmap_for_png(mmap: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut best_png: Option<(usize, usize)> = None;
    let mut i = 0;

    let scan_limit = mmap.len().min(30 * 1024 * 1024);

    while i < scan_limit.saturating_sub(8) {
        if let Some(pos) = mmap[i..scan_limit].windows(8).position(|w| w == PNG_HEADER) {
            let start = i + pos;
            let j = start + 8;
            if let Some(end_pos) = mmap[j..].windows(4).position(|w| w == PNG_FOOTER) {
                let end = j + end_pos + 4 + 4; // IEND + 4 bytes CRC
                let length = end - start;
                if best_png.map_or(true, |(_, bl)| length > bl) {
                    best_png = Some((start, length));
                }
                i = end.min(mmap.len());
                continue;
            }
            i = start + 8;
        } else {
            break;
        }
    }

    if let Some((start, length)) = best_png {
        Ok(mmap[start..start + length.min(mmap.len()-start)].to_vec())
    } else {
        Err("No PNG found".into())
    }
}

fn scan_mmap_for_tiff(mmap: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut i = 0;
    // TIFFs in design files are usually in the first few MBs
    let scan_limit = mmap.len().min(10 * 1024 * 1024);

    while i < scan_limit.saturating_sub(4) {
        if mmap[i..].starts_with(TIFF_LE) || mmap[i..].starts_with(TIFF_BE) {
            // We don't know the exact length, but image crate is good at partial parsing.
            // We take a generous chunk (up to 50MB) and let the decoder handle it.
            let end = (i + 50 * 1024 * 1024).min(mmap.len());
            return Ok(mmap[i..end].to_vec());
        }
        i += 1;
    }
    Err("No TIFF found".into())
}

/// Extracts a preview from an EPS file specifically using the binary header pointers (if present).
pub fn extract_eps_binary_pointer(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    use std::io::{Read, Seek};
    let mut file = File::open(path)?;
    let mut header = [0u8; 32];
    if file.read_exact(&mut header).is_err() {
        return Err("File too small for EPS header".into());
    }

    // Binary EPS signature: 0xC5 0xD0 0xD3 0xC6
    if &header[0..4] == &[0xC5, 0xD0, 0xD3, 0xC6] {
        // TIFF part: offset at 20, length at 24
        let tiff_offset = u32::from_le_bytes(header[20..24].try_into()?) as u64;
        let tiff_len = u32::from_le_bytes(header[24..28].try_into()?) as u64;

        if tiff_len > 0 {
            file.seek(std::io::SeekFrom::Start(tiff_offset))?;
            let mut data = vec![0u8; tiff_len as usize];
            file.read_exact(&mut data)?;
            return Ok((data, "image/tiff".to_string()));
        }
    }

    Err("No binary EPS preview header found".into())
}

/// Scans for XMP metadata containing a base64 encoded thumbnail.
/// Common in Adobe Illustrator and EPS files.
pub fn extract_xmp_thumbnail(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut buffer = String::new();
    // XMP is usually near the top, read first 1MB
    file.take(1024 * 1024).read_to_string(&mut buffer).ok();

    if let Some(start_tag) = buffer.find("<xmpGImg:image>") {
        let start = start_tag + "<xmpGImg:image>".len();
        if let Some(end_tag) = buffer[start..].find("</xmpGImg:image>") {
            let base64_data = buffer[start..start + end_tag].replace(['\n', '\r', ' '], "");

            use base64::{Engine as _, engine::general_purpose};
            let decoded = general_purpose::STANDARD.decode(base64_data)?;
            return Ok(decoded);
        }
    }

    Err("No XMP thumbnail found".into())
}
