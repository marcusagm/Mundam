use crate::db::Db;
use crate::error::AppResult;
use std::sync::Arc;
use tauri::State;

/// Request regeneration of a thumbnail by clearing its path in the database.
/// The thumbnail worker will automatically pick it up and regenerate.
#[tauri::command]
pub async fn request_thumbnail_regenerate(
    image_id: i64,
    db: State<'_, Arc<Db>>,
) -> AppResult<()> {
    Ok(db.clear_thumbnail_path(image_id).await?)
}

#[tauri::command]
pub async fn set_thumbnail_priority(
    ids: Vec<i64>,
    state: State<'_, Arc<crate::thumbnails::priority::ThumbnailPriorityState>>,
) -> AppResult<()> {
    state.set_priority(ids);
    Ok(())
}
