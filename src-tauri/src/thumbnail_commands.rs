use crate::db::Db;
use std::sync::Arc;
use tauri::State;

/// Request regeneration of a thumbnail by clearing its path in the database.
/// The thumbnail worker will automatically pick it up and regenerate.
#[tauri::command]
pub async fn request_thumbnail_regenerate(
    image_id: i64,
    db: State<'_, Arc<Db>>,
) -> Result<(), String> {
    // println!("DEBUG: Thumbnail regeneration requested for ID: {}", image_id);

    db.clear_thumbnail_path(image_id)
        .await
        .map_err(|e| format!("Failed to clear thumbnail path: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_thumbnail_priority(
    ids: Vec<i64>,
    state: State<'_, Arc<crate::thumbnail_priority::ThumbnailPriorityState>>,
) -> Result<(), String> {
    state.set_priority(ids);
    Ok(())
}
