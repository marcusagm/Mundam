use tauri::State;
use crate::db::Db;
use crate::error::AppResult;
use serde_json::Value;

#[tauri::command]
pub async fn get_setting(key: String, db: State<'_, std::sync::Arc<Db>>) -> AppResult<Option<Value>> {
    Ok(db.get_setting(&key).await?)
}

#[tauri::command]
pub async fn set_setting(key: String, value: Value, db: State<'_, std::sync::Arc<Db>>) -> AppResult<()> {
    Ok(db.set_setting(&key, &value).await?)
}

#[tauri::command]
pub async fn run_db_maintenance(db: State<'_, std::sync::Arc<Db>>) -> AppResult<()> {
    Ok(db.run_maintenance().await?)
}
