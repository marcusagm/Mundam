use chrono::{DateTime, Utc};
use imagesize::size;
use std::path::Path;
use crate::db::models::ImageMetadata;

pub fn get_image_metadata(path: &Path) -> Option<ImageMetadata> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();
    let created_at: DateTime<Utc> = metadata.created().ok().map(|c| c.into()).unwrap_or(modified_at);

    let (width, height) = match size(path) {
        Ok(dim) => (Some(dim.width as i32), Some(dim.height as i32)),
        Err(_) => (None, None),
    };

    let filename = path.file_name()?.to_string_lossy().to_string();
    let format = path.extension()?.to_string_lossy().to_string().to_lowercase();

    Some(ImageMetadata {
        id: 0,
        path: path.to_string_lossy().to_string(),
        filename,
        width,
        height,
        size: metadata.len() as i64,
        format,
        thumbnail_path: None,
        rating: 0,
        notes: None,
        modified_at,
        created_at,
        added_at: None,
    })
}
