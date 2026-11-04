use crate::core::error::AppResult;
use crate::core::settings::{AppSettings, SettingsRepository};
use std::sync::Arc;

/// Application service for managing settings.
/// Bridges the UI requirements with the domain and infrastructure.
#[derive(Clone)]
pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
    control: Option<Arc<crate::core::processing_control::ProcessingControl>>,
    mutations: Arc<tokio::sync::Mutex<()>>,
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
        Self {
            repository,
            control: None,
            mutations: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Applies successful settings writes to the common admission policy.
    pub fn with_control(
        mut self,
        control: Arc<crate::core::processing_control::ProcessingControl>,
    ) -> Self {
        self.control = Some(control);
        self
    }

    async fn persist(&self, settings: &AppSettings) -> AppResult<()> {
        if settings.processing_concurrency > 8 {
            return Err(crate::core::AppError::ValidationFailed(
                "Invalid processing concurrency".into(),
            ));
        }
        self.repository.save(settings).await?;
        if let Some(control) = &self.control {
            control
                .budget
                .configure(settings.processing_concurrency, settings.thumbnail_threads);
        }
        Ok(())
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
        let _mutation = self.mutations.lock().await;
        self.persist(&settings).await
    }

    /// Gets a specific setting by key from the extra settings map.
    pub async fn get_setting(&self, key: &str) -> AppResult<Option<serde_json::Value>> {
        let settings = self.repository.load().await?;
        match key {
            "processing_concurrency" => {
                Ok(Some(serde_json::json!(settings.processing_concurrency)))
            }
            "thumbnail_threads" => Ok(Some(serde_json::json!(settings.thumbnail_threads))),
            "cache_retention_days" => Ok(Some(serde_json::json!(settings.cache_retention_days))),
            "indexer_concurrency_limit" => {
                Ok(Some(serde_json::json!(settings.indexer_concurrency_limit)))
            }
            "auto_detect_duplicates" => {
                Ok(Some(serde_json::json!(settings.auto_detect_duplicates)))
            }
            _ => Ok(settings.extra.get(key).cloned()),
        }
    }

    /// Sets a specific setting by key in the extra settings map.
    pub async fn set_setting(&self, key: String, value: serde_json::Value) -> AppResult<()> {
        let _mutation = self.mutations.lock().await;
        let mut settings = self.repository.load().await?;
        match key.as_str() {
            "processing_concurrency" => {
                let parsed = value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|raw| raw.parse().ok()))
                    .filter(|count| *count <= 8)
                    .ok_or_else(|| {
                        crate::core::AppError::ValidationFailed(
                            "Processing concurrency must be between 0 and 8".into(),
                        )
                    })?;
                settings.processing_concurrency = parsed as usize;
            }
            "thumbnail_threads" => {
                if let Some(parsed_threads_count) = value.as_u64().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw_string| raw_string.parse().ok())
                }) {
                    settings.thumbnail_threads = parsed_threads_count as usize;
                }
            }
            "cache_retention_days" => {
                if let Some(parsed_days_count) = value.as_u64().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw_string| raw_string.parse().ok())
                }) {
                    settings.cache_retention_days = parsed_days_count as u32;
                }
            }
            "indexer_concurrency_limit" => {
                if let Some(parsed_limit) = value.as_u64().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw_string| raw_string.parse().ok())
                }) {
                    settings.indexer_concurrency_limit = parsed_limit as usize;
                }
            }
            "auto_detect_duplicates" => {
                if let Some(parsed_boolean_flag) = value.as_bool().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw_string| raw_string.parse().ok())
                }) {
                    settings.auto_detect_duplicates = parsed_boolean_flag;
                }
            }
            _ => {
                settings.extra.insert(key, value);
            }
        }
        self.persist(&settings).await
    }
}
