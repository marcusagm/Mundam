use super::types::{ProgressPayload, IndexedImage, WatcherRegistry};
use super::watcher::start_watcher;
use crate::db::Db;
use crate::db::models::ImageMetadata;
use crate::indexer::metadata::get_image_metadata;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use walkdir::WalkDir;

pub async fn run_scan(
    app: AppHandle,
    db: Arc<Db>,
    registry: Arc<tokio::sync::Mutex<WatcherRegistry>>,
    root_path: PathBuf
) {
    // Normalize root path (absolute and resolve symlinks)
    let root_path = root_path.canonicalize().unwrap_or(root_path);
    let root_str = normalize_path(&root_path.to_string_lossy());

    println!("DEBUG: Indexer::start_scan for {}", root_str);
    let root_for_watcher = root_path.clone();

    // 1. Initial Quick Scan - Collect files and folders
    let comparison_cache = db.get_all_files_comparison_data(&root_str).await.unwrap_or_default();
    let mut files_to_process: Vec<(PathBuf, String)> = Vec::new();
    let mut clean_count: usize = 0;
    let mut unique_dirs: HashSet<String> = HashSet::new();

    for entry in WalkDir::new(&root_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let path_str = normalize_path(&path.to_string_lossy());

        if entry.file_type().is_dir() {
            unique_dirs.insert(path_str);
        } else if entry.file_type().is_file() && is_image_file(path) {
            let parent = path.parent()
                .map(|p| normalize_path(&p.to_string_lossy()))
                .unwrap_or_default();
            unique_dirs.insert(parent.clone());

            let mut is_dirty = true;
            if let Some((db_size, db_mtime)) = comparison_cache.get(&path_str) {
                if let Ok(m) = entry.metadata() {
                    let disk_size = m.len() as i64;
                    let disk_mtime: DateTime<Utc> = m.modified().ok().map(|t| t.into()).unwrap_or_else(Utc::now);

                    // Strict comparison: size must match and time difference < 1s
                    if disk_size == *db_size && (disk_mtime - *db_mtime).num_seconds().abs() < 1 {
                        is_dirty = false;
                    }
                }
            }

            if is_dirty {
                files_to_process.push((path.to_path_buf(), parent));
            } else {
                clean_count += 1;
            }
        }
    }

    let total_files = files_to_process.len() + clean_count;
    println!("DEBUG: Indexer found {} images ({} changed, {} unchanged) and {} folders",
        total_files, files_to_process.len(), clean_count, unique_dirs.len());

    // Ensure root is in the set
    unique_dirs.insert(root_str.clone());

    println!("DEBUG: Ensuring folder hierarchy for {} folders...", unique_dirs.len());
    // 2. Ensure Hierarchy Exists
    let folder_map = match ensure_folder_hierarchy(&db, unique_dirs, &root_str).await {
        Ok(map) => {
            println!("DEBUG: Folder hierarchy ensured ({} entries)", map.len());
            map
        },
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
            let mut processed: usize = clean_count;
            let mut batch: Vec<(i64, ImageMetadata)> = Vec::new();

            // Initial progress for clean files
            if clean_count > 0 {
                let _ = app_worker.emit(
                    "indexer:progress",
                    ProgressPayload {
                        total: total_files,
                        processed,
                        current_file: "Verifying unchanged files...".to_string(),
                    },
                );
            }

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

                    if let Err(e) = db_worker.save_images_batch(batch.drain(..).collect()).await {
                        eprintln!("Failed to save images batch: {}", e);
                    }
                }
            }

            // Final save for remaining items in batch if the loop finished but batch isn't empty
            if !batch.is_empty() {
                if let Err(e) = db_worker.save_images_batch(batch).await {
                    eprintln!("Failed to save final images batch: {}", e);
                }
            }

            let _ = app_worker.emit("indexer:complete", total_files);
        });

        // 5. Producer - Distribute work
        for (path, parent_dir) in files_to_process {
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
    start_watcher(app, db, registry, root_for_watcher, root_str);
}

async fn ensure_folder_hierarchy(
    db: &Db,
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
            } else if let Ok(Some(id)) = db.get_folder_by_path(&pp).await {
                parent_id = Some(id);
            }
        }

        let is_root = dir_path == root_path;
        match db.upsert_folder(&dir_path, &name, parent_id, is_root).await {
            Ok(id) => { path_to_id.insert(dir_path, id); }
            Err(e) => eprintln!("Failed to upsert folder '{}': {}", dir_path, e),
        }
    }
    Ok(path_to_id)
}

fn normalize_path(path: &str) -> String {
    let p = path.trim_end_matches('/');
    if p.is_empty() { return "/".to_string(); }
    p.to_string()
}

fn is_image_file(path: &std::path::Path) -> bool {
    crate::formats::FileFormat::is_supported_extension(path)
}
