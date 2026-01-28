pub mod metadata;

use self::metadata::{get_image_metadata, ImageMetadata};
use crate::database::Db;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use walkdir::WalkDir;

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
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
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let app = self.app_handle.clone();
        let db = self.db.clone();
        let watch_path = path.clone();
        let _root_path = PathBuf::from(&root_str);

        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(1);

            let mut watcher = RecommendedWatcher::new(
                move |res| {
                    let _ = tx.blocking_send(res);
                },
                Config::default(),
            )
            .expect("Failed to create watcher");

            watcher
                .watch(&watch_path, RecursiveMode::Recursive)
                .expect("Failed to watch path");

            println!("DEBUG: Watcher started for {:?}", watch_path);

            // Keep watcher alive in this scope
            let _watcher = watcher;

            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        // Debounce by only processing specific events
                        if event.kind.is_create() || event.kind.is_modify() {
                            for file_path in event.paths {
                                if is_image_file(&file_path) {
                                    if let Some(meta) = get_image_metadata(&file_path) {
                                        // Get parent directory
                                        let parent_dir = file_path
                                            .parent()
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        
                                        // We need to ensure this folder exists.
                                        // Since we can't easily do full hierarchy check in this async loop efficiently
                                        // without refactoring, for now we try to find it.
                                        // If it's a new folder, it might not exist.
                                        // Ideally we call upsert_folder here.
                                        
                                        let name = file_path
                                            .parent()
                                            .and_then(|p| p.file_name())
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("")
                                            .to_string();

                                        // Try to find parent_id for this folder (grandparent of file)
                                        let grandparent_path = file_path
                                            .parent()
                                            .and_then(|p| p.parent())
                                            .map(|p| p.to_string_lossy().to_string());
                                        
                                        let mut parent_id = None;
                                        if let Some(gp) = grandparent_path {
                                            if let Ok(Some(id)) = db.get_folder_by_path(&gp).await {
                                                parent_id = Some(id);
                                            }
                                        }

                                        let folder_id = match db.upsert_folder(&parent_dir, &name, parent_id, false).await {
                                            Ok(id) => id,
                                            Err(e) => {
                                                eprintln!("Failed to upsert folder for watcher: {}", e);
                                                continue;
                                            }
                                        };
                                        
                                        if let Err(e) = db
                                            .save_image(folder_id, &meta)
                                            .await
                                        {
                                            eprintln!("Failed to save watched file: {}", e);
                                        } else {
                                            println!(
                                                "DEBUG: Successfully indexed watched file: {:?}",
                                                file_path
                                            );
                                            // Notify UI
                                            let _ = app.emit(
                                                "indexer:progress",
                                                ProgressPayload {
                                                    total: 1,
                                                    processed: 1,
                                                    current_file: meta.filename,
                                                },
                                            );
                                            let _ = app.emit("indexer:complete", 1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Watcher error: {:?}", e),
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
