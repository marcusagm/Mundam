mod database;
mod db_tags;
mod db_smart_folders;
mod ffmpeg;
mod indexer;
mod location_commands;
mod metadata_commands;
mod metadata_reader;
mod protocols;
mod search_logic;
mod smart_folder_commands;
mod tag_commands;
mod thumbnail_commands;
mod thumbnail_worker;
mod thumbnails;
mod db_settings;
mod settings_commands;


use crate::database::Db;
use crate::indexer::Indexer;
use std::path::PathBuf;
use tauri::Manager;

#[tauri::command]
async fn start_indexing(path: String, app: tauri::AppHandle) -> Result<(), String> {
    println!("COMMAND: start_indexing called with path: {}", path);

    // Get DB from state with safety
    let db = match app.try_state::<std::sync::Arc<Db>>() {
        Some(db) => db,
        None => return Err("Database not initialized".to_string()),
    };

    let registry = match app.try_state::<std::sync::Arc<tokio::sync::Mutex<crate::indexer::WatcherRegistry>>>() {
        Some(r) => r,
        None => return Err("Registry not initialized".to_string()),
    };

    let indexer = Indexer::new(app.clone(), db.inner(), registry.inner().clone());

    let root = PathBuf::from(path);
    indexer.start_scan(root).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Resolve paths
            let app_data = app
                .path()
                .app_local_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();

            let db_path = app_data.join("elleven.db");
            let thumbnails_dir = app_data.join("thumbnails");
            std::fs::create_dir_all(&thumbnails_dir).ok();

            // Initialize DB and Worker
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match Db::new(db_path).await {
                    Ok(db) => {
                        let db_arc = std::sync::Arc::new(db);
                        let watcher_registry = std::sync::Arc::new(tokio::sync::Mutex::new(crate::indexer::WatcherRegistry::default()));
                        
                        handle.manage(db_arc.clone());
                        handle.manage(watcher_registry.clone());

                        let worker = crate::thumbnail_worker::ThumbnailWorker::new(
                            db_arc.clone(),
                            thumbnails_dir,
                            handle.clone(),
                        );
                        worker.start().await;

                        // Start Watchers for Existing Roots
                        if let Ok(roots) = db_arc.get_all_root_folders().await {
                             println!("INFO: Starting watchers for {} roots", roots.len());
                             for (_id, path) in roots {
                                 let indexer = Indexer::new(handle.clone(), &db_arc, watcher_registry.clone());
                                 let root_path = std::path::PathBuf::from(path);
                                 indexer.start_scan(root_path).await;
                             }
                        }
                    }
                    Err(e) => eprintln!("Failed to initialize database: {}", e),
                }
            });

            Ok(())
        })
        .register_uri_scheme_protocol("thumb", move |ctx, request| {
            crate::protocols::thumb_handler(ctx.app_handle(), &request)
        })
        .register_uri_scheme_protocol("orig", move |ctx, request| {
            crate::protocols::orig_handler(ctx.app_handle(), &request)
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_mcp_bridge::init())
        .invoke_handler(tauri::generate_handler![
            start_indexing,
            tag_commands::create_tag,
            tag_commands::update_tag,
            tag_commands::delete_tag,
            tag_commands::get_all_tags,
            tag_commands::get_library_stats,
            tag_commands::add_tag_to_image,
            tag_commands::remove_tag_from_image,
            tag_commands::get_tags_for_image,
            tag_commands::add_tags_to_images_batch,
            tag_commands::get_images_filtered,
            tag_commands::get_image_count_filtered,
            tag_commands::update_image_rating,
            tag_commands::update_image_notes,
            metadata_commands::get_image_exif,
            thumbnail_commands::request_thumbnail_regenerate,
            location_commands::add_location,
            location_commands::remove_location,
            location_commands::get_locations,
            location_commands::get_all_subfolders,
            location_commands::get_subfolder_counts,
            location_commands::get_location_root_counts,
            smart_folder_commands::get_smart_folders,
            smart_folder_commands::save_smart_folder,
            smart_folder_commands::update_smart_folder,
            smart_folder_commands::delete_smart_folder,
            settings_commands::get_setting,
            settings_commands::set_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
