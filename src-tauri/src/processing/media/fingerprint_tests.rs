use super::*;
use crate::core::formats::{
    capabilities::{PreviewCapability, ThumbnailCapability},
    provider::FormatProvider,
};
use crate::core::repository::AssetQueryHandler;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

fn gradient() -> image::RgbImage {
    image::RgbImage::from_fn(32, 32, |column, _row| {
        let value = (255 - column * 6) as u8;
        image::Rgb([value, value, value])
    })
}

struct PreviewProvider {
    bytes: Vec<u8>,
    fail_preview: bool,
    cancel: Option<CancellationToken>,
    thumbnails: Arc<AtomicUsize>,
}

impl FormatProvider for PreviewProvider {
    fn name(&self) -> &'static str {
        "FINGERPRINT_TEST_PROVIDER"
    }
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["png"]
    }
    fn preview(&self) -> Option<&dyn PreviewCapability> {
        Some(self)
    }
    fn thumbnail(&self) -> Option<&dyn ThumbnailCapability> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl PreviewCapability for PreviewProvider {
    async fn generate_preview(
        &self,
        _path: &Path,
        _asset_id: &str,
    ) -> AppResult<(Vec<u8>, String)> {
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        if self.fail_preview {
            return Err(AppError::Generic("Preview unavailable".into()));
        }
        Ok((self.bytes.clone(), "image/png".into()))
    }
}

#[async_trait::async_trait]
impl ThumbnailCapability for PreviewProvider {
    async fn generate(&self, _path: &Path, _asset_id: &str, size_hint: u32) -> AppResult<Vec<u8>> {
        assert_eq!(size_hint, 1024);
        self.thumbnails.fetch_add(1, Ordering::SeqCst);
        Ok(self.bytes.clone())
    }
}

#[tokio::test]
async fn processing_fingerprint_preserves_v5_visual_hashes_and_uses_image_family() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("gradient", None).await?;
    gradient().save(fixture.data.path().join("gradient.png"))?;
    sqlx::query("UPDATE assets SET family = 'IMAGE', format_type = 'raw_sensor_alias'")
        .execute(&fixture.pool)
        .await?;
    let asset = fixture.queries.get_asset_by_id("gradient").await?;
    let original = tokio::fs::read(&asset.path).await?;
    let fingerprint = generate(asset, fixture.registry.clone(), CancellationToken::new())
        .await?
        .ok_or("Missing result")?;
    assert_eq!(
        fingerprint.perceptual_hash.as_deref(),
        Some("ffffffffffffffff")
    );
    assert_eq!(fingerprint.block_hash, Some("f".repeat(64)));
    assert_eq!(fingerprint.thumb_hash, Some("f".repeat(64)));
    assert_eq!(
        (fingerprint.width, fingerprint.height),
        (Some(32), Some(32))
    );
    assert_eq!(fingerprint.format_family.as_deref(), Some("IMAGE"));
    assert_eq!(fingerprint.fingerprint_version, 5);
    assert_eq!(
        fingerprint.content_hash,
        Some(blake3::hash(&original).to_hex().to_string())
    );
    assert_eq!(
        tokio::fs::read(fixture.data.path().join("gradient.png")).await?,
        original
    );
    Ok(())
}

#[tokio::test]
async fn processing_fingerprint_streams_binary_files_and_accepts_exact_only_images() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    for (identifier, family) in [("binary", "DOCUMENT"), ("corrupt", "IMAGE")] {
        fixture.asset(identifier, None).await?;
        sqlx::query("UPDATE assets SET family = ?, format_type = 'binary_alias' WHERE id = ?")
            .bind(family)
            .bind(identifier)
            .execute(&fixture.pool)
            .await?;
        let asset = fixture.queries.get_asset_by_id(identifier).await?;
        let bytes: Vec<u8> = (0..20001).map(|index| (index % 251) as u8).collect();
        tokio::fs::write(&asset.path, &bytes).await?;
        let result = generate(asset, fixture.registry.clone(), CancellationToken::new())
            .await?
            .ok_or("Missing exact hash")?;
        assert_eq!(
            result.content_hash,
            Some(blake3::hash(&bytes).to_hex().to_string())
        );
        assert_eq!(result.file_size, Some(20001));
        assert!(result.perceptual_hash.is_none());
        assert!(result.block_hash.is_none());
        assert!(result.thumb_hash.is_none());
    }
    Ok(())
}

#[tokio::test]
async fn processing_fingerprint_provider_fallback_and_cancellation_share_one_order() -> TestResult {
    for (fail_preview, canceled, expected_thumbnails) in
        [(false, false, 0), (true, false, 1), (true, true, 0)]
    {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("exotic", None).await?;
        let asset = fixture.queries.get_asset_by_id("exotic").await?;
        tokio::fs::write(&asset.path, b"Unsupported raster container").await?;
        let mut encoded = std::io::Cursor::new(Vec::new());
        gradient().write_to(&mut encoded, image::ImageFormat::Png)?;
        let token = CancellationToken::new();
        let thumbnails = Arc::new(AtomicUsize::new(0));
        let mut registry = FormatRegistry::new();
        registry.register(Arc::new(PreviewProvider {
            bytes: encoded.into_inner(),
            fail_preview,
            cancel: canceled.then(|| token.clone()),
            thumbnails: thumbnails.clone(),
        }));
        let result = generate(asset, Arc::new(registry), token).await?;
        assert_eq!(result.is_none(), canceled);
        if let Some(result) = result {
            assert_eq!(result.perceptual_hash.as_deref(), Some("ffffffffffffffff"));
        }
        assert_eq!(thumbnails.load(Ordering::SeqCst), expected_thumbnails);
    }
    Ok(())
}

#[test]
fn processing_fingerprint_cancellation_stops_streaming_between_blocks() -> TestResult {
    struct CancelAfterRead {
        token: CancellationToken,
        reads: usize,
    }
    impl Read for CancelAfterRead {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            buffer.fill(7);
            self.token.cancel();
            Ok(buffer.len())
        }
    }
    let token = CancellationToken::new();
    let mut reader = CancelAfterRead {
        token: token.clone(),
        reads: 0,
    };
    assert!(hash_reader(&mut reader, &token)?.is_none());
    assert_eq!(reader.reads, 1);
    assert!(hash_reader(&mut reader, &token)?.is_none());
    assert_eq!(reader.reads, 1);
    Ok(())
}
