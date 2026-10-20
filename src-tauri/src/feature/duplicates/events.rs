use crate::core::events::payloads::DomainEvent;
use crate::core::events::bus::AppEventBus;
use crate::core::formats::FormatRegistry;
use crate::core::repository::DuplicatesRepository;
use crate::core::models::DuplicateFingerprint;
use crate::feature::settings::SettingsService;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// Background worker that listens to domain events and generates
/// duplicates fingerprints for new assets.
pub struct DuplicateWorker {
    duplicates_repo: Arc<dyn DuplicatesRepository>,
    event_bus: Arc<dyn AppEventBus>,
    settings_service: SettingsService,
    format_registry: Arc<FormatRegistry>,
}

impl DuplicateWorker {
    /// Creates a new instance of DuplicateWorker.
    pub fn new(
        duplicates_repo: Arc<dyn DuplicatesRepository>,
        event_bus: Arc<dyn AppEventBus>,
        settings_service: SettingsService,
        format_registry: Arc<FormatRegistry>,
    ) -> Self {
        Self {
            duplicates_repo,
            event_bus,
            settings_service,
            format_registry,
        }
    }

    /// Starts the background listener and autonomous polling loops.
    pub fn start(
        self: Arc<Self>,
        cancellation_token: CancellationToken,
    ) -> tauri::async_runtime::JoinHandle<()> {
        info!("DuplicateWorker: starting event listener and background polling loops");

        let worker_instance_for_events = self.clone();
        let token_for_events = cancellation_token.clone();
        let events_join_handle = tauri::async_runtime::spawn(async move {
            worker_instance_for_events.listen_to_domain_events(token_for_events).await;
        });

        let worker_instance_for_polling = self.clone();
        let token_for_polling = cancellation_token.clone();
        let polling_join_handle = tauri::async_runtime::spawn(async move {
            worker_instance_for_polling.background_fingerprint_loop(token_for_polling).await;
        });

        tauri::async_runtime::spawn(async move {
            let _ = tokio::join!(events_join_handle, polling_join_handle);
            info!("DuplicateWorker: all worker tasks have stopped");
        })
    }

    /// Listens to domain events on the event bus.
    async fn listen_to_domain_events(&self, cancellation_token: CancellationToken) {
        let mut event_subscriber = self.event_bus.subscribe();
        info!("DuplicateWorker: event listener loop started and subscribing to DomainEvents");

        loop {
            tokio::select! {
                receive_result = event_subscriber.recv() => {
                    match receive_result {
                        Ok(event) => {
                            self.handle_single_event(event).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                            warn!("DuplicateWorker: lagged behind {} events in event bus", skipped_count);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            error!("DuplicateWorker: subscriber channel closed, terminating event listener");
                            break;
                        }
                    }
                }
                _ = cancellation_token.cancelled() => {
                    info!("DuplicateWorker: event listener cancelled via token. Shutting down.");
                    break;
                }
            }
        }
    }

    /// Autonomous background loop that processes assets missing fingerprints.
    async fn background_fingerprint_loop(&self, cancellation_token: CancellationToken) {
        info!("DuplicateWorker: autonomous background fingerprint polling loop started");
        let mut has_processed_any_assets_since_last_idle = false;

        loop {
            if cancellation_token.is_cancelled() {
                info!("DuplicateWorker: background fingerprint loop shutting down via cancellation token");
                break;
            }

            // 1. Check if auto_detect_duplicates is enabled in settings
            let is_auto_detect_enabled = match self.settings_service.get_settings().await {
                Ok(application_settings) => application_settings.auto_detect_duplicates,
                Err(_) => true,
            };

            if !is_auto_detect_enabled {
                debug!("DuplicateWorker: auto_detect_duplicates is disabled in settings, waiting...");
                tokio::select! {
                    _ = cancellation_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => continue,
                }
            }

            // 2. Query batch of assets needing fingerprints
            let pending_assets_result = self.duplicates_repo.get_assets_needing_fingerprints(10).await;
            let pending_assets = match pending_assets_result {
                Ok(assets_list) => assets_list,
                Err(database_error) => {
                    warn!("DuplicateWorker: error querying assets needing fingerprints: {}", database_error);
                    tokio::select! {
                        _ = cancellation_token.cancelled() => break,
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => continue,
                    }
                }
            };

            if pending_assets.is_empty() {
                if has_processed_any_assets_since_last_idle && !cancellation_token.is_cancelled() {
                    has_processed_any_assets_since_last_idle = false;
                    info!("DuplicateWorker: all pending fingerprints processed, checking if visual match scan should run");
                    if let Ok(Some(active_rule_set)) = self.duplicates_repo.get_active_rule_set().await {
                        if active_rule_set.consider_visual_match || active_rule_set.consider_crop_match {
                            info!("DuplicateWorker: running background visual match scan after completing fingerprint batches");
                            match self.duplicates_repo.run_visual_match_scan(Some(cancellation_token.clone())).await {
                                Ok(visual_groups_count) => {
                                    info!("DuplicateWorker: background visual match scan completed (found {} groups)", visual_groups_count);
                                    if visual_groups_count > 0 {
                                        let _ = self.event_bus.publish(DomainEvent::DuplicateGroupCreated {
                                            group_id: String::new(),
                                            group_type: "visual".to_string(),
                                            confidence: 0.9,
                                            canonical_asset_id: None,
                                            candidate_count: 2,
                                            rule_set_id: active_rule_set.id.clone(),
                                        });
                                    }
                                }
                                Err(visual_match_error) => {
                                    warn!("DuplicateWorker: background visual match scan failed: {}", visual_match_error);
                                }
                            }
                        }
                    }
                }

                // No pending assets, idle wait
                tokio::select! {
                    _ = cancellation_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1500)) => continue,
                }
            } else {
                info!("DuplicateWorker: background loop found {} pending asset(s) needing fingerprints", pending_assets.len());
            }

            let mut processed_any_in_batch = false;
            let total_batch_count = pending_assets.len();

            for (asset_index, pending_asset) in pending_assets.into_iter().enumerate() {
                if cancellation_token.is_cancelled() {
                    info!("DuplicateWorker: background loop cancelled during batch processing at asset {}/{}", asset_index + 1, total_batch_count);
                    break;
                }

                let asset_id_clone = pending_asset.asset_id.clone();
                let path_clone = pending_asset.path.clone();
                let format_type_clone = pending_asset.format_type.clone();
                let format_registry_clone = self.format_registry.clone();

                debug!(
                    "DuplicateWorker: [{}/{}] generating fingerprint for asset_id={}, path={}, format={}",
                    asset_index + 1,
                    total_batch_count,
                    pending_asset.asset_id,
                    pending_asset.path,
                    pending_asset.format_type
                );

                let fingerprint_result = tokio::task::spawn_blocking(move || {
                    Self::generate_fingerprint(
                        &asset_id_clone,
                        &path_clone,
                        &format_type_clone,
                        &format_registry_clone,
                    )
                })
                .await;

                match fingerprint_result {
                    Ok(Ok(fingerprint)) => {
                        let content_hash_summary = fingerprint.content_hash.clone();
                        let perceptual_hash_summary = fingerprint.perceptual_hash.clone();

                        if let Err(database_error) = self.duplicates_repo.save_fingerprint(fingerprint).await {
                            error!(
                                "DuplicateWorker: failed to save fingerprint for {}: {}",
                                pending_asset.asset_id, database_error
                            );
                        } else {
                            debug!(
                                "DuplicateWorker: saved fingerprint for asset_id={} (content_hash={:?}, perceptual_hash={:?})",
                                pending_asset.asset_id, content_hash_summary, perceptual_hash_summary
                            );
                            processed_any_in_batch = true;
                            has_processed_any_assets_since_last_idle = true;
                        }
                    }
                    Ok(Err(generation_error)) => {
                        error!(
                            "DuplicateWorker: error generating fingerprint for {}: {}",
                            pending_asset.asset_id, generation_error
                        );
                    }
                    Err(join_error) => {
                        error!(
                            "DuplicateWorker: blocking task panicked for {}: {}",
                            pending_asset.asset_id, join_error
                        );
                    }
                }
            }

            if processed_any_in_batch && !cancellation_token.is_cancelled() {
                info!("DuplicateWorker: finished processing background batch of {} asset(s), running exact match scan", total_batch_count);
                match self.duplicates_repo.run_exact_match_scan(Some(cancellation_token.clone())).await {
                    Ok(exact_groups_count) => {
                        info!("DuplicateWorker: exact match scan completed successfully after background batch (found {} groups)", exact_groups_count);
                        if exact_groups_count > 0 {
                            let _ = self.event_bus.publish(DomainEvent::DuplicateGroupCreated {
                                group_id: String::new(),
                                group_type: "exact".to_string(),
                                confidence: 1.0,
                                canonical_asset_id: None,
                                candidate_count: 2,
                                rule_set_id: "exact-match".to_string(),
                            });
                        }
                    }
                    Err(scan_error) => {
                        warn!("DuplicateWorker: failed to auto-group exact matches in background: {}", scan_error);
                    }
                }
            }
        }
    }

    /// Process a single event. 
    /// Dispatches blocking operations to `spawn_blocking` to avoid blocking the async executor.
    async fn handle_single_event(&self, event: DomainEvent) {
        match event {
            DomainEvent::AssetCreated { asset_id, path, format } => {
                info!("DuplicateWorker: received DomainEvent::AssetCreated for asset_id={}, path={}, format={}", asset_id, path, format);
                let asset_id_clone = asset_id.clone();
                let path_clone = path.clone();
                let format_registry_clone = self.format_registry.clone();
                
                // Spawn a blocking task to perform heavy CPU/IO hashing
                let fingerprint_result = tokio::task::spawn_blocking(move || {
                    Self::generate_fingerprint(&asset_id_clone, &path_clone, &format, &format_registry_clone)
                }).await;

                match fingerprint_result {
                    Ok(Ok(fingerprint)) => {
                        let content_hash_summary = fingerprint.content_hash.clone();
                        let perceptual_hash_summary = fingerprint.perceptual_hash.clone();

                        if let Err(database_error) = self.duplicates_repo.save_fingerprint(fingerprint).await {
                            error!("DuplicateWorker: failed to save fingerprint for {}: {}", asset_id, database_error);
                        } else {
                            info!(
                                "DuplicateWorker: successfully generated and saved fingerprint for asset_id={} (content_hash={:?}, perceptual_hash={:?})",
                                asset_id, content_hash_summary, perceptual_hash_summary
                            );
                            
                            let is_auto_detect_enabled = match self.settings_service.get_settings().await {
                                Ok(application_settings) => application_settings.auto_detect_duplicates,
                                Err(_) => true,
                            };

                            if is_auto_detect_enabled {
                                info!("DuplicateWorker: auto_detect_duplicates is enabled, running exact match scan for asset_id={}", asset_id);
                                match self.duplicates_repo.run_exact_match_scan(None).await {
                                    Ok(exact_groups_count) => {
                                        info!("DuplicateWorker: exact match scan succeeded for asset_id={} (found {} groups)", asset_id, exact_groups_count);
                                        if exact_groups_count > 0 {
                                            let _ = self.event_bus.publish(DomainEvent::DuplicateGroupCreated {
                                                group_id: String::new(),
                                                group_type: "exact".to_string(),
                                                confidence: 1.0,
                                                canonical_asset_id: Some(asset_id.clone()),
                                                candidate_count: 2,
                                                rule_set_id: "exact-match".to_string(),
                                            });
                                        }
                                    }
                                    Err(scan_error) => {
                                        warn!("DuplicateWorker: failed to auto-group exact matches: {}", scan_error);
                                    }
                                }
                            } else {
                                debug!("DuplicateWorker: auto-detect on import is disabled in settings");
                            }
                        }
                    }
                    Ok(Err(generation_error)) => {
                        error!("DuplicateWorker: error generating fingerprint for {}: {}", asset_id, generation_error);
                    }
                    Err(join_error) => {
                        error!("DuplicateWorker: blocking task panicked or was cancelled for {}: {}", asset_id, join_error);
                    }
                }
            }
            DomainEvent::AssetDeleted { asset_id, .. } => {
                info!("DuplicateWorker: received DomainEvent::AssetDeleted for asset_id={}, cleaning up fingerprint and candidates", asset_id);
                
                if let Err(error) = self.duplicates_repo.delete_fingerprint(&asset_id).await {
                    error!("DuplicateWorker: failed to delete fingerprint for {}: {}", asset_id, error);
                } else {
                    debug!("DuplicateWorker: deleted fingerprint for asset_id={}", asset_id);
                }

                if let Err(error) = self.duplicates_repo.remove_candidate_from_groups(&asset_id).await {
                    error!("DuplicateWorker: failed to remove candidate {} from groups: {}", asset_id, error);
                } else {
                    debug!("DuplicateWorker: removed candidate asset_id={} from all duplicate groups", asset_id);
                }
            }
            _ => {}
        }
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
            let bytes_read = file.read(&mut read_buffer)
                .map_err(|error| format!("Failed to read file {}: {}", path, error))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&read_buffer[..bytes_read]);
        }

        let content_hash = Some(hasher.finalize().to_hex().to_string());
        
        let format_is_image = crate::feature::duplicates::fingerprints::is_image_media(format, format);

        let (perceptual_hash, block_hash, thumb_hash, width, height) = {
            if format_is_image {
                let file_path = std::path::Path::new(path);

                // Stage 1: Try direct image::open() for standard formats (JPEG, PNG, WebP, etc.)
                let direct_open_result = image::open(file_path);

                let resolved_dynamic_image = match direct_open_result {
                    Ok(loaded_image) => {
                        debug!("generate_fingerprint: direct image::open succeeded for asset_id={}", asset_id);
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
                    let dhash_value = crate::feature::duplicates::fingerprints::compute_perceptual_dhash_64(&dynamic_image);
                    let multiscale_hash = crate::feature::duplicates::fingerprints::compute_multiscale_dhash_256(&dynamic_image);
                    let spatial_block_hash = crate::feature::duplicates::fingerprints::compute_spatial_block_hash(&dynamic_image);

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

