use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Fujifilm RAW image files (.raf).
///
/// RAF (RAW File Format) is Fujifilm's proprietary RAW format. Later RAF files
/// from X-Trans sensor cameras have a non-standard CFA pattern that some
/// decoders handle differently; FlashRaw fully supports all RAF variants.
///
/// # Technical Details
///
/// - **Cameras**: Fujifilm X-series, GFX medium-format
/// - **Thumbnail Format**: WebP (from embedded FlashRaw preview)
/// - **Metadata**: Dimensions (FlashRaw) + camera EXIF (rexif)
///
/// # Examples
///
/// ```rust
/// use crate::processing::media::providers::image::fujifilm::FujifilmRawFormatProvider;
/// use crate::core::formats::provider::FormatProvider;
///
/// let provider = FujifilmRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert_eq!(formats.len(), 1);
/// assert_eq!(formats[0].display_name, "Fujifilm RAW Image");
/// assert_eq!(formats[0].extensions, vec!["raf"]);
/// ```
#[derive(Default)]
pub struct FujifilmRawFormatProvider;

impl FujifilmRawFormatProvider {
    /// Creates a new instance of `FujifilmRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `FujifilmRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for FujifilmRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "FUJIFILM_RAW_PROVIDER"
    }

    /// Returns the file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["raf"]
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
            "Fujifilm RAW Image",
            vec!["raf"],
            vec!["image/x-fujifilm-raf"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the header bytes match the Fujifilm RAW format.
    ///
    /// # Arguments
    ///
    /// * `_header_bytes` - The header bytes to check (not used in current implementation).
    ///
    /// # Returns
    ///
    /// `bool` - `false` as magic byte checking is not implemented for this provider.
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
impl MetadataCapability for FujifilmRawFormatProvider {
    /// Extracts technical metadata from a Fujifilm RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Fujifilm RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object with technical metadata.
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

    /// Extracts semantic metadata from a Fujifilm RAW file.
    ///
    /// # Arguments
    ///
    /// * `_path` - Path to the Fujifilm RAW file (unused in current implementation).
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - An empty JSON object (semantic extraction not implemented).
    ///
    /// # Errors
    ///
    /// This function always returns `Ok` with an empty JSON object as semantic extraction
    /// is not implemented for Fujifilm RAW files.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for FujifilmRawFormatProvider {
    /// Generates a WebP thumbnail from a Fujifilm RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Fujifilm RAW file.
    /// * `_asset_id` - The ID of the asset (unused in current implementation).
    /// * `size_hint` - The desired size of the thumbnail (unused in current implementation).
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - A vector of bytes containing the generated thumbnail.
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
impl PreviewCapability for FujifilmRawFormatProvider {
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
