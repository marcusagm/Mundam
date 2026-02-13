use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{BigEndian, ReadBytesExt};
use std::cmp;
use image::ImageEncoder;

/// Error type for XCF parsing.
#[derive(Debug, thiserror::Error)]
pub enum XcfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid XCF format")]
    InvalidFormat,
    #[allow(dead_code)]
    #[error("Unsupported XCF version: {0}")]
    UnsupportedVersion(u16),
    #[allow(dead_code)]
    #[error("Unsupported color depth or type")]
    UnsupportedColor,
    #[error("No layers found in XCF")]
    NoLayers,
}

struct LayerInfo {
    pointer: u64,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
}

pub fn extract_xcf_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    // 1. Validate Header
    let mut magic = [0u8; 9];
    reader.read_exact(&mut magic)?;
    if &magic != b"gimp xcf " {
        return Err(XcfError::InvalidFormat.into());
    }

    // 2. Read Version
    let mut version_bytes = [0u8; 4];
    reader.read_exact(&mut version_bytes)?;
    let version = if &version_bytes == b"file" {
        0
    } else if version_bytes[0] == b'v' {
        let version_str = std::str::from_utf8(&version_bytes[1..])?;
        version_str.parse::<u16>().unwrap_or(0)
    } else { 0 };

    // 3. Metadata
    reader.read_exact(&mut [0u8])?;
    let canvas_width = reader.read_u32::<BigEndian>()?;
    let canvas_height = reader.read_u32::<BigEndian>()?;
    let _base_type = reader.read_u32::<BigEndian>()?;

    if version >= 4 {
        reader.read_u32::<BigEndian>()?; // precision
    }

    // Skip Image Properties
    skip_properties(&mut reader)?;

    // 4. Collect all Layer Pointers
    let bytes_per_offset = if version >= 11 { 8 } else { 4 };
    let mut raw_layer_pointers = Vec::new();
    loop {
        let ptr = if bytes_per_offset == 8 {
            reader.read_u64::<BigEndian>()?
        } else {
            reader.read_u32::<BigEndian>()? as u64
        };
        if ptr == 0 { break; }
        raw_layer_pointers.push(ptr);
    }

    if raw_layer_pointers.is_empty() {
        return Err(XcfError::NoLayers.into());
    }

    // 5. Inspect Layers for visibility and offsets
    let mut visible_layers = Vec::new();
    for &ptr in &raw_layer_pointers {
        reader.seek(SeekFrom::Start(ptr))?;
        let width = reader.read_u32::<BigEndian>()?;
        let height = reader.read_u32::<BigEndian>()?;
        let _layer_type = reader.read_u32::<BigEndian>()?;
        let _name = read_gimp_string(&mut reader)?;

        let mut visible = true;
        let mut off_x = 0i32;
        let mut off_y = 0i32;

        loop {
            let p_type = reader.read_u32::<BigEndian>()?;
            let p_len = reader.read_u32::<BigEndian>()?;
            if p_type == 0 { break; }
            match p_type {
                8 => { // PROP_VISIBLE
                    visible = reader.read_u32::<BigEndian>()? != 0;
                },
                15 => { // PROP_OFFSETS
                    off_x = reader.read_i32::<BigEndian>()?;
                    off_y = reader.read_i32::<BigEndian>()?;
                },
                _ => {
                    reader.seek(SeekFrom::Current(p_len as i64))?;
                }
            }
        }

        if visible {
            let hptr = if bytes_per_offset == 8 {
                reader.read_u64::<BigEndian>()?
            } else {
                reader.read_u32::<BigEndian>()? as u64
            };

            visible_layers.push(LayerInfo {
                pointer: hptr,
                width,
                height,
                offset_x: off_x,
                offset_y: off_y,
            });
        }
    }

    // 6. Create Canvas Buffer (Transparent black)
    let mut canvas_data = vec![0u8; (canvas_width * canvas_height * 4) as usize];

    // 7. Composite visible layers from BOTTOM to TOP
    visible_layers.reverse();

    for layer in visible_layers {
        if layer.pointer == 0 { continue; }
        reader.seek(SeekFrom::Start(layer.pointer))?;
        let _h_w = reader.read_u32::<BigEndian>()?;
        let _h_h = reader.read_u32::<BigEndian>()?;
        let bpp = reader.read_u32::<BigEndian>()?;

        if bpp != 3 && bpp != 4 { continue; }

        let lptr = if bytes_per_offset == 8 {
            reader.read_u64::<BigEndian>()?
        } else {
            reader.read_u32::<BigEndian>()? as u64
        };
        if lptr == 0 { continue; }

        reader.seek(SeekFrom::Start(lptr))?;
        let _lvl_w = reader.read_u32::<BigEndian>()?;
        let _lvl_h = reader.read_u32::<BigEndian>()?;

        let tiles_x = (layer.width + 63) / 64;
        let tiles_y = (layer.height + 63) / 64;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                // Seek back to level pointers for each tile to be safe
                reader.seek(SeekFrom::Start(lptr + 8 + ((ty * tiles_x + tx) * bytes_per_offset) as u64))?;
                let tptr = if bytes_per_offset == 8 {
                    reader.read_u64::<BigEndian>()?
                } else {
                    reader.read_u32::<BigEndian>()? as u64
                };
                if tptr == 0 { continue; }

                let next_ptr_pos = reader.stream_position()?;
                reader.seek(SeekFrom::Start(tptr))?;

                decode_and_composite_tile_accurate(
                    &mut reader,
                    &mut canvas_data,
                    tx, ty,
                    layer.width, layer.height,
                    canvas_width, canvas_height,
                    layer.offset_x, layer.offset_y,
                    bpp
                )?;

                reader.seek(SeekFrom::Start(next_ptr_pos))?;
            }
        }
    }

    // 8. Final Output
    let mut png_data = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_data))
        .write_image(&canvas_data, canvas_width, canvas_height, image::ExtendedColorType::Rgba8)?;

    Ok((png_data, "image/png".to_string()))
}

fn skip_properties<R: Read + Seek>(reader: &mut R) -> Result<(), XcfError> {
    loop {
        let prop_type = reader.read_u32::<BigEndian>()?;
        let prop_len = reader.read_u32::<BigEndian>()?;
        if prop_type == 0 { break; }
        reader.seek(SeekFrom::Current(prop_len as i64))?;
    }
    Ok(())
}

fn read_gimp_string<R: Read>(reader: &mut R) -> Result<String, Box<dyn std::error::Error>> {
    let length = reader.read_u32::<BigEndian>()?;
    if length == 0 { return Ok(String::new()); }
    let mut buf = vec![0u8; length as usize];
    reader.read_exact(&mut buf)?;
    if let Some(pos) = buf.iter().position(|&b| b == 0) {
        buf.truncate(pos);
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn decode_and_composite_tile_accurate<R: Read>(
    reader: &mut R,
    canvas_data: &mut [u8],
    tx: u32,
    ty: u32,
    layer_w: u32,
    layer_h: u32,
    canvas_w: u32,
    canvas_h: u32,
    off_x: i32,
    off_y: i32,
    bpp: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_start = tx * 64;
    let y_start = ty * 64;
    let tile_w = cmp::min(64, layer_w - x_start);
    let tile_h = cmp::min(64, layer_h - y_start);
    let total_pixels = tile_w * tile_h;

    // Temp buffer for the full tile RGBA
    let mut tile_rgba = vec![0u8; (total_pixels * 4) as usize];
    if bpp == 3 {
        for i in 0..total_pixels {
            tile_rgba[(i * 4 + 3) as usize] = 255;
        }
    }

    for channel in 0..bpp {
        let mut read = 0;
        while read < total_pixels {
            let det = reader.read_u8()?;
            if det < 127 {
                let count = (det as u32) + 1;
                let val = reader.read_u8()?;
                for i in 0..count {
                    if read + i < total_pixels {
                        tile_rgba[((read + i) * 4 + channel) as usize] = val;
                    }
                }
                read += count;
            } else if det == 127 {
                let count = reader.read_u16::<BigEndian>()? as u32;
                let val = reader.read_u8()?;
                for i in 0..count {
                    if read + i < total_pixels {
                        tile_rgba[((read + i) * 4 + channel) as usize] = val;
                    }
                }
                read += count;
            } else if det == 128 {
                let count = reader.read_u16::<BigEndian>()? as u32;
                for i in 0..count {
                    let val = reader.read_u8()?;
                    if read + i < total_pixels {
                        tile_rgba[((read + i) * 4 + channel) as usize] = val;
                    }
                }
                read += count;
            } else {
                let count = 256 - det as u32;
                for i in 0..count {
                    let val = reader.read_u8()?;
                    if read + i < total_pixels {
                        tile_rgba[((read + i) * 4 + channel) as usize] = val;
                    }
                }
                read += count;
            }
        }
    }

    // Composite with Alpha Blending
    for ly in 0..tile_h {
        for lx in 0..tile_w {
            let gx = off_x + (x_start + lx) as i32;
            let gy = off_y + (y_start + ly) as i32;

            if gx < 0 || gy < 0 || gx >= canvas_w as i32 || gy >= canvas_h as i32 {
                continue;
            }

            let canvas_idx = ((gy as u32 * canvas_w + gx as u32) * 4) as usize;
            let tile_idx = ((ly * tile_w + lx) * 4) as usize;

            let sa = tile_rgba[tile_idx + 3] as u32;
            if sa == 0 { continue; }

            let sr = tile_rgba[tile_idx] as u32;
            let sg = tile_rgba[tile_idx + 1] as u32;
            let sb = tile_rgba[tile_idx + 2] as u32;

            let dr = canvas_data[canvas_idx] as u32;
            let dg = canvas_data[canvas_idx + 1] as u32;
            let db = canvas_data[canvas_idx + 2] as u32;
            let da = canvas_data[canvas_idx + 3] as u32;

            // Porter-Duff Over
            let out_a = sa + (da * (255 - sa) / 255);
            if out_a > 0 {
                canvas_data[canvas_idx] = ((sr * sa + dr * da * (255 - sa) / 255) / out_a) as u8;
                canvas_data[canvas_idx + 1] = ((sg * sa + dg * da * (255 - sa) / 255) / out_a) as u8;
                canvas_data[canvas_idx + 2] = ((sb * sa + db * da * (255 - sa) / 255) / out_a) as u8;
                canvas_data[canvas_idx + 3] = out_a as u8;
            }
        }
    }

    Ok(())
}
