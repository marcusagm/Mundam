use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for Hasselblad RAW image files (.3fr, .fff).
///
/// `3FR` (3-Frame RAW) is Hasselblad's primary medium-format RAW format.
/// `FFF` is Hasselblad's Flexible File Format used in older H-series backs.
/// Both use the same FlashRaw extraction pipeline.
///
/// # Technical Details
///
/// - **Cameras**: Hasselblad H-series, CFV digital backs
/// - **Thumbnail Format**: JPEG (from embedded FlashRaw preview)
/// - **Metadata**: Dimensions (FlashRaw) + camera EXIF (rexif)
///
/// # Features
///
/// - Extracts dimensions and codec information from Hasselblad RAW files via FlashRaw.
/// - Generates JPEG thumbnails from Hasselblad RAW files via FlashRaw.
///
/// # Examples
///
/// ```rust
/// use mundam_core::formats::provider::FormatProvider;
/// use mundam_core::formats::types::MediaType;
///
/// let provider = HasselbladRawFormatProvider::new();
/// let formats = provider.supported_formats();
/// assert!(formats.contains(&MediaType::Image));
/// ```
#[derive(Default)]
pub struct HasselbladRawFormatProvider;

impl HasselbladRawFormatProvider {
    /// Creates a new instance of the Hasselblad RAW format provider.
    ///
    /// # Returns
    ///
    /// `HasselbladRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for HasselbladRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "HASSELBLAD_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["3fr", "fff"]
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
            "Hasselblad RAW Image",
            vec!["3fr", "fff"],
            vec!["image/x-hasselblad-3fr", "image/x-hasselblad-fff"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Checks if the file header bytes match Hasselblad RAW magic bytes.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The header bytes to check.
    ///
    /// # Returns
    ///
    /// `bool` - `true` if the header bytes match, `false` otherwise.
    fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
        if header_bytes.len() < 4 {
            return false;
        }
        // 3FR uses TIFF Little-Endian: II* (49 49 2A 00)
        let is_tiff_little_endian = header_bytes[0] == 0x49
            && header_bytes[1] == 0x49
            && header_bytes[2] == 0x2A
            && header_bytes[3] == 0x00;
        // FFF uses TIFF Big-Endian: MM * (4D 4D 00 2A)
        let is_tiff_big_endian = header_bytes[0] == 0x4D
            && header_bytes[1] == 0x4D
            && header_bytes[2] == 0x00
            && header_bytes[3] == 0x2A;
        is_tiff_little_endian || is_tiff_big_endian
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
impl MetadataCapability for HasselbladRawFormatProvider {
    /// Extracts technical metadata from Hasselblad RAW files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Hasselblad RAW file.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The extracted technical metadata.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::image::extract_raw_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Extracts semantic metadata from Hasselblad RAW files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Hasselblad RAW file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - The extracted semantic metadata.
    async fn extract_semantic(&self, _path: &Path) -> AppResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

#[async_trait]
impl ThumbnailCapability for HasselbladRawFormatProvider {
    /// Generates a thumbnail from a Hasselblad RAW file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Hasselblad RAW file.
    /// * `_asset_id` - The asset ID (unused).
    /// * `size_hint` - The desired size of the thumbnail.
    ///
    /// # Errors
    ///
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - The generated thumbnail as a byte vector.
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
impl PreviewCapability for HasselbladRawFormatProvider {
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
