pub mod db;
pub mod error;
mod indexer;
// Moved to media: metadata_reader, ffmpeg
mod protocols;
// Moved to thumbnails: thumbnail_worker, thumbnail_priority
mod thumbnails;
pub mod formats;
// Moved to settings: config
mod transcoding;
mod streaming;
pub mod library;
mod media;
mod settings;


use crate::db::Db;
use crate::indexer::Indexer;
use tauri::Manager;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    crate::protocols::register_all(builder)
        .setup(|app| {
            // Resolve paths
            let app_data = app
                .path()
                .app_local_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();

            let db_path = app_data.join("mundam.db");
            let thumbnails_dir = app_data.join("thumbnails");
            std::fs::create_dir_all(&thumbnails_dir).ok();

            // Initialize DB and Worker
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match Db::new(db_path).await {
                    Ok(db) => {
                        let db_arc = std::sync::Arc::new(db);
                        let watcher_registry = std::sync::Arc::new(tokio::sync::Mutex::new(crate::indexer::WatcherRegistry::default()));

                        // Load Config
                        let app_config = crate::settings::config::load_config(&db_arc).await;
                        let config_state = crate::settings::config::ConfigState(std::sync::Mutex::new(app_config.clone()));

                        let priority_state = std::sync::Arc::new(crate::thumbnails::priority::ThumbnailPriorityState::default());

                        handle.manage(db_arc.clone());
                        handle.manage(watcher_registry.clone());
                        handle.manage(config_state);
                        handle.manage(priority_state.clone());

                        let worker = crate::thumbnails::worker::ThumbnailWorker::new(
                            db_arc.clone(),
                            thumbnails_dir,
                            handle.clone(),
                            app_config,
                            priority_state,
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

            // Start HLS Streaming Server
            crate::streaming::server::spawn_server(app.handle().clone());

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_mcp_bridge::init())
        .invoke_handler(tauri::generate_handler![
            library::commands::indexing::start_indexing,
            library::commands::tags::create_tag,
            library::commands::tags::update_tag,
            library::commands::tags::delete_tag,
            library::commands::tags::get_all_tags,
            library::commands::tags::get_library_stats,
            library::commands::tags::add_tag_to_image,
            library::commands::tags::remove_tag_from_image,
            library::commands::tags::get_tags_for_image,
            library::commands::tags::add_tags_to_images_batch,
            library::commands::tags::get_images_filtered,
            library::commands::tags::get_image_count_filtered,
            library::commands::tags::update_image_rating,
            library::commands::tags::update_image_notes,
            library::commands::metadata::get_image_exif,
            thumbnails::commands::request_thumbnail_regenerate,
            thumbnails::commands::set_thumbnail_priority,
            library::commands::folders::add_location,
            library::commands::folders::remove_location,
            library::commands::folders::get_locations,
            library::commands::folders::get_all_subfolders,
            library::commands::folders::get_subfolder_counts,
            library::commands::folders::get_location_root_counts,
            library::commands::smart_folders::get_smart_folders,
            library::commands::smart_folders::save_smart_folder,
            library::commands::smart_folders::update_smart_folder,
            library::commands::smart_folders::delete_smart_folder,
            settings::commands::get_setting,
            settings::commands::set_setting,
            settings::commands::run_db_maintenance,

            library::commands::formats::get_library_supported_formats,
            media::commands::get_audio_waveform_data,

            // Transcoding commands
            transcoding::commands::needs_transcoding,
            transcoding::commands::is_native_format,
            transcoding::commands::get_stream_url,
            transcoding::commands::get_quality_options,
            transcoding::commands::transcode_file,
            transcoding::commands::is_cached,
            transcoding::commands::get_cache_stats,
            transcoding::commands::cleanup_cache,
            transcoding::commands::clear_cache,
            transcoding::commands::ffmpeg_available
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
