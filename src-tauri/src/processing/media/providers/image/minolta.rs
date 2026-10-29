use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Minolta RAW image files (.mrw).
///
/// MRW is Minolta's RAW format. Minolta's camera division was absorbed by Sony
/// in 2006, but many MRW files still exist from legacy Minolta DiMAGE and
/// α-series cameras. FlashRaw retains full support.
///
/// # Technical Specification
///
/// - **File Format**: MRW (Minolta RAW)
/// - **Extension**: `.mrw`
/// - **MIME Type**: `image/x-minolta-mrw`
/// - **MediaType**: `MediaType::Image`
/// - **ThumbnailStrategy**: `ThumbnailStrategy::Raw`
/// - **PreviewStrategy**: `PreviewStrategy::NativeExtractor`
/// - **PlaybackStrategy**: `PlaybackStrategy::None`
///
/// # Features
///
/// - **Metadata Extraction**: Extract technical metadata from Minolta RAW files.
/// - **Thumbnail Generation**: Generate thumbnails for Minolta RAW files.
///
/// # Examples
///
/// ```rust
/// use mundam_lib::processing::media::providers::image::minolta::MinoltaRawFormatProvider;
/// use mundam_lib::core::formats::provider::FormatProvider;
/// use mundam_lib::core::formats::types::MediaType;
///
/// let provider = MinoltaRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct MinoltaRawFormatProvider;

impl MinoltaRawFormatProvider {
    /// Returns a new instance of the Minolta RAW format provider.
    ///
    /// # Returns
    ///
    /// `MinoltaRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for MinoltaRawFormatProvider {
    /// Returns the name of the provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The name of the provider.
    fn name(&self) -> &'static str {
        "MINOLTA_RAW_PROVIDER"
    }

    /// Returns the supported file extensions for this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The supported file extensions.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["mrw"]
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
            "Minolta RAW Image",
            vec!["mrw"],
            vec!["image/x-minolta-mrw"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Validates MRW magic bytes (`\0MRM`).
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The header bytes of the file.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the file is a Minolta RAW file, `false` otherwise.
    fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
        header_bytes.starts_with(b"\x00MRM")
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

    /// Returns the preview capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn PreviewCapability>` - The preview capability.
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }
}

#[async_trait]
impl MetadataCapability for MinoltaRawFormatProvider {
    /// Extracts technical metadata from Minolta RAW files.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Minolta RAW file.
    /// * `asset_id` - The ID of the asset.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The technical metadata.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata such as camera model, lens information, etc.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Minolta RAW file.
    /// * `asset_id` - The ID of the asset.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The semantic metadata.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for MinoltaRawFormatProvider {
    /// Generates a thumbnail from a Minolta RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Minolta RAW file.
    /// * `asset_id` - The ID of the asset.
    /// * `size_hint` - The size hint for the thumbnail.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - The thumbnail data.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
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
impl PreviewCapability for MinoltaRawFormatProvider {
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
