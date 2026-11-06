//! PaintTool SAI v2 (.sai2) preview extractor.
//!
//! SAI v2 files use a chunk-based binary format identified by the magic string
//! `SAI-CANVAS-TYPE0`. The file layout is:
//!   - 64-byte `CanvasHeader`
//!   - Table of `CanvasEntry` (16 bytes each), count from header
//!   - Blob data regions at absolute file offsets
//!
//! This extractor handles both lossy JPEG thumbnails (wrapped in JSSF) and
//! lossless DPCM-compressed thumbnails. It prioritizes `thum` (lossy) chunks
//! first for speed, falling back to `intg` (lossless/DPCM) chunks.
//!
//! Reference implementation: Wunkolo/libsai (MIT) — sai2.hpp / sai2.cpp

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tracing::debug;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SAI2_MAGIC: &[u8; 16] = b"SAI-CANVAS-TYPE0";
const HEADER_SIZE: usize = 64;

// CanvasEntry type tags (little-endian u32)
const TAG_THUM: u32 = u32::from_le_bytes(*b"thum");
const TAG_INTG: u32 = u32::from_le_bytes(*b"intg");

// BlobDataType tags
const BLOB_JSSF: u32 = u32::from_le_bytes(*b"jssf");
const BLOB_DPCM: u32 = u32::from_le_bytes(*b"dpcm");

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum Sai2Error {
    #[error("Invalid SAI2 format: missing magic string")]
    InvalidMagic,
    #[error("SAI2 header is too short")]
    HeaderTooShort,
    #[error("No thumbnail chunk found in SAI2 canvas data")]
    ThumbnailNotFound,
    #[error("Invalid JSSF container: {0}")]
    InvalidJssfContainer(String),
    #[error("Invalid DPCM data: {0}")]
    InvalidDpcmData(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Structures — matching C++ struct layout exactly
// ---------------------------------------------------------------------------

/// Canonical SAI2 header, 64 bytes, packed.
///
/// ```text
/// Offset  Size  Field
/// 0x00    16    Identifier ("SAI-CANVAS-TYPE0")
/// 0x10    1     Flags0
/// 0x11    1     CanvasBackgroundFlags  (& 0x7 == 0 → 4ch, else 3ch)
/// 0x12    1     Flags2
/// 0x13    1     Flags3
/// 0x14    4     Width
/// 0x18    4     Height
/// 0x1C    4     PrintingResolution
/// 0x20    4     TableCount
/// 0x24    4     SelectedLayer
/// 0x28    8     UnknownA
/// 0x30    8     UnknownB
/// 0x38    4     CanvasBackgroundColor
/// 0x3C    4     LayerEffectColor
/// ```
#[derive(Debug)]
struct CanvasHeader {
    canvas_width: u32,
    canvas_height: u32,
    table_count: u32,
    canvas_background_flags: u8,
}

/// A single entry in the canvas table. 16 bytes each.
///
/// ```text
/// Offset  Size  Field
/// 0x00    4     Type (LE tag: "thum", "intg", "layr", etc.)
/// 0x04    4     LayerID
/// 0x08    8     BlobsOffset (absolute file offset)
/// ```
#[derive(Debug)]
struct CanvasEntry {
    entry_type: u32,
    blobs_offset: u64,
}

// ---------------------------------------------------------------------------
// Header & Table Parsing
// ---------------------------------------------------------------------------

fn parse_canvas_header<R: Read + Seek>(reader: &mut R) -> Result<CanvasHeader, Sai2Error> {
    let mut header_buffer = [0u8; HEADER_SIZE];
    reader.seek(SeekFrom::Start(0))?;
    reader
        .read_exact(&mut header_buffer)
        .map_err(|_| Sai2Error::HeaderTooShort)?;

    if &header_buffer[0..16] != SAI2_MAGIC {
        return Err(Sai2Error::InvalidMagic);
    }

    let canvas_background_flags = header_buffer[0x11];

    let canvas_width = u32::from_le_bytes([
        header_buffer[0x14],
        header_buffer[0x15],
        header_buffer[0x16],
        header_buffer[0x17],
    ]);
    let canvas_height = u32::from_le_bytes([
        header_buffer[0x18],
        header_buffer[0x19],
        header_buffer[0x1A],
        header_buffer[0x1B],
    ]);
    let table_count = u32::from_le_bytes([
        header_buffer[0x20],
        header_buffer[0x21],
        header_buffer[0x22],
        header_buffer[0x23],
    ]);

    debug!(
        "SAI2 header: {}x{}, table_count={}, bg_flags=0x{:02X}",
        canvas_width, canvas_height, table_count, canvas_background_flags
    );

    Ok(CanvasHeader {
        canvas_width,
        canvas_height,
        table_count,
        canvas_background_flags,
    })
}

fn parse_canvas_table<R: Read + Seek>(
    reader: &mut R,
    table_count: u32,
) -> Result<Vec<CanvasEntry>, Sai2Error> {
    // Table starts immediately after the 64-byte header
    reader.seek(SeekFrom::Start(HEADER_SIZE as u64))?;

    let mut entries = Vec::with_capacity(table_count as usize);
    for _ in 0..table_count {
        let mut entry_buffer = [0u8; 16];
        reader.read_exact(&mut entry_buffer)?;

        let entry_type = u32::from_le_bytes([
            entry_buffer[0],
            entry_buffer[1],
            entry_buffer[2],
            entry_buffer[3],
        ]);
        // entry_buffer[4..8] is LayerID — not needed for thumbnail extraction
        let blobs_offset = u64::from_le_bytes([
            entry_buffer[8],
            entry_buffer[9],
            entry_buffer[10],
            entry_buffer[11],
            entry_buffer[12],
            entry_buffer[13],
            entry_buffer[14],
            entry_buffer[15],
        ]);

        entries.push(CanvasEntry {
            entry_type,
            blobs_offset,
        });
    }

    Ok(entries)
}

/// Calculate blob data size for entry at `entry_index`.
/// Size = next_entry.blobs_offset - current.blobs_offset,
/// or file_size - current.blobs_offset for the last entry.
fn compute_blob_size(entries: &[CanvasEntry], entry_index: usize, file_size: u64) -> u64 {
    if entry_index + 1 < entries.len() {
        entries[entry_index + 1]
            .blobs_offset
            .saturating_sub(entries[entry_index].blobs_offset)
    } else {
        file_size.saturating_sub(entries[entry_index].blobs_offset)
    }
}

// ---------------------------------------------------------------------------
// JSSF → JPEG Conversion
// ---------------------------------------------------------------------------

/// Convert SAI2 JSSF thumbnail data into a standard JPEG stream.
///
/// JSSF layout (after the 12-byte blob prefix):
///   - u16 LE: width
///   - u16 LE: height
///   - u16 LE: channel count (1 or 3)
///   - 64 bytes: luma quantization table
///   - 64 bytes: chroma quantization table (if channels > 1)
///   - MCU rows, each prefixed by u16 LE size
fn convert_jssf_to_jpeg(
    jssf_data: &[u8],
    jssf_width: u16,
    jssf_height: u16,
    jssf_channels: u16,
) -> Result<Vec<u8>, Sai2Error> {
    let mut cursor = 0usize;
    let data = jssf_data;

    // Read quantization tables
    if data.len() < 64 {
        return Err(Sai2Error::InvalidJssfContainer(
            "Data too short for luma quant table".into(),
        ));
    }
    let luma_quant = &data[cursor..cursor + 64];
    cursor += 64;

    let mut chroma_quant: &[u8] = &[];
    if jssf_channels > 1 {
        if data.len() < cursor + 64 {
            return Err(Sai2Error::InvalidJssfContainer(
                "Data too short for chroma quant table".into(),
            ));
        }
        chroma_quant = &data[cursor..cursor + 64];
        cursor += 64;
    }

    let mut jpeg_output = Vec::with_capacity(data.len() + 1024);

    // Helper closures
    let push_u8 = |output: &mut Vec<u8>, value: u8| {
        output.push(value);
    };
    let push_u16_be = |output: &mut Vec<u8>, value: u16| {
        output.push((value >> 8) as u8);
        output.push(value as u8);
    };

    // SOI - Start of Image
    push_u16_be(&mut jpeg_output, 0xFFD8);

    // DQT - Define Quantization Table
    push_u16_be(&mut jpeg_output, 0xFFDB);
    let dqt_length = if jssf_channels > 1 { 65 + 67 } else { 67 };
    push_u16_be(&mut jpeg_output, dqt_length);
    // Luma table (ID 0)
    push_u8(&mut jpeg_output, 0x00); // 4-bit precision (0=8bit) | 4-bit table ID (0)
    jpeg_output.extend_from_slice(luma_quant);
    // Chroma table (ID 1) if multichannel
    if jssf_channels > 1 {
        push_u8(&mut jpeg_output, 0x01);
        jpeg_output.extend_from_slice(chroma_quant);
    }

    // DHT - Define Huffman Tables
    #[rustfmt::skip]
    static HUFFMAN_LUT_0: [u8; 29] = [
        0x00, // DC table, ID 0
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];

    #[rustfmt::skip]
    static HUFFMAN_LUT_1: [u8; 179] = [
        0x10, // AC table, ID 0
        0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04,
        0x00, 0x00, 0x01, 0x7D,
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
        0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08,
        0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72,
        0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
        0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
        0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3,
        0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
        0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
        0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
        0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
        0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
    ];

    #[rustfmt::skip]
    static HUFFMAN_LUT_2: [u8; 29] = [
        0x01, // DC table, ID 1
        0x00, 0x03, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];

    #[rustfmt::skip]
    static HUFFMAN_LUT_3: [u8; 179] = [
        0x11, // AC table, ID 1
        0x00, 0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04,
        0x00, 0x01, 0x02, 0x77,
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41,
        0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91,
        0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15, 0x62, 0x72, 0xD1,
        0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26,
        0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44,
        0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
        0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74,
        0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A,
        0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4,
        0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,
        0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA,
        0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4,
        0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
    ];

    push_u16_be(&mut jpeg_output, 0xFFC4); // DHT marker
    let dht_length: u16 = 2
        + HUFFMAN_LUT_0.len() as u16
        + HUFFMAN_LUT_1.len() as u16
        + if jssf_channels > 1 {
            HUFFMAN_LUT_2.len() as u16 + HUFFMAN_LUT_3.len() as u16
        } else {
            0
        };
    push_u16_be(&mut jpeg_output, dht_length);
    jpeg_output.extend_from_slice(&HUFFMAN_LUT_0);
    jpeg_output.extend_from_slice(&HUFFMAN_LUT_1);
    if jssf_channels > 1 {
        jpeg_output.extend_from_slice(&HUFFMAN_LUT_2);
        jpeg_output.extend_from_slice(&HUFFMAN_LUT_3);
    }

    // SOF0 - Start of Frame (Baseline DCT)
    push_u16_be(&mut jpeg_output, 0xFFC0);
    push_u16_be(&mut jpeg_output, 8 + (jssf_channels * 3));
    push_u8(&mut jpeg_output, 0x08); // Precision: 8 bits
    push_u16_be(&mut jpeg_output, jssf_height);
    push_u16_be(&mut jpeg_output, jssf_width);
    push_u8(&mut jpeg_output, jssf_channels as u8);

    for channel_index in 0..jssf_channels {
        push_u8(&mut jpeg_output, (channel_index + 1) as u8); // Component ID
        push_u8(&mut jpeg_output, 0x11); // Sampling factor 1:1
        push_u8(&mut jpeg_output, if channel_index != 0 { 1 } else { 0 }); // Quant table ID
    }

    // DRI - Define Restart Interval
    push_u16_be(&mut jpeg_output, 0xFFDD);
    push_u16_be(&mut jpeg_output, 0x0004); // Length
    push_u16_be(&mut jpeg_output, jssf_width.div_ceil(8)); // MCUs per row

    // SOS - Start of Scan
    push_u16_be(&mut jpeg_output, 0xFFDA);
    push_u16_be(&mut jpeg_output, 6 + (jssf_channels * 2));
    push_u8(&mut jpeg_output, jssf_channels as u8);
    for channel_index in 0..jssf_channels {
        push_u8(&mut jpeg_output, (channel_index + 1) as u8); // Component ID
        push_u8(
            &mut jpeg_output,
            if channel_index == 0 { 0x00 } else { 0x11 },
        ); // Huffman table
    }
    push_u8(&mut jpeg_output, 0x00); // Start of spectral selection
    push_u8(&mut jpeg_output, 0x3F); // End of spectral selection
    push_u8(&mut jpeg_output, 0x00); // Successive approximation

    // MCU rows — each prefixed with u16 LE size
    let mcu_row_count = (jssf_height as usize).div_ceil(8);
    for mcu_row_index in 0..mcu_row_count {
        if cursor + 2 > data.len() {
            break;
        }
        let mcu_row_size = u16::from_le_bytes([data[cursor], data[cursor + 1]]) as usize;
        cursor += 2;

        if cursor + mcu_row_size > data.len() {
            break;
        }
        jpeg_output.extend_from_slice(&data[cursor..cursor + mcu_row_size]);
        cursor += mcu_row_size;

        // Insert restart marker between rows (not after the last)
        if mcu_row_index < mcu_row_count - 1 {
            let restart_marker = 0xFFD0u16 | ((mcu_row_index as u16) & 0x07);
            push_u16_be(&mut jpeg_output, restart_marker);
        }
    }

    // EOI - End of Image
    push_u16_be(&mut jpeg_output, 0xFFD9);

    Ok(jpeg_output)
}

// ---------------------------------------------------------------------------
// Delta RLE Unpacker — ported faithfully from sai2::UnpackDeltaRLE16
// ---------------------------------------------------------------------------

/// Unpack SAI2's proprietary delta-RLE bitstream into signed 16-bit deltas.
///
/// The bitstream encodes per-channel deltas using a variable-length scheme:
/// - Read 32-bit LE words into a 64-bit control mask
/// - `OpCode = (2 * trailing_zeros) | next_bit`
/// - OpCode 0: write zero
/// - OpCode 1..14: read N bits + sign bit → delta value
/// - OpCode 15: RLE of zeros (7-bit count + 8)
///
/// Returns number of bytes consumed from `compressed`.
fn unpack_delta_rle_16(
    compressed: &[u8],
    decompressed: &mut [i16],
    pixel_count: u32,
    output_channels: u8,
    input_channels: u8,
) -> usize {
    let initial_length = compressed.len();
    let mut read_position = 0usize;

    let mut remaining_bits: u32 = 0;
    let mut control_mask: u64 = 0;

    for current_channel in 0..input_channels as usize {
        let mut decoded_pixel_count: u32 = 0;

        // Pixel write index advances by output_channels for each pixel
        let mut write_offset = current_channel;

        loop {
            // Refill the control mask whenever it drops below 32 bits
            while remaining_bits < 32 && read_position < compressed.len() {
                let shift_amount = remaining_bits;
                let new_word: u64;
                let bytes_left = compressed.len() - read_position;

                if bytes_left >= 4 {
                    new_word = u32::from_le_bytes([
                        compressed[read_position],
                        compressed[read_position + 1],
                        compressed[read_position + 2],
                        compressed[read_position + 3],
                    ]) as u64;
                    read_position += 4;
                    remaining_bits += 32;
                } else if bytes_left >= 2 {
                    new_word = u16::from_le_bytes([
                        compressed[read_position],
                        compressed[read_position + 1],
                    ]) as u64;
                    read_position += 2;
                    remaining_bits += 16;
                } else {
                    new_word = compressed[read_position] as u64;
                    read_position += 1;
                    remaining_bits += 8;
                }

                control_mask |= new_word << shift_amount;
            }

            if control_mask == 0 {
                // No more data — this channel is done (or data ended)
                break;
            }

            // Find the first set bit index
            let first_set_bit_index = control_mask.trailing_zeros() as u8;
            let next_set_bit_mask = control_mask >> (first_set_bit_index + 1);

            let opcode = (2 * first_set_bit_index as u32) | ((next_set_bit_mask & 1) as u32);
            remaining_bits = remaining_bits.saturating_sub(2 + first_set_bit_index as u32);

            control_mask = next_set_bit_mask >> 1;

            if opcode == 0 {
                // Write a single zero
                if write_offset < decompressed.len() {
                    decompressed[write_offset] = 0;
                }
                decoded_pixel_count += 1;
                write_offset += output_channels as usize;
            } else if opcode <= 14 {
                // The opcode is the number of bits to consume
                let bit_active_mask: i32 = -(((control_mask & (1u64 << opcode)) != 0) as i32);

                let bit_value_mask: u64 = (1u64 << opcode) - 1;
                let bit_value = control_mask & bit_value_mask;

                let channel_value: i16 = ((bit_active_mask & 1)
                    + (bit_active_mask ^ (((1i32 << opcode) | bit_value as i32) - 1)))
                    as i16;

                remaining_bits = remaining_bits.saturating_sub(opcode + 1);
                control_mask >>= opcode + 1;

                if write_offset < decompressed.len() {
                    decompressed[write_offset] = channel_value;
                }
                decoded_pixel_count += 1;
                write_offset += output_channels as usize;
            } else if opcode == 15 {
                // RLE of zeros: next 7 bits encode (count + 8)
                let zero_fill_count = ((control_mask & 0x7F) as u32) + 8;
                remaining_bits = remaining_bits.saturating_sub(7);
                control_mask >>= 7;

                for fill_index in 0..zero_fill_count as usize {
                    let target_offset = write_offset + fill_index * output_channels as usize;
                    if target_offset < decompressed.len() {
                        decompressed[target_offset] = 0;
                    }
                }

                decoded_pixel_count += zero_fill_count;
                write_offset += zero_fill_count as usize * output_channels as usize;
            } else {
                // Invalid opcode — stop
                break;
            }

            if decoded_pixel_count >= pixel_count {
                break;
            }
        }
    }

    // Pad unused channels with zero
    if input_channels < output_channels {
        for padding_channel in input_channels as usize..output_channels as usize {
            for pixel_index in 0..pixel_count as usize {
                let target_offset = pixel_index * output_channels as usize + padding_channel;
                if target_offset < decompressed.len() {
                    decompressed[target_offset] = 0;
                }
            }
        }
    }

    // Return total bytes consumed, giving back any bits not fully used
    let total_read_bytes = initial_length - (compressed.len() - read_position);
    let remaining_whole_bytes = (remaining_bits / 8) as usize;
    total_read_bytes.saturating_sub(remaining_whole_bytes)
}

// ---------------------------------------------------------------------------
// 16-bit Saturated Pixel Arithmetic — ported from C++ Pixel16Bpc
// ---------------------------------------------------------------------------

/// 16-bit per-channel pixel, stored as 4 × u16 (B, G, R, A in file order).
#[derive(Clone, Copy, Default)]
struct Pixel16Bpc {
    channels: [u16; 4],
}

impl Pixel16Bpc {
    /// Convert an 8bpc packed u32 (BGRA byte order) to 16bpc.
    fn from_8bpc(pixel: u32) -> Self {
        Self {
            channels: [
                (pixel & 0xFF) as u16,
                ((pixel >> 8) & 0xFF) as u16,
                ((pixel >> 16) & 0xFF) as u16,
                ((pixel >> 24) & 0xFF) as u16,
            ],
        }
    }

    /// Saturated 16bpc → 8bpc conversion, clamping each channel to 0..255.
    fn to_8bpc_saturated(self) -> u32 {
        let mut result = 0u32;
        for channel_index in 0..4 {
            let clamped_value = if self.channels[channel_index] > 0xFF {
                0xFF
            } else {
                self.channels[channel_index] as u8
            };
            result |= (clamped_value as u32) << (channel_index * 8);
        }
        result
    }
}

/// Wrapping add of two 16bpc pixels (plain add, may overflow within u16).
fn add_16bpc(lhs: Pixel16Bpc, rhs: Pixel16Bpc) -> Pixel16Bpc {
    let mut result = Pixel16Bpc::default();
    for channel_index in 0..4 {
        result.channels[channel_index] =
            lhs.channels[channel_index].wrapping_add(rhs.channels[channel_index]);
    }
    result
}

/// Saturated add of two 16bpc pixels.
fn add_saturated_16bpc(lhs: Pixel16Bpc, rhs: Pixel16Bpc) -> Pixel16Bpc {
    let mut result = Pixel16Bpc::default();
    for channel_index in 0..4 {
        result.channels[channel_index] =
            lhs.channels[channel_index].saturating_add(rhs.channels[channel_index]);
    }
    result
}

/// Saturated subtract of two 16bpc pixels.
fn sub_saturated_16bpc(lhs: Pixel16Bpc, rhs: Pixel16Bpc) -> Pixel16Bpc {
    let mut result = Pixel16Bpc::default();
    for channel_index in 0..4 {
        result.channels[channel_index] =
            lhs.channels[channel_index].saturating_sub(rhs.channels[channel_index]);
    }
    result
}

// ---------------------------------------------------------------------------
// DPCM Row Unpacker — ported from sai2::DeltaUnpackRow16Bpc
// ---------------------------------------------------------------------------

/// Apply the SAI2 plane predictor to a row of delta-encoded 16bpc pixels.
///
/// The predictor formula (per pixel):
/// ```text
/// predicted = SubSaturated( AddSaturated( SubSaturated(Add(Sum, Above), Diagonal), FF00 ), FF00 )
/// Sum = Add(predicted, Delta)
/// Output = Saturate16→8(Sum)
/// ```
///
/// This is similar to the PNG "Up" filter but operates in 16-bit saturated space.
fn delta_unpack_row_16bpc(
    destination_8bpc: &mut [u32],
    previous_row_8bpc: &[u32],
    delta_encoded_16bpc: &[i16],
    pixel_count: u32,
) {
    let pixel_ff00 = Pixel16Bpc {
        channels: [0xFF00, 0xFF00, 0xFF00, 0xFF00],
    };

    let mut previous_row_pixel_16bpc = Pixel16Bpc::default();
    let mut sum_16bpc = Pixel16Bpc::default();

    for pixel_index in 0..pixel_count as usize {
        // Convert the pixel above (from previous row) to 16bpc
        let current_above_pixel_16bpc = Pixel16Bpc::from_8bpc(previous_row_8bpc[pixel_index]);

        // Read current delta (4 channels × i16)
        let delta_base = pixel_index * 4;
        let current_pixel_delta = Pixel16Bpc {
            channels: [
                delta_encoded_16bpc[delta_base] as u16,
                delta_encoded_16bpc[delta_base + 1] as u16,
                delta_encoded_16bpc[delta_base + 2] as u16,
                delta_encoded_16bpc[delta_base + 3] as u16,
            ],
        };

        // Plane predictor:
        // Sum = Add(
        //     SubSaturated(
        //         AddSaturated(
        //             SubSaturated(
        //                 Add(Sum, CurAbove),
        //                 PreviousDiagonal
        //             ),
        //             FF00
        //         ),
        //         FF00
        //     ),
        //     Delta
        // )
        sum_16bpc = add_16bpc(
            sub_saturated_16bpc(
                add_saturated_16bpc(
                    sub_saturated_16bpc(
                        add_16bpc(sum_16bpc, current_above_pixel_16bpc),
                        previous_row_pixel_16bpc,
                    ),
                    pixel_ff00,
                ),
                pixel_ff00,
            ),
            current_pixel_delta,
        );

        // Saturate 16u → 8u
        destination_8bpc[pixel_index] = sum_16bpc.to_8bpc_saturated();

        previous_row_pixel_16bpc = current_above_pixel_16bpc;
    }
}

// ---------------------------------------------------------------------------
// DPCM Thumbnail Decoder — ported from sai2::ExtractDpcmToBGRA
// ---------------------------------------------------------------------------

/// Decode a DPCM-compressed thumbnail blob into BGRA pixel data.
///
/// The blob layout (after the 4-byte `dpcm` tag that was already consumed):
///   - tile_count × u32 LE: compressed size of each tile
///   - tile data (for each Y row of tiles):
///       - for each X tile: checksum(u16) + compressed row data
///       - trailing checksum(u16) after all X tiles
fn decode_dpcm_thumbnail(
    dpcm_data: &[u8],
    canvas_width: u32,
    canvas_height: u32,
    thumbnail_channels: u8,
) -> Result<Vec<u8>, Sai2Error> {
    const TILE_SIZE: u32 = 256;

    let tiles_x = canvas_width.div_ceil(TILE_SIZE);
    let tiles_y = canvas_height.div_ceil(TILE_SIZE);
    let tiles_count = (tiles_x * tiles_y) as usize;

    let mut cursor = 0usize;

    // Read tile compressed sizes
    let mut tile_sizes = Vec::with_capacity(tiles_count);
    for _ in 0..tiles_count {
        if cursor + 4 > dpcm_data.len() {
            return Err(Sai2Error::InvalidDpcmData(
                "Not enough data for tile sizes".into(),
            ));
        }
        let tile_size = u32::from_le_bytes([
            dpcm_data[cursor],
            dpcm_data[cursor + 1],
            dpcm_data[cursor + 2],
            dpcm_data[cursor + 3],
        ]) as usize;
        tile_sizes.push(tile_size);
        cursor += 4;
    }

    let mut image_pixels = vec![0u32; (canvas_width * canvas_height) as usize];

    for tile_y_index in 0..tiles_y {
        let tile_begin_y = tile_y_index * TILE_SIZE;
        let tile_end_y = (tile_begin_y + TILE_SIZE).min(canvas_height);
        let tile_size_y = (tile_end_y - tile_begin_y) as usize;

        // CompositeRow is 256 pixels wide, shared across tiles in this Y row
        let mut composite_row = [0u32; 256];

        for tile_x_index in 0..tiles_x {
            let tile_flat_index = (tile_y_index * tiles_x + tile_x_index) as usize;
            if tile_flat_index >= tile_sizes.len() {
                break;
            }
            let tile_data_size = tile_sizes[tile_flat_index];

            if cursor + tile_data_size > dpcm_data.len() {
                debug!(
                    "DPCM: tile ({},{}) data exceeds buffer, skipping remaining",
                    tile_x_index, tile_y_index
                );
                break;
            }

            let mut tile_bytes = &dpcm_data[cursor..cursor + tile_data_size];
            let tile_end_cursor = cursor + tile_data_size;

            // Skip 2-byte tile checksum
            if tile_bytes.len() < 2 {
                cursor = tile_end_cursor;
                continue;
            }
            tile_bytes = &tile_bytes[2..];

            let tile_begin_x = tile_x_index * TILE_SIZE;
            let tile_end_x = (tile_begin_x + TILE_SIZE).min(canvas_width);
            let tile_size_x = (tile_end_x - tile_begin_x) as usize;

            // Process each row within the tile
            let mut previous_row: Vec<u32> = composite_row[..tile_size_x].to_vec();

            for tile_row_index in 0..tile_size_y {
                // Decompress this row's deltas
                let mut row_deltas = vec![0i16; tile_size_x * 4];

                let consumed_bytes = unpack_delta_rle_16(
                    tile_bytes,
                    &mut row_deltas,
                    tile_size_x as u32,
                    4,
                    thumbnail_channels,
                );

                if consumed_bytes == 0 {
                    break;
                }
                if consumed_bytes > tile_bytes.len() {
                    break;
                }
                tile_bytes = &tile_bytes[consumed_bytes..];

                // Destination row in the full image
                let image_row_y = tile_begin_y + tile_row_index as u32;
                let image_row_start = (image_row_y * canvas_width + tile_begin_x) as usize;

                // Apply the plane predictor
                let mut destination_row = vec![0u32; tile_size_x];
                delta_unpack_row_16bpc(
                    &mut destination_row,
                    &previous_row,
                    &row_deltas,
                    tile_size_x as u32,
                );

                // Write to image and update previous_row for next iteration
                image_pixels[image_row_start..image_row_start + tile_size_x]
                    .copy_from_slice(&destination_row);
                previous_row = destination_row;
            }

            // Copy last row into composite_row for inter-tile prediction
            composite_row[..tile_size_x].copy_from_slice(&previous_row[..tile_size_x]);

            cursor = tile_end_cursor;
        }

        // Consume the trailing 2-byte alignment checksum after each tile row Y
        if cursor + 2 <= dpcm_data.len() {
            cursor += 2;
        }
    }

    // Force full alpha for 3-channel images
    if thumbnail_channels == 3 {
        for pixel in &mut image_pixels {
            *pixel |= 0xFF000000;
        }
    }

    // Convert u32 array to u8 BGRA bytes
    let mut output_bytes = Vec::with_capacity(image_pixels.len() * 4);
    for pixel in &image_pixels {
        output_bytes.extend_from_slice(&pixel.to_le_bytes());
    }

    Ok(output_bytes)
}

// ---------------------------------------------------------------------------
// PNG Encoding
// ---------------------------------------------------------------------------

fn encode_bgra_as_png(
    bgra_pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    use image::{ImageBuffer, Rgba};

    let mut rgba_pixels = bgra_pixels.to_vec();
    // BGRA → RGBA: swap B and R
    for chunk in rgba_pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, rgba_pixels)
            .ok_or("Invalid pixel buffer dimensions")?;

    let mut png_output = std::io::Cursor::new(Vec::new());
    image_buffer.write_to(&mut png_output, image::ImageFormat::Png)?;

    Ok((png_output.into_inner(), "image/png".into()))
}

// ---------------------------------------------------------------------------
// Blob Extraction
// ---------------------------------------------------------------------------

/// Extract thumbnail from a blob region at the given offset.
///
/// Blob prefix layout:
///   - u32 LE: Width
///   - u32 LE: Height
///   - u32 LE: BlobDataType tag ("jssf" or "dpcm")
///   - variable: blob-specific data
fn extract_thumbnail_from_blob<R: Read + Seek>(
    reader: &mut R,
    blob_offset: u64,
    blob_size: u64,
    header: &CanvasHeader,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    reader.seek(SeekFrom::Start(blob_offset))?;

    // Read blob prefix: Width(4) + Height(4) + BlobDataType(4) = 12 bytes
    let mut blob_prefix = [0u8; 12];
    reader.read_exact(&mut blob_prefix)?;

    let _blob_width = u32::from_le_bytes([
        blob_prefix[0],
        blob_prefix[1],
        blob_prefix[2],
        blob_prefix[3],
    ]);
    let _blob_height = u32::from_le_bytes([
        blob_prefix[4],
        blob_prefix[5],
        blob_prefix[6],
        blob_prefix[7],
    ]);
    let blob_data_type = u32::from_le_bytes([
        blob_prefix[8],
        blob_prefix[9],
        blob_prefix[10],
        blob_prefix[11],
    ]);

    let remaining_blob_size = blob_size.saturating_sub(12) as usize;
    let read_size = remaining_blob_size.min(64 * 1024 * 1024); // Cap at 64MB safety
    let mut blob_data = vec![0u8; read_size];
    reader.read_exact(&mut blob_data)?;

    if blob_data_type == BLOB_JSSF {
        // JSSF: read the inner JSSF header (u16 width, u16 height, u16 channels)
        if blob_data.len() < 6 {
            return Err(Box::new(Sai2Error::InvalidJssfContainer(
                "JSSF data too short".into(),
            )));
        }
        let jssf_width = u16::from_le_bytes([blob_data[0], blob_data[1]]);
        let jssf_height = u16::from_le_bytes([blob_data[2], blob_data[3]]);
        let jssf_channels = u16::from_le_bytes([blob_data[4], blob_data[5]]);

        debug!(
            "JSSF thumbnail: {}x{}, {} channels",
            jssf_width, jssf_height, jssf_channels
        );

        let jssf_payload = &blob_data[6..];
        let jpeg_data = convert_jssf_to_jpeg(jssf_payload, jssf_width, jssf_height, jssf_channels)?;

        return Ok((jpeg_data, "image/jpeg".into()));
    }

    if blob_data_type == BLOB_DPCM {
        let thumbnail_channels: u8 = if (header.canvas_background_flags & 0x07) == 0 {
            4
        } else {
            3
        };

        debug!(
            "DPCM thumbnail: {}x{}, {} channels",
            header.canvas_width, header.canvas_height, thumbnail_channels
        );

        let bgra_pixels = decode_dpcm_thumbnail(
            &blob_data,
            header.canvas_width,
            header.canvas_height,
            thumbnail_channels,
        )?;

        return encode_bgra_as_png(&bgra_pixels, header.canvas_width, header.canvas_height);
    }

    // Unknown blob type — format the tag for debug
    let tag_bytes = blob_data_type.to_le_bytes();
    let tag_string = String::from_utf8_lossy(&tag_bytes);
    Err(Box::new(Sai2Error::InvalidJssfContainer(format!(
        "Unknown blob data type: {} (0x{:08X})",
        tag_string, blob_data_type
    ))))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn extract_sai2_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();

    let header = parse_canvas_header(&mut file)?;
    let entries = parse_canvas_table(&mut file, header.table_count)?;

    // Priority: try lossy thumbnail first (faster, smaller), then lossless
    let priority_order = [TAG_THUM, TAG_INTG];

    for target_tag in &priority_order {
        for (entry_index, entry) in entries.iter().enumerate() {
            if entry.entry_type != *target_tag {
                continue;
            }

            let blob_size = compute_blob_size(&entries, entry_index, file_size);
            if blob_size < 12 {
                continue;
            }

            match extract_thumbnail_from_blob(&mut file, entry.blobs_offset, blob_size, &header) {
                Ok(result) => return Ok(result),
                Err(error) => {
                    debug!(
                        "Failed to extract from entry {} (type 0x{:08X}): {}",
                        entry_index, entry.entry_type, error
                    );
                    continue;
                }
            }
        }
    }

    Err(Box::new(Sai2Error::ThumbnailNotFound))
}

pub fn extract_sai2_dimensions(path: &Path) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let header = parse_canvas_header(&mut file)?;
    Ok((header.canvas_width, header.canvas_height))
}

pub fn extract_sai2_metadata(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut technical_metadata = serde_json::json!({
        "container": "SAI v2",
        "metadata_support": "Limited"
    });

    if let Ok((width, height)) = extract_sai2_dimensions(path) {
        technical_metadata["width"] = serde_json::json!(width);
        technical_metadata["height"] = serde_json::json!(height);
        technical_metadata["metadata_source"] = serde_json::json!("header");
    }

    Ok(serde_json::json!({
        "technical": technical_metadata,
        "semantic": {}
    }))
}
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DIR: &str = "/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Imagens/Design/Paint tool SAI/sai2";

    #[test]
    fn test_elfinha_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("elfinha4.sai2");
        if path.exists() {
            let (width, height) = extract_sai2_dimensions(&path)?;
            // Real canvas dimensions (not TableCount/SelectedLayer)
            assert_eq!(width, 2490);
            assert_eq!(height, 3588);
        }

        Ok(())
    }

    #[test]
    fn test_design_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("design.sai2");
        if path.exists() {
            let (width, height) = extract_sai2_dimensions(&path)?;
            assert_eq!(width, 2480);
            assert_eq!(height, 3508);
        }

        Ok(())
    }

    #[test]
    fn test_milk_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("Milk.sai2");
        if path.exists() {
            let (width, height) = extract_sai2_dimensions(&path)?;
            assert_eq!(width, 4093);
            assert_eq!(height, 4093);
        }

        Ok(())
    }

    #[test]
    fn test_elfinha_preview() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("elfinha4.sai2");
        if path.exists() {
            let (data, mime) = extract_sai2_preview(&path)?;
            assert!(!data.is_empty(), "Preview data should not be empty");
            assert!(
                mime == "image/jpeg" || mime == "image/png",
                "Unexpected MIME: {}",
                mime
            );

            // Save for visual inspection
            let output_path = Path::new(TEST_DIR).join("_test_elfinha4_preview.png");
            if mime == "image/jpeg" {
                let output_path = Path::new(TEST_DIR).join("_test_elfinha4_preview.jpg");
                std::fs::write(&output_path, &data)?;
            } else {
                std::fs::write(&output_path, &data)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_design_preview() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("design.sai2");
        if path.exists() {
            let (data, mime) = extract_sai2_preview(&path)?;
            assert!(!data.is_empty());

            let extension = if mime == "image/jpeg" { "jpg" } else { "png" };
            let output_path =
                Path::new(TEST_DIR).join(format!("_test_design_preview.{}", extension));
            std::fs::write(&output_path, &data)?;
        }

        Ok(())
    }

    #[test]
    fn test_milk_preview() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("Milk.sai2");
        if path.exists() {
            let (data, mime) = extract_sai2_preview(&path)?;
            assert!(!data.is_empty());

            let extension = if mime == "image/jpeg" { "jpg" } else { "png" };
            let output_path = Path::new(TEST_DIR).join(format!("_test_milk_preview.{}", extension));
            std::fs::write(&output_path, &data)?;
        }

        Ok(())
    }

    #[test]
    fn test_all_sai2_files_extract() -> Result<(), Box<dyn std::error::Error>> {
        let test_directory = Path::new(TEST_DIR);
        if !test_directory.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(test_directory)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "sai2")
                && !path
                    .file_name()
                    .ok_or("Expected a value in test_all_sai2_files_extract")?
                    .to_str()
                    .ok_or("Expected a value in test_all_sai2_files_extract")?
                    .starts_with('_')
            {
                let file_name = path
                    .file_name()
                    .ok_or("Expected a value in test_all_sai2_files_extract")?
                    .to_str()
                    .ok_or("Expected a value in test_all_sai2_files_extract")?;
                match extract_sai2_preview(&path) {
                    Ok((data, mime)) => {
                        assert!(!data.is_empty(), "Empty preview for {}", file_name);
                        println!("✓ {} → {} bytes ({})", file_name, data.len(), mime);
                    }
                    Err(error) => {
                        panic!("✗ {} → Error: {}", file_name, error);
                    }
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_header_parsing_consistency() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(TEST_DIR).join("elfinha4.sai2");
        if !path.exists() {
            return Ok(());
        }
        let mut file = File::open(&path)?;
        let header = parse_canvas_header(&mut file)?;

        assert!(header.canvas_width > 0, "Width must be positive");
        assert!(header.canvas_height > 0, "Height must be positive");
        assert!(header.table_count > 0, "Must have at least one table entry");

        let entries = parse_canvas_table(&mut file, header.table_count)?;
        assert_eq!(entries.len(), header.table_count as usize);

        // Verify we find thum and intg entries
        let has_thum = entries.iter().any(|entry| entry.entry_type == TAG_THUM);
        let has_intg = entries.iter().any(|entry| entry.entry_type == TAG_INTG);
        assert!(has_thum, "Should have a 'thum' entry");
        assert!(has_intg, "Should have an 'intg' entry");

        Ok(())
    }
}
