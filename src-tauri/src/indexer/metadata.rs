use chrono::{DateTime, Utc};
use imagesize::size;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct ImageMetadata {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: i64,
    pub format: String,
    pub thumbnail_path: Option<String>,
    #[sqlx(default)]
    pub rating: i32,
    #[sqlx(default)]
    pub notes: Option<String>,
    pub modified_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    pub added_at: Option<DateTime<Utc>>,
}

pub fn get_image_metadata(path: &Path) -> Option<ImageMetadata> {
    let metadata = std::fs::metadata(path).ok()?;
    let file_size = metadata.len();

    let modified_at: DateTime<Utc> = metadata.modified().ok()?.into();
    let created_at: DateTime<Utc> = metadata
        .created()
        .ok()
        .map(|c| c.into())
        .unwrap_or(modified_at);

    let filename = path.file_name()?.to_string_lossy().to_string();
    let extension = path.extension()?.to_string_lossy().to_lowercase();

    // Fast header scanning for dimensions
    let (width, height) = match size(path) {
        Ok(s) => (Some(s.width as i32), Some(s.height as i32)),
        Err(_) => (None, None),
    };

    let format = match crate::formats::FileFormat::detect(path) {
        Some(fmt) => fmt.extensions[0].to_string(),
        None => extension,
    };

    Some(ImageMetadata {
        id: 0, // Placeholder for new files
        path: path.to_string_lossy().to_string(),
        filename,
        width,
        height,
        size: file_size as i64,
        format,
        thumbnail_path: None,
        rating: 0,
        notes: None,
        modified_at,
        created_at,
        added_at: None,
    })
}
