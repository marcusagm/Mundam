pub mod metadata;

use self::metadata::{get_image_metadata, ImageMetadata};
use crate::database::Db;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use walkdir::WalkDir;

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
}

use notify::event::{ModifyKind, RenameMode};

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
struct IndexedImage {
    metadata: ImageMetadata,
    parent_dir: String, 
}

#[derive(Default)]
pub struct WatcherRegistry {
    pub watchers: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    // pub pending_removals: std::collections::HashSet<String>,
}

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

    pub async fn start_scan(&self, root_path: PathBuf) {
        // Normalize root path (absolute and resolve symlinks)
        let root_path = root_path.canonicalize().unwrap_or(root_path);
        let root_str = normalize_path(&root_path.to_string_lossy());
        
        println!("DEBUG: Indexer::start_scan for {}", root_str);
        let app = self.app_handle.clone();
        let db = self.db.clone();
        let root_for_watcher = root_path.clone();

        // 1. Initial Quick Scan - Collect files and folders
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        let mut unique_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

        for entry in WalkDir::new(&root_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if entry.file_type().is_dir() {
                unique_dirs.insert(normalize_path(&path.to_string_lossy()));
            } else if entry.file_type().is_file() && is_image_file(path) {
                let parent = path.parent()
                    .map(|p| normalize_path(&p.to_string_lossy()))
                    .unwrap_or_default();
                 unique_dirs.insert(parent.clone());
                 files.push((path.to_path_buf(), parent));
            }
        }

        let total_files = files.len();
        println!("DEBUG: Indexer found {} images and {} folders", total_files, unique_dirs.len());

        // Ensure root is in the set
        unique_dirs.insert(root_str.clone());

        // 2. Ensure Hierarchy Exists
        let folder_map = match self.ensure_folder_hierarchy(unique_dirs, &root_str).await {
            Ok(map) => map,
            Err(e) => {
                eprintln!("Failed to ensure folder hierarchy: {}", e);
                HashMap::new()
            }
        };

        // 3. Prune Orphaned Folders
        if !folder_map.is_empty() {
             let db_folders = match db.get_folders_under_root(&root_str).await {
                 Ok(list) => list,
                 Err(_) => Vec::new()
             };
             
             let valid_paths: std::collections::HashSet<String> = folder_map.keys().cloned().collect();
             
             for (id, path) in db_folders {
                 let normalized_db_path = normalize_path(&path);
                 if !valid_paths.contains(&normalized_db_path) {
                     println!("DEBUG: Pruning orphaned folder: {}", normalized_db_path);
                     let _ = db.delete_folder(id).await;
                 }
             }
        } 

        if total_files > 0 {
            let chunk_size = (total_files / 100).clamp(1, 200);
            let (tx, mut rx) = mpsc::channel::<IndexedImage>(100);

            // 4. Spawn Worker to save images
            let app_worker = app.clone();
            let db_worker = db.clone();
            let folder_map_worker = folder_map.clone();
            
            tokio::spawn(async move {
                let mut processed: usize = 0;
                let mut batch: Vec<(i64, ImageMetadata)> = Vec::new();

                while let Some(indexed) = rx.recv().await {
                    processed += 1;
                    
                    if let Some(&folder_id) = folder_map_worker.get(&indexed.parent_dir) {
                        batch.push((folder_id, indexed.metadata.clone()));
                    }

                    if processed % chunk_size == 0 || processed == total_files {
                        let _ = app_worker.emit(
                            "indexer:progress",
                            ProgressPayload {
                                total: total_files,
                                processed,
                                current_file: indexed.metadata.filename.clone(),
                            },
                        );

                        for (fid, img) in batch.drain(..) {
                            if let Err(e) = db_worker.save_image(fid, &img).await {
                                eprintln!("Failed to save image: {}", e);
                            }
                        }
                    }
                }

                let _ = app_worker.emit("indexer:complete", total_files);
            });

            // 5. Producer - Distribute work
            for (path, parent_dir) in files {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Some(meta) = get_image_metadata(&path) {
                        let _ = tx_clone.send(IndexedImage {
                            metadata: meta,
                            parent_dir,
                        }).await;
                    }
                });
            }
        } else {
            let _ = app.emit("indexer:complete", 0);
        }

        // 6. Start File Watcher
        self.start_watcher(root_for_watcher, root_str);
    }
    
    async fn ensure_folder_hierarchy(
        &self,
        folders: std::collections::HashSet<String>,
        root_path: &str,
    ) -> Result<HashMap<String, i64>, String> {
        let mut path_to_id: HashMap<String, i64> = HashMap::new();
        let mut sorted_dirs: Vec<String> = folders.into_iter().collect();
        sorted_dirs.sort_by_key(|a| a.len());
        
        for dir_path in sorted_dirs {
            let dir_path = normalize_path(&dir_path);
            let path_buf = PathBuf::from(&dir_path);
            let name = path_buf.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let parent_path_str = path_buf.parent().map(|p| normalize_path(&p.to_string_lossy()));
            
            let mut parent_id = None;
            if let Some(pp) = parent_path_str {
                if let Some(id) = path_to_id.get(&pp) {
                    parent_id = Some(*id);
                } else if let Ok(Some(id)) = self.db.get_folder_by_path(&pp).await {
                    parent_id = Some(id);
                }
            }
            
            let is_root = dir_path == root_path;
            match self.db.upsert_folder(&dir_path, &name, parent_id, is_root).await {
                Ok(id) => { path_to_id.insert(dir_path, id); }
                Err(e) => eprintln!("Failed to upsert folder '{}': {}", dir_path, e),
            }
        }
        Ok(path_to_id)
    }

    fn start_watcher(&self, path: PathBuf, root_str: String) {
        use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

        let app = self.app_handle.clone();
        let db = self.db.clone();
        let watch_path = path.canonicalize().unwrap_or(path);
        let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from(""));
        let root_str_clone = root_str.clone();
        let registry = self.registry.clone();
        
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel::<Event>(100);
            let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
            
            // Register stop handle
            {
                let mut reg = registry.lock().await;
                // If there's already a watcher for this path, stop it first
                if let Some(old_tx) = reg.watchers.insert(root_str_clone.clone(), stop_tx) {
                    let _ = old_tx.send(());
                }
            }

            let debouncer_window = Duration::from_millis(600);
            
            let mut watcher = RecommendedWatcher::new(
                move |res: notify::Result<Event>| {
                    if let Ok(event) = res {
                         let _ = tx.blocking_send(event);
                    }
                },
                Config::default(),
            ).expect("Failed to create watcher");

            watcher.watch(&watch_path, RecursiveMode::Recursive).expect("Failed to watch path");
            let _watcher_ref = watcher; // Keep alive

            let mut buffer_added: HashMap<String, ImageMetadata> = HashMap::new();
            let mut buffer_added_folders: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut buffer_removed: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut buffer_renamed: HashMap<String, String> = HashMap::new();
            let mut pending_renames: HashMap<usize, String> = HashMap::new();
            let mut refresh_needed = false;
            
            let mut timer = tokio::time::interval(debouncer_window);
            
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        println!("DEBUG: Watcher task received STOP for {}", root_str_clone);
                        break;
                    }
                    Some(event) = rx.recv() => {
                        if event.paths.iter().any(|p| p.starts_with(&app_data_dir)) { continue; }
                        println!("DEBUG: Watcher RAW - {:?}", event);

                        match event.kind {
                            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                if event.paths.len() == 2 {
                                    let from = normalize_path(&event.paths[0].to_string_lossy());
                                    let to = normalize_path(&event.paths[1].to_string_lossy());
                                    
                                    if buffer_added_folders.remove(&from) {
                                        buffer_added_folders.insert(to);
                                    } else if let Some(meta) = buffer_added.remove(&from) {
                                        buffer_added.insert(to, meta);
                                    } else {
                                        buffer_renamed.insert(from, to);
                                    }
                                }
                            },
                            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                if !event.paths.is_empty() {
                                    let path_str = normalize_path(&event.paths[0].to_string_lossy());
                                    if let Some(tracker) = event.attrs.tracker() {
                                        pending_renames.insert(tracker, path_str);
                                    } else {
                                        buffer_removed.insert(path_str);
                                    }
                                }
                            },
                            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                                if !event.paths.is_empty() {
                                    let path_str = normalize_path(&event.paths[0].to_string_lossy());
                                    let matched_from = if let Some(tracker) = event.attrs.tracker() {
                                        pending_renames.remove(&tracker)
                                    } else {
                                        None
                                    };

                                    if let Some(from) = matched_from {
                                        if buffer_added_folders.remove(&from) {
                                            buffer_added_folders.insert(path_str.clone());
                                        } else if let Some(meta) = buffer_added.remove(&from) {
                                            buffer_added.insert(path_str.clone(), meta);
                                        } else {
                                            buffer_renamed.insert(from, path_str.clone());
                                        }
                                    } else {
                                        if path_str != root_str_clone {
                                            if event.paths[0].is_dir() {
                                                buffer_added_folders.insert(path_str);
                                            } else if is_image_file(&event.paths[0]) {
                                                if let Some(meta) = get_image_metadata(&event.paths[0]) {
                                                    buffer_added.insert(path_str, meta);
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            _ => {
                                for path in event.paths {
                                    let path_str = normalize_path(&path.to_string_lossy());
                                    if path.exists() {
                                        if path_str != root_str_clone {
                                            if path.is_dir() {
                                                buffer_removed.remove(&path_str);
                                                buffer_added_folders.insert(path_str);
                                            } else if is_image_file(&path) {
                                                buffer_removed.remove(&path_str);
                                                if let Some(meta) = get_image_metadata(&path) {
                                                    buffer_added.insert(path_str, meta);
                                                }
                                            }
                                        }
                                    } else {
                                        buffer_added.remove(&path_str);
                                        buffer_added_folders.remove(&path_str);
                                        buffer_removed.insert(path_str);
                                    }
                                }
                            }
                        }
                    }
                    _ = timer.tick() => {
                        for (_, path) in pending_renames.drain() {
                            buffer_removed.insert(path);
                        }

                        // Heuristics for non-tracked renames
                        let removed_list: Vec<String> = buffer_removed.iter().cloned().collect();
                        for from_path in removed_list {
                            if !buffer_removed.contains(&from_path) { continue; }
                            
                            let from_buf = Path::new(&from_path);
                            // Folder Heuristic: Share parent
                            let folder_match = buffer_added_folders.iter().find(|to_path| {
                                Path::new(*to_path).parent() == from_buf.parent()
                            }).cloned();

                            if let Some(to_path) = folder_match {
                                println!("DEBUG: Watcher - Pairing split FOLDER RENAME: {} -> {}", from_path, to_path);
                                buffer_renamed.insert(from_path.clone(), to_path.clone());
                                buffer_removed.remove(&from_path);
                                buffer_added_folders.remove(&to_path);
                                continue;
                            }

                            // Image Heuristic: Metadata match
                            if is_image_file(from_buf) {
                                if let Ok(Some((size, created))) = db.get_file_comparison_data(&from_path).await {
                                    let image_match = buffer_added.iter().find(|(_, m)| {
                                        m.size == size && m.created_at == created
                                    }).map(|(t, _)| t.clone());

                                    if let Some(to_path) = image_match {
                                        println!("DEBUG: Watcher - Pairing split IMAGE RENAME: {} -> {}", from_path, to_path);
                                        buffer_renamed.insert(from_path.clone(), to_path.clone());
                                        buffer_removed.remove(&from_path);
                                        buffer_added.remove(&to_path);
                                    }
                                }
                            }
                        }

                        if buffer_added.is_empty() && buffer_added_folders.is_empty() && 
                           buffer_removed.is_empty() && buffer_renamed.is_empty() && !refresh_needed {
                            continue;
                        }

                        let mut res_added: Vec<AddedItemContext> = Vec::new();
                        let mut res_removed: Vec<RemovedItemContext> = Vec::new();
                        let mut res_updated: Vec<AddedItemContext> = Vec::new();

                        // A. Process Renames
                        for (from, to) in buffer_renamed.drain() {
                            let to_path = PathBuf::from(&to);
                            let new_name = to_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                            
                            if to_path.is_dir() {
                                println!("DEBUG: Watcher - Processing FOLDER RENAME: {} -> {}", from, to);
                                match db.rename_folder(&from, &to, &new_name).await {
                                    Ok(true) => { println!("DEBUG: Watcher - Success folder rename: {} -> {}", from, to); },
                                    Ok(false) => {
                                        println!("DEBUG: Watcher - Folder rename returned false (source {} not in DB). Treating as New.", from);
                                        buffer_added_folders.insert(to);
                                    },
                                    Err(e) => eprintln!("Failed folder rename: {}", e),
                                }
                                refresh_needed = true;
                            } else {
                                let parent = normalize_path(&to_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
                                let folder_id = match db.get_folder_by_path(&parent).await {
                                    Ok(Some(id)) => id,
                                    _ => db.ensure_folder_hierarchy(&parent).await.unwrap_or(0)
                                };
                                
                                if folder_id > 0 {
                                    match db.rename_image(&from, &to, &new_name, folder_id).await {
                                        Ok(Some((meta, old_fid))) => {
                                            res_updated.push(AddedItemContext {
                                                metadata: meta,
                                                folder_id,
                                                old_folder_id: if old_fid != folder_id { Some(old_fid) } else { None },
                                            });
                                        },
                                        _ => {
                                            if let Some(meta) = get_image_metadata(&to_path) {
                                                buffer_added.insert(to, meta);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // B. Process Removed
                        for path in buffer_removed.drain() {
                            let db = db.clone();
                            let app = app.clone();
                            let path_clone = path.clone();
                            let app_data_dir = app_data_dir.clone();
                            
                            // Immediate UI feedback for images
                            if let Ok(Some((img_id, fid, tags))) = db.get_image_context(&path_clone).await {
                                res_removed.push(RemovedItemContext { id: img_id, folder_id: fid, tag_ids: tags });
                            }

                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                
                                // Before deleting, check if it's a folder or an image
                                match db.get_image_context(&path_clone).await {
                                    Ok(Some((_img_id, _fid, _tags))) => {
                                        // Still in DB at this path? If so, it wasn't adopted.
                                        if let Ok(Some((deleted_id, _, _))) = db.delete_image_by_path_returning_context(&path_clone).await {
                                            println!("DEBUG: Watcher - Finalized removal for: {}", path_clone);
                                            let thumb = app_data_dir.join("thumbnails").join(format!("{}.webp", deleted_id));
                                            let _ = std::fs::remove_file(thumb);
                                            
                                            // No need to emit again if we already gave immediate feedback, 
                                            // UNLESS it was a brand new detection (but it wouldn't be here).
                                            // Actually, immediate feedback is better.
                                        }
                                    },
                                    Ok(None) => {
                                        // Check if it's a folder
                                        if let Ok(Some(fid)) = db.get_folder_by_path(&path_clone).await {
                                            if !std::path::Path::new(&path_clone).exists() {
                                                 println!("DEBUG: Watcher - Deleting folder (delay expired): {}", path_clone);
                                                 let _ = db.delete_folder(fid).await;
                                                 let _ = app.emit("library:batch-change", BatchChangePayload {
                                                     added: vec![], removed: vec![], updated: vec![], needs_refresh: true
                                                 });
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                            });
                        }

                        // C. Process Added Folders
                        for path in buffer_added_folders.drain() {
                            println!("DEBUG: Watcher - Ensuring folder: {}", path);
                            if let Ok(_) = db.ensure_folder_hierarchy(&path).await {
                                refresh_needed = true;
                            }
                        }

                        // D. Process Added Images
                        for (path, meta) in buffer_added.drain() {
                            let parent = normalize_path(&Path::new(&path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
                            if let Ok(fid) = db.ensure_folder_hierarchy(&parent).await {
                                match db.save_image(fid, &meta).await {
                                    Ok((id, old_fid, is_new)) => {
                                        let mut meta_with_id = meta.clone();
                                        meta_with_id.id = id;
                                        
                                        let ctx = AddedItemContext { 
                                            metadata: meta_with_id, 
                                            folder_id: fid, 
                                            old_folder_id: old_fid 
                                        };
                                        
                                        if is_new {
                                            res_added.push(ctx);
                                        } else {
                                            res_updated.push(ctx);
                                        }
                                    },
                                    Err(e) => eprintln!("Error saving: {}", e),
                                }
                            }
                        }

                        if !res_added.is_empty() || !res_removed.is_empty() || !res_updated.is_empty() || refresh_needed {
                            let _ = app.emit("library:batch-change", BatchChangePayload {
                                added: res_added,
                                removed: res_removed,
                                updated: res_updated,
                                needs_refresh: refresh_needed,
                            });
                            refresh_needed = false;
                        }
                    }
                }
            }
        });
    }
}

fn normalize_path(path: &str) -> String {
    let p = path.trim_end_matches('/');
    if p.is_empty() { return "/".to_string(); }
    p.to_string()
}

fn is_image_file(path: &std::path::Path) -> bool {
    crate::formats::FileFormat::is_supported_extension(path)
}
