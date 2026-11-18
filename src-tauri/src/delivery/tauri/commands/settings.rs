use crate::core::error::AppResult;
use crate::core::settings::AppSettings;
use crate::delivery::tauri::admission::IpcRequest;
use crate::feature::settings::SettingsService;
use tauri::State;

/// Retrieves the current application settings.
///
/// # Arguments
///
/// * `settings_service` - State containing the settings service.
///
/// # Returns
///
/// Default settings for the application.
#[tauri::command]
pub async fn get_app_settings(
    _ipc: IpcRequest,
    settings_service: State<'_, SettingsService>,
) -> AppResult<AppSettings> {
    settings_service.get_settings().await
}

/// Updates the application settings.
///
/// # Arguments
///
/// * `settings` - Settings to persist.
/// * `settings_service` - State containing the settings service.
///
/// # Returns
///
/// Ok(()) if settings were persisted successfully.
#[tauri::command]
pub async fn update_app_settings(
    _ipc: IpcRequest,
    settings: AppSettings,
    settings_service: State<'_, SettingsService>,
) -> AppResult<()> {
    settings_service.update_settings(settings).await
}

/// Retrieves a specific setting by key.
#[tauri::command]
pub async fn get_setting(
    _ipc: IpcRequest,
    key: String,
    settings_service: State<'_, SettingsService>,
) -> AppResult<Option<serde_json::Value>> {
    settings_service.get_setting(&key).await
}

/// Updates a specific setting by key.
#[tauri::command]
pub async fn set_setting(
    _ipc: IpcRequest,
    key: String,
    value: serde_json::Value,
    settings_service: State<'_, SettingsService>,
) -> AppResult<()> {
    settings_service.set_setting(key, value).await
}
