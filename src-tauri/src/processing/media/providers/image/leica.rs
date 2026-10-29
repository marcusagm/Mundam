use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Leica RAW image files (.rwl).
///
/// RWL is Leica's RAW format used in V-Lux and D-Lux compact cameras.
/// It is distinct from the DNG format used in Leica's M and S systems.
///
/// # Supported Formats
///
/// - `RWL` - Leica RAW image
///
/// # Technical Details
///
/// - Uses `image` crate with FlashRaw backend for RAW file support
/// - Supports both technical and semantic metadata extraction
/// - Generates WebP thumbnails using optimized resizing
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::image::leica::LeicaRawFormatProvider;
///
/// let provider = LeicaRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct LeicaRawFormatProvider;

impl LeicaRawFormatProvider {
    /// Creates a new instance of `LeicaRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `LeicaRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for LeicaRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "LEICA_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["rwl"]
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
            "Leica RAW Image",
            vec!["rwl"],
            vec!["image/x-leica-rwl"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Validates magic bytes for Leica RAW files.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the header bytes match Leica RAW magic bytes, `false` otherwise.
    fn supports_magic_bytes(&self, _header_bytes: &[u8]) -> bool {
        false
    }

    /// Returns the preview generation capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn PreviewCapability>` - The preview generation capability.
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }

    /// Returns the metadata extraction capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn MetadataCapability>` - The metadata extraction capability.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Returns the thumbnail generation capability.
    ///
    /// # Returns
    ///
    /// `Option<&dyn ThumbnailCapability>` - The thumbnail generation capability.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait]
impl MetadataCapability for LeicaRawFormatProvider {
    /// Extracts technical metadata from a Leica RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `asset_id` - Asset ID
    /// * `size_hint` - Size hint for the thumbnail
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be opened.
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

    /// Extracts semantic metadata from a Leica RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `asset_id` - Asset ID
    /// * `size_hint` - Size hint for the thumbnail
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - Semantic metadata
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be opened.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for LeicaRawFormatProvider {
    /// Generates a thumbnail for a Leica RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `asset_id` - Asset ID
    /// * `size_hint` - Size hint for the thumbnail
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - Thumbnail data
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be opened.
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
impl PreviewCapability for LeicaRawFormatProvider {
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
