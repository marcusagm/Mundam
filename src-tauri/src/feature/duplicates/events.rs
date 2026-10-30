//! Automatic fingerprint processing uses one durable queue and shared scan admission.

use super::coordinator::DuplicateProcessingCoordinator;
use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::formats::FormatRegistry;
use crate::core::models::DuplicateFingerprint;
use crate::core::processing::{ProcessingKind, ProcessingOutcome, ProcessingRepository};
use crate::core::repository::{AssetQueryHandler, DuplicatesRepository};
use crate::feature::settings::SettingsService;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

/// Events wake a single polling executor instead of launching a second hashing path.
pub struct DuplicateWorker {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    event_bus: Arc<dyn AppEventBus>,
    settings_service: SettingsService,
    format_registry: Arc<FormatRegistry>,
    processing: Arc<dyn ProcessingRepository>,
    queries: Arc<dyn AssetQueryHandler>,
    coordinator: Arc<DuplicateProcessingCoordinator>,
}

impl DuplicateWorker {
    /// Injects durable attempt bookkeeping and the coordinator used by manual scans.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        event_bus: Arc<dyn AppEventBus>,
        settings_service: SettingsService,
        format_registry: Arc<FormatRegistry>,
        processing: Arc<dyn ProcessingRepository>,
        queries: Arc<dyn AssetQueryHandler>,
        coordinator: Arc<DuplicateProcessingCoordinator>,
    ) -> Self {
        Self {
            duplicates_repo,
            event_bus,
            settings_service,
            format_registry,
            processing,
            queries,
            coordinator,
        }
    }

    /// Processes recoverable work and coalesces comparisons after batches or idle transitions.
    pub fn start(
        self: Arc<Self>,
        token: CancellationToken,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let mut subscriber = self.event_bus.subscribe();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut dirty_count = 0;
            let mut visual_dirty = false;
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {},
                    event = subscriber.recv() => match event {
                        Ok(DomainEvent::AssetCreated { .. } | DomainEvent::ScanCompleted { .. }) => {},
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            warn!(count, "Duplicate events lagged; reconciling durable work");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                if !self
                    .settings_service
                    .get_settings()
                    .await
                    .map(|settings| settings.auto_detect_duplicates)
                    .unwrap_or(true)
                {
                    continue;
                }
                match self.process_batch(&token).await {
                    Ok((processed, idle)) => {
                        dirty_count += processed;
                        visual_dirty |= processed > 0;
                        if ((idle && visual_dirty) || dirty_count >= 100) && !token.is_cancelled() {
                            match self.compare(&token, idle).await {
                                Ok(()) => {
                                    dirty_count = 0;
                                    if idle {
                                        visual_dirty = false;
                                    }
                                }
                                Err(error) => {
                                    warn!(%error, "Automatic duplicate comparison failed")
                                }
                            }
                        }
                    }
                    Err(error) => warn!(%error, "Automatic fingerprint processing failed"),
                }
            }
        })
    }

    async fn process_batch(
        &self,
        token: &CancellationToken,
    ) -> crate::core::error::AppResult<(usize, bool)> {
        let pending = self
            .processing
            .pending(ProcessingKind::Fingerprint, 10)
            .await?;
        let idle = pending.is_empty();
        let mut processed = 0;
        for asset_id in pending {
            let _admission = tokio::select! {
                _ = token.cancelled() => break,
                admission = self.coordinator.acquire() => admission,
            };
            let Some(ticket) = self
                .processing
                .begin(ProcessingKind::Fingerprint, &asset_id)
                .await?
            else {
                continue;
            };
            let result = self.process_fingerprint(&ticket, token).await;
            if token.is_cancelled() {
                break;
            }
            let outcome = match result {
                Ok(outcome) => {
                    processed += 1;
                    outcome
                }
                Err(error) => {
                    warn!(asset_id, %error, "Fingerprint failed; applying bounded retry policy");
                    ProcessingOutcome::Failed(error.to_string())
                }
            };
            self.processing.finish(&ticket, outcome).await?;
        }
        Ok((processed, idle))
    }

    async fn process_fingerprint(
        &self,
        ticket: &crate::core::processing::ProcessingTicket,
        token: &CancellationToken,
    ) -> crate::core::error::AppResult<ProcessingOutcome> {
        let asset = self.queries.get_asset_by_id(&ticket.asset_id).await?;
        let registry = self.format_registry.clone();
        let computation_token = token.clone();
        let fingerprint = tokio::task::spawn_blocking(move || {
            Self::generate_fingerprint(
                &asset.id,
                &asset.path.to_string_lossy(),
                &asset.format_type,
                &registry,
                &computation_token,
            )
        })
        .await
        .map_err(|error| {
            crate::core::error::AppError::Internal(format!("Fingerprint task failed: {error}"))
        })?
        .map_err(crate::core::error::AppError::Internal)?;
        if token.is_cancelled() || !self.processing.is_current(ticket).await? {
            return Ok(ProcessingOutcome::Skipped(
                "Attempt was superseded or canceled".into(),
            ));
        }
        self.duplicates_repo.save_fingerprint(fingerprint).await?;
        Ok(ProcessingOutcome::Completed)
    }

    async fn compare(
        &self,
        token: &CancellationToken,
        include_visual: bool,
    ) -> crate::core::error::AppResult<()> {
        let _admission = tokio::select! {
            _ = token.cancelled() => return Ok(()),
            admission = self.coordinator.acquire() => admission,
        };
        let mut groups_processed = self
            .duplicates_repo
            .run_exact_match_scan(Some(token.clone()))
            .await?;
        if include_visual && !token.is_cancelled() {
            groups_processed += self
                .duplicates_repo
                .run_visual_match_scan(Some(token.clone()))
                .await?;
        }
        if !token.is_cancelled() {
            self.event_bus
                .publish(DomainEvent::DuplicateGroupsChanged { groups_processed })?;
        }
        Ok(())
    }

    /// Synchronous function to read the file and compute a Blake3 content hash
    /// and a perceptual (dHash) hash for image assets.
    /// Runs inside `spawn_blocking` to avoid blocking the async executor.
    ///
    /// Uses streaming hashing via Blake3 to avoid loading the entire file into memory,
    /// which is critical for large assets (RAW photos, videos, etc.).
    ///
    /// # Arguments
    /// * `asset_id` - The UUID of the asset to fingerprint.
    /// * `path` - The canonical filesystem path to the asset file.
    /// * `format` - The asset's format family string as stored in the `assets.family` column
    ///   (e.g. `"image"`, `"image/jpeg"`, `"video"`, `"audio"`). A value starting with
    ///   `"image/"` or equal to `"image"` will trigger perceptual hash computation.
    ///
    /// # Errors
    /// Returns a `String` error if the file cannot be opened, read, or its metadata
    /// cannot be retrieved.
    fn generate_fingerprint(
        asset_id: &str,
        path: &str,
        format: &str,
        format_registry: &FormatRegistry,
        cancellation_token: &CancellationToken,
    ) -> Result<DuplicateFingerprint, String> {
        use std::io::Read;

        // Read file size
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("Failed to read metadata for {}: {}", path, error))?;

        let file_size = metadata.len() as i64;

        debug!(
            "generate_fingerprint: hashing asset_id={}, path={}, format={}, file_size={}",
            asset_id, path, format, file_size
        );

        // Compute Blake3 hash using streaming (8KB chunks) to avoid loading the whole file
        let mut file = std::fs::File::open(path)
            .map_err(|error| format!("Failed to open file {}: {}", path, error))?;

        let mut hasher = blake3::Hasher::new();
        let mut read_buffer = [0u8; 8192];

        loop {
            if cancellation_token.is_cancelled() {
                return Err("Fingerprint computation canceled".into());
            }
            let bytes_read = file
                .read(&mut read_buffer)
                .map_err(|error| format!("Failed to read file {}: {}", path, error))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&read_buffer[..bytes_read]);
        }

        let content_hash = Some(hasher.finalize().to_hex().to_string());

        if cancellation_token.is_cancelled() {
            return Err("Fingerprint computation canceled".into());
        }
        let format_is_image =
            crate::feature::duplicates::fingerprints::is_image_media(format, format);

        let (perceptual_hash, block_hash, thumb_hash, width, height) = {
            if format_is_image {
                let file_path = std::path::Path::new(path);

                // Stage 1: Try direct image::open() for standard formats (JPEG, PNG, WebP, etc.)
                let direct_open_result = image::open(file_path);

                let resolved_dynamic_image = match direct_open_result {
                    Ok(loaded_image) => {
                        debug!(
                            "generate_fingerprint: direct image::open succeeded for asset_id={}",
                            asset_id
                        );
                        Some(loaded_image)
                    }
                    Err(open_error) => {
                        debug!(
                            "generate_fingerprint: direct image::open failed for asset_id={} ({}), attempting FormatProvider fallback",
                            asset_id, open_error
                        );
                        // Stage 2: FormatProvider fallback for exotic formats (RAW, PSD, HDR, EXR, etc.)
                        // This runs inside spawn_blocking, so block_on is safe here.
                        crate::feature::duplicates::fingerprints::resolve_image_via_format_provider(
                            format_registry,
                            file_path,
                            asset_id,
                        )
                    }
                };

                if let Some(dynamic_image) = resolved_dynamic_image {
                    use image::GenericImageView;
                    let (image_width, image_height) = dynamic_image.dimensions();
                    let dhash_value =
                        crate::feature::duplicates::fingerprints::compute_perceptual_dhash_64(
                            &dynamic_image,
                        );
                    let multiscale_hash =
                        crate::feature::duplicates::fingerprints::compute_multiscale_dhash_256(
                            &dynamic_image,
                        );
                    let spatial_block_hash =
                        crate::feature::duplicates::fingerprints::compute_spatial_block_hash(
                            &dynamic_image,
                        );

                    debug!(
                        "generate_fingerprint: resolved image for asset_id={}, dimensions={}x{}, dhash={:016x}",
                        asset_id, image_width, image_height, dhash_value
                    );

                    (
                        Some(format!("{:016x}", dhash_value)),
                        Some(spatial_block_hash),
                        Some(multiscale_hash),
                        Some(image_width as i32),
                        Some(image_height as i32),
                    )
                } else {
                    warn!(
                        "generate_fingerprint: could not decode image pixels for asset_id={}, path={}",
                        asset_id, path
                    );
                    (None, None, None, None, None)
                }
            } else {
                (None, None, None, None, None)
            }
        };

        Ok(DuplicateFingerprint {
            asset_id: asset_id.to_string(),
            content_hash,
            perceptual_hash,
            block_hash,
            thumb_hash,
            width,
            height,
            file_size: Some(file_size),
            mime_type: None,
            format_family: Some(format.to_string()),
            color_profile: None,
            orientation: None,
            fingerprint_version: 5, // v5: FormatProvider fallback for exotic format hashing
            updated_at: chrono::Utc::now(),
        })
    }
}
