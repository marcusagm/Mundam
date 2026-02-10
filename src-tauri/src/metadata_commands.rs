use crate::error::{AppError, AppResult};
use crate::metadata_reader;
use std::collections::HashMap;
use std::path::PathBuf;

#[tauri::command]
pub async fn get_image_exif(path: String) -> AppResult<HashMap<String, String>> {
    // Check if file exists
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(AppError::NotFound(format!("File not found: {}", path)));
    }

    // Run on blocking thread since file I/O can be slow
    let res = tauri::async_runtime::spawn_blocking(move || metadata_reader::read_exif(&path_buf))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(res)
}
