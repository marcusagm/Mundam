pub mod metadata;

use self::metadata::{get_image_metadata, ImageMetadata};
use crate::database::Db;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// Struct to hold image path with its relative directory
struct IndexedImage {
    metadata: ImageMetadata,
    relative_dir: String,  // e.g., "Vacation/Beach" or "" for root
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

        // 1. Initial Quick Scan - Collect files with relative paths
        let files: Vec<(PathBuf, String)> = WalkDir::new(&root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_image_file(e.path()))
            .map(|e| {
                let file_path = e.path().to_path_buf();
                // Calculate relative directory (not including filename)
                let relative_dir = file_path
                    .parent()
                    .and_then(|p| p.strip_prefix(&root_path).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                (file_path, relative_dir)
            })
            .collect();

        let total_files = files.len();
        println!("DEBUG: Indexer found {} images", total_files);

        // Get Location ID
        let location_path = root_path.to_string_lossy().to_string();
        let location_name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&location_path)
            .to_string();

        let location_id = match db
            .get_or_create_location(&location_path, &location_name)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to get/create location: {}", e);
                return;
            }
        };

        // 2. Build subfolder hierarchy BEFORE processing images
        let unique_dirs: std::collections::HashSet<String> = files
            .iter()
            .filter(|(_, dir)| !dir.is_empty())
            .map(|(_, dir)| dir.clone())
            .collect();
        
        println!("DEBUG: Found {} unique subdirectories", unique_dirs.len());
        for dir in &unique_dirs {
            println!("DEBUG: Subdirectory: '{}'", dir);
        }
        
        // Create subfolders and build lookup map
        let subfolder_map = match self.create_subfolder_hierarchy(location_id, unique_dirs).await {
            Ok(map) => map,
            Err(e) => {
                eprintln!("Failed to create subfolder hierarchy: {}", e);
                HashMap::new()
            }
        };
        
        println!("DEBUG: Created {} subfolders", subfolder_map.len());

        if total_files > 0 {
            // Adaptive Chunking Strategy
            let chunk_size = (total_files / 100).clamp(1, 200);
            let (tx, mut rx) = mpsc::channel::<IndexedImage>(100);

            // 3. Spawn Worker to save images
            let app_worker = app.clone();
            let db_worker = db.clone();
            let subfolder_map_worker = subfolder_map.clone();
            
            tokio::spawn(async move {
                let mut processed: usize = 0;
                let mut batch: Vec<(Option<i64>, ImageMetadata)> = Vec::new();

                while let Some(indexed) = rx.recv().await {
                    processed += 1;
                    let subfolder_id = if indexed.relative_dir.is_empty() {
                        None
                    } else {
                        subfolder_map_worker.get(&indexed.relative_dir).copied()
                    };
                    batch.push((subfolder_id, indexed.metadata.clone()));

                    if processed % chunk_size == 0 || processed == total_files {
                        let _ = app_worker.emit(
                            "indexer:progress",
                            ProgressPayload {
                                total: total_files,
                                processed,
                                current_file: indexed.metadata.filename.clone(),
                            },
                        );

                        // Save batch with subfolder_ids
                        for (sf_id, img) in batch.drain(..) {
                            if let Err(e) = db_worker
                                .save_image_with_subfolder(location_id, sf_id, &img)
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

            // 4. Producer - Distribute work
            for (path, relative_dir) in files {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Some(meta) = get_image_metadata(&path) {
                        let _ = tx_clone.send(IndexedImage {
                            metadata: meta,
                            relative_dir,
                        }).await;
                    }
                });
            }
        } else {
            let _ = app.emit("indexer:complete", 0);
        }

        // 5. Start File Watcher for real-time updates
        self.start_watcher(root_for_watcher, location_id, root_str);
    }
    
    /// Create subfolder hierarchy in database and return path -> id mapping
    async fn create_subfolder_hierarchy(
        &self,
        location_id: i64,
        dirs: std::collections::HashSet<String>,
    ) -> Result<HashMap<String, i64>, String> {
        let mut path_to_id: HashMap<String, i64> = HashMap::new();
        
        // Sort paths to ensure parents are created before children
        let mut sorted_dirs: Vec<String> = dirs.into_iter().collect();
        sorted_dirs.sort();
        
        for dir_path in sorted_dirs {
            // Split path into components
            let components: Vec<&str> = dir_path.split(std::path::MAIN_SEPARATOR).collect();
            let name = components.last().unwrap_or(&"").to_string();
            
            // Find parent ID
            let parent_id = if components.len() > 1 {
                let parent_path = components[..components.len()-1].join(&std::path::MAIN_SEPARATOR.to_string());
                path_to_id.get(&parent_path).copied()
            } else {
                None
            };
            
            // Create subfolder
            println!("DEBUG: Creating subfolder '{}' with parent_id {:?}", dir_path, parent_id);
            match self.db.get_or_create_subfolder(location_id, &dir_path, &name, parent_id).await {
                Ok(id) => {
                    println!("DEBUG: Created subfolder ID {} for '{}'", id, dir_path);
                    path_to_id.insert(dir_path, id);
                }
                Err(e) => {
                    eprintln!("Failed to create subfolder '{}': {}", dir_path, e);
                }
            }
        }
        
        println!("DEBUG: Total subfolders created: {}", path_to_id.len());
        
        Ok(path_to_id)
    }

    fn start_watcher(&self, path: PathBuf, location_id: i64, root_str: String) {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let app = self.app_handle.clone();
        let db = self.db.clone();
        let watch_path = path.clone();
        let root_path = PathBuf::from(&root_str);

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
                                        // Calculate relative directory
                                        let relative_dir = file_path
                                            .parent()
                                            .and_then(|p| p.strip_prefix(&root_path).ok())
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        
                                        // Get or create subfolder if needed
                                        let subfolder_id = if relative_dir.is_empty() {
                                            None
                                        } else {
                                            // Get subfolder name
                                            let name = file_path
                                                .parent()
                                                .and_then(|p| p.file_name())
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("")
                                                .to_string();
                                            
                                            // Find parent subfolder
                                            let components: Vec<&str> = relative_dir.split(std::path::MAIN_SEPARATOR).collect();
                                            let parent_id = if components.len() > 1 {
                                                let parent_path = components[..components.len()-1].join(&std::path::MAIN_SEPARATOR.to_string());
                                                // Try to get parent - this is async so we need a simpler approach
                                                // For watched files, we'll just set parent_id to None
                                                // The hierarchy is already created during initial scan
                                                None
                                            } else {
                                                None
                                            };
                                            
                                            match db.get_or_create_subfolder(location_id, &relative_dir, &name, parent_id).await {
                                                Ok(id) => Some(id),
                                                Err(e) => {
                                                    eprintln!("Failed to get/create subfolder: {}", e);
                                                    None
                                                }
                                            }
                                        };
                                        
                                        if let Err(e) = db
                                            .save_image_with_subfolder(location_id, subfolder_id, &meta)
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
