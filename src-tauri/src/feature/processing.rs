//! Processing read handlers expose bounded snapshots independently of Tauri delivery.

use crate::core::error::{AppError, AppResult};
use crate::core::processing::{ProcessingRepository, ThumbnailProcessingStatus};
use std::collections::HashSet;

/// Returns thumbnail outcomes for visible assets without a per-component RPC stream.
///
/// # Errors
/// Returns a validation error for batches above 200 unique IDs, or a repository error.
pub async fn thumbnail_statuses(
    repository: &dyn ProcessingRepository,
    asset_ids: Vec<String>,
) -> AppResult<Vec<ThumbnailProcessingStatus>> {
    let identifiers: HashSet<String> = asset_ids.into_iter().collect();
    if identifiers.len() > 200 {
        return Err(AppError::ValidationFailed(
            "Thumbnail status batches are limited to 200 assets".into(),
        ));
    }
    repository
        .thumbnail_statuses(&identifiers.into_iter().collect::<Vec<_>>())
        .await
}
