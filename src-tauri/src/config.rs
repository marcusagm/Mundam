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
            thumbnail_threads: 0, // 0 = Auto-detect
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
    
    // Auto-detect if set to 0
    if config.thumbnail_threads == 0 {
         let available = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
         // Use half the threads for background work, minimum 1
         config.thumbnail_threads = std::cmp::max(1, available / 2);
         println!("INFO: Auto-detected {} threads. Using {} for background tasks.", available, config.thumbnail_threads);
    }

    config
}
