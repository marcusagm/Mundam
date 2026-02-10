pub mod metadata;
pub mod types;
pub use types::*;
pub mod watcher;
pub mod scan;

use crate::db::Db;
use std::sync::Arc;
use tauri::AppHandle;


pub struct Indexer {
    app_handle: AppHandle,
    db: Arc<Db>,
    registry: Arc<tokio::sync::Mutex<WatcherRegistry>>,
}

impl Indexer {
    pub fn new(app_handle: AppHandle, db: &Db, registry: Arc<tokio::sync::Mutex<WatcherRegistry>>) -> Self {
        Self {
            app_handle,
            db: Arc::new(Db { pool: db.pool.clone() }),
            registry,
        }
    }

    pub async fn stop_watcher(&self, root_path: &str) {
        let path = normalize_path(root_path);
        let mut registry = self.registry.lock().await;
        if let Some(tx) = registry.watchers.remove(&path) {
            println!("DEBUG: Stopping watcher for root: {}", path);
            let _ = tx.send(());
        }
    }

    pub async fn start_scan(&self, root_path: std::path::PathBuf) {
        scan::run_scan(
            self.app_handle.clone(),
            self.db.clone(),
            self.registry.clone(),
            root_path
        ).await;
    }
}

fn normalize_path(path: &str) -> String {
    let p = path.trim_end_matches('/');
    if p.is_empty() { return "/".to_string(); }
    p.to_string()
}
