use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Sigma RAW image files (.x3f).
///
/// X3F is Sigma's proprietary RAW format based on their Foveon X3 sensor
/// technology. The three-layer sensor captures red, green, and blue at every
/// pixel site, making X3F files larger than typical RAW files.
///
/// # Technical Details
///
/// * **File Format**: Sigma RAW Image (.x3f)
/// * **Thumbnail Format**: WebP image (via FlashRaw)
/// * **Metadata**: Dimensions and camera settings via FlashRaw
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::image::sigma::SigmaRawFormatProvider;
///
/// let provider = SigmaRawFormatProvider::new();
/// assert!(provider.supported_extensions().contains(&"x3f"));
/// ```
#[derive(Default)]
pub struct SigmaRawFormatProvider;

impl SigmaRawFormatProvider {
    /// Creates a new instance of `SigmaRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `Self` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for SigmaRawFormatProvider {
    /// Returns the unique name for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "SIGMA_RAW_PROVIDER"
    }

    /// Returns the file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["x3f"]
    }

    /// Returns the detailed format definitions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<SupportedFormat>` - The list of supported formats with their details.
    fn supported_formats(&self) -> Vec<SupportedFormat> {
        use crate::core::formats::types::{
            MediaType, PlaybackStrategy, PreviewStrategy, ThumbnailStrategy,
        };
        vec![SupportedFormat::with_metadata(
            "Sigma RAW Image",
            vec!["x3f"],
            vec!["image/x-sigma-x3f"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the header bytes correspond to a valid Sigma RAW file.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The header bytes to check.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the header bytes correspond to a valid Sigma RAW file, `false` otherwise.
    fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
        header_bytes.starts_with(b"FOVb")
    }

    /// Returns the preview capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn PreviewCapability>` - The preview capability for this provider.
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }

    /// Returns the metadata capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn MetadataCapability>` - The metadata capability for this provider.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Returns the thumbnail capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn ThumbnailCapability>` - The thumbnail capability for this provider.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait]
impl MetadataCapability for SigmaRawFormatProvider {
    /// Extracts technical metadata from a Sigma RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Sigma RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The technical metadata extracted from the Sigma RAW file.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    /// * `AppError::Generic` - If the metadata extraction fails.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::x3f::extract_x3f_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from a Sigma RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Sigma RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The semantic metadata extracted from the Sigma RAW file.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for SigmaRawFormatProvider {
    /// Generates a thumbnail from a Sigma RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Sigma RAW file.
    /// * `asset_id` - The ID of the asset.
    /// * `size_hint` - The size hint for the thumbnail.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - The thumbnail generated from the Sigma RAW file.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    /// * `AppError::Generic` - If the thumbnail generation fails.
    #[instrument(skip(self, path))]
    async fn generate(&self, path: &Path, _asset_id: &str, size_hint: u32) -> AppResult<Vec<u8>> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::x3f::generate_x3f_thumbnail(
                &path_owned,
                size_hint,
            )
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[async_trait]
impl PreviewCapability for SigmaRawFormatProvider {
    /// Generates a WebP preview using FlashRaw embedded preview extraction or brute-force scanning.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to generate a preview from.
    /// * `_asset_id` - The ID of the asset.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - A vector of bytes containing the preview image.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - If all extraction tiers fail.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn generate_preview(&self, path: &Path, _asset_id: &str) -> AppResult<(Vec<u8>, String)> {
        let path_owned = path.to_path_buf();
        let bytes = tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::x3f::extract_x3f_preview(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)??;

        Ok((bytes, "image/jpeg".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_sigma_raw_provider_capabilities() {
        let provider = SigmaRawFormatProvider::new();
        assert_eq!(provider.name(), "SIGMA_RAW_PROVIDER");
        assert!(provider.supported_extensions().contains(&"x3f"));
        assert!(provider.supports_magic_bytes(b"FOVb\x00\x00\x00\x00"));

        let sample_file_path = Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/x3f/SDIM0024.X3F");
        if sample_file_path.exists() {
            // Test metadata extraction
            let metadata_result = provider.extract_technical(sample_file_path).await;
            assert!(
                metadata_result.is_ok(),
                "Metadata extraction failed: {:?}",
                metadata_result.err()
            );
            let metadata_value = metadata_result.unwrap();

            assert!(
                metadata_value.get("width").is_some(),
                "Metadata lacks width"
            );
            assert!(
                metadata_value.get("height").is_some(),
                "Metadata lacks height"
            );
            assert!(
                metadata_value.get("Model").is_some(),
                "Metadata lacks camera model"
            );

            let image_width = metadata_value["width"].as_u64().unwrap();
            let image_height = metadata_value["height"].as_u64().unwrap();
            assert_eq!(image_width, 5424);
            assert_eq!(image_height, 3616);

            // Test preview extraction
            let preview_result = provider
                .generate_preview(sample_file_path, "test_asset_id")
                .await;
            assert!(
                preview_result.is_ok(),
                "Preview generation failed: {:?}",
                preview_result.err()
            );
            let (preview_bytes, mime_type) = preview_result.unwrap();
            assert_eq!(mime_type, "image/jpeg");
            assert!(
                preview_bytes.starts_with(&[0xFF, 0xD8]),
                "Preview is not a valid JPEG"
            );

            // Test thumbnail generation
            let thumbnail_result = provider
                .generate(sample_file_path, "test_asset_id", 200)
                .await;
            assert!(
                thumbnail_result.is_ok(),
                "Thumbnail generation failed: {:?}",
                thumbnail_result.err()
            );
            let thumbnail_bytes = thumbnail_result.unwrap();
            assert!(!thumbnail_bytes.is_empty(), "Generated thumbnail is empty");
        }
    }
}
