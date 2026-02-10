use crate::formats;

#[tauri::command]
pub fn get_library_supported_formats() -> Vec<formats::FileFormat> {
    formats::SUPPORTED_FORMATS.to_vec()
}
