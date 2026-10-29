use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Canon RAW image files (.cr2, .cr3, .crw).
///
/// Covers all three Canon RAW generations:
/// - **CRW**: Original Canon RAW (older EOS film/early digital cameras)
/// - **CR2**: Canon RAW v2 (TIFF-based, most Canon DSLRs)
/// - **CR3**: Canon RAW v3 (ISOBMFF-based, mirrorless EOS R system)
///
/// All variants use the same FlashRaw → brute-force JPEG extraction pipeline.
///
/// # Technical Details
///
/// - **Cameras**: Canon EOS, PowerShot
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
/// let provider = CanonRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```

#[derive(Default)]
pub struct CanonRawFormatProvider;

impl CanonRawFormatProvider {
    /// Creates a new instance of `CanonRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `CanonRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for CanonRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "CANON_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["cr2", "cr3", "crw"]
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
            "Canon RAW Image",
            vec!["cr2", "cr3", "crw"],
            vec![
                "image/x-canon-cr2",
                "image/x-canon-cr3",
                "image/x-canon-crw",
            ],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Validates Canon RAW magic bytes.
    ///
    /// # Arguments
    ///
    /// * `_header_bytes` - The first bytes of the file (not used for Canon RAW).
    ///
    /// # Returns
    ///
    /// `bool` - False, as Canon RAW validation is done via file extension.
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
impl MetadataCapability for CanonRawFormatProvider {
    /// Extracts dimensions via FlashRaw and camera EXIF via rexif.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Canon RAW file.
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be opened.
    /// * `AppError::Generic` - If FlashRaw parsing fails.
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

    /// Extracts semantic metadata from the Canon RAW file.
    ///
    /// # Arguments
    ///
    /// * `_path` - Path to the Canon RAW file (not used in this implementation).
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - An empty JSON object.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for CanonRawFormatProvider {
    /// Generates a WebP thumbnail using FlashRaw embedded preview extraction.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Canon RAW file.
    /// * `_asset_id` - Asset ID (not used in this implementation).
    /// * `size_hint` - Desired thumbnail size.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - If all extraction tiers fail.
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
impl PreviewCapability for CanonRawFormatProvider {
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
