use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};

use crate::core::error::{AppError, AppResult};
use crate::core::formats::FormatRegistry;
use crate::core::ledger::command::LedgerCommand;
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::processing::{
    ProcessingKind, ProcessingOutcome, ProcessingRepository, ProcessingTicket,
};
use crate::core::repository::AssetQueryHandler;
use crate::core::workflows::thumbnails::priority::ThumbnailPriorityState;
use crate::processing::media::image_utils;

/// Background worker that orchestrates thumbnail generation.
///
/// Under Hexagonal Architecture, this is a "Processing Worker" (Infrastructure/Application layer)
/// that implements the workflow of consuming assets and producing thumbnails.
/// It uses a Hybrid Queue:
/// 1. Ordered priorities from the latest viewport window (UI-driven).
/// 2. Recoverable pending work from the processing repository.
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
    processing: Arc<dyn ProcessingRepository>,
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
        processing: Arc<dyn ProcessingRepository>,
    ) -> Self {
        Self {
            format_registry,
            ledger,
            query_handler,
            priority_state,
            thumbnails_dir,
            processing,
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
    pub fn start(
        self,
        token: CancellationToken,
        app_handle: tauri::AppHandle,
    ) -> tauri::async_runtime::JoinHandle<()> {
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
            let mut tasks = tokio::task::JoinSet::new();

            loop {
                while let Some(result) = tasks.try_join_next() {
                    if let Err(error) = result {
                        error!(%error, "Thumbnail task failed");
                    }
                }
                if token.is_cancelled() {
                    info!("ThumbnailWorker: Shutdown requested");
                    break;
                }

                // 1. Acquire permit FIRST.
                // This ensures we only pull from the queue when a slot is free,
                // fetching the absolutely freshest priority items from the UI.
                let permit = tokio::select! {
                    _ = token.cancelled() => break,
                    result = semaphore.clone().acquire_owned() => match result {
                        Ok(permit) => permit,
                        Err(_) => break,
                    }
                };

                // 2. Poll for work (UI priorities first, skipping assets already in flight)
                let mut next_id = None;
                let mut is_priority = true;

                while let Some(candidate) =
                    worker_arc.priority_state.pop_batch(1).into_iter().next()
                {
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
                    let reservation = ThumbnailReservation {
                        asset_id: id.clone(),
                        in_flight: in_flight_clone,
                    };
                    tasks.spawn(async move {
                        let _reservation = reservation;
                        if let Err(e) = worker_clone.process_single(id.clone(), is_priority).await {
                            error!("ThumbnailWorker: Process error: {}", e);
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
            while let Some(result) = tasks.join_next().await {
                if let Err(error) = result {
                    error!(%error, "Thumbnail task failed while draining");
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
        let Some(ticket) = self
            .processing
            .begin(ProcessingKind::Thumbnail, &id)
            .await?
        else {
            return Ok(());
        };
        let result = self.generate_single(&ticket).await;
        if self.query_handler.get_by_id(&id).await?.is_none() {
            if let Some(app_data_directory) = self.thumbnails_dir.parent() {
                crate::infra::asset_cache::remove_asset_artifacts(
                    app_data_directory,
                    &std::collections::HashSet::from([id]),
                )
                .await?;
            }
        }
        match result {
            Ok(outcome) => self.processing.finish(&ticket, outcome).await,
            Err(error) => {
                self.processing
                    .finish(&ticket, ProcessingOutcome::Failed(error.to_string()))
                    .await?;
                Err(error)
            }
        }
    }

    /// Produces output only for an admitted attempt; failures remain retryable work.
    async fn generate_single(&self, ticket: &ProcessingTicket) -> AppResult<ProcessingOutcome> {
        let id = ticket.asset_id.clone();
        let asset = match self.query_handler.get_asset_by_id(&id).await {
            Ok(a) => a,
            Err(error) => return Err(error),
        };
        if !self.processing.is_current(ticket).await? {
            return Ok(ProcessingOutcome::Skipped(
                "Attempt is no longer current".into(),
            ));
        }

        let provider = match self.format_registry.resolve(&asset.path, &[]) {
            Some(p) => p,
            None => {
                return Ok(ProcessingOutcome::Skipped(
                    "No format provider is available".into(),
                ));
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
            let is_three_dimensional_model =
                provider.supported_formats().iter().any(|supported_format| {
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
                        if let Err(write_error) =
                            tokio::fs::write(&preview_path, preview_bytes).await
                        {
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
        let Some(thumbnail_capability) = provider.thumbnail() else {
            if !commands.is_empty() {
                self.ledger.execute(LedgerCommand::Batch(commands)).await?;
            }
            return Ok(ProcessingOutcome::Skipped(
                "Provider has no thumbnail capability".into(),
            ));
        };
        let bytes = thumbnail_capability.generate(&path, &id, 300).await?;
        let final_bytes = tokio::task::spawn_blocking(move || prepare_thumbnail(bytes))
            .await
            .map_err(|error| {
                AppError::Internal(format!("Thumbnail encoding task failed: {error}"))
            })??;
        if !self.processing.is_current(ticket).await? {
            return Ok(ProcessingOutcome::Skipped("Attempt was superseded".into()));
        }
        let filename = format!("{}.webp", id);
        tokio::fs::write(self.thumbnails_dir.join(&filename), final_bytes).await?;
        commands.push(LedgerCommand::UpdateThumbnail {
            asset_id: id,
            thumbnail_path: filename,
        });
        if self.processing.is_current(ticket).await? {
            self.ledger.execute(LedgerCommand::Batch(commands)).await?;
            Ok(ProcessingOutcome::Completed)
        } else {
            Ok(ProcessingOutcome::Skipped(
                "Attempt was superseded before commit".into(),
            ))
        }
    }
}

/// Releases admission state even if a child future panics or is aborted during shutdown.
struct ThumbnailReservation {
    asset_id: String,
    in_flight: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl Drop for ThumbnailReservation {
    fn drop(&mut self) {
        self.in_flight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.asset_id);
    }
}

fn prepare_thumbnail(bytes: Vec<u8>) -> AppResult<Vec<u8>> {
    if bytes.is_empty() || !image_utils::is_valid_image(&bytes) {
        return Err(AppError::Internal(
            "Thumbnail provider returned empty or invalid image data".into(),
        ));
    }
    if image_utils::detect_image_format(&bytes) == Some(image_utils::ImageFormat::Webp) {
        if let Some((width, height)) = image_utils::get_image_dimensions(&bytes) {
            if width > 0 && height > 0 && width.max(height) <= 330 {
                return Ok(bytes);
            }
        }
    }
    let image = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Internal(format!("Failed to decode thumbnail: {error}")))?;
    let image = image.thumbnail(300, 300);
    let encoder = webp::Encoder::from_image(&image)
        .map_err(|error| AppError::Internal(format!("Failed to encode thumbnail: {error}")))?;
    Ok(encoder.encode(75.0).to_vec())
}

#[cfg(test)]
#[path = "thumbnail_worker_tests.rs"]
mod tests;
