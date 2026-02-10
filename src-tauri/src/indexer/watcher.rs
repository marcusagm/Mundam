use crate::db::Db;
use crate::db::models::ImageMetadata;
use crate::indexer::metadata::get_image_metadata;
use super::types::{BatchChangePayload, AddedItemContext, RemovedItemContext, WatcherRegistry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub fn start_watcher(
    app: AppHandle,
    db: Arc<Db>,
    registry: Arc<tokio::sync::Mutex<WatcherRegistry>>,
    path: PathBuf,
    root_str: String
) {
    let watch_path = path.canonicalize().unwrap_or(path);
    let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from(""));
    let root_str_clone = root_str.clone();

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
                    // println!("DEBUG: Watcher RAW - {:?}", event);

                    match event.kind {
                        EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) => {
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
                        EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::From)) => {
                            if !event.paths.is_empty() {
                                let path_str = normalize_path(&event.paths[0].to_string_lossy());
                                if let Some(tracker) = event.attrs.tracker() {
                                    pending_renames.insert(tracker, path_str);
                                } else {
                                    buffer_removed.insert(path_str);
                                }
                            }
                        },
                        EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::To)) => {
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
                                        let path = &event.paths[0];
                                        if path.is_dir() {
                                            buffer_added_folders.insert(path_str);
                                        } else if is_image_file(path) {
                                            if let Some(meta) = get_image_metadata(path) {
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

fn normalize_path(path: &str) -> String {
    let p = path.trim_end_matches('/');
    if p.is_empty() { return "/".to_string(); }
    p.to_string()
}

fn is_image_file(path: &std::path::Path) -> bool {
    crate::formats::FileFormat::is_supported_extension(path)
}
