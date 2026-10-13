use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported application languages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppLanguage {
    English,
    Portuguese,
}

/// Default language is English.
impl Default for AppLanguage {
    fn default() -> Self {
        Self::English
    }
}

/// Core application configuration that must persist across sessions.
///
/// This structure is the source of truth for UI and Backend behavior
/// and is kept outside the main SQLite database for resiliency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Custom path for the asset database (if any).
    pub database_path: Option<PathBuf>,

    /// Maximum number of parallel worker threads for processing.
    /// 0 means auto-detect (half of available cores).
    pub worker_threads: usize,

    /// Preferred UI language.
    pub ui_language: AppLanguage,

    /// Whether to automatically scan watched directories on boot.
    pub auto_scan_enabled: bool,

    /// Maximum number of concurrent file-processing tasks during indexing.
    /// Controls the tokio::sync::Semaphore limit in the fan-out producer.
    /// 200 is optimal for NVMe SSDs; lower values (50-100) are safer for HDDs/SD.
    pub indexer_concurrency_limit: usize,

    /// Number of threads used for thumbnail generation.
    pub thumbnail_threads: usize,

    /// Number of days to retain cached transcoded files and thumbnails.
    pub cache_retention_days: u32,

    /// Whether to automatically check and group exact duplicates on asset creation/import.
    #[serde(default = "default_auto_detect_duplicates")]
    pub auto_detect_duplicates: bool,

    /// Extra generic settings (e.g., appearance, shortcuts).
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Provides fallback default value of true for auto_detect_duplicates.
fn default_auto_detect_duplicates() -> bool {
    true
}

/// Default settings for the application.
impl Default for AppSettings {
    /// Default settings for the application.
    ///
    /// # Returns
    ///
    /// Default settings for the application.
    fn default() -> Self {
        let available_parallelism = std::thread::available_parallelism()
            .map(|parallelism_count| parallelism_count.get())
            .unwrap_or(1);

        Self {
            database_path: None,
            worker_threads: std::cmp::max(1, available_parallelism / 2),
            ui_language: AppLanguage::default(),
            auto_scan_enabled: true,
            indexer_concurrency_limit: 200,
            thumbnail_threads: std::cmp::max(1, available_parallelism / 2),
            cache_retention_days: 30,
            auto_detect_duplicates: true,
            extra: std::collections::HashMap::new(),
        }
    }
}
