use crate::core::error::AppResult;
use crate::core::settings::{AppSettings, SettingsRepository};
use std::sync::Arc;

/// Application service for managing settings.
/// Bridges the UI requirements with the domain and infrastructure.
#[derive(Clone)]
pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
}

/// Implementation of SettingsService.
impl SettingsService {
    /// Creates a new settings service.
    ///
    /// # Arguments
    ///
    /// * `repository` - Repository for managing settings.
    ///
    /// # Returns
    ///
    /// New SettingsService instance.
    pub fn new(repository: Arc<dyn SettingsRepository>) -> Self {
        Self { repository }
    }

    /// Loads the current application settings.
    ///
    /// # Returns
    ///
    /// Default settings for the application.
    pub async fn get_settings(&self) -> AppResult<AppSettings> {
        self.repository.load().await
    }

    /// Updates and persists application settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - Settings to persist.
    ///
    /// # Returns
    ///
    /// Ok(()) if settings were persisted successfully.
    pub async fn update_settings(&self, settings: AppSettings) -> AppResult<()> {
        self.repository.save(&settings).await
    }

    /// Gets a specific setting by key from the extra settings map.
    pub async fn get_setting(&self, key: &str) -> AppResult<Option<serde_json::Value>> {
        let settings = self.repository.load().await?;
        match key {
            "thumbnail_threads" => Ok(Some(serde_json::json!(settings.thumbnail_threads))),
            "cache_retention_days" => Ok(Some(serde_json::json!(settings.cache_retention_days))),
            "indexer_concurrency_limit" => Ok(Some(serde_json::json!(settings.indexer_concurrency_limit))),
            "auto_detect_duplicates" => Ok(Some(serde_json::json!(settings.auto_detect_duplicates))),
            _ => Ok(settings.extra.get(key).cloned()),
        }
    }

    /// Sets a specific setting by key in the extra settings map.
    pub async fn set_setting(&self, key: String, value: serde_json::Value) -> AppResult<()> {
        let mut settings = self.repository.load().await?;
        match key.as_str() {
            "thumbnail_threads" => {
                if let Some(parsed_threads_count) = value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|raw_string| raw_string.parse().ok()))
                {
                    settings.thumbnail_threads = parsed_threads_count as usize;
                }
            }
            "cache_retention_days" => {
                if let Some(parsed_days_count) = value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|raw_string| raw_string.parse().ok()))
                {
                    settings.cache_retention_days = parsed_days_count as u32;
                }
            }
            "indexer_concurrency_limit" => {
                if let Some(parsed_limit) = value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|raw_string| raw_string.parse().ok()))
                {
                    settings.indexer_concurrency_limit = parsed_limit as usize;
                }
            }
            "auto_detect_duplicates" => {
                if let Some(parsed_boolean_flag) = value
                    .as_bool()
                    .or_else(|| value.as_str().and_then(|raw_string| raw_string.parse().ok()))
                {
                    settings.auto_detect_duplicates = parsed_boolean_flag;
                }
            }
            _ => {
                settings.extra.insert(key, value);
            }
        }
        self.repository.save(&settings).await
    }
}
