use crate::core::error::AppResult;
use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use async_trait::async_trait;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// PNG signature for magic byte detection.
const PNG_SIGNATURE: &[u8; 8] = b"\x89\x50\x4e\x47\x0d\x0a\x1a\x0a";
/// PNG IEND chunk for end of PNG data.
const PNG_IEND: &[u8; 4] = b"IEND";

/// Provider for Affinity files (.afphoto, .afdesign, .afpub)
///
/// This provider handles both standard AFPhoto, AFDESIGN and AFPUB files,
/// extracting technical metadata such as width, height, and color mode,
/// as well as semantic data such as layer names.
///
/// # Technical Details
///
/// - **File Format**: AFPhoto, AFDESIGN, AFPUB
/// - **Preview Format**: PNG image
/// - **Metadata**: JSON data containing design information
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::project::affinity::AffinityFormatProvider;
///
/// let provider = AffinityFormatProvider::new();
/// let formats = provider.supported_formats();
///
/// assert_eq!(formats.len(), 3);
/// assert_eq!(formats[0].name, "Affinity Photo Image");
/// assert_eq!(formats[0].extensions, vec!["afphoto"]);
/// assert_eq!(formats[1].name, "Affinity Designer Image");
/// assert_eq!(formats[1].extensions, vec!["afdesign"]);
/// assert_eq!(formats[2].name, "Affinity Publisher Image");
/// assert_eq!(formats[2].extensions, vec!["afpub"]);
/// ```
#[derive(Default)]
pub struct AffinityFormatProvider;

/// Implementation of `AffinityFormatProvider`.
impl AffinityFormatProvider {
    /// Create a new instance of `AffinityFormatProvider`.
    ///
    /// # Returns
    ///
    /// A new instance of `AffinityFormatProvider`.
    pub fn new() -> Self {
        Self
    }

    /// Extract the largest PNG preview using binary signature scanning.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the PNG preview data.
    fn extract_largest_png(&self, path: &Path) -> AppResult<Vec<u8>> {
        let mut file = File::open(path).map_err(crate::core::error::AppError::Io)?;
        let file_size = file
            .metadata()
            .map_err(crate::core::error::AppError::Io)?
            .len();

        // Affinity files are usually large, previews are often at the end.
        let scan_size = 15 * 1024 * 1024; // 15MB scan window
        let start_offset = file_size.saturating_sub(scan_size);

        file.seek(SeekFrom::Start(start_offset))
            .map_err(crate::core::error::AppError::Io)?;

        let mut buffer = Vec::with_capacity((file_size - start_offset) as usize);
        file.read_to_end(&mut buffer)
            .map_err(crate::core::error::AppError::Io)?;

        let mut best_png: Option<(usize, usize)> = None;

        let mut index = 0;
        while index <= buffer.len().saturating_sub(8) {
            if &buffer[index..index + 8] == PNG_SIGNATURE {
                // Found PNG, limit search for IEND
                let search_limit = (index + 50 * 1024 * 1024).min(buffer.len());
                if let Some(iend_relative_offset) = self.find_iend(&buffer[index + 8..search_limit])
                {
                    let png_length = iend_relative_offset + 8 + 4 + 4; // Signature + data until IEND + IEND + CRC

                    if best_png.is_none_or(|(_, previous_size)| png_length > previous_size) {
                        best_png = Some((index, png_length));
                    }
                    index += png_length;
                    continue;
                }
            }
            index += 1;
        }

        if let Some((start, length)) = best_png {
            let end = (start + length).min(buffer.len());
            Ok(buffer[start..end].to_vec())
        } else {
            Err(crate::core::error::AppError::FormatNotSupported(
                "No PNG preview found in Affinity file".to_string(),
            ))
        }
    }

    /// Find the IEND chunk in the given data.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to search for the IEND chunk.
    ///
    /// # Returns
    ///
    /// An `Option<usize>` containing the relative offset of the IEND chunk if found, `None` otherwise.
    fn find_iend(&self, data: &[u8]) -> Option<usize> {
        data.windows(4).position(|window| window == PNG_IEND)
    }
}

/// Helper function to parse PNG chunks and extract dimensions and DPI.
///
/// # Arguments
///
/// * `png_data` - The PNG data to parse.
///
/// # Returns
///
/// A `(Option<u32>, Option<u32>, Option<u32>)` tuple containing the width, height, and DPI of the PNG data.
fn extract_png_metadata_details(png_data: &[u8]) -> (Option<u32>, Option<u32>, Option<u32>) {
    if png_data.len() < 33 || &png_data[12..16] != b"IHDR" {
        return (None, None, None);
    }

    let width = Some(u32::from_be_bytes(
        png_data[16..20].try_into().unwrap_or([0; 4]),
    ));
    let height = Some(u32::from_be_bytes(
        png_data[20..24].try_into().unwrap_or([0; 4]),
    ));
    let mut dots_per_inch = None;

    let mut offset = 33;
    while offset + 8 <= png_data.len() {
        let chunk_length =
            u32::from_be_bytes(png_data[offset..offset + 4].try_into().unwrap_or([0; 4])) as usize;
        let chunk_type = &png_data[offset + 4..offset + 8];

        if chunk_type == b"pHYs" && offset + 8 + chunk_length <= png_data.len() {
            let physical_dimensions_data = &png_data[offset + 8..offset + 8 + chunk_length];
            if physical_dimensions_data.len() >= 9 {
                let pixels_per_unit_x =
                    u32::from_be_bytes(physical_dimensions_data[0..4].try_into().unwrap_or([0; 4]));
                let unit_specifier = physical_dimensions_data[8];
                if unit_specifier == 1 {
                    // Convert pixels per meter to DPI (dots per inch)
                    // 1 meter = 39.3701 inches
                    let computed_dots_per_inch =
                        (pixels_per_unit_x as f64 / 39.3701).round() as u32;
                    dots_per_inch = Some(computed_dots_per_inch);
                }
            }
            break;
        }

        if chunk_type == b"IEND" {
            break;
        }

        offset += 4 + 4 + chunk_length + 4;
    }

    (width, height, dots_per_inch)
}

/// Trait for format provider.
impl FormatProvider for AffinityFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "AFFINITY_PROJECT_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// A `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["afphoto", "afdesign", "afpub"]
    }

    /// Returns the detailed format definitions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<SupportedFormat>` - List of supported formats with metadata.
    fn supported_formats(&self) -> Vec<SupportedFormat> {
        use crate::core::formats::types::{
            MediaType, PlaybackStrategy, PreviewStrategy, ThumbnailStrategy,
        };

        vec![
            SupportedFormat::with_metadata(
                "Affinity Photo Image",
                vec!["afphoto"],
                vec!["application/vnd.serif.affinity"],
                MediaType::Project,
                ThumbnailStrategy::NativeExtractor,
                PreviewStrategy::NativeExtractor,
                PlaybackStrategy::None,
            ),
            SupportedFormat::with_metadata(
                "Affinity Designer Image",
                vec!["afdesign"],
                vec!["application/vnd.serif.affinity"],
                MediaType::Project,
                ThumbnailStrategy::NativeExtractor,
                PreviewStrategy::NativeExtractor,
                PlaybackStrategy::None,
            ),
            SupportedFormat::with_metadata(
                "Affinity Publisher Image",
                vec!["afpub"],
                vec!["application/vnd.serif.affinity"],
                MediaType::Project,
                ThumbnailStrategy::NativeExtractor,
                PreviewStrategy::NativeExtractor,
                PlaybackStrategy::None,
            ),
        ]
    }

    /// Returns the metadata extraction capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn MetadataCapability>` - The metadata extraction capability.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Returns the thumbnail extraction capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn ThumbnailCapability>` - The thumbnail extraction capability.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }

    /// Returns the preview extraction capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn PreviewCapability>` - The preview extraction capability.
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }
}

/// Trait for metadata capability.
#[async_trait]
impl MetadataCapability for AffinityFormatProvider {
    /// Extracts technical metadata such as width, height, and DPI from the Affinity file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Affinity file.
    ///
    /// # Returns
    ///
    /// `serde_json::Value` - A JSON value containing the technical metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be read.
    /// * `AppError::Generic` - If the Affinity parsing fails.
    /// * `AppError::FormatNotSupported` - If the Affinity file does not contain a valid PNG preview.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        let provider = Self::new();

        tokio::task::spawn_blocking(move || {
            let png_data = provider.extract_largest_png(&path_owned)?;
            let (width, height, dots_per_inch) = extract_png_metadata_details(&png_data);

            let mut technical_metadata = serde_json::json!({
                "format": "Affinity",
            });

            if let Some(width_value) = width {
                technical_metadata["width"] = serde_json::json!(width_value);
            }
            if let Some(height_value) = height {
                technical_metadata["height"] = serde_json::json!(height_value);
            }
            if let Some(dots_per_inch_value) = dots_per_inch {
                technical_metadata["dpi"] = serde_json::json!(dots_per_inch_value);
            }

            Ok(technical_metadata)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extract semantic metadata from the Affinity file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Affinity file.
    ///
    /// # Returns
    ///
    /// `serde_json::Value` - A JSON value containing the semantic metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - Affinity files do not contain semantic metadata.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

/// Trait for thumbnail capability.
#[async_trait]
impl ThumbnailCapability for AffinityFormatProvider {
    /// Generate a thumbnail for the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    /// * `asset_id` - The unique asset identifier.
    /// * `size_hint` - The desired size of the thumbnail.
    ///
    /// # Returns
    ///
    /// `Vec<u8>` - The thumbnail data.
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be read.
    /// * `AppError::Generic` - If the Affinity parsing fails.
    /// * `AppError::FormatNotSupported` - If the Affinity file does not contain a valid PNG preview.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    async fn generate(&self, path: &Path, _asset_id: &str, _size_hint: u32) -> AppResult<Vec<u8>> {
        let path_owned = path.to_path_buf();
        let provider = Self::new();

        tokio::task::spawn_blocking(move || provider.extract_largest_png(&path_owned))
            .await
            .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

/// Trait for preview capability.
#[async_trait]
impl PreviewCapability for AffinityFormatProvider {
    /// Generate a preview for the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    /// * `asset_id` - The unique asset identifier.
    ///
    /// # Returns
    ///
    /// `(Vec<u8>, String)` - A tuple containing the preview data and its MIME type.
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be read.
    /// * `AppError::Generic` - If the Affinity parsing fails.
    /// * `AppError::FormatNotSupported` - If the Affinity file does not contain a valid PNG preview.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    async fn generate_preview(&self, path: &Path, _asset_id: &str) -> AppResult<(Vec<u8>, String)> {
        let path_owned = path.to_path_buf();
        let provider = Self::new();

        tokio::task::spawn_blocking(move || {
            let png_data = provider.extract_largest_png(&path_owned)?;
            Ok((png_data, "image/png".to_string()))
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_png_metadata_details() {
        let png_data = vec![
            // Signature
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // IHDR length: 13
            0x00, 0x00, 0x00, 0x0d, // IHDR type: "IHDR"
            0x49, 0x48, 0x44, 0x52, // Width: 300
            0x00, 0x00, 0x01, 0x2c, // Height: 400
            0x00, 0x00, 0x01, 0x90, // Other fields: 5 bytes
            0x08, 0x02, 0x00, 0x00, 0x00, // CRC: 4 bytes
            0x00, 0x00, 0x00, 0x00, // pHYs length: 9
            0x00, 0x00, 0x00, 0x09, // pHYs type: "pHYs"
            0x70, 0x48, 0x59, 0x73, // Pixels per unit X: 2835 (72 DPI)
            0x00, 0x00, 0x0b, 0x13, // Pixels per unit Y: 2835
            0x00, 0x00, 0x0b, 0x13, // Unit: 1 (meter)
            0x01, // CRC: 4 bytes
            0x00, 0x00, 0x00, 0x00, // IEND length: 0
            0x00, 0x00, 0x00, 0x00, // IEND type: "IEND"
            0x49, 0x45, 0x4e, 0x44, // CRC: 4 bytes
            0x00, 0x00, 0x00, 0x00,
        ];

        let (width, height, dots_per_inch) = extract_png_metadata_details(&png_data);
        assert_eq!(width, Some(300));
        assert_eq!(height, Some(400));
        assert_eq!(dots_per_inch, Some(72));
    }

    #[tokio::test]
    async fn test_affinity_extract_real_file() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Project/afdesign/paella_icons.afdesign");
        if path.exists() {
            let provider = AffinityFormatProvider::new();
            let metadata = provider.extract_technical(path).await?;
            assert_eq!(metadata["format"], "Affinity");
            assert!(metadata["width"].is_number());
            assert!(metadata["height"].is_number());

            let preview = provider.generate_preview(path, "test_asset").await?;
            assert!(!preview.0.is_empty());
            assert_eq!(preview.1, "image/png");
        }

        Ok(())
    }
}
