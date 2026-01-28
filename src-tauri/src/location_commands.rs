use crate::database::Db;
use crate::indexer::Indexer;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
pub struct LocationInfo {
    pub id: i64,
    pub path: String,
    pub name: String,
}

/// Add a new folder location and start indexing it
#[tauri::command]
pub async fn add_location(
    path: String,
    app: AppHandle,
    db: State<'_, Arc<Db>>,
) -> Result<LocationInfo, String> {
    println!("COMMAND: add_location called with path: {}", path);
    
    let root = PathBuf::from(&path);
    
    // Validate path exists and is a directory
    if !root.exists() {
        return Err("Path does not exist".to_string());
    }
    if !root.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    
    // Get or create location in database
    let location_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();
    
    let location_id = db
        .get_or_create_location(&path, &location_name)
        .await
        .map_err(|e| format!("Failed to create location: {}", e))?;
    
    // Start indexing in background
    let indexer = Indexer::new(app.clone(), db.inner());
    tokio::spawn(async move {
        indexer.start_scan(root).await;
    });
    
    Ok(LocationInfo {
        id: location_id,
        path,
        name: location_name,
    })
}

/// Remove a folder location and delete all associated images and thumbnails
#[tauri::command]
pub async fn remove_location(
    location_id: i64,
    app: AppHandle,
    db: State<'_, Arc<Db>>,
) -> Result<(), String> {
    println!("COMMAND: remove_location called for ID: {}", location_id);
    
    // Get thumbnail paths before deletion
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
    db.delete_location(location_id)
        .await
        .map_err(|e| format!("Failed to delete location: {}", e))?;
    
    println!("DEBUG: Location {} deleted successfully", location_id);
    Ok(())
}

/// Get all registered locations
#[tauri::command]
pub async fn get_locations(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<LocationInfo>, String> {
    let locations = db
        .get_locations()
        .await
        .map_err(|e| format!("Failed to get locations: {}", e))?;
    
    Ok(locations
        .into_iter()
        .map(|(id, path, name)| LocationInfo { id, path, name })
        .collect())
}

#[derive(Serialize)]
pub struct SubfolderInfo {
    pub id: i64,
    pub location_id: i64,
    pub parent_id: Option<i64>,
    pub relative_path: String,
    pub name: String,
}

/// Get all subfolders (for tree building)
#[tauri::command]
pub async fn get_all_subfolders(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<SubfolderInfo>, String> {
    println!("COMMAND: get_all_subfolders called");
    let subfolders = db
        .get_all_subfolders()
        .await
        .map_err(|e| format!("Failed to get subfolders: {}", e))?;
    
    println!("DEBUG: Found {} subfolders in database", subfolders.len());
    
    Ok(subfolders
        .into_iter()
        .map(|(id, location_id, parent_id, relative_path, name)| SubfolderInfo {
            id,
            location_id,
            parent_id,
            relative_path,
            name,
        })
        .collect())
}

/// Get image counts per subfolder
#[tauri::command]
pub async fn get_subfolder_counts(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<(i64, i64)>, String> {
    db.get_subfolder_counts()
        .await
        .map_err(|e| format!("Failed to get subfolder counts: {}", e))
}

/// Get image counts per location ROOT (images with no subfolder)
#[tauri::command]
pub async fn get_location_root_counts(
    db: State<'_, Arc<Db>>,
) -> Result<Vec<(i64, i64)>, String> {
    db.get_location_root_counts()
        .await
        .map_err(|e| format!("Failed to get location root counts: {}", e))
}
