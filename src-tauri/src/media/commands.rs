use crate::error::{AppError, AppResult};
use crate::media::ffmpeg::get_audio_waveform;
use std::path::PathBuf;
use tauri::command;

#[command]
pub async fn get_audio_waveform_data(
    app: tauri::AppHandle,
    path: String,
) -> AppResult<Vec<f32>> {
    let input_path = PathBuf::from(&path);
    if !input_path.exists() {
        return Err(AppError::NotFound(format!("File not found: {}", path)));
    }

    Ok(get_audio_waveform(&app, &input_path).map_err(|e| AppError::Generic(e.to_string()))?)
}
