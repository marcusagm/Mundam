use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Leaf RAW image files (.mos).
///
/// MOS is Leaf's proprietary medium-format RAW format used in Leaf Aptus and
/// Credo digital backs (now part of Phase One). FlashRaw handles MOS decoding.
///
/// # Technical Specification
///
/// - **File Format**: MOS (Leaf RAW)
/// - **Extension**: `.mos`
/// - **MIME Type**: `image/x-leaf-mos`
/// - ** MediaType**: `MediaType::Image`
/// - **ThumbnailStrategy**: `ThumbnailStrategy::Raw`
/// - **PreviewStrategy**: `PreviewStrategy::NativeExtractor`
/// - **PlaybackStrategy**: `PlaybackStrategy::None`
///
/// # Features
///
/// - **Metadata Extraction**: Extract technical metadata from Leaf RAW files.
/// - **Thumbnail Generation**: Generate thumbnails for Leaf RAW files.
///
/// # Examples
///
/// ```rust
/// use mundam_lib::processing::media::providers::image::leaf::LeafRawFormatProvider;
/// use mundam_lib::core::formats::provider::FormatProvider;
/// use mundam_lib::core::formats::types::MediaType;
///
/// let provider = LeafRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct LeafRawFormatProvider;

impl LeafRawFormatProvider {
    /// Creates a new instance of the Leaf RAW format provider.
    ///
    /// # Returns
    ///
    /// `LeafRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for LeafRawFormatProvider {
    /// Returns the unique name for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "LEAF_RAW_PROVIDER"
    }

    /// Returns the supported file extensions for this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The supported file extensions.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["mos"]
    }

    /// Returns the supported formats for this provider.
    ///
    /// # Returns
    ///
    /// `Vec<SupportedFormat>` - The supported formats.
    fn supported_formats(&self) -> Vec<SupportedFormat> {
        use crate::core::formats::types::{
            MediaType, PlaybackStrategy, PreviewStrategy, ThumbnailStrategy,
        };
        vec![SupportedFormat::with_metadata(
            "Leaf RAW Image",
            vec!["mos"],
            vec!["image/x-leaf-mos"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the provider supports files with the given magic bytes.
    ///
    /// # Arguments
    ///
    /// * `_header_bytes` - The magic bytes to check.
    ///
    /// # Returns
    ///
    /// `bool` - Whether the provider supports the given magic bytes.
    fn supports_magic_bytes(&self, _header_bytes: &[u8]) -> bool {
        false
    }

    /// Returns the preview capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn PreviewCapability>` - The preview capability.
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }

    /// Returns the metadata capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn MetadataCapability>` - The metadata capability.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Returns the thumbnail capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn ThumbnailCapability>` - The thumbnail capability.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait]
impl MetadataCapability for LeafRawFormatProvider {
    /// Extracts technical metadata from the Leaf RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Leaf RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The technical metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from the Leaf RAW file.
    ///
    /// # Arguments
    ///
    /// * `_path` - Path to the Leaf RAW file (not used in this implementation).
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
impl ThumbnailCapability for LeafRawFormatProvider {
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
impl PreviewCapability for LeafRawFormatProvider {
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
