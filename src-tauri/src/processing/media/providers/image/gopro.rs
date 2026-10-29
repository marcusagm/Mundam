use crate::core::formats::capabilities::{
    MetadataCapability, PreviewCapability, ThumbnailCapability,
};
use crate::core::formats::provider::{FormatProvider, SupportedFormat};
use crate::core::AppResult;
use async_trait::async_trait;
use std::path::Path;
use tracing::instrument;

/// Provider for GoPro RAW image files (.gpr).
///
/// GPR (GoPro RAW) is based on Adobe DNG but uses VC-5 (CineForm wavelet)
/// compression for the sensor data. This makes it incompatible with standard
/// RAW decoders (FlashRaw/FlashRaw returns `FileUnsupported`, FFmpeg fails with
/// "Unknown compression method 9").
///
/// Most GPR files contain **no** embedded JPEG previews, unlike standard DNG.
///
/// # Technical Details
///
/// - **Cameras**: GoPro HERO5 Black through HERO12+
/// - **Container**: TIFF/EP with DNG extensions
/// - **Sensor Compression**: VC-5 (CineForm wavelet), not standard Lossless JPEG
/// - **Metadata**: Standard TIFF IFD0 dimensions + EXIF via `rexif`
/// - **Thumbnail/Preview**: FlashRaw's Rust VC-5 decoder and RAW development pipeline
///
/// # Examples
///
/// ```no_run
/// use mundam_lib::processing::media::providers::image::gopro::GoproRawFormatProvider;
///
/// let provider = GoproRawFormatProvider::new();
/// let formats = provider.supported_formats();
///
/// assert_eq!(formats.len(), 1);
/// assert_eq!(formats[0].name, "GoPro RAW Image");
/// assert_eq!(formats[0].extensions, vec!["gpr"]);
/// ```
#[derive(Default)]
pub struct GoproRawFormatProvider;

impl GoproRawFormatProvider {
    /// Creates a new instance of `GoproRawFormatProvider`.
    ///
    /// # Returns
    ///
    /// `GoproRawFormatProvider` - A new instance of the provider.
    pub fn new() -> Self {
        Self
    }
}

impl FormatProvider for GoproRawFormatProvider {
    /// Returns the unique identifier for this provider.
    ///
    /// # Returns
    ///
    /// `&'static str` - The unique name for this provider.
    fn name(&self) -> &'static str {
        "GOPRO_RAW_PROVIDER"
    }

    /// Returns the list of file extensions supported by this provider.
    ///
    /// # Returns
    ///
    /// `Vec<&'static str>` - The file extensions supported by this provider.
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["gpr"]
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
            "GoPro RAW Image",
            vec!["gpr"],
            vec!["image/x-gopro-gpr"],
            MediaType::Image,
            ThumbnailStrategy::Raw,
            PreviewStrategy::NativeExtractor,
            PlaybackStrategy::None,
        )]
    }

    /// Validates if the file header matches the GoPro RAW magic bytes.
    ///
    /// GPR files use the standard TIFF header (Little-Endian `II*` or Big-Endian `MM*`).
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The first bytes of the file.
    ///
    /// # Returns
    ///
    /// `bool` - True if it's a valid GoPro RAW file.
    fn supports_magic_bytes(&self, header_bytes: &[u8]) -> bool {
        header_bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
            || header_bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
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
impl MetadataCapability for GoproRawFormatProvider {
    /// Extracts dimensions from the TIFF IFD0 and camera EXIF data via rexif.
    ///
    /// Does **not** use FlashRaw (which fails on VC-5), instead parses the
    /// TIFF/DNG container structure directly for reliable dimension extraction.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the GPR file.
    ///
    /// # Returns
    ///
    /// `AppResult<serde_json::Value>` - A JSON object containing the extracted metadata.
    ///
    /// # Errors
    ///
    /// * `AppError::Io` - If the file cannot be read.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn extract_technical(&self, path: &Path) -> AppResult<serde_json::Value> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::gpr::extract_gpr_metadata(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }

    /// Semantic extraction for GoPro RAW images is not implemented.
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
impl ThumbnailCapability for GoproRawFormatProvider {
    /// Generates a WebP thumbnail by decoding VC-5 sensor data with FlashRaw.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the GPR file.
    /// * `_asset_id` - The ID of the asset (unused).
    /// * `size_hint` - Maximum dimension (width or height) in pixels.
    ///
    /// # Returns
    ///
    /// `AppResult<Vec<u8>>` - WebP-encoded thumbnail bytes.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - If FlashRaw cannot decode the GPR image.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn generate(&self, path: &Path, _asset_id: &str, size_hint: u32) -> AppResult<Vec<u8>> {
        let path_owned = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::gpr::generate_gpr_thumbnail(
                &path_owned,
                size_hint,
            )
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)?
    }
}

#[async_trait]
impl PreviewCapability for GoproRawFormatProvider {
    /// Generates a JPEG preview by decoding VC-5 sensor data with FlashRaw.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the GPR file.
    /// * `_asset_id` - The ID of the asset (unused).
    ///
    /// # Returns
    ///
    /// `AppResult<(Vec<u8>, String)>` - JPEG bytes and MIME type.
    ///
    /// # Errors
    ///
    /// * `AppError::Generic` - If all extraction tiers fail.
    /// * `AppError::ExtractionProcessTimeout` - If the blocking task times out.
    #[instrument(skip(self, path))]
    async fn generate_preview(&self, path: &Path, _asset_id: &str) -> AppResult<(Vec<u8>, String)> {
        let path_owned = path.to_path_buf();
        let bytes = tokio::task::spawn_blocking(move || {
            crate::processing::media::extractors::gpr::extract_gpr_preview(&path_owned)
        })
        .await
        .map_err(|_| crate::core::error::AppError::ExtractionProcessTimeout)??;

        Ok((bytes, "image/jpeg".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn gpr_sample_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("file-samples")
            .join("Arquivo para testes")
            .join("Image")
            .join("gpr")
            .join("GOPR0002.GPR")
    }

    #[tokio::test]
    async fn test_gopro_raw_provider_capabilities() {
        let provider = GoproRawFormatProvider::new();
        assert_eq!(provider.name(), "GOPRO_RAW_PROVIDER");
        assert!(provider.supported_extensions().contains(&"gpr"));
        assert!(provider.supports_magic_bytes(&[0x49, 0x49, 0x2A, 0x00]));
        assert!(provider.supports_magic_bytes(&[0x4D, 0x4D, 0x00, 0x2A]));
    }

    #[tokio::test]
    async fn test_gopro_raw_metadata_extraction() {
        let provider = GoproRawFormatProvider::new();
        let sample_file_path = gpr_sample_path();
        if !sample_file_path.exists() {
            return;
        }

        let metadata_result = provider.extract_technical(&sample_file_path).await;
        assert!(
            metadata_result.is_ok(),
            "Metadata extraction failed: {:?}",
            metadata_result.err()
        );
        let metadata_value = metadata_result.unwrap();

        let image_width = metadata_value["width"].as_u64().unwrap();
        let image_height = metadata_value["height"].as_u64().unwrap();
        assert_eq!(image_width, 5568, "GOPR0002 should be 5568px wide");
        assert_eq!(image_height, 4176, "GOPR0002 should be 4176px tall");

        assert!(
            metadata_value.get("Model").is_some(),
            "Metadata should contain camera Model"
        );
    }

    #[tokio::test]
    async fn test_gopro_raw_thumbnail_generation() {
        let provider = GoproRawFormatProvider::new();
        let sample_file_path = gpr_sample_path();
        if !sample_file_path.exists() {
            return;
        }

        let thumbnail_result = provider
            .generate(&sample_file_path, "test_asset_id", 300)
            .await;
        assert!(
            thumbnail_result.is_ok(),
            "Thumbnail generation failed: {:?}",
            thumbnail_result.err()
        );
        let thumbnail_bytes = thumbnail_result.unwrap();
        assert!(!thumbnail_bytes.is_empty(), "Generated thumbnail is empty");
    }

    #[tokio::test]
    async fn test_gopro_raw_preview_generation() {
        let provider = GoproRawFormatProvider::new();
        let sample_file_path = gpr_sample_path();
        if !sample_file_path.exists() {
            return;
        }

        let preview_result = provider
            .generate_preview(&sample_file_path, "test_asset_id")
            .await;
        assert!(
            preview_result.is_ok(),
            "Preview generation failed: {:?}",
            preview_result.err()
        );
        let (preview_bytes, mime_type) = preview_result.unwrap();
        assert_eq!(mime_type, "image/jpeg");
        assert!(
            preview_bytes.starts_with(&[0xFF, 0xD8]),
            "Preview is not a valid JPEG"
        );
        assert!(
            preview_bytes.len() > 10_000,
            "Preview seems too small: {} bytes",
            preview_bytes.len()
        );
    }
}
