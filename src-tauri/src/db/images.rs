//! Image management and metadata queries.

use crate::db::models::ImageMetadata;
use super::Db;

impl Db {
    /// Updates the star rating for a specific image.
    pub async fn update_image_rating(&self, id: i64, rating: i32) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE images SET rating = ? WHERE id = ?", rating, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Updates the user notes for a specific image.
    pub async fn update_image_notes(&self, id: i64, notes: String) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE images SET notes = ? WHERE id = ?", notes, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retrieves images that do not have a thumbnail generated yet.
    pub async fn get_images_needing_thumbnails(
        &self,
        limit: i32,
    ) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT id, path FROM images WHERE thumbnail_path IS NULL AND thumbnail_attempts < 3 LIMIT ?",
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.id, r.path)).collect())
    }

    /// Retrieves specific images needing thumbnails by their IDs.
    pub async fn get_images_needing_thumbnails_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<(i64, String)>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT id, path FROM images WHERE id IN ({}) AND thumbnail_path IS NULL AND thumbnail_attempts < 3",
            placeholders.join(",")
        );

        let mut query_builder = sqlx::query_as::<_, (i64, String)>(&query);
        for id in ids {
            query_builder = query_builder.bind(id);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    /// Increments the thumbnail failure count and records the last error message.
    pub async fn record_thumbnail_error(&self, image_id: i64, error: String) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE images SET thumbnail_attempts = thumbnail_attempts + 1, thumbnail_last_error = ? WHERE id = ?",
            error,
            image_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Updates the path to the generated thumbnail for an image.
    pub async fn update_thumbnail_path(
        &self,
        image_id: i64,
        path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE images SET thumbnail_path = ? WHERE id = ?", path, image_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clears the thumbnail path, effectively flagging it for regeneration.
    pub async fn clear_thumbnail_path(&self, image_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE images SET thumbnail_path = NULL WHERE id = ?", image_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Saves or updates a single image record.
    ///
    /// Returns `(id, old_folder_id_if_moved, was_newly_inserted)`.
    pub async fn save_image(
        &self,
        folder_id: i64,
        img: &crate::db::models::ImageMetadata,
    ) -> Result<(i64, Option<i64>, bool), sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        self.save_image_internal(&mut *conn, folder_id, img).await
    }

    /// Batch saves multiple image records within a transaction.
    pub async fn save_images_batch(
        &self,
        items: Vec<(i64, crate::db::models::ImageMetadata)>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for (folder_id, img) in items {
            if let Err(e) = self.save_image_internal(&mut *tx, folder_id, &img).await {
                eprintln!("Failed to save image in batch: {}", e);
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Internal logic for saving/updating an image, reusable for transactions.
    async fn save_image_internal(
        &self,
        conn: &mut sqlx::SqliteConnection,
        folder_id: i64,
        img: &crate::db::models::ImageMetadata,
    ) -> Result<(i64, Option<i64>, bool), sqlx::Error> {
        // 1. Check if path already exists
        let existing: Option<(i64, i64)> = sqlx::query_as("SELECT id, folder_id FROM images WHERE path = ?")
            .bind(&img.path)
            .fetch_optional(&mut *conn)
            .await?;

        if let Some((id, old_fid)) = existing {
            sqlx::query!(
                "UPDATE images SET
                    folder_id = ?, filename = ?, width = ?, height = ?, size = ?, format = ?, modified_at = ?
                 WHERE path = ?",
                folder_id, img.filename, img.width, img.height, img.size, img.format, img.modified_at, img.path
            )
            .execute(&mut *conn)
            .await?;

            let old_fid_if_changed = if old_fid != folder_id { Some(old_fid) } else { None };
            return Ok((id, old_fid_if_changed, false));
        }

        // 2. Cross-root MOVE detection (fuzzy match by size and creation time if path is gone)
        let candidates: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT id, folder_id, path FROM images WHERE size = ? AND created_at = ?"
        )
        .bind(img.size)
        .bind(img.created_at)
        .fetch_all(&mut *conn)
        .await?;

        for (id, old_fid, old_path) in candidates {
            if !std::path::Path::new(&old_path).exists() {
                sqlx::query!(
                    "UPDATE images SET
                        path = ?, folder_id = ?, filename = ?, format = ?, modified_at = ?
                     WHERE id = ?",
                    img.path, folder_id, img.filename, img.format, img.modified_at, id
                )
                .execute(&mut *conn)
                .await?;
                return Ok((id, Some(old_fid), false));
            }
        }

        // 3. True New File
        let res = sqlx::query!(
            "INSERT INTO images (folder_id, path, filename, width, height, size, format, created_at, modified_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                folder_id = excluded.folder_id,
                filename = excluded.filename,
                width = excluded.width,
                height = excluded.height,
                size = excluded.size,
                format = excluded.format,
                modified_at = excluded.modified_at",
            folder_id, img.path, img.filename, img.width, img.height, img.size, img.format, img.created_at, img.modified_at
        )
        .execute(conn)
        .await?;

        Ok((res.last_insert_rowid(), None, true))
    }

    /// Retrieve context (image ID, folder ID, tags) for an image.
    pub async fn get_image_context(
        &self,
        path: &str
    ) -> Result<Option<(i64, i64, Vec<i64>)>, sqlx::Error> {
        let row = sqlx::query!("SELECT id as \"id!\", folder_id as \"folder_id!\" FROM images WHERE path = ?", path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            let tags = sqlx::query!("SELECT tag_id as \"tag_id!\" FROM image_tags WHERE image_id = ?", r.id)
                .fetch_all(&self.pool)
                .await?;

            let tag_ids = tags.into_iter().map(|t| t.tag_id).collect();
            Ok(Some((r.id, r.folder_id, tag_ids)))
        } else {
            Ok(None)
        }
    }

    /// Get size and creation date for comparison to detect file changes.
    pub async fn get_file_comparison_data(
        &self,
        path: &str
    ) -> Result<Option<(i64, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
        // Using explicit strings for cross-compatibility if needed, though Sqlite datetime usually maps well.
        let row: Option<(i64, String)> = sqlx::query_as("SELECT size, created_at FROM images WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some((s, c_at)) = row {
             let created_dt = chrono::DateTime::parse_from_rfc3339(&c_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
             Ok(Some((s, created_dt)))
        } else {
            Ok(None)
        }
    }

    /// Retrieves comparison data (size, modified_at) for all images under a root path.
    /// Used for fast initial scanning.
    pub async fn get_all_files_comparison_data(
        &self,
        root_path: &str,
    ) -> Result<std::collections::HashMap<String, (i64, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
        let pattern = format!("{}%", root_path);
        let rows: Vec<(String, i64, String)> = sqlx::query_as(
            "SELECT path, size, modified_at FROM images WHERE path LIKE ?"
        )
        .bind(pattern)
        .fetch_all(&self.pool)
        .await?;

        let mut map = std::collections::HashMap::with_capacity(rows.len());
        for (path, size, m_at) in rows {
            let dt = chrono::DateTime::parse_from_rfc3339(&m_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            map.insert(path, (size, dt));
        }
        Ok(map)
    }

    /// Deletes an image record and returns its metadata context.
    pub async fn delete_image_by_path_returning_context(
        &self,
        path: &str
    ) -> Result<Option<(i64, i64, Vec<i64>)>, sqlx::Error> {
        let context = self.get_image_context(path).await?;

        if let Some((image_id, _, _)) = context {
            sqlx::query!("DELETE FROM images WHERE id = ?", image_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(context)
    }

    /// Updates image metadata due to a rename or move operation on the filesystem.
    pub async fn rename_image(
        &self,
        old_path: &str,
        new_path: &str,
        new_filename: &str,
        new_folder_id: i64
    ) -> Result<Option<(ImageMetadata, i64)>, sqlx::Error> {
        let row: Option<(i64, i64, i32, i32, i64, String, String, String, Option<String>, i32, Option<String>)> = sqlx::query_as(
            "SELECT id, folder_id, width, height, size, format, created_at, modified_at, thumbnail_path, rating, notes FROM images WHERE path = ?"
        )
        .bind(old_path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, old_folder_id, w, h, s, f, c_at, _m_at, thumb, rating, notes)) = row {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query!(
                "UPDATE images SET path = ?, filename = ?, folder_id = ?, modified_at = ? WHERE id = ?",
                new_path, new_filename, new_folder_id, now, id
            )
            .execute(&self.pool)
            .await?;

            let created_dt = chrono::DateTime::parse_from_rfc3339(&c_at).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_else(|_| chrono::Utc::now());
            let modified_dt = chrono::Utc::now();

            Ok(Some((ImageMetadata {
                id,
                path: new_path.to_string(),
                filename: new_filename.to_string(),
                width: Some(w),
                height: Some(h),
                size: s,
                created_at: created_dt,
                modified_at: modified_dt,
                thumbnail_path: thumb,
                rating,
                notes,
                format: f,
                added_at: None,
            }, old_folder_id)))
        } else {
            Ok(None)
        }
    }
}
