use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Phase One RAW image files (.iiq).
///
/// IIQ (Intelligent Image Quality) is Phase One's proprietary RAW format used
/// in their medium-format IQ digital backs. Phase One cameras produce very
/// high-resolution files (100+ MP) with large embedded previews.
///
/// # Technical Specification
///
/// - **File Format**: IIQ (Phase One RAW)
/// - **Extension**: `.iiq`
/// - **MIME Type**: `image/x-phaseone-iiq`
/// - **MediaType**: `MediaType::Image`
/// - **ThumbnailStrategy**: `ThumbnailStrategy::Raw`
/// - **PreviewStrategy**: `PreviewStrategy::NativeExtractor`
/// - **PlaybackStrategy**: `PlaybackStrategy::None`
///
/// # Features
///
/// - **Metadata Extraction**: Extract technical metadata from Phase One RAW files.
/// - **Thumbnail Generation**: Generate thumbnails for Phase One RAW files.
///
/// # Examples
///
/// ```rust
/// use mundam_lib::processing::media::providers::image::phaseone::PhaseOneRawFormatProvider;
/// use mundam_lib::core::formats::provider::FormatProvider;
/// use mundam_lib::core::formats::types::MediaType;
///
/// let provider = PhaseOneRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct PhaseOneRawFormatProvider;

impl PhaseOneRawFormatProvider {
    /// Creates a new instance of the Phase One RAW format provider.
    ///
    /// # Returns
    ///
    /// `Self` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for PhaseOneRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "PHASEONE_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["iiq"]
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
        vec![SupportedFormat::with_metadata(
            "Phase One RAW Image",
            vec!["iiq"],
            vec!["image/x-phaseone-iiq"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the given header bytes correspond to a valid Phase One RAW file.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The header bytes of the file to check.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the header bytes correspond to a valid Phase One RAW file, `false` otherwise.
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
impl MetadataCapability for PhaseOneRawFormatProvider {
    /// Extracts technical metadata from a Phase One RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Phase One RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object containing the extracted metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::Transcoding` - If the file cannot be processed.
    /// * `AppError::ExtractionProcessTimeout` - If the extraction process times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from a Phase One RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Phase One RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object containing the extracted metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - Always returns an empty JSON object for now.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for PhaseOneRawFormatProvider {
    /// Generates a thumbnail from a Phase One RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Phase One RAW file.
    /// * `asset_id` - The ID of the asset.
    /// * `size_hint` - The size hint for the thumbnail.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - A vector of bytes containing the generated thumbnail.
    ///
    /// # Errors
    ///
    /// * `AppError::Transcoding` - If the file cannot be processed.
    /// * `AppError::ExtractionProcessTimeout` - If the extraction process times out.
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
impl PreviewCapability for PhaseOneRawFormatProvider {
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
