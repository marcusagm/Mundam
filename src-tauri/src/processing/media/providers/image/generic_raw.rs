use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for generic RAW image files (.raw).
///
/// The `.raw` extension is a non-specific RAW identifier used by several older
/// cameras that pre-date standardized vendor formats (e.g., Leica Digilux,
/// some Panasonic compacts). The FlashRaw + brute-force JPEG pipeline is applied;
/// success depends on the specific camera's RAW dialect.
///
/// # Note
///
/// This provider handles the bare `.raw` extension only. Vendor-specific
/// extensions (`.arw`, `.nef`, etc.) are handled by dedicated providers.
///
/// # Technical Details
///
/// - **File Format**: RAW
/// - **Preview Format**: PNG image
/// - **Metadata**: JSON data containing image information
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::image::generic_raw::GenericRawFormatProvider;
///
/// let provider = GenericRawFormatProvider::new();
/// let formats = provider.supported_formats();
///
/// assert_eq!(formats.len(), 1);
/// assert_eq!(formats[0].name, "Generic RAW Image");
/// assert_eq!(formats[0].extensions, vec!["raw"]);
/// ```
#[derive(Default)]
pub struct GenericRawFormatProvider;

impl GenericRawFormatProvider {
    /// Creates a new instance of `GenericRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `GenericRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for GenericRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "GENERIC_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["raw"]
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
            "Generic RAW Image",
            vec!["raw"],
            vec!["image/x-raw"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Validates if the file header matches the RAW magic bytes.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The first bytes of the file.
    ///
    /// # Returns
    ///
    /// `bool` - True if it's a valid RAW file.
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
impl MetadataCapability for GenericRawFormatProvider {
    /// Attempts FlashRaw dimension extraction; gracefully returns an empty object
    /// for unsupported RAW dialects.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to extract metadata from.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object containing the extracted metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
                .unwrap_or_else(|_| serde_json::json!({}))
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)
    }

    /// Semantic extraction for generic RAW images is not implemented.
    ///
    /// # Arguments
    ///
    /// * `_path` - The path to the file to extract semantic data from.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - An empty JSON object.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for GenericRawFormatProvider {
    /// Attempts thumbnail generation via FlashRaw and brute-force JPEG scan.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to generate a thumbnail from.
    /// * `_asset_id` - The ID of the asset.
    /// * `size_hint` - A hint for the desired thumbnail size.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - A vector of bytes containing the thumbnail image.
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
impl PreviewCapability for GenericRawFormatProvider {
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
