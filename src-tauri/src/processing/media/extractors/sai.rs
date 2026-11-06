//! PaintTool SAI v1 (.sai) preview extractor.
//!
//! Ported from V1 backend.

use image::{ExtendedColorType, ImageEncoder};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during SAI v1 parsing.
#[derive(Debug, Error)]
pub enum SaiError {
    #[error("SAI file size is not page-aligned")]
    InvalidFileSize,
    #[error("SAI page checksum mismatch")]
    ChecksumMismatch(usize),
    #[error("SAI file does not contain a /thumbnail entry")]
    ThumbnailNotFound,
    #[error("SAI thumbnail header has invalid magic")]
    InvalidThumbnailMagic,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image encoding error: {0}")]
    ImageEncoding(String),
}

const PAGE_SIZE: usize = 4096;
const PAGE_U32_COUNT: usize = PAGE_SIZE / 4;
const TABLE_SPAN: usize = PAGE_SIZE / 8;
const FAT_ENTRIES_PER_PAGE: usize = 64;
const FAT_ENTRY_SIZE: usize = 64;
const THUMBNAIL_MAGIC_BM32: u32 = 0x3233_4D42;

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

fn decrypt_table_page(page_data: &mut [u32; PAGE_U32_COUNT], page_index: usize) {
    let mut previous_data = (page_index & !0x1FF) as u32;
    for current_word in page_data.iter_mut() {
        let cipher_word = *current_word;
        let xored = previous_data ^ cipher_word ^ key_sum(previous_data);
        *current_word = xored.rotate_left(16);
        previous_data = cipher_word;
    }
}

fn decrypt_data_page(page_data: &mut [u32; PAGE_U32_COUNT], checksum_vector: u32) {
    let mut previous_data = checksum_vector;
    for current_word in page_data.iter_mut() {
        let cipher_word = *current_word;
        *current_word = cipher_word.wrapping_sub(previous_data ^ key_sum(previous_data));
        previous_data = cipher_word;
    }
}

fn compute_page_checksum(page_data: &[u32; PAGE_U32_COUNT]) -> u32 {
    let mut checksum = 0u32;
    for &word in page_data.iter() {
        checksum = checksum.rotate_left(1) ^ word;
    }
    checksum | 1
}

#[derive(Debug, Clone, Copy)]
struct PageTableEntry {
    checksum: u32,
    next_page_index: u32,
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum FatEntryType {
    Folder,
    File,
    Unknown(u8),
}

#[derive(Debug, Clone)]
struct FatEntry {
    name: String,
    _entry_type: FatEntryType,
    page_index: u32,
    size: u32,
}

fn parse_fat_entries(page_bytes: &[u8; PAGE_SIZE]) -> Vec<FatEntry> {
    let mut entries = Vec::with_capacity(FAT_ENTRIES_PER_PAGE);
    for entry_index in 0..FAT_ENTRIES_PER_PAGE {
        let offset = entry_index * FAT_ENTRY_SIZE;
        let entry_slice = &page_bytes[offset..offset + FAT_ENTRY_SIZE];
        let flags = u32::from_le_bytes([
            entry_slice[0],
            entry_slice[1],
            entry_slice[2],
            entry_slice[3],
        ]);
        if flags == 0 {
            break;
        }
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
            entry_slice[40],
            entry_slice[41],
            entry_slice[42],
            entry_slice[43],
        ]);
        let size = u32::from_le_bytes([
            entry_slice[44],
            entry_slice[45],
            entry_slice[46],
            entry_slice[47],
        ]);
        entries.push(FatEntry {
            name,
            _entry_type: entry_type,
            page_index,
            size,
        });
    }
    entries
}

struct SaiPageReader<R: Read + Seek> {
    reader: R,
    page_count: usize,
    cached_table: Option<(usize, [u32; PAGE_U32_COUNT])>,
}

impl<R: Read + Seek> SaiPageReader<R> {
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

    fn read_raw_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        if page_index >= self.page_count {
            return Err(SaiError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Page index out of bounds",
            )));
        }
        self.reader
            .seek(SeekFrom::Start((page_index * PAGE_SIZE) as u64))?;
        let mut raw_bytes = [0u8; PAGE_SIZE];
        self.reader.read_exact(&mut raw_bytes)?;
        let mut page_u32 = [0u32; PAGE_U32_COUNT];
        for (index, chunk) in raw_bytes.chunks_exact(4).enumerate() {
            page_u32[index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Ok(page_u32)
    }

    fn fetch_table_page(
        &mut self,
        table_page_index: usize,
    ) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        if let Some((cached_index, cached_data)) = &self.cached_table {
            if *cached_index == table_page_index {
                return Ok(*cached_data);
            }
        }
        let mut page_data = self.read_raw_page(table_page_index)?;
        decrypt_table_page(&mut page_data, table_page_index);
        self.cached_table = Some((table_page_index, page_data));
        Ok(page_data)
    }

    fn fetch_data_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        let table_index = (page_index / TABLE_SPAN) * TABLE_SPAN;
        let table_data = self.fetch_table_page(table_index)?;
        let table_entries = parse_table_entries(&table_data);
        let expected_checksum = table_entries[page_index % TABLE_SPAN].checksum;
        let mut page_data = self.read_raw_page(page_index)?;
        decrypt_data_page(&mut page_data, expected_checksum);
        if compute_page_checksum(&page_data) != expected_checksum {
            return Err(SaiError::ChecksumMismatch(page_index));
        }
        Ok(page_data)
    }

    fn fetch_page(&mut self, page_index: usize) -> Result<[u32; PAGE_U32_COUNT], SaiError> {
        if page_index % TABLE_SPAN == 0 {
            self.fetch_table_page(page_index)
        } else {
            self.fetch_data_page(page_index)
        }
    }

    fn page_to_bytes(page_data: &[u32; PAGE_U32_COUNT]) -> [u8; PAGE_SIZE] {
        let mut bytes = [0u8; PAGE_SIZE];
        for (index, &word) in page_data.iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    fn read_file_data(
        &mut self,
        start_page_index: usize,
        total_size: usize,
    ) -> Result<Vec<u8>, SaiError> {
        let mut result_buffer = Vec::with_capacity(total_size);
        let mut current_page_index = start_page_index;
        let mut bytes_remaining = total_size;
        while bytes_remaining > 0 && current_page_index != 0 {
            if current_page_index % TABLE_SPAN == 0 {
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
            let table_index = (current_page_index / TABLE_SPAN) * TABLE_SPAN;
            let table_data = self.fetch_table_page(table_index)?;
            let table_entries = parse_table_entries(&table_data);
            current_page_index =
                table_entries[current_page_index % TABLE_SPAN].next_page_index as usize;
        }
        Ok(result_buffer)
    }
}

fn find_root_entry<R: Read + Seek>(
    page_reader: &mut SaiPageReader<R>,
    target_name: &str,
) -> Result<Option<FatEntry>, SaiError> {
    let mut current_page = 2;
    while current_page != 0 {
        let page_data = page_reader.fetch_page(current_page)?;
        let page_bytes = SaiPageReader::<R>::page_to_bytes(&page_data);
        for entry in parse_fat_entries(&page_bytes) {
            if entry.name == target_name {
                return Ok(Some(entry));
            }
        }
        let table_index = (current_page / TABLE_SPAN) * TABLE_SPAN;
        let table_data = page_reader.fetch_table_page(table_index)?;
        let table_entries = parse_table_entries(&table_data);
        current_page = table_entries[current_page % TABLE_SPAN].next_page_index as usize;
    }
    Ok(None)
}

/// Extrai metadados de arquivos SAI v1.
pub fn extract_sai_metadata(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut technical_metadata = serde_json::json!({
        "container": "SAI v1",
        "metadata_support": "Limited"
    });

    if let Ok((width, height)) = extract_sai_dimensions(path) {
        technical_metadata["width"] = serde_json::json!(width);
        technical_metadata["height"] = serde_json::json!(height);
        technical_metadata["metadata_source"] = serde_json::json!("thumbnail");
    }

    Ok(serde_json::json!({
        "technical": technical_metadata,
        "semantic": {}
    }))
}

/// Extrai apenas as dimensões do thumbnail do arquivo SAI.
pub fn extract_sai_dimensions(path: &Path) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = SaiPageReader::new(file)?;
    let entry = find_root_entry(&mut reader, "thumbnail")?.ok_or(SaiError::ThumbnailNotFound)?;
    let raw = reader.read_file_data(entry.page_index as usize, entry.size as usize)?;
    if raw.len() < 12 {
        return Err(SaiError::InvalidThumbnailMagic.into());
    }
    let width = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let height = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    Ok((width, height))
}

pub fn extract_sai_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = SaiPageReader::new(file)?;
    let entry = find_root_entry(&mut reader, "thumbnail")?.ok_or(SaiError::ThumbnailNotFound)?;
    let raw = reader.read_file_data(entry.page_index as usize, entry.size as usize)?;
    if raw.len() < 12 {
        return Err(SaiError::InvalidThumbnailMagic.into());
    }
    let width = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let height = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    let magic = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);
    if magic != THUMBNAIL_MAGIC_BM32 {
        return Err(SaiError::InvalidThumbnailMagic.into());
    }
    let pixel_data_size = (width * height * 4) as usize;
    if raw.len() < 12 + pixel_data_size {
        return Err("Thumbnail data truncated".into());
    }
    let mut pixels = raw[12..12 + pixel_data_size].to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png))
        .write_image(&pixels, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| SaiError::ImageEncoding(e.to_string()))?;
    Ok((png, "image/png".to_string()))
}
