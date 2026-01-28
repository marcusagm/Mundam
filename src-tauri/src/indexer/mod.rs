pub mod metadata;

use self::metadata::{get_image_metadata, ImageMetadata};
use crate::database::Db;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
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

pub struct Indexer {
    app_handle: AppHandle,
    db: Arc<Db>,
}

impl Indexer {
    pub fn new(app_handle: AppHandle, db: &Db) -> Self {
        Self {
            app_handle,
            db: Arc::new(Db {
                pool: db.pool.clone(),
            }),
        }
    }

    pub async fn start_scan(&self, root_path: PathBuf) {
        println!("DEBUG: Indexer::start_scan for {:?}", root_path);
        let app = self.app_handle.clone();
        let db = self.db.clone();
        let root_for_watcher = root_path.clone();
        let root_str = root_path.to_string_lossy().to_string();

        // 1. Initial Quick Scan - Collect files with absolute parent paths
        let files: Vec<(PathBuf, String)> = WalkDir::new(&root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_image_file(e.path()))
            .map(|e| {
                let file_path = e.path().to_path_buf();
                // Get absolute parent directory
                let parent_dir = file_path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                (file_path, parent_dir)
            })
            .collect();

        let total_files = files.len();
        println!("DEBUG: Indexer found {} images", total_files);

        // Get Location ID
        // 2. Identify all unique folders (including Root)
        let mut unique_dirs: std::collections::HashSet<String> = files
            .iter()
            .filter(|(_, dir)| !dir.is_empty())
            .map(|(_, dir)| dir.clone())
            .collect();
            
        // Ensure root is in the set
        unique_dirs.insert(root_str.clone());

        println!("DEBUG: Found {} unique folders", unique_dirs.len());

        // 3. Ensure Hierarchy Exists
        let folder_map = match self.ensure_folder_hierarchy(unique_dirs, &root_str).await {
            Ok(map) => map,
            Err(e) => {
                eprintln!("Failed to ensure folder hierarchy: {}", e);
                HashMap::new()
            }
        };

        // We need the ID of the root folder for the watcher or logging?
        let _location_id = *folder_map.get(&root_str).unwrap_or(&0); 

        if total_files > 0 {
            // Adaptive Chunking Strategy
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
                    } else {
                        eprintln!("Warning: Folder ID not found for '{}'", indexed.parent_dir);
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

                        // Save batch
                        for (fid, img) in batch.drain(..) {
                            if let Err(e) = db_worker
                                .save_image(fid, &img)
                                .await
                            {
                                eprintln!("Failed to save image: {}", e);
                            }
                        }
                    }
                }

                let _ = app_worker.emit("indexer:complete", total_files);
                println!("DEBUG: Indexer complete. Processed {}", processed);
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

        // 5. Start File Watcher for real-time updates
        self.start_watcher(root_for_watcher, 0, root_str);
    }
    
    /// Ensure folder hierarchy exists in database
    async fn ensure_folder_hierarchy(
        &self,
        folders: std::collections::HashSet<String>,
        root_path: &str,
    ) -> Result<HashMap<String, i64>, String> {
        let mut path_to_id: HashMap<String, i64> = HashMap::new();
        
        // Sort paths by length to ensure parents are processed first
        let mut sorted_dirs: Vec<String> = folders.into_iter().collect();
        sorted_dirs.sort_by_key(|a| a.len());
        
        for dir_path in sorted_dirs {
            let path_buf = PathBuf::from(&dir_path);
            let name = path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            
            // Find parent
            // We check if the parent path exists in our map (which means it's part of the current scan)
            // Or we check DB? 
            // Better: Check map first. If not in map, check DB (it might be an existing folder outside this scan context).
            
            let parent_path_str = path_buf
                .parent()
                .map(|p| p.to_string_lossy().to_string());
            
            let mut parent_id = None;
            if let Some(pp) = parent_path_str {
                if let Some(id) = path_to_id.get(&pp) {
                    parent_id = Some(*id);
                } else if let Ok(Some(id)) = self.db.get_folder_by_path(&pp).await {
                    parent_id = Some(id);
                }
            }
            
            let is_root = dir_path == root_path;
            
            println!("DEBUG: Upserting folder '{}' (Parent: {:?}, Root: {})", dir_path, parent_id, is_root);
            
            match self.db.upsert_folder(&dir_path, &name, parent_id, is_root).await {
                Ok(id) => {
                    path_to_id.insert(dir_path, id);
                }
                Err(e) => {
                    eprintln!("Failed to upsert folder '{}': {}", dir_path, e);
                }
            }
        }
        
        println!("DEBUG: Hierarchy processed. Map size: {}", path_to_id.len());
        Ok(path_to_id)
    }

    fn start_watcher(&self, path: PathBuf, _location_id: i64, root_str: String) {
        use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

        let app = self.app_handle.clone();
        let db = self.db.clone();
        let watch_path = path.clone();
        let _root_path = PathBuf::from(&root_str);

        // Get app data dir for thumbnail deletion
        let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from(""));
        
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel::<Event>(100);

            // Debouncer buffer
            let debouncer_window = Duration::from_millis(500);
            
            let mut watcher = RecommendedWatcher::new(
                move |res: notify::Result<Event>| {
                    if let Ok(event) = res {
                         let _ = tx.blocking_send(event);
                    }
                },
                Config::default(),
            )
            .expect("Failed to create watcher");

            watcher
                .watch(&watch_path, RecursiveMode::Recursive)
                .expect("Failed to watch path");

            println!("DEBUG: Robust Watcher started for {:?}", watch_path);

            // Keep watcher alive
            let _watcher = watcher;

            // Event Loop with Debouncing
            let mut buffer_added: HashMap<String, ImageMetadata> = HashMap::new();
            let mut buffer_removed: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut buffer_renamed: HashMap<String, String> = HashMap::new();
            let mut pending_renames: HashMap<usize, String> = HashMap::new();
            
            let mut timer = tokio::time::interval(debouncer_window);
            
            // Loop that selects between incoming events and timer tick
            loop {
                tokio::select! {
                        Some(event) = rx.recv() => {
                        // DEBUG LOG
                        println!("RAW EVENT: {:?}", event);

                        // Aggregate events
                         match event.kind {
                            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                if event.paths.len() == 2 {
                                    let from = event.paths[0].to_string_lossy().to_string();
                                    let to = event.paths[1].to_string_lossy().to_string();
                                    
                                    // Check if relevant
                                    if is_image_file(&event.paths[0]) || is_image_file(&event.paths[1]) {
                                        buffer_renamed.insert(from, to);
                                    }
                                }
                            },
                            // Handle Split Rename (From)
                            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                if !event.paths.is_empty() {
                                    let path_str = event.paths[0].to_string_lossy().to_string();
                                    if let Some(tracker) = event.attrs.tracker() {
                                        pending_renames.insert(tracker, path_str);
                                    } else {
                                        buffer_removed.insert(path_str);
                                    }
                                }
                            },
                            // Handle Split Rename (To)
                            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                                if !event.paths.is_empty() {
                                    let path_str = event.paths[0].to_string_lossy().to_string();
                                    
                                    // Try to match pending rename
                                    let matched_from = if let Some(tracker) = event.attrs.tracker() {
                                        pending_renames.remove(&tracker)
                                    } else {
                                        None
                                    };

                                    if let Some(from) = matched_from {
                                        buffer_renamed.insert(from, path_str);
                                    } else {
                                        if let Some(meta) = get_image_metadata(&event.paths[0]) {
                                            buffer_added.insert(path_str, meta);
                                        }
                                    }
                                }
                            },
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                for path in event.paths {
                                    if is_image_file(&path) {
                                        let path_str = path.to_string_lossy().to_string();
                                        
                                        // Check if file exists to distinguish Add/Mod vs Rename(From)
                                        if path.exists() {
                                            // It exists, so it's an Add or Modify
                                            buffer_removed.remove(&path_str);
                                            if let Some(meta) = get_image_metadata(&path) {
                                                buffer_added.insert(path_str, meta);
                                            }
                                        } else {
                                            // It does NOT exist, implies Rename(From) or deletion detected as Mod
                                            buffer_added.remove(&path_str);
                                            buffer_removed.insert(path_str);
                                        }
                                    }
                                }
                            },
                            EventKind::Remove(_) => {
                                for path in event.paths {
                                     // For remove, we might not be able to check extension if file is gone?
                                     // Notify sends the path. We should check if extension matches OUR types.
                                     // Even if file is gone, path still has extension.
                                     if is_image_file(&path) {
                                         let path_str = path.to_string_lossy().to_string();
                                         buffer_added.remove(&path_str); // If pending add, remove it
                                         buffer_removed.insert(path_str);
                                     }
                                }
                            },
                             _ => {}
                         }
                    }
                    _ = timer.tick() => {
                        // Flush pending renames as removals
                        for (_, path) in pending_renames.drain() {
                            buffer_removed.insert(path);
                        }

                        // Heuristic: Attempt to pair Removed + Added as Rename based on metadata (Size + CreatedAt)
                        // This handles cases where OS emits split events without tracker correlation (e.g. macOS FSEvents generic)
                        let removed_paths: Vec<String> = buffer_removed.iter().cloned().collect();
                        for from_path in removed_paths {
                             if !buffer_removed.contains(&from_path) { continue; }

                             if let Ok(Some((old_size, old_created))) = db.get_file_comparison_data(&from_path).await {
                                 let match_key = buffer_added.iter().find_map(|(to_path, meta)| {
                                     if meta.size == old_size && meta.created_at == old_created {
                                         return Some(to_path.clone());
                                     }
                                     None
                                 });

                                 if let Some(to_path) = match_key {
                                     println!("DEBUG: Heuristic Match found: {} -> {}", from_path, to_path);
                                     buffer_renamed.insert(from_path.clone(), to_path.clone());
                                     buffer_removed.remove(&from_path);
                                     buffer_added.remove(&to_path);
                                 }
                             }
                        }

                        // Process Buffer if not empty
                        if buffer_added.is_empty() && buffer_removed.is_empty() && buffer_renamed.is_empty() {
                            continue;
                        }

                        let mut added_list: Vec<AddedItemContext> = Vec::new();
                        let mut removed_list: Vec<RemovedItemContext> = Vec::new();
                        let mut updated_list: Vec<AddedItemContext> = Vec::new();

                        // 0. Process Renames
                        for (from_str, to_str) in buffer_renamed.drain() {
                             let to_path = PathBuf::from(&to_str);
                             let new_name = to_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                             
                             // Resolve Folder ID for new path
                             let parent_dir = to_path.parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                                
                             // Try to get folder ID
                             let mut folder_id = match db.get_folder_by_path(&parent_dir).await {
                                Ok(Some(id)) => id,
                                _ => 0
                             };
                             
                             // If folder unknown, try to upsert (like in Add)
                             if folder_id == 0 {
                                 if let Ok(id) = db.upsert_folder(&parent_dir, &parent_dir, None, false).await {
                                     folder_id = id;
                                 }
                             }
                             
                             if folder_id > 0 {
                                 match db.rename_image(&from_str, &to_str, &new_name, folder_id).await {
                                     Ok(Some((meta, old_folder_id))) => {
                                         println!("DEBUG: Watcher detected RENAME: {} -> {}", from_str, to_str);
                                         
                                         let moved_val = if old_folder_id != folder_id { Some(old_folder_id) } else { None };
                                         
                                         updated_list.push(AddedItemContext { 
                                             metadata: meta, 
                                             folder_id,
                                             old_folder_id: moved_val,
                                         });
                                         
                                         // Clean up other buffers to avoid double processing
                                         buffer_removed.remove(&from_str);
                                         buffer_added.remove(&to_str);
                                     },
                                     _ => {
                                         // Rename failed (old not found?), verify existence and treat as Add
                                         if to_path.exists() {
                                             if let Some(meta) = get_image_metadata(&to_path) {
                                                 buffer_added.insert(to_str, meta);
                                             }
                                         }
                                     }
                                 }
                             }
                        }

                        // 1. Process Removed
                        for path_str in buffer_removed.drain() {
                            // Use context-aware delete
                            match db.delete_image_by_path_returning_context(&path_str).await {
                                Ok(Some((img_id, folder_id, tag_ids))) => {
                                    println!("DEBUG: Watcher detected REMOVE: {}", path_str);
                                    
                                    // Delete local thumbnail if exists
                                    let thumb_filename = format!("{}.webp", img_id);
                                    let thumb_path = app_data_dir.join("thumbnails").join(thumb_filename);
                                    if thumb_path.exists() {
                                        let _ = std::fs::remove_file(thumb_path);
                                    }

                                    removed_list.push(RemovedItemContext {
                                        id: img_id,
                                        folder_id,
                                        tag_ids,
                                    });
                                },
                                Ok(None) => {
                                    // Not in DB, ignore
                                },
                                Err(e) => eprintln!("Error deleting watched file: {}", e),
                            }
                        }

                        // 2. Process Added
                        for (path_str, meta) in buffer_added.drain() {
                             let path_buf = PathBuf::from(&path_str);
                             
                             // Get/Ensure Folder
                             let parent_dir = path_buf.parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                                
                             let name = path_buf.parent()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("").to_string();
                             
                             // We try to find parent... simplified logic from indexer:
                             // If folder doesn't exist, we try to create it non-recursively for now,
                             // or just attach to root? Ideally we want correct parent.
                             // For now, let's try to get existing ID or create.
                            
                            // Try to look up folder
                            let mut folder_id = match db.get_folder_by_path(&parent_dir).await {
                                Ok(Some(id)) => id,
                                _ => 0 // Fallback
                            };

                            if folder_id == 0 {
                                // Try creating
                                if let Ok(id) = db.upsert_folder(&parent_dir, &name, None, false).await {
                                    folder_id = id;
                                }
                            }
                            
                            if folder_id > 0 {
                                match db.save_image(folder_id, &meta).await {
                                    Ok(is_new) => {
                                        if is_new {
                                            println!("DEBUG: Watcher detected ADD: {}", path_str);
                                            added_list.push(AddedItemContext {
                                                metadata: meta,
                                                folder_id,
                                                old_folder_id: None,
                                            });
                                        } else {
                                            println!("DEBUG: Watcher detected MOD (Update): {}", path_str);
                                            updated_list.push(AddedItemContext {
                                                metadata: meta,
                                                folder_id,
                                                old_folder_id: Some(folder_id), // Same folder
                                            });
                                        }
                                    },
                                    Err(e) => eprintln!("Failed to save image: {}", e),
                                }
                            } else {
                                eprintln!("DEBUG: Could not determine folder for {}", path_str);
                            }
                        }

                        // 3. Emit Batch Event
                        if !added_list.is_empty() || !removed_list.is_empty() || !updated_list.is_empty() {
                            let _ = app.emit("library:batch-change", BatchChangePayload {
                                added: added_list,
                                removed: removed_list,
                                updated: updated_list,
                            });
                        }
                    }
                }
            }
        });
    }
}

/// Check if a file has a supported extension for indexing
/// This list matches the thumbnail generation supported formats
fn is_image_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    matches!(ext.as_str(),
        // Tier 1: Fast path - Common formats
        "jpg" | "jpeg" | "jpe" | "jfif" | "png" | "webp" | "gif" | "bmp" | "ico" |
        
        // Tier 2: FFmpeg - Modern codecs
        "heic" | "heif" | "hif" | "avif" | "jxl" |
        
        // Tier 2: FFmpeg - RAW formats
        "cr2" | "cr3" | "arw" | "nef" | "dng" | "raf" | "orf" | "pef" | "rw2" |
        "3fr" | "mrw" | "nrw" | "sr2" | "srw" | "x3f" | "erf" | "crw" | "raw" |
        
        // Tier 2: FFmpeg - Design & Vector
        "psd" | "psb" | "ai" | "eps" | "svg" | "tif" | "tiff" |
        
        // Tier 2: FFmpeg - HDR & Special
        "exr" | "hdr" | "tga" |
        
        // Tier 3: ZIP Preview - Affinity Suite
        "af" | "afdesign" | "afphoto" | "afpub" |
        
        // Tier 3: ZIP Preview - Other apps
        "clip" | "xmind" | "graffle" |
        
        // Tier 4: Icon fallback - 3D formats
        "c4d" | "3ds" | "obj" | "fbx" | "blend" | "stl" | "dae" |
        "skp" | "dwg" | "dxf" | "max" | "lwo" | "lws" | "ma" | "mb" |
        
        // Tier 4: Icon fallback - Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "fon" | "fnt" |
        
        // Tier 4: Icon fallback - Design (non-ZIP)
        "cdr" | "indd" | "xd" | "fig" | "sketch"
    )
}
