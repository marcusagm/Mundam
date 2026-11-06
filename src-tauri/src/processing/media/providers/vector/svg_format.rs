use crate::core::error::AppResult;
use crate::core::formats::capabilities::{MetadataCapability, ThumbnailCapability};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use async_trait::async_trait;
use resvg::usvg;
use std::path::Path;
use tiny_skia::Pixmap;

/// Provider for SVG vector files
#[derive(Default)]
pub struct SvgFormatProvider;

/// Implementation of `SvgFormatProvider`.
impl SvgFormatProvider {
    /// Create a new instance of `SvgFormatProvider`.
    ///
    /// # Returns
    ///
    /// A new instance of `SvgFormatProvider`.
    pub fn new() -> Self {
        Self
    }
}

/// Helper function to load SVG data, automatically decompressing if it is SVGZ.
fn load_svg_data(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file_data = std::fs::read(path)?;
    if file_data.starts_with(&[0x1f, 0x8b]) {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut decoder = GzDecoder::new(&file_data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        Ok(decompressed_data)
    } else {
        Ok(file_data)
    }
}

/// Trait for format provider.
impl FormatProvider for SvgFormatProvider {
    /// Get the name of the format provider.
    ///
    /// # Returns
    ///
    /// A `&'static str` containing the name of the format provider.
    fn name(&self) -> &'static str {
        "SVG_VECTOR_PROVIDER"
    }

    /// Get the supported extensions for the format.
    ///
    /// # Returns
    ///
    /// A `Vec<&'static str>` containing the supported extensions.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["svg", "svgz"]
    }

    fn supported_formats(&self) -> Vec<SupportedFormat> {
        use crate::core::formats::types::{
            MediaType, PlaybackStrategy, PreviewStrategy, ThumbnailStrategy,
        };

        vec![SupportedFormat::with_metadata(
            "Scalable Vector Graphics",
            vec!["svg", "svgz"],
            vec!["image/svg+xml"],
            MediaType::Vector,
            ThumbnailStrategy::NativeExtractor,
            PreviewStrategy::BrowserNative,
            PlaybackStrategy::None,
        )]
    }

    /// Check if the given header bytes support the format.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The header bytes of the file.
    ///
    /// # Returns
    ///
    /// `true` if the format is supported, `false` otherwise.
    fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
        header_bytes.starts_with(b"<?xml")
            || header_bytes.starts_with(b"<svg")
            || header_bytes.starts_with(&[0x1f, 0x8b]) // Gzip magic bytes for SVGZ
    }

    /// Get the metadata capability for the format.
    ///
    /// # Returns
    ///
    /// An `Option<&dyn MetadataCapability>` containing the metadata capability.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Get the thumbnail capability for the format.
    ///
    /// # Returns
    ///
    /// An `Option<&dyn ThumbnailCapability>` containing the thumbnail capability.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

/// Trait for metadata capability.
#[async_trait]
impl MetadataCapability for SvgFormatProvider {
    /// Extract technical metadata from the given SVG/SVGZ file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` containing the technical metadata.
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let svg_data = load_svg_data(&path_owned)
                .map_err(|error| crate::core::error::AppError::Generic(error.to_string()))?;

            let options = usvg::Options::default();
            let tree = usvg::Tree::from_data(&svg_data, &options).map_err(|error| {
                crate::core::error::AppError::Generic(format!("SVG parse error: {}", error))
            })?;

            let width = tree.size().width().round() as u32;
            let height = tree.size().height().round() as u32;

            Ok(serde_json::json!({
                "format": "SVG",
                "width": width,
                "height": height,
                "dpi": 96,
            }))
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extract semantic metadata from the given SVG/SVGZ file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` containing the semantic metadata.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

/// Trait for thumbnail capability.
#[async_trait]
impl ThumbnailCapability for SvgFormatProvider {
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
    /// A `Vec<u8>` containing the thumbnail data.
    async fn generate(&self, path: &Path, _asset_id: &str, size_hint: u32) -> AppResult<Vec<u8>> {
        let path_owned = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let svg_data = load_svg_data(&path_owned)
                .map_err(|error| crate::core::error::AppError::Generic(error.to_string()))?;

            let mut font_database = usvg::fontdb::Database::new();
            font_database.load_system_fonts();

            let options = usvg::Options::default();
            let tree = usvg::Tree::from_data(&svg_data, &options).map_err(|error| {
                crate::core::error::AppError::Generic(format!("SVG parse error: {}", error))
            })?;

            let bounding_size = usvg::Size::from_wh(size_hint as f32, size_hint as f32)
                .ok_or_else(|| {
                    crate::core::error::AppError::Generic("Invalid SVG target size".to_string())
                })?;
            let target_size = tree.size().scale_to(bounding_size);

            let transform = tiny_skia::Transform::from_scale(
                target_size.width() / tree.size().width(),
                target_size.height() / tree.size().height(),
            );

            let mut pixmap = Pixmap::new(target_size.width() as u32, target_size.height() as u32)
                .ok_or_else(|| {
                crate::core::error::AppError::Generic("Failed to create pixmap buffer".to_string())
            })?;

            resvg::render(&tree, transform, &mut pixmap.as_mut());

            let encoder = webp::Encoder::from_rgba(
                pixmap.data(),
                target_size.width() as u32,
                target_size.height() as u32,
            );
            let webp_data = encoder.encode(85.0);

            Ok(webp_data.to_vec())
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_svg_metadata_and_thumbnail() -> Result<(), Box<dyn std::error::Error>> {
        let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="150" height="250"><rect width="150" height="250" fill="blue"/></svg>"#;
        let mut temporary_file = NamedTempFile::new()?;
        temporary_file.write_all(svg_content.as_bytes())?;
        let path = temporary_file.path();

        let provider = SvgFormatProvider::new();

        // Test metadata extraction
        let metadata = provider.extract_technical(path).await?;
        assert_eq!(metadata["format"], "SVG");
        assert_eq!(metadata["width"], 150);
        assert_eq!(metadata["height"], 250);

        // Test thumbnail generation
        let thumbnail = provider.generate(path, "test_asset_id", 100).await?;
        assert!(!thumbnail.is_empty(), "Thumbnail data should not be empty");

        Ok(())
    }

    #[tokio::test]
    async fn test_svgz_metadata_and_thumbnail() -> Result<(), Box<dyn std::error::Error>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="400"><circle cx="150" cy="200" r="100" fill="green"/></svg>"#;
        let mut temporary_file = NamedTempFile::new()?;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(svg_content.as_bytes())?;
        let compressed_data = encoder.finish()?;

        temporary_file.write_all(&compressed_data)?;
        let path = temporary_file.path();

        let provider = SvgFormatProvider::new();

        // Test metadata extraction
        let metadata = provider.extract_technical(path).await?;
        assert_eq!(metadata["format"], "SVG");
        assert_eq!(metadata["width"], 300);
        assert_eq!(metadata["height"], 400);

        // Test thumbnail generation
        let thumbnail = provider.generate(path, "test_asset_id", 100).await?;
        assert!(!thumbnail.is_empty(), "Thumbnail data should not be empty");

        Ok(())
    }
}
