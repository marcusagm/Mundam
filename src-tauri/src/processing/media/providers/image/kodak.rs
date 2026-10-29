use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// A format provider for Kodak RAW files (.kdc).
///
/// KDC is Kodak's RAW format, primarily from older Kodak DSLR cameras (EasyShare,
/// DCS series). FlashRaw support is limited for the oldest variants; the brute-force
/// JPEG scan tier provides a reliable fallback.
///
/// This provider uses the `image` crate with FlashRaw support to extract metadata
/// and generate thumbnails for Kodak RAW images.
///
/// # Supported Formats
///
/// - `KDC` - Kodak RAW image
///
/// # Technical Details
///
/// - Uses `image::open` with FlashRaw backend for RAW file support
/// - Supports both technical and semantic metadata extraction
/// - Generates WebP thumbnails using optimized resizing
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::image::kodak::KodakRawFormatProvider;
///
/// let provider = KodakRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct KodakRawFormatProvider;

impl KodakRawFormatProvider {
    /// Creates a new instance of the Kodak RAW format provider.
    ///
    /// # Returns
    ///
    /// `KodakRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for KodakRawFormatProvider {
    /// Returns the unique name for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "KODAK_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["kdc"]
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
            "Kodak RAW Image",
            vec!["kdc"],
            vec!["image/x-kodak-kdc"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the given header bytes match the magic bytes for Kodak RAW files.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - A byte slice containing the header bytes of the file.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the header bytes match Kodak RAW magic bytes, `false` otherwise.
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

    /// Returns the metadata extraction capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn MetadataCapability>` - The metadata extraction capability.
    fn metadata(&self) -> Option<&dyn MetadataCapability> {
        Some(self)
    }

    /// Returns the thumbnail generation capability for this provider.
    ///
    /// # Returns
    ///
    /// `Option<&dyn ThumbnailCapability>` - The thumbnail generation capability.
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait]
impl MetadataCapability for KodakRawFormatProvider {
    /// Attempts to extract technical metadata from a Kodak RAW file using FlashRaw.
    ///
    /// This method extracts metadata from Kodak RAW (.kdc) files using FlashRaw.
    /// The extraction is performed in a blocking thread to avoid blocking the async runtime.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Kodak RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object containing the extracted technical metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::kdc::extract_kdc_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from a Kodak RAW file.
    ///
    /// This method currently returns an empty JSON object as semantic metadata
    /// extraction is not yet implemented for Kodak RAW files.
    ///
    /// # Arguments
    ///
    /// * `_path` - The path to the Kodak RAW file (unused in current implementation).
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - An empty JSON object.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for KodakRawFormatProvider {
    /// Generates a thumbnail for a Kodak RAW file.
    ///
    /// This method generates a thumbnail for Kodak RAW (.kdc) files using FlashRaw.
    /// The generation is performed in a blocking thread to avoid blocking the async runtime.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the Kodak RAW file.
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
            crate::processing::media::extractors::kdc::generate_kdc_thumbnail(
                &path_owned,
                size_hint,
            )
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[async_trait]
impl PreviewCapability for KodakRawFormatProvider {
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
            crate::processing::media::extractors::kdc::extract_kdc_preview(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)??;

        Ok((bytes, "image/jpeg".to_string()))
    }
}
