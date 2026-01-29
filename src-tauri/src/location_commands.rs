use crate::database::Db;
use crate::indexer::Indexer;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
pub struct FolderNode {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub parent_id: Option<i64>,
    pub is_root: bool,
}

/// Add a new root folder and start indexing it
#[tauri::command]
pub async fn add_location(
    path: String,
    app: AppHandle,
    db: State<'_, Arc<Db>>,
) -> Result<FolderNode, String> {
    println!("COMMAND: add_location (add_root) called with path: {}", path);
    
    let root = PathBuf::from(&path);
    
    // Validate path exists and is a directory
    if !root.exists() {
        return Err("Path does not exist".to_string());
    }
    if !root.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    
    // Check if a parent folder already exists
    let mut parent_id = None;
    let mut current = root.parent();
    while let Some(p) = current {
        let p_str = p.to_string_lossy().to_string();
        // We use get_folder_by_path, assuming it's exposed or we can access it
        // db is State<Arc<Db>>, so db.get_folder_by_path should work
        if let Some(id) = db.get_folder_by_path(&p_str).await.unwrap_or(None) {
            parent_id = Some(id);
            break;
        }
        current = p.parent();
    }
    
    let is_root = parent_id.is_none();
    
    // Upsert folder
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();
    
    let id = db
        .upsert_folder(&path, &name, parent_id, is_root)
        .await
        .map_err(|e| format!("Failed to add folder: {}", e))?;
    
    // Attempt to adopt orphaned roots (e.g. if we added a parent after its child)
    // Only strictly needed if this is a new folder
    if let Err(e) = db.adopt_orphaned_children(id, &path).await {
        eprintln!("Warning: Failed to adopt orphaned children: {}", e);
    }
    
    // Start indexing in background
    let registry = match app.try_state::<Arc<tokio::sync::Mutex<crate::indexer::WatcherRegistry>>>() {
        Some(r) => r,
        None => return Err("Registry not initialized".to_string()),
    };
    let indexer = Indexer::new(app.clone(), db.inner(), registry.inner().clone());
    tokio::spawn(async move {
        indexer.start_scan(root).await;
    });
    
    Ok(FolderNode {
        id,
        path,
        name,
        parent_id,
        is_root,
    })
}

/// Remove a folder (and its content)
#[tauri::command]
pub async fn remove_location(
    location_id: i64,
    app: AppHandle,
    db: State<'_, Arc<Db>>,
) -> Result<(), String> {
    // Get location path for stopping watcher
    let location_path = match db.get_folder_path(location_id).await {
        Ok(Some(p)) => p,
        _ => return Err("Folder not found".to_string()),
    };

    // Get thumbnail paths before deletion using get_location_thumbnails (which uses CTE now)
    let thumbnail_paths = db
        .get_location_thumbnails(location_id)
        .await
        .map_err(|e| format!("Failed to get thumbnails: {}", e))?;
    
    // Delete thumbnails from filesystem
    let thumbnails_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("thumbnails");
    
    let mut deleted_count = 0;
    for thumb_filename in thumbnail_paths {
        let thumb_path = thumbnails_dir.join(&thumb_filename);
        if thumb_path.exists() {
            if let Err(e) = std::fs::remove_file(&thumb_path) {
                eprintln!("Failed to delete thumbnail {:?}: {}", thumb_path, e);
            } else {
                deleted_count += 1;
            }
        }
    }
    println!("DEBUG: Deleted {} thumbnail files", deleted_count);
    
    // Delete from database
    db.delete_folder(location_id)
        .await
        .map_err(|e| format!("Failed to delete folder: {}", e))?;
    
    // Stop the watcher via Indexer
    let registry = match app.try_state::<Arc<tokio::sync::Mutex<crate::indexer::WatcherRegistry>>>() {
        Some(r) => r,
        None => return Err("Registry not initialized".to_string()),
    };
    let indexer = Indexer::new(app.clone(), db.inner(), registry.inner().clone());
    indexer.stop_watcher(&location_path).await;

    println!("DEBUG: Folder {} deleted successfully", location_id);
    Ok(())
}

/// Get all folders (roots and hierarchy)
#[tauri::command]
pub async fn get_locations(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<FolderNode>, String> {
    // We return ALL folders now, frontend can build the tree
    let folders = db
        .get_folder_hierarchy()
        .await
        .map_err(|e| format!("Failed to get folder hierarchy: {}", e))?;
    
    Ok(folders
        .into_iter()
        .map(|(id, parent_id, path, name, is_root)| FolderNode {
            id,
            path,
            name,
            parent_id,
            is_root,
        })
        .collect())
}

// Deprecated or Aliased commands for compatibility if needed (but we are refactoring frontend)

#[tauri::command]
pub async fn get_all_subfolders(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<FolderNode>, String> {
    // Just alias to get_locations for now, or return empty if frontend expects distinct subfolders
    // But better to fail fast or return same data
    get_locations(db).await
}

#[tauri::command]
pub async fn get_subfolder_counts(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<(i64, i64)>, String> {
    db.get_folder_counts_recursive()
        .await
        .map_err(|e| format!("Failed to get folder counts: {}", e))
}

#[tauri::command]
pub async fn get_location_root_counts(
    _db: State<'_, Arc<Db>>,
) -> Result<Vec<(i64, i64)>, String> {
    // No longer relevant with unified folders
    Ok(vec![])
}
