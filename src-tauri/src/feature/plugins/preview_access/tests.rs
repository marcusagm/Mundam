use super::*;
use crate::{
    core::{
        ledger::{command::LedgerCommand, port::TransactionalAssetLedger},
        processing::{ProcessingKind, ProcessingRepository, ProcessingResult, PublishedThumbnail},
    },
    infra::filesystem::plugin_previews::NativePluginPreviews,
    processing::workers::test_support::{ProcessingFixture, TestResult},
};
use async_trait::async_trait;
use std::io::Cursor;

mod approval;

struct Publisher {
    fixture: ProcessingFixture,
    storage: NativePluginPreviews,
    revision: PluginPreviewRevision,
    cache_file: String,
    encoded: Vec<u8>,
}

impl Publisher {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("asset", None).await?;
        let ticket = fixture
            .processing
            .begin(ProcessingKind::Thumbnail, "asset")
            .await?
            .ok_or("missing native reservation")?;
        let cache_file = format!("{}.{}.webp", ticket.asset_id, ticket.lease_id);
        let directory = fixture.data.path().join("thumbnails");
        let mut webp = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3).write_to(&mut webp, image::ImageFormat::WebP)?;
        let encoded = webp.into_inner();
        tokio::fs::write(directory.join(&cache_file), &encoded).await?;
        assert!(
            fixture
                .ledger
                .commit_processing(
                    &ticket,
                    ProcessingResult::Thumbnail {
                        thumbnail: Some(PublishedThumbnail {
                            path: cache_file.clone(),
                            content_digest: *blake3::hash(&encoded).as_bytes(),
                        }),
                        preview_path: None,
                        metadata: None,
                        unsupported_reason: None,
                    }
                )
                .await?
        );
        let revision = fixture
            .ledger
            .plugin_preview_source("asset")
            .await?
            .ok_or("missing published preview")?
            .revision;
        Ok(Self {
            fixture,
            storage: NativePluginPreviews::new(directory),
            revision,
            cache_file,
            encoded,
        })
    }

    async fn prepare(&self, cancellation: CancellationToken) -> AppResult<PluginPreviewLease> {
        prepare_image_preview(
            self.fixture.ledger.clone(),
            &self.storage,
            "asset".into(),
            self.revision,
            Instant::now() + MAX_LEASE_TIME,
            cancellation,
        )
        .await
    }

    async fn regenerate(&self) -> AppResult<()> {
        self.fixture
            .ledger
            .execute(LedgerCommand::RegenerateThumbnail {
                asset_id: "asset".into(),
            })
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn plugin_preview_lease_scopes_ranges_and_keeps_verified_bytes() -> TestResult {
    let publisher = Publisher::new().await?;
    let cancel = CancellationToken::new();
    let mut lease = publisher.prepare(cancel.clone()).await?;
    let other = publisher.prepare(CancellationToken::new()).await?;
    let offer = lease.offer();
    assert_eq!(offer.byte_count, publisher.encoded.len());
    assert!(matches!(
        lease.read(other.offer().handle, 0, 4).await,
        Err(AppError::Forbidden(_))
    ));
    assert!(matches!(
        lease.read(offer.handle, usize::MAX, 4).await,
        Err(AppError::ValidationFailed(_))
    ));

    let path = publisher
        .fixture
        .data
        .path()
        .join("thumbnails")
        .join(&publisher.cache_file);
    let mut overwritten = publisher.encoded.clone();
    let last = overwritten.last_mut().ok_or("empty fixture")?;
    *last ^= 1;
    tokio::fs::write(path, overwritten).await?;
    assert_eq!(
        lease.read(offer.handle, 0, offer.byte_count).await?,
        publisher.encoded
    );
    assert!(matches!(
        publisher.prepare(CancellationToken::new()).await,
        Err(AppError::InvalidProcessingOutput)
    ));

    cancel.cancel();
    assert!(matches!(
        lease.read(offer.handle, 0, 4).await,
        Err(AppError::Cancelled)
    ));
    Ok(())
}

#[tokio::test]
async fn plugin_preview_lease_stops_when_publication_or_budget_expires() -> TestResult {
    let publisher = Publisher::new().await?;
    let mut budget = publisher.prepare(CancellationToken::new()).await?;
    let offer = budget.offer();
    for _ in 0..MAX_READ_CALLS {
        assert_eq!(budget.read(offer.handle, 0, 4).await?, b"RIFF");
    }
    assert!(matches!(
        budget.read(offer.handle, 0, 4).await,
        Err(AppError::Forbidden(_))
    ));

    let mut expired = publisher.prepare(CancellationToken::new()).await?;
    expired.deadline = Instant::now();
    assert!(matches!(
        expired.read(expired.offer().handle, 0, 4).await,
        Err(AppError::Cancelled)
    ));

    let mut stale = publisher.prepare(CancellationToken::new()).await?;
    publisher.regenerate().await?;
    assert!(matches!(
        stale.read(stale.offer().handle, 0, 4).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        publisher.prepare(CancellationToken::new()).await,
        Err(AppError::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn plugin_preview_revalidates_the_source_after_storage_read() -> TestResult {
    struct SupersededRead<'a>(&'a Publisher);
    #[async_trait]
    impl PluginPreviewStorage for SupersededRead<'_> {
        async fn read_thumbnail(&self, file_name: &str) -> AppResult<Vec<u8>> {
            let old_bytes = self.0.storage.read_thumbnail(file_name).await?;
            self.0.regenerate().await?;
            Ok(old_bytes)
        }
    }
    let publisher = Publisher::new().await?;
    let prepared = prepare_image_preview(
        publisher.fixture.ledger.clone(),
        &SupersededRead(&publisher),
        "asset".into(),
        publisher.revision,
        Instant::now() + MAX_LEASE_TIME,
        CancellationToken::new(),
    )
    .await;
    assert!(matches!(prepared, Err(AppError::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn plugin_preview_storage_rejects_oversized_files_paths_and_non_images() -> TestResult {
    let publisher = Publisher::new().await?;
    for invalid in [
        "../outside.webp",
        "C:\\outside.webp",
        "asset.invalid.webp",
        "asset.webp:stream",
    ] {
        assert!(matches!(
            publisher.storage.read_thumbnail(invalid).await,
            Err(AppError::ValidationFailed(_))
        ));
    }
    sqlx::query("UPDATE assets SET family = 'VIDEO' WHERE id = 'asset'")
        .execute(&publisher.fixture.pool)
        .await?;
    assert!(matches!(
        publisher.prepare(CancellationToken::new()).await,
        Err(AppError::FormatNotSupported(_))
    ));
    let path = publisher
        .fixture
        .data
        .path()
        .join("thumbnails")
        .join(&publisher.cache_file);
    let file = tokio::fs::OpenOptions::new().write(true).open(path).await?;
    file.set_len((MAX_PLUGIN_PREVIEW_BYTES + 1) as u64).await?;
    drop(file);
    assert!(matches!(
        publisher
            .storage
            .read_thumbnail(&publisher.cache_file)
            .await,
        Err(AppError::InvalidProcessingOutput)
    ));
    Ok(())
}
