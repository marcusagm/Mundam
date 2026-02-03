use tauri::State;
use crate::database::Db;
use serde_json::Value;

#[tauri::command]
pub async fn get_setting(key: String, db: State<'_, std::sync::Arc<Db>>) -> Result<Option<Value>, String> {
    db.get_setting(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(key: String, value: Value, db: State<'_, std::sync::Arc<Db>>) -> Result<(), String> {
    db.set_setting(&key, &value).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_db_maintenance(db: State<'_, std::sync::Arc<Db>>) -> Result<(), String> {
    db.run_maintenance().await.map_err(|e| e.to_string())
}
