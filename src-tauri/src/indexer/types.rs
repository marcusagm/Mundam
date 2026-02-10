use serde::Serialize;
use crate::db::models::ImageMetadata;
use std::collections::HashMap;

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct BatchChangePayload {
    pub added: Vec<AddedItemContext>,
    pub removed: Vec<RemovedItemContext>,
    pub updated: Vec<AddedItemContext>,
    pub needs_refresh: bool,
}

#[derive(Clone, Serialize, Debug)]
pub struct AddedItemContext {
    #[serde(flatten)]
    pub metadata: ImageMetadata,
    pub folder_id: i64,
    pub old_folder_id: Option<i64>,
}

#[derive(Clone, Serialize, Debug)]
pub struct RemovedItemContext {
    pub id: i64,
    pub folder_id: i64,
    pub tag_ids: Vec<i64>,
}

/// Struct to hold image path with its parent directory path
pub struct IndexedImage {
    pub metadata: ImageMetadata,
    pub parent_dir: String,
}

#[derive(Default)]
pub struct WatcherRegistry {
    pub watchers: HashMap<String, tokio::sync::oneshot::Sender<()>>,
}
