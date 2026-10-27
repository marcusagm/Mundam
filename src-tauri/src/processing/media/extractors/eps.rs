//! Encapsulated PostScript (.eps) preview extractor.
//!
//! Ported from V1 backend.

use crate::processing::media::extractors::{ai, binary_jpeg};
use std::path::Path;

pub fn extract_eps_ps_preview(
    path: &Path,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    if let Ok(res) = binary_jpeg::extract_eps_binary_pointer(path) {
        return Ok(res);
    }
    if let Ok(pdf) = convert_ps_to_pdf(path) {
        return Ok((pdf, "application/pdf".to_string()));
    }
    if let Ok(data) = binary_jpeg::extract_xmp_thumbnail(path) {
        return Ok((data, "image/png".to_string()));
    }
    if let Ok(res) = binary_jpeg::extract_any_embedded(path) {
        return Ok(res);
    }
    if let Ok(pdf) = ai::extract_ai_pdf_stream(path) {
        return Ok((pdf, "application/pdf".to_string()));
    }
    Err("No preview found in EPS/PS file".into())
}

fn convert_ps_to_pdf(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let temp_db = std::env::temp_dir().join(format!("mundam_eps_{}.pdf", uuid::Uuid::new_v4()));
    let mut success = false;
    if cfg!(target_os = "macos") {
        if let Ok(out) = std::process::Command::new("pstopdf")
            .arg(path)
            .arg("-o")
            .arg(&temp_db)
            .output()
        {
            if out.status.success() {
                success = true;
            }
        }
    }
    if !success {
        let cmds = if cfg!(target_os = "windows") {
            vec!["gswin64c", "gswin32c", "gs"]
        } else {
            vec!["gs"]
        };
        for cmd in cmds {
            let mut gs_cmd = std::process::Command::new(cmd);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                gs_cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = gs_cmd
                .args(["-sDEVICE=pdfwrite", "-dSAFER", "-dBATCH", "-dNOPAUSE"])
                .arg(format!("-sOutputFile={}", temp_db.display()))
                .arg(path)
                .output()
            {
                if out.status.success() {
                    success = true;
                    break;
                }
            }
        }
    }
    if success && temp_db.exists() {
        let data = std::fs::read(&temp_db)?;
        let _ = std::fs::remove_file(&temp_db);
        return Ok(data);
    }
    let _ = std::fs::remove_file(&temp_db);
    Err("Failed to convert PS to PDF".into())
}

/// Extract technical metadata from PostScript/EPS files.
pub fn extract_eps_metadata(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let file_content = std::fs::read(path)?;
    let scan_limit = file_content.len().min(8192);
    let header_chunk = &file_content[..scan_limit];
    let header_text = String::from_utf8_lossy(header_chunk);

    let mut bounding_box = None;
    let mut hi_res_bounding_box = None;

    for line in header_text.lines() {
        if line.starts_with("%%HiResBoundingBox:") {
            hi_res_bounding_box = parse_bounding_box_line(line);
        } else if line.starts_with("%%BoundingBox:") {
            bounding_box = parse_bounding_box_line(line);
        }
    }

    let final_box = hi_res_bounding_box.or(bounding_box);

    let mut width = None;
    let mut height = None;
    if let Some((lower_left_x, lower_left_y, upper_right_x, upper_right_y)) = final_box {
        if upper_right_x > lower_left_x && upper_right_y > lower_left_y {
            width = Some((upper_right_x - lower_left_x).round() as u32);
            height = Some((upper_right_y - lower_left_y).round() as u32);
        }
    }

    let mut pages_count = 1;
    if width.is_none() || height.is_none() {
        if let Ok(pdf_data) = convert_ps_to_pdf(path) {
            if let Ok(pdf_metadata) =
                crate::processing::media::extractors::pdf::extract_pdf_metadata(&pdf_data)
            {
                if let Some(extracted_width) = pdf_metadata["technical"]["width"].as_u64() {
                    width = Some(extracted_width as u32);
                }
                if let Some(extracted_height) = pdf_metadata["technical"]["height"].as_u64() {
                    height = Some(extracted_height as u32);
                }
                if let Some(pdf_pages_count) = pdf_metadata["technical"]["pages_count"].as_u64() {
                    pages_count = pdf_pages_count as u32;
                }
            }
        }
    }

    let mut technical_metadata = serde_json::json!({
        "container": "EPS/PS",
        "metadata_support": "Standard",
        "pages_count": pages_count,
        "dpi": 72,
    });

    if let Some(width_value) = width {
        technical_metadata["width"] = serde_json::json!(width_value);
    }
    if let Some(height_value) = height {
        technical_metadata["height"] = serde_json::json!(height_value);
    }

    Ok(serde_json::json!({
        "technical": technical_metadata,
        "semantic": {}
    }))
}

fn parse_bounding_box_line(line: &str) -> Option<(f32, f32, f32, f32)> {
    let coordinate_parts: Vec<&str> = line.split_whitespace().skip(1).collect();
    if coordinate_parts.len() >= 4 {
        let lower_left_x = coordinate_parts[0].parse::<f32>().ok()?;
        let lower_left_y = coordinate_parts[1].parse::<f32>().ok()?;
        let upper_right_x = coordinate_parts[2].parse::<f32>().ok()?;
        let upper_right_y = coordinate_parts[3].parse::<f32>().ok()?;
        Some((lower_left_x, lower_left_y, upper_right_x, upper_right_y))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_eps_metadata() {
        let eps_content = "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 10 20 410 520\n%%EOF";
        let mut temporary_file = NamedTempFile::new().expect("Failed to create temporary file");
        temporary_file
            .write_all(eps_content.as_bytes())
            .expect("Failed to write to temporary file");
        let path = temporary_file.path();

        let metadata = extract_eps_metadata(path).expect("Failed to extract EPS metadata");
        let technical = &metadata["technical"];
        assert_eq!(technical["container"], "EPS/PS");
        assert_eq!(technical["width"], 400);
        assert_eq!(technical["height"], 500);
    }
}
