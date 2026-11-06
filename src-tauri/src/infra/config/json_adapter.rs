use crate::core::error::{AppError, AppResult};
use crate::core::settings::{AppSettings, SettingsRepository};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

/// Infrastructure adapter that persists settings in a JSON file.
pub struct JsonSettingsAdapter {
    file_path: PathBuf,
}

/// Implementation of SettingsRepository for JsonSettingsAdapter.
impl JsonSettingsAdapter {
    /// Creates a new adapter pointing to a specific file path.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the JSON file.
    ///
    /// # Returns
    ///
    /// New JsonSettingsAdapter instance.
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

/// Implementation of SettingsRepository for JsonSettingsAdapter.
#[async_trait]
impl SettingsRepository for JsonSettingsAdapter {
    /// Loads settings from the persistent storage.
    /// If storage doesn't exist, returns default settings.
    ///
    /// # Returns
    ///
    /// Default settings for the application.
    async fn load(&self) -> AppResult<AppSettings> {
        if !self.file_path.exists() {
            return Ok(AppSettings::default());
        }

        let content = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| AppError::Generic(format!("Failed to read settings file: {}", e)))?;

        let settings: AppSettings = serde_json::from_str(&content)
            .map_err(|e| AppError::Generic(format!("Failed to parse settings JSON: {}", e)))?;

        Ok(settings)
    }

    /// Persists settings to the storage.
    ///
    /// # Arguments
    ///
    /// * `settings` - Settings to persist.
    ///
    /// # Returns
    ///
    /// Ok(()) if settings were persisted successfully.
    async fn save(&self, settings: &AppSettings) -> AppResult<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::Generic(format!("Failed to create settings directory: {}", e))
            })?;
        }

        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| AppError::Generic(format!("Failed to serialize settings: {}", e)))?;

        fs::write(&self.file_path, content)
            .await
            .map_err(|e| AppError::Generic(format!("Failed to write settings file: {}", e)))?;

        Ok(())
    }
}

/// Tests for JsonSettingsAdapter.
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Tests the persistence of settings in a JSON file.
    ///
    /// # Returns
    ///
    /// Ok(()) if settings were persisted successfully.
    #[tokio::test]
    async fn test_json_settings_persistence() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("settings.json");
        let adapter = JsonSettingsAdapter::new(file_path);

        // Should return default if file doesn't exist
        let mut settings = adapter.load().await?;
        assert!(settings.worker_threads > 0);

        // Modify and save
        settings.worker_threads = 42;
        adapter.save(&settings).await?;

        // Reload and verify
        let reloaded = adapter.load().await?;
        assert_eq!(reloaded.worker_threads, 42);

        Ok(())
    }
}
