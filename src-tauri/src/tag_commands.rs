use crate::db::Db;
use crate::db::models::{Tag, ImageMetadata, LibraryStats};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn create_tag(
    db: State<'_, Arc<Db>>,
    name: String,
    parent_id: Option<i64>,
    color: Option<String>,
) -> Result<i64, String> {
    db.create_tag(&name, parent_id, color)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag(
    db: State<'_, Arc<Db>>,
    id: i64,
    name: Option<String>,
    color: Option<String>,
    parent_id: Option<i64>,
    order_index: Option<i64>,
) -> Result<(), String> {
    db.update_tag(id, name, color, parent_id, order_index)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(db: State<'_, Arc<Db>>, id: i64) -> Result<(), String> {
    db.delete_tag(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_tags(db: State<'_, Arc<Db>>) -> Result<Vec<Tag>, String> {
    db.get_all_tags().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_library_stats(
    db: State<'_, Arc<Db>>,
) -> Result<LibraryStats, String> {
    db.get_library_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag_to_image(
    db: State<'_, Arc<Db>>,
    image_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.add_tag_to_image(image_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_image(
    db: State<'_, Arc<Db>>,
    image_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.remove_tag_from_image(image_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tags_for_image(db: State<'_, Arc<Db>>, image_id: i64) -> Result<Vec<Tag>, String> {
    db.get_tags_for_image(image_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tags_to_images_batch(
    db: State<'_, Arc<Db>>,
    image_ids: Vec<i64>,
    tag_ids: Vec<i64>,
) -> Result<(), String> {
    db.add_tags_to_images_batch(image_ids, tag_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_images_filtered(
    db: State<'_, Arc<Db>>,
    limit: i32,
    offset: i32,
    tag_ids: Vec<i64>,
    match_all: bool,
    untagged: Option<bool>,
    folder_id: Option<i64>,
    recursive: bool,
    sort_by: Option<String>,
    sort_order: Option<String>,
    advanced_query: Option<String>,
    search_query: Option<String>,
) -> Result<Vec<ImageMetadata>, String> {
    db.get_images_filtered(limit, offset, tag_ids, match_all, untagged, folder_id, recursive, sort_by, sort_order, advanced_query, search_query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_image_count_filtered(
    db: State<'_, Arc<Db>>,
    tag_ids: Vec<i64>,
    match_all: bool,
    untagged: Option<bool>,
    folder_id: Option<i64>,
    recursive: bool,
    advanced_query: Option<String>,
    search_query: Option<String>,
) -> Result<i64, String> {
    db.get_image_count_filtered(tag_ids, match_all, untagged, folder_id, recursive, advanced_query, search_query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_image_rating(
    db: State<'_, Arc<Db>>,
    id: i64,
    rating: i32,
) -> Result<(), String> {
    db.update_image_rating(id, rating)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_image_notes(
    db: State<'_, Arc<Db>>,
    id: i64,
    notes: String,
) -> Result<(), String> {
    db.update_image_notes(id, notes)
        .await
        .map_err(|e| e.to_string())
}
