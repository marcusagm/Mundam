use crate::database::Db;
use crate::thumbnails::{generate_thumbnail, get_thumbnail_filename};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

pub struct ThumbnailWorker {
    db: Arc<Db>,
    thumbnails_dir: PathBuf,
    app_handle: AppHandle,
    config: crate::config::AppConfig,
}

impl ThumbnailWorker {
    pub fn new(db: Arc<Db>, thumbnails_dir: PathBuf, app_handle: AppHandle, config: crate::config::AppConfig) -> Self {
        Self {
            db,
            thumbnails_dir,
            app_handle,
            config,
        }
    }

    pub async fn start(self) {
        let db = self.db.clone();
        let app = self.app_handle.clone();
        let thumb_dir = self.thumbnails_dir.clone();
        let config = self.config.clone();

        // println!("DEBUG: Thumbnail worker started. Dir: {:?}", thumb_dir);

        tauri::async_runtime::spawn(async move {
            loop {
                // Fetch valid batch size from config
                // println!("DEBUG: Fetching images needing thumbnails...");
                match db.get_images_needing_thumbnails(config.indexer_batch_size).await {
                    Ok(images) => {
                        if images.is_empty() {
                            // println!("DEBUG: No images need thumbnails. Sleeping.");
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }

                        println!(
                            "DEBUG: Found {} images needing thumbnails. Starting batch...",
                            images.len()
                        );

                        // Clone thumb_dir for the move closure
                        let thumb_dir_clone = thumb_dir.clone();
                        let num_threads = config.thumbnail_threads;

                        // Use a blocking thread for CPU-intensive work
                        // Limit parallelism to configured threads (since we leverage FFmpeg external processes mostly)
                        let db_updates = tauri::async_runtime::spawn_blocking(move || {
                            use rayon::prelude::*;
                            use rayon::ThreadPoolBuilder;
                            
                            // Create a limited thread pool
                            let pool = ThreadPoolBuilder::new()
                                .num_threads(num_threads)
                                .build()
                                .unwrap();
                            
                            pool.install(|| {
                                images
                                    .par_iter()
                                    .map(|(id, img_path)| {
                                        let input_path = Path::new(&img_path);
                                        if !input_path.exists() {
                                            return (*id, Err("File not found".to_string()));
                                        }
    
                                        let thumb_name = get_thumbnail_filename(&img_path);

    
                                        // Generate thumbnail
                                        match generate_thumbnail(input_path, &thumb_dir_clone, &thumb_name, 300) {
                                            Ok(generated_filename) => {
                                                (*id, Ok(generated_filename))
                                            }
                                            Err(e) => {
                                                // Capture the error
                                                (*id, Err(e.to_string()))
                                            }
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }) // Close pool.install
                        })
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Blocking task failed: {}", e);
                            Vec::new()
                        });

                        println!(
                            "DEBUG: Batch processed. Updating DB for {} items.",
                            db_updates.len()
                        );

                        #[derive(serde::Serialize, Clone)]
                        struct ThumbnailPayload {
                            id: i64,
                            path: String,
                        }

                        // Perform DB updates sequentially (async)
                        for (id, result) in db_updates {
                            match result {
                                Ok(filename) => {
                                    if let Err(e) = db.update_thumbnail_path(id, &filename).await {
                                        eprintln!("Error updating DB for thumbnail: {}", e);
                                    } else {
                                        let payload = ThumbnailPayload {
                                            id,
                                            path: filename.clone(),
                                        };
                                        let _ = app.emit("thumbnail:ready", payload);
                                    }
                                }
                                Err(err_msg) => {
                                    eprintln!("Thumbnail error for ID {}: {}", id, err_msg);
                                    if let Err(e) = db.record_thumbnail_error(id, err_msg).await {
                                        eprintln!("Failed to record thumbnail error in DB: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Thumbnail worker DB error: {}", e);
                        sleep(Duration::from_secs(10)).await;
                    }
                }

                // Short sleep to prevent tight loop if database is empty/erroring excessively,
                // but kept short for responsiveness
                sleep(Duration::from_millis(100)).await;
            }
        });
    }
}
