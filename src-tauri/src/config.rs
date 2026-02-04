use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use crate::database::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub thumbnail_threads: usize,
    pub indexer_batch_size: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            thumbnail_threads: 2, // Default conservative
            indexer_batch_size: 6,
        }
    }
}

pub struct ConfigState(pub Mutex<AppConfig>);

pub async fn load_config(db: &Db) -> AppConfig {
    let mut config = AppConfig::default();

    // Load overrides from DB
    if let Ok(Some(val)) = db.get_setting("thumbnail_threads").await {
        if let Some(v) = val.as_u64() {
             config.thumbnail_threads = v as usize;
        }
    }
    
    // Auto-detect CPU counts if set to 0 or unconfigured (logic can be enhanced later)
    // For now we trust the DB or default.
    // Ideally update default if we want to be smarter:
    // if config.thumbnail_threads == 0 { config.thumbnail_threads = num_cpus::get() / 2; }

    config
}
