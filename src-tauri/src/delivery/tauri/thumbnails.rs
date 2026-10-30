use std::sync::Arc;
use tauri::State;
use tracing::instrument;

use crate::core::error::AppResult;
use crate::core::workflows::thumbnails::priority::ThumbnailPriorityState;

/// Commands for thumbnail priority management.
///
/// Under Hexagonal Architecture, this is the "Delivery Layer" (Infrastructure/Application layer)
/// that translates UI requests into domain/workflow actions.
///
/// Prioritizes a batch of asset IDs for thumbnail generation.
/// Replaces the viewport window while preserving visible order. An empty list clears it.
#[tauri::command]
#[instrument(skip(priority_state))]
pub async fn set_thumbnail_priority(
    ids: Vec<String>,
    priority_state: State<'_, Arc<ThumbnailPriorityState>>,
) -> AppResult<()> {
    priority_state.push_priorities(ids);
    Ok(())
}

/// Reads a bounded batch of authoritative thumbnail processing outcomes.
///
/// # Errors
/// Propagates validation and repository errors from the application handler.
#[tauri::command]
pub async fn get_thumbnail_processing_status(
    asset_ids: Vec<String>,
    processing: State<'_, Arc<dyn crate::core::processing::ProcessingRepository>>,
) -> AppResult<Vec<crate::core::processing::ThumbnailProcessingStatus>> {
    crate::feature::processing::thumbnail_statuses(processing.inner().as_ref(), asset_ids).await
}

#[cfg(test)]
#[path = "thumbnail_ipc_tests.rs"]
mod tests;
