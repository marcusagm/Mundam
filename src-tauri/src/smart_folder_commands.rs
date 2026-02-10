use crate::db::Db;
use crate::db::models::SmartFolder;
use crate::error::AppResult;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_smart_folders(db: State<'_, Arc<Db>>) -> AppResult<Vec<SmartFolder>> {
    Ok(db.get_smart_folders().await?)
}

#[tauri::command]
pub async fn save_smart_folder(
    db: State<'_, Arc<Db>>,
    name: String,
    query: String,
) -> AppResult<i64> {
    Ok(db.save_smart_folder(&name, &query).await?)
}

#[tauri::command]
pub async fn update_smart_folder(
    db: State<'_, Arc<Db>>,
    id: i64,
    name: String,
    query: String,
) -> AppResult<()> {
    Ok(db.update_smart_folder(id, &name, &query).await?)
}

#[tauri::command]
pub async fn delete_smart_folder(db: State<'_, Arc<Db>>, id: i64) -> AppResult<()> {
    Ok(db.delete_smart_folder(id).await?)
}
