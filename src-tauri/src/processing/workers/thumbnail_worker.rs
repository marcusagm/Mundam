use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};
use tauri::Manager;

use crate::core::error::{AppError, AppResult};
use crate::core::formats::FormatRegistry;
use crate::core::ledger::command::LedgerCommand;
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::repository::AssetQueryHandler;
use crate::core::workflows::thumbnails::priority::ThumbnailPriorityState;
use crate::processing::media::image_utils;

/// Background worker that orchestrates thumbnail generation.
///
/// Under Hexagonal Architecture, this is a "Processing Worker" (Infrastructure/Application layer)
/// that implements the workflow of consuming assets and producing thumbnails.
/// It uses a Hybrid Queue:
/// 1. LIFO: High-priority items from `ThumbnailPriorityState` (UI-driven).
/// 2. FIFO: Background items from the Database (Indexing-driven).
pub struct ThumbnailWorker {
    /// Registry for resolving format-specific thumbnail capabilities.
    format_registry: Arc<FormatRegistry>,
    /// Ledger for committing state changes (UpdateThumbnail).
    ledger: Arc<dyn TransactionalAssetLedger>,
    /// Query handler for fetching assets waiting for thumbnails.
    query_handler: Arc<dyn AssetQueryHandler>,
    /// Shared state for UI-requested priorities.
    priority_state: Arc<ThumbnailPriorityState>,
    /// Output directory for thumbnails.
    thumbnails_dir: PathBuf,
}

/// Implementation of the ThumbnailWorker struct.
impl ThumbnailWorker {
    /// Creates a new instance of the ThumbnailWorker.
    ///
    /// # Arguments
    ///
    /// * `format_registry` - The registry for resolving format-specific thumbnail capabilities.
    /// * `ledger` - The ledger for committing state changes (UpdateThumbnail).
    /// * `query_handler` - The query handler for fetching assets waiting for thumbnails.
    /// * `priority_state` - The shared state for UI-requested priorities.
    /// * `thumbnails_dir` - The output directory for thumbnails.
    ///
    /// # Returns
    ///
    /// A new instance of `ThumbnailWorker`.
    pub fn new(
        format_registry: Arc<FormatRegistry>,
        ledger: Arc<dyn TransactionalAssetLedger>,
        query_handler: Arc<dyn AssetQueryHandler>,
        priority_state: Arc<ThumbnailPriorityState>,
        thumbnails_dir: PathBuf,
    ) -> Self {
        Self {
            format_registry,
            ledger,
            query_handler,
            priority_state,
            thumbnails_dir,
        }
    }

    /// Starts the orchestration loop in a background task.
    ///
    /// # Arguments
    ///
    /// * `token` - The cancellation token for the background task.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` for the background task.
    pub fn start(self, token: CancellationToken, app_handle: tauri::AppHandle) -> tauri::async_runtime::JoinHandle<()> {
        tauri::async_runtime::spawn(async move {
            info!("ThumbnailWorker: Orchestrator loop started");
            let worker_arc = Arc::new(self);
            
            // Resolve maximum concurrency via user settings
            let settings_service = app_handle.state::<crate::feature::settings::SettingsService>();
            let max_concurrent = match settings_service.get_settings().await {
                Ok(settings) if settings.thumbnail_threads > 0 => settings.thumbnail_threads,
                _ => std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4)
                        .clamp(2, 8),
            };
                
            let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
            // IDs currently being processed; prevents several slots from picking the same asset.
            let in_flight: Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
                Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

            loop {
                if token.is_cancelled() {
                    info!("ThumbnailWorker: Shutdown requested");
                    break;
                }
                
                // 1. Acquire permit FIRST. 
                // This ensures we only pull from the queue when a slot is free,
                // fetching the absolutely freshest priority items from the UI.
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => break, // Semaphore closed
                };

                // 2. Poll for work (UI priorities first, skipping assets already in flight)
                let mut next_id = None;
                let mut is_priority = true;

                while let Some(candidate) = worker_arc.priority_state.pop_batch(1).into_iter().next() {
                    let already_running = in_flight
                        .lock()
                        .map(|set| set.contains(&candidate))
                        .unwrap_or(false);
                    if !already_running {
                        next_id = Some(candidate);
                        break;
                    }
                }

                if next_id.is_none() {
                    is_priority = false;
                    let running_count = in_flight.lock().map(|set| set.len()).unwrap_or(0);
                    if let Ok(ids) = worker_arc
                        .query_handler
                        .get_assets_needing_thumbnails((running_count + 1) as u32)
                        .await
                    {
                        let running = in_flight.lock().map(|set| set.clone()).unwrap_or_default();
                        next_id = ids.into_iter().find(|id| !running.contains(id));
                    }
                }

                // 3. Dispatch work or wait
                if let Some(id) = next_id {
                    let worker_clone = worker_arc.clone();
                    if let Ok(mut set) = in_flight.lock() {
                        set.insert(id.clone());
                    }
                    let in_flight_clone = in_flight.clone();
                    tokio::spawn(async move {
                        if let Err(e) = worker_clone.process_single(id.clone(), is_priority).await {
                            error!("ThumbnailWorker: Process error: {}", e);
                        }
                        if let Ok(mut set) = in_flight_clone.lock() {
                            set.remove(&id);
                        }
                        // Permit is returned when dropped here
                        drop(permit);
                    });
                } else {
                    // No work found. Drop permit instantly so it's available.
                    drop(permit);
                    tokio::select! {
                        _ = token.cancelled() => break,
                        _ = sleep(Duration::from_millis(500)) => continue,
                    }
                }
            }
        })
    }

    /// Processes a single asset and commits its result immediately.
    ///
    /// # Arguments
    /// * `id` - The ID of the asset to process.
    /// * `is_priority` - Whether this item came from the priority queue.
    #[instrument(skip_all, fields(asset_id = %id, priority = %is_priority))]
    async fn process_single(&self, id: String, is_priority: bool) -> AppResult<()> {
        let asset = match self.query_handler.get_asset_by_id(&id).await {
            Ok(a) => a,
            Err(_) => return Ok(()),
        };

        // Skip if already has thumbnail
        if asset.thumbnail_path.is_some() {
            debug!("WORKER: Skipping thumbnail for asset {} (already exists)", id);
            return Ok(());
        }

        let provider = match self.format_registry.resolve(&asset.path, &[]) {
            Some(p) => p,
            None => {
                debug!("WORKER: No provider found for asset {}. Marking as processed.", id);
                self.ledger
                    .execute(LedgerCommand::UpdateThumbnail {
                        asset_id: id.clone(),
                        thumbnail_path: "".to_string(),
                    })
                    .await?;
                return Ok(());
            }
        };

        let mut commands = Vec::new();
        let path = asset.path.clone();

        // 1. Technical Metadata Extraction
        if let Some(meta_cap) = provider.metadata() {
            if let Ok(tech_meta) = meta_cap.extract_technical(&path).await {
                let width = tech_meta.get("width").and_then(|v| v.as_i64());
                let height = tech_meta.get("height").and_then(|v| v.as_i64());
                let duration_secs = tech_meta.get("duration_secs").and_then(|v| v.as_f64());

                commands.push(LedgerCommand::UpdateTechnicalMetadata(
                    crate::core::ledger::command::UpdateTechnicalMetadataPayload {
                        asset_id: id.clone(),
                        width,
                        height,
                        duration_secs,
                        technical_payload: Some(tech_meta),
                        semantic_payload: None,
                    },
                ));
            }
        }

        // 2. Generate 3D Model Preview (GLB only — consumed by asset:// type=glb protocol)
        //
        // Image previews are NOT generated here. They are produced on-demand by the
        // asset:// protocol handler (`type=preview`) with lazy disk caching in `previews/`.
        // This avoids writing large JPG/PNG files to `thumbnails/` that would never be read.
        if let Some(preview_capability) = provider.preview() {
            let is_three_dimensional_model = provider
                .supported_formats()
                .iter()
                .any(|supported_format| {
                    let name_lower = supported_format.name.to_lowercase();
                    name_lower.contains("3d")
                        || name_lower.contains("model")
                        || name_lower.contains("gltf")
                        || name_lower.contains("obj")
                        || name_lower.contains("fbx")
                        || name_lower.contains("usd")
                        || name_lower.contains("cad")
                });

            if is_three_dimensional_model {
                if let Ok((preview_bytes, mime)) =
                    preview_capability.generate_preview(&path, &id).await
                {
                    let glb_extension = match mime.as_str() {
                        "model/gltf-binary" => Some("glb"),
                        "model/gltf+json" => Some("gltf"),
                        _ => None, // Skip non-3D preview outputs
                    };

                    if let Some(extension) = glb_extension {
                        let preview_filename = format!("{}.{}", id, extension);
                        let preview_path = self.thumbnails_dir.join(&preview_filename);
                        if let Err(write_error) = std::fs::write(&preview_path, preview_bytes) {
                            error!(
                                "ThumbnailWorker: Failed to write 3D preview for {}: {}",
                                id, write_error
                            );
                        }
                    }
                }
            }
        }

        // 3. Generate Thumbnail
        let mut final_thumb_path = "".to_string();

        if let Some(thumb_cap) = provider.thumbnail() {
            match thumb_cap.generate(&path, &id, 300).await {
                Ok(bytes) if !bytes.is_empty() => {
                    let detected_format = image_utils::detect_image_format(&bytes);
                    if detected_format.is_none() {
                        error!("ThumbnailWorker: Provider for {} returned invalid image bytes", id);
                    } else {
                        let id_for_io = id.clone();
                        let should_transcode = if detected_format == Some(image_utils::ImageFormat::Webp) {
                            if let Some((w, h)) = image_utils::get_image_dimensions(&bytes) {
                                !(270..=330).contains(&w) || !(w == h || (270..=330).contains(&h))
                            } else {
                                true
                            }
                        } else {
                            true
                        };

                        let final_bytes_result = if should_transcode {
                            tokio::task::spawn_blocking(move || {
                                let img = image::load_from_memory(&bytes).map_err(|e| {
                                    AppError::Internal(format!("Failed to load thumbnail for {}: {}", id_for_io, e))
                                })?;
                                let encoder = webp::Encoder::from_image(&img).map_err(|e| {
                                    AppError::Internal(format!("Failed to create WebP encoder: {}", e))
                                })?;
                                Ok::<Vec<u8>, AppError>(encoder.encode(75.0).to_vec())
                            })
                            .await
                        } else {
                            Ok(Ok(bytes))
                        };

                        match final_bytes_result {
                            Ok(Ok(final_bytes)) => {
                                let filename = format!("{}.webp", id);
                                let output_path = self.thumbnails_dir.join(&filename);
                                if let Err(e) = std::fs::write(&output_path, final_bytes) {
                                    error!("ThumbnailWorker: Failed to write thumbnail for {}: {}", id, e);
                                } else {
                                    final_thumb_path = filename;
                                }
                            }
                            Ok(Err(e)) => {
                                error!("ThumbnailWorker: Transcoding failed for {}: {}", id, e);
                            }
                            Err(e) => {
                                error!("ThumbnailWorker: Transcoding task joined with error for {}: {}", id, e);
                            }
                        }
                    }
                }
                Ok(_) => {
                    debug!("ThumbnailWorker: Provider for {} generated empty thumbnail (skip).", id);
                }
                Err(e) => {
                    error!("ThumbnailWorker: Generation failed for {}: {}", id, e);
                }
            }
        }

        // Add the thumbnail update command
        commands.push(LedgerCommand::UpdateThumbnail {
            asset_id: id.clone(),
            thumbnail_path: final_thumb_path,
        });

        // 4. Commit Results Individually
        if let Err(e) = self.ledger.execute(LedgerCommand::Batch(commands)).await {
            error!("ThumbnailWorker: Ledger commit failed for {}: {}", id, e);
        }

        if self.query_handler.get_by_id(&id).await?.is_none() {
            if let Some(app_data_directory) = self.thumbnails_dir.parent() {
                crate::infra::asset_cache::remove_asset_artifacts(
                    app_data_directory,
                    &std::collections::HashSet::from([id]),
                )
                .await?;
            }
        }

        Ok(())
    }
}
