//! PaintTool SAI v2 (.sai2) preview extractor.
//!
//! SAI v2 files use an unencrypted, chunk-based binary format. The file begins
//! with a 64-byte header containing a magic string ("SAI-CANVAS"), canvas
//! dimensions, and a chunk count. Following the header is a Chunk List (an array
//! of chunk descriptors), then the actual chunk data.
//!
//! Thumbnails are stored in chunks with type `"thum"`. The thumbnail data can
//! be either:
//! - **ThumbnailLossy**: A JPEG image wrapped in a JSSF container.
//! - **ThumbnailLossless**: Raw DPCM-compressed BGRA tiles (256×256).
//!
//! This module currently implements JPEG thumbnail extraction (the most common
//! case for files created with SAI v2 after 2017). DPCM lossless support can be
//! added in the future without breaking the module's API.
//!
//! Reference: Wunkolo's SaiThumbs (C++, MIT): <https://github.com/Wunkolo/SaiThumbs>

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Magic string at the start of every SAI2 file (first 10 bytes of header).
const SAI2_MAGIC: &[u8; 10] = b"SAI-CANVAS";

/// Total size of the SAI2 file header in bytes.
const HEADER_SIZE: usize = 64;

/// Size of a single chunk descriptor in the Chunk List.
const CHUNK_DESCRIPTOR_SIZE: usize = 16;

/// Canvas data type identifier for lossy JPEG thumbnails.
const CANVAS_TYPE_THUMBNAIL_LOSSY: u32 = 0x11;

/// Canvas data type identifier for lossless DPCM thumbnails.
const CANVAS_TYPE_THUMBNAIL_LOSSLESS: u32 = 0x12;

/// JSSF container magic bytes (marks the start of JPEG data within a chunk).
const JSSF_MAGIC: &[u8; 4] = b"JSSF";

/// Standard JPEG SOI (Start of Image) marker: 0xFF 0xD8.
const JPEG_SOI_MARKER: [u8; 2] = [0xFF, 0xD8];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during SAI2 file parsing.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum Sai2Error {
    /// The file does not begin with the expected "SAI-CANVAS" magic string.
    #[error("Invalid SAI2 format: missing 'SAI-CANVAS' magic at file start")]
    InvalidMagic,

    /// The file header is too short (less than 64 bytes).
    #[error("SAI2 header is too short (expected at least {HEADER_SIZE} bytes)")]
    HeaderTooShort,

    /// No thumbnail chunk was found in the canvas data.
    #[error("No thumbnail chunk found in SAI2 canvas data")]
    ThumbnailNotFound,

    /// The thumbnail chunk type is not yet supported (DPCM lossless).
    #[error("Unsupported thumbnail type: DPCM lossless (not yet implemented)")]
    DpcmNotImplemented,

    /// The JSSF container could not be parsed or does not contain valid JPEG.
    #[error("Invalid JSSF container: {0}")]
    InvalidJssfContainer(String),

    /// Generic I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// File Header
// ---------------------------------------------------------------------------

/// Parsed SAI2 file header containing canvas metadata and chunk layout info.
#[derive(Debug)]
#[allow(dead_code)]
struct Sai2Header {
    /// Canvas width in pixels.
    canvas_width: u32,
    /// Canvas height in pixels.
    canvas_height: u32,
    /// Number of chunks described in the Chunk List.
    chunk_count: u32,
}

/// Parses the 64-byte SAI2 header from a file reader.
///
/// # Header Structure (64 bytes)
/// | Offset | Size | Description              |
/// |--------|------|--------------------------|
/// | 0      | 10   | Magic "SAI-CANVAS"       |
/// | 10     | 6    | Type suffix (e.g. "-TYPE0") — skipped |
/// | 16     | 16   | Alignment/padding/unknown |
/// | 32     | 4    | Canvas width             |
/// | 36     | 4    | Canvas height            |
/// | 40     | 4    | Chunk count (N)          |
/// | 44     | 20   | Reserved/padding         |
///
/// # Errors
/// Returns `Sai2Error::InvalidMagic` if the first 10 bytes don't match.
fn parse_sai2_header<R: Read + Seek>(reader: &mut R) -> Result<Sai2Header, Sai2Error> {
    let mut header_buffer = [0u8; HEADER_SIZE];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut header_buffer).map_err(|_| Sai2Error::HeaderTooShort)?;

    // Verify magic bytes (first 10 bytes)
    if &header_buffer[0..10] != SAI2_MAGIC {
        return Err(Sai2Error::InvalidMagic);
    }

    // Offsets based on Wunkolo/SaiThumbs
    // 0-10: Magic
    // 32-35: Width
    // 36-39: Height
    // 40-43: Chunk Count

    let canvas_width = u32::from_le_bytes([
        header_buffer[32], header_buffer[33], header_buffer[34], header_buffer[35],
    ]);
    let canvas_height = u32::from_le_bytes([
        header_buffer[36], header_buffer[37], header_buffer[38], header_buffer[39],
    ]);
    let chunk_count = u32::from_le_bytes([
        header_buffer[40], header_buffer[41], header_buffer[42], header_buffer[43],
    ]);

    if chunk_count > 100_000 {
        return Err(Sai2Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "SAI2 chunk count too large")));
    }

    Ok(Sai2Header {
        canvas_width,
        canvas_height,
        chunk_count,
    })
}

// ---------------------------------------------------------------------------
// Chunk List
// ---------------------------------------------------------------------------

/// Descriptor for a single chunk in the SAI2 Chunk List.
#[derive(Debug)]
struct ChunkDescriptor {
    /// 4-byte ASCII type tag (e.g., "thum", "layr", "lpix").
    type_tag: [u8; 4],
    /// Size of the chunk data in bytes.
    data_size: u64,
    /// Absolute offset of the chunk data within the file.
    data_offset: u64,
}

/// Parses the Chunk List from the file.
///
/// The Chunk List immediately follows the 64-byte header. Each descriptor
/// is 16 bytes: 4 bytes type tag + 4 bytes padding + 8 bytes chunk size.
/// The actual data offsets are computed by accumulating chunk sizes after
/// the Chunk List itself.
///
/// # Errors
/// Returns I/O errors if reading fails.
fn parse_chunk_list<R: Read + Seek>(
    reader: &mut R,
    chunk_count: u32,
) -> Result<Vec<ChunkDescriptor>, Sai2Error> {
    let chunk_list_offset = HEADER_SIZE as u64;
    let chunk_list_total_size = (chunk_count as u64) * (CHUNK_DESCRIPTOR_SIZE as u64);

    // The actual chunk data starts right after the Chunk List
    let data_region_start = chunk_list_offset + chunk_list_total_size;

    reader.seek(SeekFrom::Start(chunk_list_offset))?;

    let mut descriptors = Vec::with_capacity(chunk_count as usize);
    let mut running_offset = data_region_start;

    for _chunk_index in 0..chunk_count {
        let mut descriptor_buffer = [0u8; CHUNK_DESCRIPTOR_SIZE];
        reader.read_exact(&mut descriptor_buffer)?;

        let mut type_tag = [0u8; 4];
        type_tag.copy_from_slice(&descriptor_buffer[0..4]);

        // Bytes 4-7 are typically padding/flags — ignored for now
        let data_size = u64::from_le_bytes([
            descriptor_buffer[8], descriptor_buffer[9],
            descriptor_buffer[10], descriptor_buffer[11],
            descriptor_buffer[12], descriptor_buffer[13],
            descriptor_buffer[14], descriptor_buffer[15],
        ]);

        descriptors.push(ChunkDescriptor {
            type_tag,
            data_size,
            data_offset: running_offset,
        });

        running_offset = running_offset
            .checked_add(data_size)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Chunk offset overflow"))?;
    }

    Ok(descriptors)
}

// ---------------------------------------------------------------------------
// Canvas Data Iteration
// ---------------------------------------------------------------------------

/// Entry within a canvas data section that describes a sub-element (layer, thumbnail, etc.).
#[derive(Debug)]
#[allow(dead_code)]
struct CanvasDataEntry {
    /// Type identifier (e.g., 0x11 = ThumbnailLossy, 0x12 = ThumbnailLossless).
    canvas_type: u32,
    /// Size of this entry's data in bytes.
    data_size: u64,
    /// Absolute offset of this entry's data within the file.
    data_offset: u64,
}

/// Iterates the canvas data entries within a chunk.
///
/// Canvas data is structured as a sequence of tagged entries:
/// | Offset | Size | Description                   |
/// |--------|------|-------------------------------|
/// | 0      | 4    | Canvas type (u32 LE)          |
/// | 4      | 4    | Data size (u32 LE)            |
/// | 8      | N    | Data (N bytes)                |
///
/// # Errors
/// Returns I/O errors if reads fail.
fn iterate_canvas_data<R: Read + Seek>(
    reader: &mut R,
    chunk_descriptor: &ChunkDescriptor,
) -> Result<Vec<CanvasDataEntry>, Sai2Error> {
    let mut entries = Vec::new();
    let mut cursor_position = chunk_descriptor.data_offset;
    let chunk_end = chunk_descriptor.data_offset + chunk_descriptor.data_size;

    while cursor_position < chunk_end {
        reader.seek(SeekFrom::Start(cursor_position))?;

        let mut entry_header = [0u8; 8];
        if reader.read_exact(&mut entry_header).is_err() {
            break;
        }

        let canvas_type = u32::from_le_bytes([
            entry_header[0], entry_header[1], entry_header[2], entry_header[3],
        ]);
        let data_size = u32::from_le_bytes([
            entry_header[4], entry_header[5], entry_header[6], entry_header[7],
        ]) as u64;

        let data_offset = cursor_position + 8;

        // A canvas type of 0 signals the end of valid entries
        if canvas_type == 0 {
            break;
        }

        entries.push(CanvasDataEntry {
            canvas_type,
            data_size,
            data_offset,
        });

        cursor_position = data_offset + data_size;
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// JSSF Container → JPEG Extraction
// ---------------------------------------------------------------------------

/// Extracts the JPEG byte stream from a JSSF container.
///
/// JSSF is a SAI-specific container wrapping JPEG data. The structure is:
/// | Offset | Size | Description        |
/// |--------|------|--------------------|
/// | 0      | 4    | Magic "JSSF"       |
/// | 4      | 4    | Container flags    |
/// | 8      | 4    | JPEG data size     |
/// | 12     | 4    | Unknown/padding    |
/// | 16     | N    | Raw JPEG data      |
///
/// # Errors
/// Returns `Sai2Error::InvalidJssfContainer` if the magic doesn't match
/// or the JPEG SOI marker is not at the expected position.
fn extract_jpeg_from_jssf<R: Read + Seek>(
    reader: &mut R,
    entry: &CanvasDataEntry,
) -> Result<Vec<u8>, Sai2Error> {
    reader.seek(SeekFrom::Start(entry.data_offset))?;

    // Read the 16-byte JSSF header
    let mut jssf_header = [0u8; 16];
    reader.read_exact(&mut jssf_header)?;

    if &jssf_header[0..4] != JSSF_MAGIC {
        return Err(Sai2Error::InvalidJssfContainer(
            "Missing JSSF magic bytes".to_string(),
        ));
    }

    let jpeg_size = u32::from_le_bytes([
        jssf_header[8], jssf_header[9], jssf_header[10], jssf_header[11],
    ]) as usize;

    // Read the actual JPEG data
    let mut jpeg_data = vec![0u8; jpeg_size];
    reader.read_exact(&mut jpeg_data)?;

    // Validate JPEG SOI marker
    if jpeg_data.len() >= 2 && jpeg_data[0..2] != JPEG_SOI_MARKER {
        return Err(Sai2Error::InvalidJssfContainer(
            "JPEG data does not start with SOI marker (0xFF 0xD8)".to_string(),
        ));
    }

    Ok(jpeg_data)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extracts the embedded thumbnail from a PaintTool SAI v2 (.sai2) file.
///
/// The extraction pipeline:
/// 1. Parse the 64-byte header to validate magic and get chunk count.
/// 2. Parse the Chunk List to locate all chunk descriptors.
/// 3. For each chunk tagged `"thum"`, iterate its canvas data entries.
/// 4. Find the thumbnail entry (lossy JPEG preferred).
/// 5. Extract the JPEG from the JSSF container and return it.
///
/// # Arguments
/// * `sai2_file_path` - Path to the .sai2 file on disk.
///
/// # Returns
/// A tuple of (JPEG bytes, MIME type string).
///
/// # Errors
/// Returns `Err` if the file format is invalid, no thumbnail chunk exists,
/// or the thumbnail uses DPCM encoding (not yet supported).
pub fn extract_sai2_preview(sai2_file_path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let mut file = File::open(sai2_file_path)?;
    let header = parse_sai2_header(&mut file)?;

    // Log the header to verify offsets are correct now
    println!("DEBUG: SAI2 Header Parsed: {}x{}, chunks: {}", header.canvas_width, header.canvas_height, header.chunk_count);

    let chunk_descriptors = match parse_chunk_list(&mut file, header.chunk_count) {
        Ok(d) => d,
        Err(e) => {
            println!("DEBUG: SAI2 Chunk List Parse Error: {}", e);
            return Err(Box::new(e));
        }
    };

    // Search for thumbnail chunks (type tag "thum")
    for descriptor in &chunk_descriptors {
        let type_string = String::from_utf8_lossy(&descriptor.type_tag);

        if type_string != "thum" {
            continue;
        }

        let canvas_entries = iterate_canvas_data(&mut file, descriptor)?;

        for entry in &canvas_entries {
            match entry.canvas_type {
                CANVAS_TYPE_THUMBNAIL_LOSSY => {
                    let jpeg_data = extract_jpeg_from_jssf(&mut file, entry)?;
                    return Ok((jpeg_data, "image/jpeg".to_string()));
                }
                CANVAS_TYPE_THUMBNAIL_LOSSLESS => {
                    println!("DEBUG: SAI2 Found DPCM thumbnail (unsupported). Skipping...");
                    // DPCM lossless thumbnails are not yet implemented.
                    // Skip and continue to search for a lossy JPEG alternative.
                    continue;
                }
                _ => continue,
            }
        }
    }

    println!("DEBUG: SAI2 No usable thumbnail found (checked {} chunks)", chunk_descriptors.len());
    Err(Box::new(Sai2Error::ThumbnailNotFound))
}
