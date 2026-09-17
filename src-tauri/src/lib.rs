#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod bootstrap;
pub mod core;
pub mod delivery;
pub mod feature;
pub mod infra;
pub mod lifecycle;
pub mod processing;
use crate::lifecycle::LifecycleRegistry;
use tauri::Manager;

/// Runs the application.
#[allow(clippy::expect_used)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize structured tracing
    crate::infra::telemetry::init_telemetry();

    let builder = tauri::Builder::default().plugin(tauri_plugin_clipboard_manager::init());
    builder
        .register_uri_scheme_protocol("thumb", move |ctx, request| {
            crate::delivery::protocols::asset::handler(ctx.app_handle(), &request)
        })
        .register_uri_scheme_protocol("asset", move |ctx, request| {
            crate::delivery::protocols::asset::handler(ctx.app_handle(), &request)
        })
        .register_uri_scheme_protocol("video", move |ctx, request| {
            crate::delivery::protocols::video::handler(ctx.app_handle(), &request)
        })
        .register_uri_scheme_protocol("audio", move |ctx, request| {
            crate::delivery::protocols::audio::handler(ctx.app_handle(), &request)
        })
        .setup(|app| {
            let app_handle = app.handle();

            crate::bootstrap::system::init_directories(app_handle);
            crate::bootstrap::system::init_lifecycle(app_handle);
            crate::bootstrap::system::init_settings(app_handle);
            crate::bootstrap::system::init_events(app_handle);
            crate::bootstrap::system::init_formats(app_handle);

            let handle = app_handle.clone();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = crate::bootstrap::database::init(&handle).await {
                    tracing::error!("Failed to initialize database: {}", e);
                    return;
                }

                crate::bootstrap::streaming::init(&handle).await;
                crate::bootstrap::workers::init(&handle);
                crate::bootstrap::library::init(&handle).await;
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            // IPC Commands
            delivery::tauri::commands::queries::get_assets,
            delivery::tauri::commands::queries::get_asset,
            delivery::tauri::commands::queries::list_folders,
            delivery::tauri::commands::queries::list_tags,
            delivery::tauri::commands::queries::search_assets,
            delivery::tauri::commands::queries::get_tags_for_asset,
            delivery::tauri::commands::queries::get_all_subfolders,
            delivery::tauri::commands::queries::get_subfolder_counts,
            delivery::tauri::commands::queries::get_location_root_counts,
            delivery::tauri::commands::queries::get_smart_folders,
            delivery::tauri::commands::queries::get_library_stats,
            delivery::tauri::commands::queries::get_asset_exif,
            delivery::tauri::commands::queries::get_asset_colors,
            delivery::tauri::commands::queries::get_library_cache_stats,
            delivery::tauri::commands::mutations::create_folder,
            delivery::tauri::commands::mutations::remove_location,
            delivery::tauri::commands::mutations::start_indexing,
            delivery::tauri::commands::mutations::set_asset_folder,
            delivery::tauri::commands::mutations::update_asset_tags,
            delivery::tauri::commands::mutations::create_tag,
            delivery::tauri::commands::mutations::update_tag,
            delivery::tauri::commands::mutations::delete_tag,
            delivery::tauri::commands::mutations::add_tags_to_assets_batch,
            delivery::tauri::commands::mutations::remove_tags_from_assets_batch,
            delivery::tauri::commands::mutations::replace_tags_for_assets_batch,
            delivery::tauri::commands::mutations::save_smart_folder,
            delivery::tauri::commands::mutations::update_smart_folder,
            delivery::tauri::commands::mutations::delete_smart_folder,
            delivery::tauri::commands::mutations::update_asset_rating,
            delivery::tauri::commands::mutations::update_asset_notes,
            delivery::tauri::commands::mutations::reextract_asset_colors,
            delivery::tauri::commands::mutations::request_thumbnail_regenerate,
            delivery::tauri::commands::mutations::run_db_maintenance,
            delivery::tauri::commands::mutations::send_telemetry_log,
            delivery::tauri::commands::mutations::verify_thumbnails,
            delivery::tauri::commands::mutations::cleanup_cache,
            delivery::tauri::commands::mutations::clear_cache,
            
            // Duplicates
            delivery::tauri::commands::duplicates::get_duplicate_groups,
            delivery::tauri::commands::duplicates::get_duplicate_candidates,
            delivery::tauri::commands::duplicates::resolve_duplicate_group,
            delivery::tauri::commands::duplicates::start_duplicate_scan,
            delivery::tauri::commands::duplicates::cancel_duplicate_scan,
            delivery::tauri::commands::mutations::copy_files_to_clipboard,
            delivery::tauri::commands::mutations::rename_file,
            delivery::tauri::commands::mutations::toggle_favorite,
            delivery::tauri::commands::mutations::move_to_trash,
            delivery::tauri::commands::mutations::restore_from_trash,
            delivery::tauri::commands::mutations::empty_trash,
            delivery::tauri::commands::queries::get_library_supported_formats,
            delivery::tauri::commands::queries::get_audio_waveform_data,
            delivery::tauri::thumbnails::set_thumbnail_priority,
            // Settings Commands
            delivery::tauri::commands::settings::get_app_settings,
            delivery::tauri::commands::settings::update_app_settings,
            delivery::tauri::commands::settings::get_setting,
            delivery::tauri::commands::settings::set_setting,
            delivery::tauri::commands::queries::get_streaming_token,
            delivery::tauri::commands::streaming::needs_transcoding,
            delivery::tauri::commands::streaming::is_native_format,
            delivery::tauri::commands::streaming::get_stream_url,
            delivery::tauri::commands::streaming::get_quality_options,
            delivery::tauri::commands::streaming::ffmpeg_available,
            delivery::tauri::commands::streaming::is_cached,
            delivery::tauri::commands::streaming::get_streaming_cache_stats,
            delivery::tauri::commands::streaming::transcode_file,
            delivery::tauri::commands::streaming::cleanup_cache_streaming,
            delivery::tauri::commands::streaming::clear_cache_streaming
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                tracing::info!("Close requested. Orchestrating graceful shutdown.");
                api.prevent_close();

                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(lifecycle) = handle.try_state::<std::sync::Arc<LifecycleRegistry>>()
                    {
                        lifecycle.shutdown_all().await;
                    }
                    handle.exit(0);
                });
            }
        });
}
