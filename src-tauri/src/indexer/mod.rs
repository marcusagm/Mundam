pub mod metadata;

use self::metadata::{get_image_metadata, ImageMetadata};
use crate::database::Db;
use serde::Serialize;
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

        // 1. Initial Quick Scan - Count files
        let files: Vec<PathBuf> = WalkDir::new(&root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_image_file(e.path()))
            .map(|e| e.path().to_path_buf())
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

        if total_files > 0 {
            // Adaptive Chunking Strategy
            let chunk_size = (total_files / 100).clamp(1, 200);
            let (tx, mut rx) = mpsc::channel::<ImageMetadata>(100);

            // 2. Spawn Workers
            let app_worker = app.clone();
            let db_worker = db.clone();
            tokio::spawn(async move {
                let mut processed: usize = 0;
                let mut batch: Vec<ImageMetadata> = Vec::new();

                while let Some(msg) = rx.recv().await {
                    let msg: ImageMetadata = msg;
                    processed += 1;
                    batch.push(msg.clone());

                    if processed % chunk_size == 0 || processed == total_files {
                        let _ = app_worker.emit(
                            "indexer:progress",
                            ProgressPayload {
                                total: total_files,
                                processed,
                                current_file: msg.filename.clone(),
                            },
                        );

                        let current_batch = batch.split_off(0);
                        if let Err(e) = db_worker
                            .save_images_batch(location_id, current_batch)
                            .await
                        {
                            eprintln!("Batch save failed: {}", e);
                        }
                    }
                }

                let _ = app_worker.emit("indexer:complete", total_files);
                println!("DEBUG: Indexer complete. Processed {}", processed);
            });

            // 3. Producer - Distribute work
            for path in files {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Some(meta) = get_image_metadata(&path) {
                        let _ = tx_clone.send(meta).await;
                    }
                });
            }
        } else {
            let _ = app.emit("indexer:complete", 0);
        }

        // 4. Start File Watcher for real-time updates
        self.start_watcher(root_for_watcher, location_id);
    }

    fn start_watcher(&self, path: PathBuf, location_id: i64) {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let app = self.app_handle.clone();
        let db = self.db.clone();
        let watch_path = path.clone();

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
                            for path in event.paths {
                                if is_image_file(&path) {
                                    if let Some(meta) = get_image_metadata(&path) {
                                        if let Err(e) = db
                                            .save_images_batch(location_id, vec![meta.clone()])
                                            .await
                                        {
                                            eprintln!("Failed to save watched file: {}", e);
                                        } else {
                                            println!(
                                                "DEBUG: Successfully indexed watched file: {:?}",
                                                path
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

                                            // Clear the progress after a short delay
                                            let app_complete = app.clone();
                                            tokio::spawn(async move {
                                                tokio::time::sleep(
                                                    std::time::Duration::from_millis(1500),
                                                )
                                                .await;
                                                let _ = app_complete
                                                    .emit("indexer:complete", 1 as usize);
                                            });
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

