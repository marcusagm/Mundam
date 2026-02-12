use crate::db::Db;
use crate::thumbnails::{generate_thumbnail, get_thumbnail_filename};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};
use crate::thumbnails::priority::ThumbnailPriorityState;

pub struct ThumbnailWorker {
    db: Arc<Db>,
    thumbnails_dir: PathBuf,
    app_handle: AppHandle,
    config: crate::settings::config::AppConfig,
    priority_state: Arc<ThumbnailPriorityState>,
}

impl ThumbnailWorker {
    pub fn new(
        db: Arc<Db>,
        thumbnails_dir: PathBuf,
        app_handle: AppHandle,
        config: crate::settings::config::AppConfig,
        priority_state: Arc<ThumbnailPriorityState>,
    ) -> Self {
        Self {
            db,
            thumbnails_dir,
            app_handle,
            config,
            priority_state,
        }
    }

    pub async fn start(self) {
        let db = self.db.clone();
        let app = self.app_handle.clone();
        let thumb_dir = self.thumbnails_dir.clone();
        let config = self.config.clone();
        let priority_state = self.priority_state.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                // 1. Check Priority Queue First
                let priority_ids = priority_state.priority_ids.lock().unwrap().iter().cloned().collect::<Vec<i64>>();

                let mut images = Vec::new();
                let mut is_priority_batch = false;

                if !priority_ids.is_empty() {
                    if let Ok(priority_imgs) = db.get_images_needing_thumbnails_by_ids(&priority_ids).await {
                         if !priority_imgs.is_empty() {
                             // println!("DEBUG: Processing {} priority thumbnails", priority_imgs.len());
                             images = priority_imgs;
                             is_priority_batch = true;
                         }
                    }
                }

                // 2. If no priority work, check regular queue
                if images.is_empty() {
                     match db.get_images_needing_thumbnails(config.indexer_batch_size).await {
                        Ok(imgs) => {
                            images = imgs;
                        },
                        Err(e) => {
                             eprintln!("Thumbnail worker DB error: {}", e);
                             sleep(Duration::from_secs(10)).await;
                             continue;
                        }
                     }
                }

                if images.is_empty() {
                    // No work at all
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                if !is_priority_batch {
                    println!(
                        "DEBUG: Found {} images needing thumbnails. Starting batch...",
                        images.len()
                    );
                }

                // Clone thumb_dir for the move closure
                let thumb_dir_clone = thumb_dir.clone();
                let num_threads = config.thumbnail_threads;
                let app_for_blocking = app.clone();

                // Use a blocking thread for CPU-intensive work
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
                                match generate_thumbnail(Some(&app_for_blocking), input_path, &thumb_dir_clone, &thumb_name, 300) {
                                    Ok(generated_filename) => {
                                        (*id, Ok(generated_filename))
                                    }
                                    Err(e) => {
                                        (*id, Err(e.to_string()))
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .await
                .unwrap_or_else(|e| {
                    eprintln!("Blocking task failed: {}", e);
                    Vec::new()
                });

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

                // If we processed a priority batch, we loop immediately to check for more or resume normal work.
                // If it was a normal batch, we also loop immediately but maybe yield.
                if !is_priority_batch {
                     sleep(Duration::from_millis(100)).await;
                } else {
                    // Give a tiny yield just in case
                     sleep(Duration::from_millis(10)).await;
                }
            }
        });
    }
}
