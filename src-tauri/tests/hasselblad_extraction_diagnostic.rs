//! Diagnostic integration test for Hasselblad 3FR and FFF extraction tiers.
//!
//! This test exercises each extraction tier independently against real sample files
//! to identify which tiers succeed or fail, and reports quality metrics (dimensions,
//! byte sizes) for each successful extraction.

use std::path::Path;

/// Represents the result of a single extraction tier attempt.
struct TierResult {
    tier_name: &'static str,
    success: bool,
    image_width: Option<u32>,
    image_height: Option<u32>,
    byte_count: Option<usize>,
    error_message: Option<String>,
}

impl TierResult {
    fn success(
        tier_name: &'static str,
        image_width: u32,
        image_height: u32,
        byte_count: usize,
    ) -> Self {
        Self {
            tier_name,
            success: true,
            image_width: Some(image_width),
            image_height: Some(image_height),
            byte_count: Some(byte_count),
            error_message: None,
        }
    }

    fn failure(tier_name: &'static str, error_message: String) -> Self {
        Self {
            tier_name,
            success: false,
            image_width: None,
            image_height: None,
            byte_count: None,
            error_message: Some(error_message),
        }
    }
}

/// Tests FlashRaw sensor decoding (Tier 0).
fn test_flashraw_sensor_decode(file_path: &Path) -> TierResult {
    let tier_name = "Tier 0: FlashRaw sensor decode";
    let file_bytes = match std::fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(error) => return TierResult::failure(tier_name, format!("File read error: {error}")),
    };
    let decoder = match flashraw::FlashRaw::open(file_bytes) {
        Ok(decoder) => decoder,
        Err(error) => {
            return TierResult::failure(tier_name, format!("FlashRaw open error: {error}"))
        }
    };
    match decoder.decode(flashraw::DecodeOptions {
        output: flashraw::OutputFormat::Rgba8,
        preview_fallback: true,
        ..Default::default()
    }) {
        Ok(frame) => {
            TierResult::success(tier_name, frame.width, frame.height, frame.data.byte_len())
        }
        Err(error) => TierResult::failure(tier_name, format!("FlashRaw decode error: {error}")),
    }
}

/// Tests FlashRaw's largest embedded preview extraction (Tier 1).
fn test_flashraw_extraction(file_path: &Path) -> TierResult {
    let tier_name = "Tier 1: FlashRaw";
    let file_bytes = match std::fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(error) => return TierResult::failure(tier_name, format!("File read error: {error}")),
    };
    let decoder = match flashraw::FlashRaw::open(file_bytes) {
        Ok(decoder) => decoder,
        Err(error) => {
            return TierResult::failure(tier_name, format!("FlashRaw open error: {error}"))
        }
    };
    let Some(preview) = decoder.preview() else {
        return TierResult::failure(tier_name, "No embedded preview found".into());
    };
    let preview_bytes = match decoder.preview_bytes(preview) {
        Ok(bytes) => bytes,
        Err(error) => {
            return TierResult::failure(tier_name, format!("Preview read error: {error}"))
        }
    };
    match image::load_from_memory(preview_bytes) {
        Ok(decoded_image) => TierResult::success(
            tier_name,
            decoded_image.width(),
            decoded_image.height(),
            preview_bytes.len(),
        ),
        Err(_) if preview.encoding == flashraw::PreviewEncoding::Rgb8 => TierResult::success(
            tier_name,
            preview.width,
            preview.height,
            preview_bytes.len(),
        ),
        Err(error) => TierResult::failure(tier_name, format!("Preview decode error: {error}")),
    }
}

/// Tests brute-force JPEG scan (Tier 2).
fn test_brute_force_extraction(file_path: &Path) -> TierResult {
    let tier_name = "Tier 2: Brute-force JPEG scan";

    let file_handle = match std::fs::File::open(file_path) {
        Ok(handle) => handle,
        Err(error) => return TierResult::failure(tier_name, format!("File open error: {}", error)),
    };

    let memory_map = match unsafe { memmap2::MmapOptions::new().map(&file_handle) } {
        Ok(mapped) => mapped,
        Err(error) => return TierResult::failure(tier_name, format!("Mmap error: {}", error)),
    };

    let scan_limit = memory_map.len().min(30 * 1024 * 1024);
    let mut best_bytes: Option<Vec<u8>> = None;
    let mut best_pixel_count = 0u32;
    let mut scan_offset = 0usize;
    let mut jpeg_candidates_found = 0u32;

    while scan_offset < scan_limit.saturating_sub(4) {
        let is_jpeg_marker = memory_map[scan_offset] == 0xFF
            && memory_map[scan_offset + 1] == 0xD8
            && memory_map[scan_offset + 2] == 0xFF;

        if is_jpeg_marker {
            jpeg_candidates_found += 1;
            if let Ok(decoded_image) = image::load_from_memory(&memory_map[scan_offset..]) {
                let pixel_count = decoded_image.width() * decoded_image.height();
                eprintln!(
                    "    [BruteForce] JPEG at offset 0x{:08X}: {}x{} ({} pixels)",
                    scan_offset,
                    decoded_image.width(),
                    decoded_image.height(),
                    pixel_count
                );
                if pixel_count > best_pixel_count {
                    best_pixel_count = pixel_count;

                    // Find EOI marker
                    let eoi_limit = (scan_offset + 20 * 1024 * 1024).min(memory_map.len());
                    let mut end_offset = scan_offset + 2;
                    while end_offset < eoi_limit.saturating_sub(1) {
                        if memory_map[end_offset] == 0xFF && memory_map[end_offset + 1] == 0xD9 {
                            end_offset += 2;
                            break;
                        }
                        end_offset += 1;
                    }
                    best_bytes = Some(memory_map[scan_offset..end_offset].to_vec());
                }
                scan_offset += 2048;
                continue;
            }
        }
        scan_offset += 1;
    }

    eprintln!(
        "    [BruteForce] Total JPEG SOI markers found: {}",
        jpeg_candidates_found
    );

    if let Some(bytes) = best_bytes {
        match image::load_from_memory(&bytes) {
            Ok(decoded_image) => TierResult::success(
                tier_name,
                decoded_image.width(),
                decoded_image.height(),
                bytes.len(),
            ),
            Err(decode_error) => TierResult::failure(
                tier_name,
                format!("Best JPEG re-decode failed: {}", decode_error),
            ),
        }
    } else {
        TierResult::failure(tier_name, "No valid JPEG found in scan range".into())
    }
}

/// Tests TIFF IFD-based JPEG extraction (Tier 1.5 — TIFF-aware).
fn test_tiff_ifd_extraction(file_path: &Path) -> TierResult {
    let tier_name = "Tier 1.5: TIFF IFD JPEG extraction";

    let file_bytes = match std::fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(error) => return TierResult::failure(tier_name, format!("File read error: {}", error)),
    };

    if file_bytes.len() < 8 {
        return TierResult::failure(tier_name, "File too small for TIFF header".into());
    }

    // Check TIFF magic
    let is_little_endian = file_bytes[0] == 0x49 && file_bytes[1] == 0x49;
    let is_big_endian = file_bytes[0] == 0x4D && file_bytes[1] == 0x4D;

    if !is_little_endian && !is_big_endian {
        return TierResult::failure(
            tier_name,
            format!(
                "Not a TIFF file (magic: {:02X} {:02X})",
                file_bytes[0], file_bytes[1]
            ),
        );
    }

    let read_u16 = |offset: usize| -> u16 {
        if is_little_endian {
            u16::from_le_bytes([file_bytes[offset], file_bytes[offset + 1]])
        } else {
            u16::from_be_bytes([file_bytes[offset], file_bytes[offset + 1]])
        }
    };

    let read_u32 = |offset: usize| -> u32 {
        if is_little_endian {
            u32::from_le_bytes([
                file_bytes[offset],
                file_bytes[offset + 1],
                file_bytes[offset + 2],
                file_bytes[offset + 3],
            ])
        } else {
            u32::from_be_bytes([
                file_bytes[offset],
                file_bytes[offset + 1],
                file_bytes[offset + 2],
                file_bytes[offset + 3],
            ])
        }
    };

    let tiff_magic = read_u16(2);
    eprintln!(
        "    [TIFF] Endian: {}, Magic: 0x{:04X}",
        if is_little_endian { "LE" } else { "BE" },
        tiff_magic
    );

    if tiff_magic != 0x002A && tiff_magic != 0x002B {
        return TierResult::failure(
            tier_name,
            format!("Unknown TIFF magic: 0x{:04X}", tiff_magic),
        );
    }

    let mut ifd_offset = read_u32(4) as usize;
    let mut ifd_index = 0u32;
    let mut best_jpeg_data: Option<Vec<u8>> = None;
    let mut best_jpeg_pixels = 0u32;

    while ifd_offset > 0 && ifd_offset < file_bytes.len().saturating_sub(2) {
        let entry_count = read_u16(ifd_offset) as usize;
        eprintln!(
            "    [TIFF] IFD #{} at offset 0x{:08X}: {} entries",
            ifd_index, ifd_offset, entry_count
        );

        let mut compression_value: Option<u32> = None;
        let mut strip_offsets: Vec<u32> = Vec::new();
        let mut strip_byte_counts: Vec<u32> = Vec::new();
        let mut jpeg_if_offset: Option<u32> = None;
        let mut jpeg_if_byte_count: Option<u32> = None;
        let mut sub_ifd_offsets: Vec<u32> = Vec::new();
        let mut image_width: Option<u32> = None;
        let mut image_height: Option<u32> = None;

        for entry_index in 0..entry_count {
            let tag_offset = ifd_offset + 2 + entry_index * 12;
            if tag_offset + 12 > file_bytes.len() {
                break;
            }

            let tag_id = read_u16(tag_offset);
            let _tag_type = read_u16(tag_offset + 2);
            let tag_count = read_u32(tag_offset + 4);
            let tag_value_or_offset = read_u32(tag_offset + 8);

            match tag_id {
                0x0100 => {
                    // ImageWidth
                    image_width = Some(tag_value_or_offset);
                }
                0x0101 => {
                    // ImageLength (Height)
                    image_height = Some(tag_value_or_offset);
                }
                0x0103 => {
                    // Compression
                    compression_value = Some(tag_value_or_offset);
                }
                0x0111 => {
                    // StripOffsets
                    if tag_count == 1 {
                        strip_offsets.push(tag_value_or_offset);
                    } else {
                        let data_offset = tag_value_or_offset as usize;
                        for strip_index in 0..tag_count.min(64) as usize {
                            let offset_position = data_offset + strip_index * 4;
                            if offset_position + 4 <= file_bytes.len() {
                                strip_offsets.push(read_u32(offset_position));
                            }
                        }
                    }
                }
                0x0117 => {
                    // StripByteCounts
                    if tag_count == 1 {
                        strip_byte_counts.push(tag_value_or_offset);
                    } else {
                        let data_offset = tag_value_or_offset as usize;
                        for strip_index in 0..tag_count.min(64) as usize {
                            let offset_position = data_offset + strip_index * 4;
                            if offset_position + 4 <= file_bytes.len() {
                                strip_byte_counts.push(read_u32(offset_position));
                            }
                        }
                    }
                }
                0x0201 => {
                    // JPEGInterchangeFormat (thumbnail JPEG offset)
                    jpeg_if_offset = Some(tag_value_or_offset);
                }
                0x0202 => {
                    // JPEGInterchangeFormatLength
                    jpeg_if_byte_count = Some(tag_value_or_offset);
                }
                0x014A => {
                    // SubIFDs
                    if tag_count == 1 {
                        sub_ifd_offsets.push(tag_value_or_offset);
                    } else {
                        let data_offset = tag_value_or_offset as usize;
                        for sub_index in 0..tag_count.min(16) as usize {
                            let offset_position = data_offset + sub_index * 4;
                            if offset_position + 4 <= file_bytes.len() {
                                sub_ifd_offsets.push(read_u32(offset_position));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        eprintln!(
            "    [TIFF]   Compression: {:?}, Dimensions: {:?}x{:?}",
            compression_value, image_width, image_height
        );

        // Check for JPEG via JPEGInterchangeFormat
        if let (Some(jpeg_offset), Some(jpeg_length)) = (jpeg_if_offset, jpeg_if_byte_count) {
            let jpeg_start = jpeg_offset as usize;
            let jpeg_end = jpeg_start + jpeg_length as usize;
            if jpeg_end <= file_bytes.len() {
                let jpeg_slice = &file_bytes[jpeg_start..jpeg_end];
                if let Ok(decoded_image) = image::load_from_memory(jpeg_slice) {
                    let pixel_count = decoded_image.width() * decoded_image.height();
                    eprintln!(
                        "    [TIFF]   JPEGInterchangeFormat: {}x{} at 0x{:08X} ({} bytes)",
                        decoded_image.width(),
                        decoded_image.height(),
                        jpeg_offset,
                        jpeg_length
                    );
                    if pixel_count > best_jpeg_pixels {
                        best_jpeg_pixels = pixel_count;
                        best_jpeg_data = Some(jpeg_slice.to_vec());
                    }
                }
            }
        }

        // Check for JPEG via Compression=6 + StripOffsets
        if compression_value == Some(6)
            && !strip_offsets.is_empty()
            && !strip_byte_counts.is_empty()
        {
            for (strip_idx, (offset, count)) in strip_offsets
                .iter()
                .zip(strip_byte_counts.iter())
                .enumerate()
            {
                let strip_start = *offset as usize;
                let strip_end = strip_start + *count as usize;
                if strip_end <= file_bytes.len() {
                    let strip_slice = &file_bytes[strip_start..strip_end];
                    if let Ok(decoded_image) = image::load_from_memory(strip_slice) {
                        let pixel_count = decoded_image.width() * decoded_image.height();
                        eprintln!(
                            "    [TIFF]   Strip[{}] JPEG: {}x{} at 0x{:08X} ({} bytes)",
                            strip_idx,
                            decoded_image.width(),
                            decoded_image.height(),
                            offset,
                            count
                        );
                        if pixel_count > best_jpeg_pixels {
                            best_jpeg_pixels = pixel_count;
                            best_jpeg_data = Some(strip_slice.to_vec());
                        }
                    }
                }
            }
        }

        // Check SubIFDs — these often contain the high-quality JPEG preview in 3FR/FFF
        for sub_ifd_start in &sub_ifd_offsets {
            let sub_offset = *sub_ifd_start as usize;
            if sub_offset + 2 >= file_bytes.len() {
                continue;
            }
            let sub_entry_count = read_u16(sub_offset) as usize;
            eprintln!(
                "    [TIFF]   SubIFD at 0x{:08X}: {} entries",
                sub_ifd_start, sub_entry_count
            );

            let mut sub_compression: Option<u32> = None;
            let mut sub_strip_offsets: Vec<u32> = Vec::new();
            let mut sub_strip_byte_counts: Vec<u32> = Vec::new();
            let mut sub_jpeg_offset: Option<u32> = None;
            let mut sub_jpeg_length: Option<u32> = None;
            let mut sub_width: Option<u32> = None;
            let mut sub_height: Option<u32> = None;

            for sub_entry_index in 0..sub_entry_count {
                let sub_tag_offset = sub_offset + 2 + sub_entry_index * 12;
                if sub_tag_offset + 12 > file_bytes.len() {
                    break;
                }

                let sub_tag_id = read_u16(sub_tag_offset);
                let sub_tag_count = read_u32(sub_tag_offset + 4);
                let sub_tag_value = read_u32(sub_tag_offset + 8);

                match sub_tag_id {
                    0x0100 => sub_width = Some(sub_tag_value),
                    0x0101 => sub_height = Some(sub_tag_value),
                    0x0103 => sub_compression = Some(sub_tag_value),
                    0x0111 => {
                        if sub_tag_count == 1 {
                            sub_strip_offsets.push(sub_tag_value);
                        } else {
                            let data_off = sub_tag_value as usize;
                            for strip_idx in 0..sub_tag_count.min(64) as usize {
                                let pos = data_off + strip_idx * 4;
                                if pos + 4 <= file_bytes.len() {
                                    sub_strip_offsets.push(read_u32(pos));
                                }
                            }
                        }
                    }
                    0x0117 => {
                        if sub_tag_count == 1 {
                            sub_strip_byte_counts.push(sub_tag_value);
                        } else {
                            let data_off = sub_tag_value as usize;
                            for strip_idx in 0..sub_tag_count.min(64) as usize {
                                let pos = data_off + strip_idx * 4;
                                if pos + 4 <= file_bytes.len() {
                                    sub_strip_byte_counts.push(read_u32(pos));
                                }
                            }
                        }
                    }
                    0x0201 => sub_jpeg_offset = Some(sub_tag_value),
                    0x0202 => sub_jpeg_length = Some(sub_tag_value),
                    _ => {}
                }
            }

            eprintln!(
                "    [TIFF]   SubIFD Compression: {:?}, Dimensions: {:?}x{:?}",
                sub_compression, sub_width, sub_height
            );

            // Try JPEGInterchangeFormat in SubIFD
            if let (Some(jpeg_off), Some(jpeg_len)) = (sub_jpeg_offset, sub_jpeg_length) {
                let start = jpeg_off as usize;
                let end = start + jpeg_len as usize;
                if end <= file_bytes.len() {
                    if let Ok(decoded) = image::load_from_memory(&file_bytes[start..end]) {
                        let pixels = decoded.width() * decoded.height();
                        eprintln!(
                            "    [TIFF]   SubIFD JPEGInterchangeFormat: {}x{} at 0x{:08X}",
                            decoded.width(),
                            decoded.height(),
                            jpeg_off
                        );
                        if pixels > best_jpeg_pixels {
                            best_jpeg_pixels = pixels;
                            best_jpeg_data = Some(file_bytes[start..end].to_vec());
                        }
                    }
                }
            }

            // Try Compression=6 + StripOffsets in SubIFD
            if sub_compression == Some(6)
                && !sub_strip_offsets.is_empty()
                && !sub_strip_byte_counts.is_empty()
            {
                for (idx, (offset, count)) in sub_strip_offsets
                    .iter()
                    .zip(sub_strip_byte_counts.iter())
                    .enumerate()
                {
                    let start = *offset as usize;
                    let end = start + *count as usize;
                    if end <= file_bytes.len() {
                        if let Ok(decoded) = image::load_from_memory(&file_bytes[start..end]) {
                            let pixels = decoded.width() * decoded.height();
                            eprintln!(
                                "    [TIFF]   SubIFD Strip[{}] JPEG: {}x{} at 0x{:08X}",
                                idx,
                                decoded.width(),
                                decoded.height(),
                                offset
                            );
                            if pixels > best_jpeg_pixels {
                                best_jpeg_pixels = pixels;
                                best_jpeg_data = Some(file_bytes[start..end].to_vec());
                            }
                        }
                    }
                }
            }
        }

        // Move to next IFD
        let next_ifd_position = ifd_offset + 2 + entry_count * 12;
        if next_ifd_position + 4 <= file_bytes.len() {
            ifd_offset = read_u32(next_ifd_position) as usize;
        } else {
            break;
        }
        ifd_index += 1;

        // Safety limit
        if ifd_index > 20 {
            break;
        }
    }

    if let Some(jpeg_data) = best_jpeg_data {
        match image::load_from_memory(&jpeg_data) {
            Ok(decoded_image) => TierResult::success(
                tier_name,
                decoded_image.width(),
                decoded_image.height(),
                jpeg_data.len(),
            ),
            Err(decode_error) => TierResult::failure(
                tier_name,
                format!("Best IFD JPEG decode failed: {}", decode_error),
            ),
        }
    } else {
        TierResult::failure(tier_name, "No JPEG found in TIFF IFD structure".into())
    }
}

/// Runs all extraction tiers on a single file and prints a diagnostic report.
fn diagnose_file(file_path: &Path) {
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
    let file_size_megabytes = std::fs::metadata(file_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0) as f64
        / (1024.0 * 1024.0);

    eprintln!("\n{}", "=".repeat(80));
    eprintln!("FILE: {}", file_name);
    eprintln!("SIZE: {:.1} MB", file_size_megabytes);
    eprintln!("PATH: {}", file_path.display());
    eprintln!("{}", "=".repeat(80));

    let tier_results = vec![
        test_tiff_ifd_extraction(file_path),
        test_flashraw_sensor_decode(file_path),
        test_flashraw_extraction(file_path),
        test_brute_force_extraction(file_path),
    ];

    eprintln!("\n  SUMMARY:");
    for result in &tier_results {
        if result.success {
            eprintln!(
                "  ✅ {} → {}x{} ({} bytes)",
                result.tier_name,
                result.image_width.unwrap_or(0),
                result.image_height.unwrap_or(0),
                result.byte_count.unwrap_or(0),
            );
        } else {
            eprintln!(
                "  ❌ {} → {}",
                result.tier_name,
                result.error_message.as_deref().unwrap_or("unknown error")
            );
        }
    }

    // Report but don't assert — we want to see all files
    let any_success = tier_results.iter().any(|result| result.success);
    if !any_success {
        eprintln!("  ⚠️  ALL extraction tiers FAILED for file: {}", file_name);
    }
}

#[test]
fn test_hasselblad_3fr_extraction() {
    let sample_directory =
        Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/3fr");

    if !sample_directory.exists() {
        eprintln!("SKIPPED: 3FR sample directory not found");
        return;
    }

    let sample_files: Vec<_> = std::fs::read_dir(sample_directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("3fr"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !sample_files.is_empty(),
        "No .3fr sample files found in directory"
    );

    for sample_file in &sample_files {
        diagnose_file(&sample_file.path());
    }
}

#[test]
fn test_hasselblad_fff_extraction() {
    let sample_directory =
        Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/fff");

    if !sample_directory.exists() {
        eprintln!("SKIPPED: FFF sample directory not found");
        return;
    }

    let sample_files: Vec<_> = std::fs::read_dir(sample_directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("fff"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !sample_files.is_empty(),
        "No .fff sample files found in directory"
    );

    for sample_file in &sample_files {
        diagnose_file(&sample_file.path());
    }
}
