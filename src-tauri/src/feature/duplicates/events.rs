use crate::core::events::payloads::DomainEvent;
use crate::core::events::bus::AppEventBus;
use crate::core::formats::FormatRegistry;
use crate::core::repository::DuplicatesRepository;
use crate::core::models::DuplicateFingerprint;
use crate::feature::settings::SettingsService;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

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
        })
    }

    /// Listens to domain events on the event bus.
    async fn listen_to_domain_events(&self, cancellation_token: CancellationToken) {
        let mut event_subscriber = self.event_bus.subscribe();
        info!("DuplicateWorker: started and listening for DomainEvents");

        loop {
            tokio::select! {
                receive_result = event_subscriber.recv() => {
                    match receive_result {
                        Ok(event) => {
                            self.handle_single_event(event).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                            warn!("DuplicateWorker: lagged behind {} events", skipped_count);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            error!("DuplicateWorker subscriber channel closed. Exiting.");
                            break;
                        }
                    }
                }
                _ = cancellation_token.cancelled() => {
                    info!("DuplicateWorker: Event listener cancelled via token. Shutting down.");
                    break;
                }
            }
        }
    }

    /// Autonomous background loop that processes assets missing fingerprints.
    async fn background_fingerprint_loop(&self, cancellation_token: CancellationToken) {
        info!("DuplicateWorker: background fingerprint polling loop started");

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
                // No pending assets, idle wait
                tokio::select! {
                    _ = cancellation_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1500)) => continue,
                }
            }

            let mut processed_any_in_batch = false;

            for pending_asset in pending_assets {
                if cancellation_token.is_cancelled() {
                    break;
                }

                let asset_id_clone = pending_asset.asset_id.clone();
                let path_clone = pending_asset.path.clone();
                let format_type_clone = pending_asset.format_type.clone();
                let format_registry_clone = self.format_registry.clone();

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
                        if let Err(database_error) = self.duplicates_repo.save_fingerprint(fingerprint).await {
                            error!(
                                "DuplicateWorker: failed to save fingerprint for {}: {}",
                                pending_asset.asset_id, database_error
                            );
                        } else {
                            processed_any_in_batch = true;
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
                if let Err(scan_error) = self.duplicates_repo.run_exact_match_scan(Some(cancellation_token.clone())).await {
                    warn!("DuplicateWorker: failed to auto-group exact matches in background: {}", scan_error);
                } else {
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
        }
    }

    /// Process a single event. 
    /// Dispatches blocking operations to `spawn_blocking` to avoid blocking the async executor.
    async fn handle_single_event(&self, event: DomainEvent) {
        match event {
            DomainEvent::AssetCreated { asset_id, path, format } => {
                let asset_id_clone = asset_id.clone();
                let path_clone = path.clone();
                let format_registry_clone = self.format_registry.clone();
                
                // Spawn a blocking task to perform heavy CPU/IO hashing
                let fingerprint_result = tokio::task::spawn_blocking(move || {
                    Self::generate_fingerprint(&asset_id_clone, &path_clone, &format, &format_registry_clone)
                }).await;

                match fingerprint_result {
                    Ok(Ok(fingerprint)) => {
                        if let Err(database_error) = self.duplicates_repo.save_fingerprint(fingerprint).await {
                            error!("DuplicateWorker: failed to save fingerprint for {}: {}", asset_id, database_error);
                        } else {
                            info!("DuplicateWorker: successfully generated and saved fingerprint for {}", asset_id);
                            
                            let is_auto_detect_enabled = match self.settings_service.get_settings().await {
                                Ok(application_settings) => application_settings.auto_detect_duplicates,
                                Err(_) => true,
                            };

                            if is_auto_detect_enabled {
                                if let Err(scan_error) = self.duplicates_repo.run_exact_match_scan(None).await {
                                    warn!("DuplicateWorker: failed to auto-group exact matches: {}", scan_error);
                                } else {
                                    let _ = self.event_bus.publish(DomainEvent::DuplicateGroupCreated {
                                        group_id: String::new(),
                                        group_type: "exact".to_string(),
                                        confidence: 1.0,
                                        canonical_asset_id: Some(asset_id.clone()),
                                        candidate_count: 2,
                                        rule_set_id: "exact-match".to_string(),
                                    });
                                }
                            } else {
                                tracing::debug!("DuplicateWorker: auto-detect on import is disabled in settings");
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
                info!("DuplicateWorker: cleaning up fingerprint and candidates for deleted asset {}", asset_id);
                
                if let Err(error) = self.duplicates_repo.delete_fingerprint(&asset_id).await {
                    error!("DuplicateWorker: failed to delete fingerprint for {}: {}", asset_id, error);
                }

                if let Err(error) = self.duplicates_repo.remove_candidate_from_groups(&asset_id).await {
                    error!("DuplicateWorker: failed to remove candidate {} from groups: {}", asset_id, error);
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
                    Ok(loaded_image) => Some(loaded_image),
                    Err(_) => {
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

                    (
                        Some(format!("{:016x}", dhash_value)),
                        Some(spatial_block_hash),
                        Some(multiscale_hash),
                        Some(image_width as i32),
                        Some(image_height as i32),
                    )
                } else {
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

