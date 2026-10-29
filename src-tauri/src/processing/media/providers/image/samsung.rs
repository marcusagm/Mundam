use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Samsung RAW image files (.srw).
///
/// # Technical Details
///
/// - **Cameras**: Samsung NX and Galaxy camera series
/// - **Thumbnail Format**: WebP (from embedded FlashRaw preview)
/// - **Metadata**: Dimensions (FlashRaw) + camera EXIF (rexif)
///
/// # Features
///
/// - Extracts dimensions via FlashRaw and camera EXIF via rexif.
/// - Generates WebP thumbnails from embedded FlashRaw previews.
///
/// # Examples
///
/// ```rust
/// use mundam_core::formats::provider::FormatProvider;
/// use mundam_core::formats::types::MediaType;
///
/// let provider = SamsungRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct SamsungRawFormatProvider;

impl SamsungRawFormatProvider {
    /// Creates a new instance of `SamsungRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `Self` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for SamsungRawFormatProvider {
    /// Returns the unique name for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "SAMSUNG_RAW_PROVIDER"
    }

    /// Returns the file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["srw"]
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
            "Samsung RAW Image",
            vec!["srw"],
            vec!["image/x-samsung-srw"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the provider supports the given magic bytes.
    ///
    /// # Arguments
    ///
    /// * `_header_bytes` - The header bytes to check (not used for Samsung RAW).
    ///
    /// # Returns
    ///
    /// `bool` - `false` as Samsung RAW detection relies on file extension.
    fn supports_magic_bytes(&self, _header_bytes: &[u8]) -> bool {
        false
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
impl MetadataCapability for SamsungRawFormatProvider {
    /// Extracts technical metadata from the Samsung RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Samsung RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - Technical metadata extracted from the file.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking extraction task times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from the Samsung RAW file.
    ///
    /// # Arguments
    ///
    /// * `_path` - Path to the Samsung RAW file (not used in this implementation).
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - An empty JSON object.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - If any error occurs during semantic metadata extraction.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for SamsungRawFormatProvider {
    /// Generates a thumbnail for the Samsung RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Samsung RAW file.
    /// * `_asset_id` - Asset ID (not used in this implementation).
    /// * `size_hint` - Hint for the desired thumbnail size.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - The generated thumbnail as a vector of bytes.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking thumbnail generation task times out.
    #[instrument(skip(self, path))]
    async fn generate(&self, path: &Path, _asset_id: &str, size_hint: u32) -> AppResult<Vec<u8>> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::generate_raw_thumbnail(
                &path_owned,
                size_hint,
            )
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[async_trait]
impl PreviewCapability for SamsungRawFormatProvider {
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
            crate::processing::media::extractors::image::extract_raw_preview(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)??;

        Ok((bytes, "image/jpeg".to_string()))
    }
}
