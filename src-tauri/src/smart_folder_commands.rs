use crate::database::Db;
use crate::db_smart_folders::SmartFolder;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_smart_folders(db: State<'_, Arc<Db>>) -> Result<Vec<SmartFolder>, String> {
    db.get_smart_folders().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_smart_folder(
    db: State<'_, Arc<Db>>,
    name: String,
    query: String,
) -> Result<i64, String> {
    db.save_smart_folder(&name, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_smart_folder(
    db: State<'_, Arc<Db>>,
    id: i64,
    name: String,
    query: String,
) -> Result<(), String> {
    db.update_smart_folder(id, &name, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_smart_folder(db: State<'_, Arc<Db>>, id: i64) -> Result<(), String> {
    db.delete_smart_folder(id).await.map_err(|e| e.to_string())
}
