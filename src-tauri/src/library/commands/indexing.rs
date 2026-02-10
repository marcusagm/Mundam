use crate::db::Db;
use crate::error::AppResult;
use crate::indexer::Indexer;
use std::path::PathBuf;
use tauri::Manager;

/// Start indexing a directory.
///
/// # Errors
/// Returns error if database or watcher registry is not initialized.
#[tauri::command]
pub async fn start_indexing(path: String, app: tauri::AppHandle) -> AppResult<()> {
    println!("COMMAND: start_indexing called with path: {}", path);

    // Get DB from state with safety
    let db = app.try_state::<std::sync::Arc<Db>>()
        .ok_or_else(|| crate::error::AppError::Internal("Database not initialized".to_string()))?;

    let registry = app.try_state::<std::sync::Arc<tokio::sync::Mutex<crate::indexer::WatcherRegistry>>>()
        .ok_or_else(|| crate::error::AppError::Internal("Registry not initialized".to_string()))?;

    let indexer = Indexer::new(app.clone(), db.inner(), registry.inner().clone());

    let root = PathBuf::from(path);
    indexer.start_scan(root).await;
    Ok(())
}
