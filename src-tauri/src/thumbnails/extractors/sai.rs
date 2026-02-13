//! PaintTool SAI v1 (.sai) preview extractor.
//!
//! SAI v1 files are encrypted containers wrapping a virtual file system (VFS).
//! Each 4096-byte page is encrypted using an ECB-style XOR cipher with a static
//! 256-entry key table. The VFS contains a FAT-like directory structure with a
//! `/thumbnail` entry that holds raw BGRA pixel data prefixed by a small header.
//!
//! This module ports the decryption and VFS logic from Wunkolo's `libsai` (C++, MIT)
//! into pure Rust. The architecture is modular: encryption, page I/O, VFS navigation,
//! and thumbnail extraction are separated to allow future expansion (full layer
//! rendering, metadata extraction, etc.).
//!
//! Reference: <https://github.com/Wunkolo/libsai>

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use image::ImageEncoder;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of a single page (block) in bytes.
const PAGE_SIZE: usize = 4096;

/// Number of `u32` values within a single page.
const PAGE_U32_COUNT: usize = PAGE_SIZE / 4;

/// Every 512th page is a Table page; the other 511 are Data pages.
const TABLE_SPAN: usize = PAGE_SIZE / 8; // 512

/// Maximum FAT entries per page (64 entries × 64 bytes = 4096).
const FAT_ENTRIES_PER_PAGE: usize = 64;

/// Size of a single FAT entry in bytes.
const FAT_ENTRY_SIZE: usize = 64;

/// Magic value expected in the thumbnail header (little-endian "BM32").
const THUMBNAIL_MAGIC_BM32: u32 = 0x3233_4D42; // "BM32" as LE u32

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during SAI v1 parsing.
#[derive(Debug, thiserror::Error)]
pub enum SaiError {
    /// The file size is not page-aligned (multiple of 4096).
    #[error("SAI file size is not page-aligned (must be a multiple of {PAGE_SIZE})")]
    InvalidFileSize,

    /// A page checksum did not match after decryption, indicating corruption.
    #[error("SAI page checksum mismatch at page index {0} — possible file corruption")]
    ChecksumMismatch(usize),

    /// The `/thumbnail` virtual file was not found inside the SAI container.
    #[error("SAI file does not contain a /thumbnail entry")]
    ThumbnailNotFound,

    /// The thumbnail header magic does not match the expected `BM32`.
    #[error("SAI thumbnail header has invalid magic (expected BM32)")]
    InvalidThumbnailMagic,

    /// Generic I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Image encoding error.
    #[error("Image encoding error: {0}")]
    ImageEncoding(String),
}

// ---------------------------------------------------------------------------
// Decryption Keys (static, from Wunkolo's libsai — MIT licensed)
// ---------------------------------------------------------------------------

/// Static 256-entry symmetric key used for decrypting user-created `.sai` files.
/// Extracted from PaintTool SAI by Wunkolo via reverse engineering.
#[rustfmt::skip]
const USER_KEY: [u32; 256] = [
    0x9913D29E, 0x83F58D3D, 0xD0BE1526, 0x86442EB7, 0x7EC69BFB, 0x89D75F64, 0xFB51B239, 0xFF097C56,
    0xA206EF1E, 0x973D668D, 0xC383770D, 0x1CB4CCEB, 0x36F7108B, 0x40336BCD, 0x84D123BD, 0xAFEF5DF3,
    0x90326747, 0xCBFFA8DD, 0x25B94703, 0xD7C5A4BA, 0xE40A17A0, 0xEADAE6F2, 0x6B738250, 0x76ECF24A,
    0x6F2746CC, 0x9BF95E24, 0x1ECA68C5, 0xE71C5929, 0x7817E56C, 0x2F99C471, 0x395A32B9, 0x61438343,
    0x5E3E4F88, 0x80A9332C, 0x1879C69F, 0x7A03D354, 0x12E89720, 0xF980448E, 0x03643576, 0x963C1D7B,
    0xBBED01D6, 0xC512A6B1, 0x51CB492B, 0x44BADEC9, 0xB2D54BC1, 0x4E7C2893, 0x1531C9A3, 0x43A32CA5,
    0x55B25A87, 0x70D9FA79, 0xEF5B4AE3, 0x8AE7F495, 0x923A8505, 0x1D92650C, 0xC94A9A5C, 0x27D4BB14,
    0x1372A9F7, 0x0C19A7FE, 0x64FA1A53, 0xF1A2EB6D, 0x9FEB910F, 0x4CE10C4E, 0x20825601, 0x7DFC98C4,
    0xA046C808, 0x8E90E7BE, 0x601DE357, 0xF360F37C, 0x00CD6F77, 0xCC6AB9D4, 0x24CC4E78, 0xAB1E0BFC,
    0x6A8BC585, 0xFD70ABF0, 0xD4A75261, 0x1ABF5834, 0x45DCFE17, 0x5F67E136, 0x948FD915, 0x65AD9EF5,
    0x81AB20E9, 0xD36EAF42, 0x0F7F45C7, 0x1BAE72D9, 0xBE116AC6, 0xDF58B4D5, 0x3F0B960E, 0xC2613F98,
    0xB065F8B0, 0x6259F975, 0xC49AEE84, 0x29718963, 0x0B6D991D, 0x09CF7A37, 0x692A6DF8, 0x67B68B02,
    0x2E10DBC2, 0x6C34E93C, 0xA84B50A1, 0xAC6FC0BB, 0x5CA6184C, 0x34E46183, 0x42B379A9, 0x79883AB6,
    0x08750921, 0x35AF2B19, 0xF7AA886A, 0x49F281D3, 0xA1768059, 0x14568CFD, 0x8B3625F6, 0x3E1B2D9D,
    0xF60E14CE, 0x1157270A, 0xDB5C7EB3, 0x738A0AFA, 0x19C248E5, 0x590CBD62, 0x7B37C312, 0xFC00B148,
    0xD808CF07, 0xD6BD1C82, 0xBD50F1D8, 0x91DEA3B8, 0xFA86B340, 0xF5DF2A80, 0x9A7BEA6E, 0x1720B8F1,
    0xED94A56B, 0xBF02BE28, 0x0D419FA8, 0x073B4DBC, 0x829E3144, 0x029F43E1, 0x71E6D51F, 0xA9381F09,
    0x583075E0, 0xE398D789, 0xF0E31106, 0x75073EB5, 0x5704863E, 0x6EF1043B, 0xBC407F33, 0x8DBCFB25,
    0x886C8F22, 0x5AF4DD7A, 0x2CEACA35, 0x8FC969DC, 0x9DB8D6B4, 0xC65EDC2F, 0xE60F9316, 0x0A84519A,
    0x3A294011, 0xDCF3063F, 0x41621623, 0x228CB75B, 0x28E9D166, 0xAE631B7F, 0x06D8C267, 0xDA693C94,
    0x54A5E860, 0x7C2170F4, 0xF2E294CB, 0x5B77A0F9, 0xB91522A6, 0xEC549500, 0x10DD78A7, 0x3823E458, 0x77D3635A,
    0x018E3069, 0xE039D055, 0xD5C341BF, 0x9C2400EA, 0x85C0A1D1, 0x66059C86, 0x0416FF1A, 0xE27E05C8,
    0xB19C4C2D, 0xFE4DF58F, 0xD2F0CE2A, 0x32E013C0, 0xEED637D7, 0xE9FEC1E8, 0xA4890DCA, 0xF4180313,
    0x7291738C, 0xE1B053A2, 0x9801267E, 0x2DA15BDB, 0xADC4DA4F, 0xCF95D474, 0xC0265781, 0x1F226CED,
    0xA7472952, 0x3C5F0273, 0xC152BA68, 0xDD66F09B, 0x93C7EDCF, 0x4F147404, 0x3193425D, 0x26B5768A,
    0x0E683B2E, 0x952FDF30, 0x2A6BAE46, 0xA3559270, 0xB781D897, 0xEB4ECB51, 0xDE49394D, 0x483F629C,
    0x2153845E, 0xB40D64E2, 0x47DB0ED0, 0x302D8E4B, 0x4BF8125F, 0x2BD2B0AC, 0x3DC836EC, 0xC7871965,
    0xB64C5CDE, 0x9EA8BC27, 0xD1853490, 0x3B42EC6F, 0x63A4FD91, 0xAA289D18, 0x4D2B1E49, 0xB8A060AD,
    0xB5F6C799, 0x6D1F7D1C, 0xBA8DAAE6, 0xE51A0FC3, 0xD94890E7, 0x167DF6D2, 0x879BCD41, 0x5096AC1B,
    0x05ACB5DA, 0x375D24EE, 0x7F2EB6AA, 0xA535F738, 0xCAD0AD10, 0xF8456E3A, 0x23FD5492, 0xB3745532,
    0x53C1A272, 0x469DFCDF, 0xE897BF7D, 0xA6BBE2AE, 0x68CE38AF, 0x5D783D0B, 0x524F21E4, 0x4A257B31,
    0xCE7A07B2, 0x562CE045, 0x33B708A4, 0x8CEE8AEF, 0xC8FB71FF, 0x74E52FAB, 0xCDB18796,
];

// ---------------------------------------------------------------------------
// Low-level Cryptography
// ---------------------------------------------------------------------------

/// Computes the key-sum for a given 32-bit vector using the static key table.
/// This is the core primitive used by both table and data decryption.
fn key_sum(vector: u32) -> u32 {
    let byte0 = (vector & 0xFF) as usize;
    let byte1 = ((vector >> 8) & 0xFF) as usize;
    let byte2 = ((vector >> 16) & 0xFF) as usize;
    let byte3 = ((vector >> 24) & 0xFF) as usize;

    USER_KEY[byte0]
        .wrapping_add(USER_KEY[byte1])
        .wrapping_add(USER_KEY[byte2])
        .wrapping_add(USER_KEY[byte3])
}

/// Decrypts a Table page in-place.
///
/// Table pages use a chained XOR + rotate cipher where each ciphertext word
/// feeds the next round as the vector. The initial vector is the page index
/// masked to the nearest table boundary.
fn decrypt_table_page(page_data: &mut [u32; PAGE_U32_COUNT], page_index: usize) {
    let mut previous_data = (page_index & !0x1FF) as u32;

    for current_word in page_data.iter_mut() {
        let cipher_word = *current_word;
        let xored = previous_data ^ cipher_word ^ key_sum(previous_data);
        *current_word = xored.rotate_left(16);
        previous_data = cipher_word;
    }
}

/// Decrypts a Data page in-place.
///
/// Data pages use a subtraction-based cipher where the checksum from the
/// corresponding table entry serves as the initial vector.
fn decrypt_data_page(page_data: &mut [u32; PAGE_U32_COUNT], checksum_vector: u32) {
    let mut previous_data = checksum_vector;

    for current_word in page_data.iter_mut() {
        let cipher_word = *current_word;
        *current_word = cipher_word.wrapping_sub(previous_data ^ key_sum(previous_data));
        previous_data = cipher_word;
    }
}

/// Computes the checksum for a decrypted page.
///
/// The checksum is a rotate-left-1 + XOR accumulation over all 1024 u32 values.
/// The lowest bit is always set, making all valid checksums odd.
fn compute_page_checksum(page_data: &[u32; PAGE_U32_COUNT]) -> u32 {
    let mut checksum = 0u32;
    for &word in page_data.iter() {
        checksum = checksum.rotate_left(1) ^ word;
    }
    checksum | 1
}

// ---------------------------------------------------------------------------
// Page Table Entry
// ---------------------------------------------------------------------------

/// A single entry within a Table page, describing a corresponding Data page.
#[derive(Debug, Clone, Copy)]
struct PageTableEntry {
    /// Checksum used to decrypt and verify the associated data page.
    checksum: u32,
    /// Index of the next page in a linked chain (0 = end of chain).
    next_page_index: u32,
}

/// Extracts the 512 table entries from a decrypted table page.
fn parse_table_entries(page_data: &[u32; PAGE_U32_COUNT]) -> Vec<PageTableEntry> {
    let mut entries = Vec::with_capacity(TABLE_SPAN);
    for entry_index in 0..TABLE_SPAN {
        entries.push(PageTableEntry {
            checksum: page_data[entry_index * 2],
            next_page_index: page_data[entry_index * 2 + 1],
        });
    }
    entries
}

// ---------------------------------------------------------------------------
// FAT Entry (Virtual File System directory entry)
// ---------------------------------------------------------------------------

/// Type of a FAT entry: file or folder.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FatEntryType {
    /// A directory/folder containing further FAT entries.
    Folder,
    /// A regular file with data at the indicated page.
    File,
    /// Unrecognized entry type.
    Unknown(u8),
}

/// A single File Allocation Table entry within the SAI virtual file system.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FatEntry {
    /// Non-zero flags indicate an active entry; zero means unused/end.
    flags: u32,
    /// The name of this file or folder (up to 32 ASCII bytes).
    name: String,
    /// Whether this entry is a file or folder.
    entry_type: FatEntryType,
    /// Starting page index for this entry's data (or subfolder FAT block).
    page_index: u32,
    /// Size of the file data in bytes (only meaningful for files).
    size: u32,
}

/// Parses 64 FAT entries from a decrypted page's raw bytes.
fn parse_fat_entries(page_bytes: &[u8; PAGE_SIZE]) -> Vec<FatEntry> {
    let mut entries = Vec::with_capacity(FAT_ENTRIES_PER_PAGE);

    for entry_index in 0..FAT_ENTRIES_PER_PAGE {
        let offset = entry_index * FAT_ENTRY_SIZE;
        let entry_slice = &page_bytes[offset..offset + FAT_ENTRY_SIZE];

        let flags = u32::from_le_bytes([
            entry_slice[0], entry_slice[1], entry_slice[2], entry_slice[3],
        ]);

        // An entry with flags == 0 is unused; stop iterating.
        if flags == 0 {
            break;
        }

        // Name is 32 bytes starting at offset 4, null-terminated ASCII.
        let name_bytes = &entry_slice[4..36];
        let name_end = name_bytes.iter().position(|&byte| byte == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();

        let type_byte = entry_slice[38];
        let entry_type = match type_byte {
            0x10 => FatEntryType::Folder,
            0x80 => FatEntryType::File,
            other => FatEntryType::Unknown(other),
        };

        let page_index = u32::from_le_bytes([
            entry_slice[40], entry_slice[41], entry_slice[42], entry_slice[43],
        ]);
        let size = u32::from_le_bytes([
            entry_slice[44], entry_slice[45], entry_slice[46], entry_slice[47],
        ]);

        entries.push(FatEntry {
            flags,
            name,
            entry_type,
            page_index,
            size,
        });
    }

    entries
}

// ---------------------------------------------------------------------------
// Encrypted Page Reader (core I/O abstraction)
// ---------------------------------------------------------------------------

/// Low-level reader for SAI v1 encrypted page I/O.
///
/// Handles reading raw pages from disk, decrypting them (table vs data),
/// and caching the most recently accessed table page for performance.
/// This struct is the foundation for all higher-level VFS operations.
struct SaiPageReader<R: Read + Seek> {
    /// The underlying file reader.
    reader: R,
    /// Total number of pages in the file.
    page_count: usize,
    /// Cached decrypted table page (index, data).
    cached_table: Option<(usize, [u32; PAGE_U32_COUNT])>,
}

impl<R: Read + Seek> SaiPageReader<R> {
    /// Creates a new page reader from a seekable byte stream.
    ///
    /// # Errors
    /// Returns `SaiError::InvalidFileSize` if the file size is not a multiple of 4096.
    fn new(mut reader: R) -> Result<Self, SaiError> {
        let file_size = reader.seek(SeekFrom::End(0))? as usize;
        if file_size % PAGE_SIZE != 0 || file_size == 0 {
            return Err(SaiError::InvalidFileSize);
        }

        Ok(SaiPageReader {
            reader,
            page_count: file_size / PAGE_SIZE,
            cached_table: None,
        })
    }

    /// Returns the nearest table page index for a given page index.
    fn nearest_table_index(page_index: usize) -> usize {
        (page_index / TABLE_SPAN) * TABLE_SPAN
    }

    /// Returns `true` if the given page index is a table page.
    fn is_table_index(page_index: usize) -> bool {
        page_index % TABLE_SPAN == 0
    }

    /// Reads and decrypts a raw page from the file, returning it as a u32 array.
    fn read_raw_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        if page_index >= self.page_count {
            return Err(SaiError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Page index {} exceeds file page count {}", page_index, self.page_count),
            )));
        }

        self.reader.seek(SeekFrom::Start((page_index * PAGE_SIZE) as u64))?;
        let mut raw_bytes = [0u8; PAGE_SIZE];
        self.reader.read_exact(&mut raw_bytes)?;

        // Reinterpret as u32 array (little-endian)
        let mut page_u32 = [0u32; PAGE_U32_COUNT];
        for (index, chunk) in raw_bytes.chunks_exact(4).enumerate() {
            page_u32[index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }

        Ok(page_u32)
    }

    /// Fetches a decrypted table page, using a single-entry cache.
    ///
    /// # Errors
    /// Returns `SaiError::Io` if the page cannot be read.
    fn fetch_table_page(&mut self, table_page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        // Cache hit
        if let Some((cached_index, cached_data)) = &self.cached_table {
            if *cached_index == table_page_index {
                return Ok(*cached_data);
            }
        }

        // Cache miss — read, decrypt, and store
        let mut page_data = self.read_raw_page(table_page_index)?;
        decrypt_table_page(&mut page_data, table_page_index);

        self.cached_table = Some((table_page_index, page_data));
        Ok(page_data)
    }

    /// Fetches a decrypted data page, verifying its checksum.
    ///
    /// # Errors
    /// Returns `SaiError::ChecksumMismatch` if decrypted data doesn't match.
    fn fetch_data_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        let table_index = Self::nearest_table_index(page_index);
        let table_data = self.fetch_table_page(table_index)?;
        let table_entries = parse_table_entries(&table_data);

        let entry_offset = page_index % TABLE_SPAN;
        let expected_checksum = table_entries[entry_offset].checksum;

        let mut page_data = self.read_raw_page(page_index)?;
        decrypt_data_page(&mut page_data, expected_checksum);

        let actual_checksum = compute_page_checksum(&page_data);
        if actual_checksum != expected_checksum {
            return Err(SaiError::ChecksumMismatch(page_index));
        }

        Ok(page_data)
    }

    /// Fetches any page (auto-detects table vs data).
    fn fetch_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        if Self::is_table_index(page_index) {
            self.fetch_table_page(page_index)
        } else {
            self.fetch_data_page(page_index)
        }
    }

    /// Converts a page's u32 data back into raw bytes.
    fn page_to_bytes(page_data: &[u32; PAGE_U32_COUNT]) -> [u8; PAGE_SIZE] {
        let mut bytes = [0u8; PAGE_SIZE];
        for (index, &word) in page_data.iter().enumerate() {
            let word_bytes = word.to_le_bytes();
            bytes[index * 4] = word_bytes[0];
            bytes[index * 4 + 1] = word_bytes[1];
            bytes[index * 4 + 2] = word_bytes[2];
            bytes[index * 4 + 3] = word_bytes[3];
        }
        bytes
    }

    /// Reads a contiguous stream of bytes from a chain of pages starting at `start_page_index`.
    ///
    /// SAI files store data as linked-list chains of pages. Table pages in between
    /// are skipped transparently, and the `NextPageIndex` field guides traversal.
    ///
    /// # Errors
    /// Returns errors from page fetch/decrypt or if the chain is broken.
    fn read_file_data(
        &mut self,
        start_page_index: usize,
        total_size: usize,
    ) -> Result<Vec<u8>, SaiError> {
        let mut result_buffer = Vec::with_capacity(total_size);
        let mut current_page_index = start_page_index;
        let mut bytes_remaining = total_size;

        while bytes_remaining > 0 && current_page_index != 0 {
            // Skip table pages — they are metadata, not file content
            if Self::is_table_index(current_page_index) {
                // Advance to the next data page after this table
                current_page_index += 1;
                continue;
            }

            let page_data = self.fetch_data_page(current_page_index)?;
            let page_bytes = Self::page_to_bytes(&page_data);

            let bytes_to_copy = bytes_remaining.min(PAGE_SIZE);
            result_buffer.extend_from_slice(&page_bytes[..bytes_to_copy]);
            bytes_remaining -= bytes_to_copy;

            if bytes_remaining == 0 {
                break;
            }

            // Follow the page chain via the table entry's next_page_index
            let table_index = Self::nearest_table_index(current_page_index);
            let table_data = self.fetch_table_page(table_index)?;
            let table_entries = parse_table_entries(&table_data);
            let entry_offset = current_page_index % TABLE_SPAN;
            let next_page = table_entries[entry_offset].next_page_index as usize;

            if next_page == 0 {
                break;
            }
            current_page_index = next_page;
        }

        Ok(result_buffer)
    }
}

// ---------------------------------------------------------------------------
// Virtual File System Navigation
// ---------------------------------------------------------------------------

/// Searches the root directory (page index 2) for a FAT entry with the given name.
///
/// Currently supports single-level lookups (no nested path resolution) which is
/// sufficient for `/thumbnail`. The architecture supports extending to full path
/// traversal for future layer reading.
fn find_root_entry<R: Read + Seek>(
    page_reader: &mut SaiPageReader<R>,
    target_name: &str,
) -> Result<Option<FatEntry>, SaiError> {
    // The root FAT block is always at page index 2
    let root_page_data = page_reader.fetch_page(2)?;
    let root_page_bytes = SaiPageReader::<R>::page_to_bytes(&root_page_data);
    let fat_entries = parse_fat_entries(&root_page_bytes);

    for entry in &fat_entries {
        if entry.name == target_name {
            return Ok(Some(entry.clone()));
        }
    }

    // The root directory might span multiple pages via chaining
    let table_index = SaiPageReader::<R>::nearest_table_index(2);
    let table_data = page_reader.fetch_table_page(table_index)?;
    let table_entries = parse_table_entries(&table_data);
    let mut next_page = table_entries[2 % TABLE_SPAN].next_page_index as usize;

    while next_page != 0 {
        let page_data = page_reader.fetch_page(next_page)?;
        let page_bytes = SaiPageReader::<R>::page_to_bytes(&page_data);
        let fat_entries = parse_fat_entries(&page_bytes);

        for entry in &fat_entries {
            if entry.name == target_name {
                return Ok(Some(entry.clone()));
            }
        }

        let chain_table_index = SaiPageReader::<R>::nearest_table_index(next_page);
        let chain_table_data = page_reader.fetch_table_page(chain_table_index)?;
        let chain_entries = parse_table_entries(&chain_table_data);
        next_page = chain_entries[next_page % TABLE_SPAN].next_page_index as usize;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Thumbnail Extraction
// ---------------------------------------------------------------------------

/// SAI v1 thumbnail header: width, height, magic ("BM32").
struct SaiThumbnailHeader {
    /// Width of the thumbnail in pixels.
    width: u32,
    /// Height of the thumbnail in pixels.
    height: u32,
}

/// Parses the 12-byte thumbnail header (width, height, magic).
fn parse_thumbnail_header(data: &[u8]) -> Result<SaiThumbnailHeader, SaiError> {
    if data.len() < 12 {
        return Err(SaiError::InvalidThumbnailMagic);
    }

    let width = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let magic = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    if magic != THUMBNAIL_MAGIC_BM32 {
        return Err(SaiError::InvalidThumbnailMagic);
    }

    Ok(SaiThumbnailHeader { width, height })
}

/// Converts raw BGRA pixel data to RGBA in-place.
///
/// SAI stores pixels as BGRA. This swaps the B and R channels to produce
/// standard RGBA for image encoding.
fn convert_bgra_to_rgba(pixel_data: &mut [u8]) {
    for pixel in pixel_data.chunks_exact_mut(4) {
        pixel.swap(0, 2); // Swap B and R
    }
}

/// Encodes raw RGBA pixel data into a PNG byte buffer.
fn encode_rgba_to_png(
    pixel_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, SaiError> {
    let mut png_buffer = Vec::new();
    let cursor = std::io::Cursor::new(&mut png_buffer);

    image::codecs::png::PngEncoder::new(cursor)
        .write_image(pixel_data, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|error| SaiError::ImageEncoding(error.to_string()))?;

    Ok(png_buffer)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extracts the embedded thumbnail from a PaintTool SAI v1 (.sai) file.
///
/// This performs the complete pipeline: open file → decrypt pages → navigate
/// the VFS → find the `/thumbnail` entry → read its data → decode BGRA →
/// encode as PNG.
///
/// # Arguments
/// * `sai_file_path` - Path to the .sai file on disk.
///
/// # Returns
/// A tuple of (PNG bytes, MIME type string).
///
/// # Errors
/// Returns `Err` if the file cannot be opened, decryption fails, the VFS
/// structure is corrupt, or no thumbnail entry exists.
pub fn extract_sai_preview(sai_file_path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(sai_file_path)?;
    let mut page_reader = SaiPageReader::new(file)?;

    // Locate the "thumbnail" entry in the root directory
    let thumbnail_entry = find_root_entry(&mut page_reader, "thumbnail")?
        .ok_or(SaiError::ThumbnailNotFound)?;

    // Read the full thumbnail file data through the page chain
    let thumbnail_raw_data = page_reader.read_file_data(
        thumbnail_entry.page_index as usize,
        thumbnail_entry.size as usize,
    )?;

    // Parse the 12-byte header (width, height, magic)
    let header = parse_thumbnail_header(&thumbnail_raw_data)?;

    let pixel_count = (header.width as usize) * (header.height as usize);
    let expected_pixel_data_size = pixel_count * 4;
    let header_size = 12;

    if thumbnail_raw_data.len() < header_size + expected_pixel_data_size {
        return Err(format!(
            "SAI thumbnail data too short: expected {} bytes of pixel data, got {}",
            expected_pixel_data_size,
            thumbnail_raw_data.len() - header_size,
        ).into());
    }

    // Extract pixel data (after the 12-byte header) and convert BGRA → RGBA
    let mut pixel_data = thumbnail_raw_data[header_size..header_size + expected_pixel_data_size].to_vec();
    convert_bgra_to_rgba(&mut pixel_data);

    let png_data = encode_rgba_to_png(&pixel_data, header.width, header.height)?;
    Ok((png_data, "image/png".to_string()))
}
