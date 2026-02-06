use std::path::PathBuf;
use tauri::command;
use crate::ffmpeg::get_audio_waveform;

#[command]
pub async fn get_audio_waveform_data(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<f32>, String> {
    let input_path = PathBuf::from(path);
    if !input_path.exists() {
        return Err("File not found".to_string());
    }

    get_audio_waveform(&app, &input_path)
        .map_err(|e| e.to_string())
}
