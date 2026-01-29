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
}

impl ThumbnailWorker {
    pub fn new(db: Arc<Db>, thumbnails_dir: PathBuf, app_handle: AppHandle) -> Self {
        Self {
            db,
            thumbnails_dir,
            app_handle,
        }
    }

    pub async fn start(self) {
        let db = self.db.clone();
        let app = self.app_handle.clone();
        let thumb_dir = self.thumbnails_dir.clone();

        // println!("DEBUG: Thumbnail worker started. Dir: {:?}", thumb_dir);

        tauri::async_runtime::spawn(async move {
            loop {
                // Fetch valid batch size (e.g., 5) to maximize responsiveness
                // println!("DEBUG: Fetching images needing thumbnails...");
                match db.get_images_needing_thumbnails(5).await {
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

                        // Use a blocking thread for CPU-intensive work
                        // Limit parallelism to 2 threads to reduce CPU usage
                        let db_updates = tauri::async_runtime::spawn_blocking(move || {
                            use rayon::prelude::*;
                            use rayon::ThreadPoolBuilder;
                            
                            // Create a limited thread pool (2 threads max)
                            let pool = ThreadPoolBuilder::new()
                                .num_threads(2)
                                .build()
                                .unwrap();
                            
                            pool.install(|| {
                                images
                                    .par_iter()

                                .filter_map(|(id, img_path)| {
                                    let input_path = Path::new(&img_path);
                                    if !input_path.exists() {
                                        println!("DEBUG: Image path not found: {:?}", input_path);
                                        return None;
                                    }

                                    let thumb_name = get_thumbnail_filename(&img_path);
                                    let output_path = thumb_dir_clone.join(&thumb_name);

                                    // println!("DEBUG: Generating thumbnail for ID: {}", id);
                                    // println!(
                                    //     "DEBUG: Processing ID: {} | Path: {:?}",
                                    //     id, input_path
                                    // );

                                    // Generate thumbnail
                                    match generate_thumbnail(input_path, &output_path, 300) {
                                        Ok(_) => {
                                            // println!("DEBUG: Success ID: {}", id);
                                            let filename_only = output_path
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string();
                                            Some((*id, filename_only))
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error generating thumbnail for {:?}: {}",
                                                input_path, e
                                            );
                                            None
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

                        // Define payload struct locally or import it.
                        // Since we can't easily add a struct outside in replace_file_content without context,
                        // we'll use an ad-hoc tuple or a serde_json::json! macro if available, or just a simple struct.
                        // Let's use a struct defined inside the async block or just a JSON object if tauri supports it directly...
                        // Tauri's emit takes any Serialize type.

                        #[derive(serde::Serialize, Clone)]
                        struct ThumbnailPayload {
                            id: i64,
                            path: String,
                        }

                        // Perform DB updates sequentially (async)
                        for (id, filename) in db_updates {
                            if let Err(e) = db.update_thumbnail_path(id, &filename).await {
                                eprintln!("Error updating DB for thumbnail: {}", e);
                            } else {
                                // println!("DEBUG: Thumbs saved: ID {}", id);
                                let payload = ThumbnailPayload {
                                    id,
                                    path: filename.clone(),
                                };
                                let _ = app.emit("thumbnail:ready", payload);
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
