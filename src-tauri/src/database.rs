use crate::indexer::metadata::ImageMetadata;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn new(path: PathBuf) -> Result<Self, sqlx::Error> {
        // Ensure the database file is created if it doesn't exist
        use sqlx::sqlite::SqliteConnectOptions;
        use sqlx::Executor;
        use std::str::FromStr;

        let url = format!("sqlite:{}", path.to_string_lossy());
        let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migration/schema
        let schema = include_str!("schema.sql");
        pool.execute(schema).await?;

        // Manual migration for existing databases (SQLite "ADD COLUMN IF NOT EXISTS" workaround)
        // We attempt to add columns and ignore errors if they exist
        let _ = sqlx::query("ALTER TABLE images ADD COLUMN rating INTEGER DEFAULT 0")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE images ADD COLUMN notes TEXT")
            .execute(&pool)
            .await;

        Ok(Self { pool })
    }

    pub async fn update_image_rating(&self, id: i64, rating: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET rating = ? WHERE id = ?")
            .bind(rating)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_image_notes(&self, id: i64, notes: String) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET notes = ? WHERE id = ?")
            .bind(notes)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_images_batch(
        &self,
        location_id: i64,
        images: Vec<ImageMetadata>,
    ) -> Result<(), sqlx::Error> {
        if images.is_empty() {
            return Ok(());
        }

        // We use a manual transaction for speed
        let mut tx = self.pool.begin().await?;

        for img in images {
            // Updated to use UPSERT logic tailored for SQLite
            // We want to insert if new, or update metadata if exists, BUT PRESERVE thumbnail_path
            sqlx::query(
                "INSERT INTO images (location_id, path, filename, width, height, size, created_at, modified_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(path) DO UPDATE SET
                    location_id=excluded.location_id,
                    filename=excluded.filename,
                    width=excluded.width,
                    height=excluded.height,
                    size=excluded.size,
                    modified_at=excluded.modified_at
                 -- Intentionally NOT updating thumbnail_path to preserve it"
            )
            .bind(location_id)
            .bind(&img.path)
            .bind(&img.filename)
            .bind(img.width.map(|w| w as i32))
            .bind(img.height.map(|h| h as i32))
            .bind(img.size as i64)
            .bind(img.created_at)
            .bind(img.modified_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_or_create_location(&self, path: &str, name: &str) -> Result<i64, sqlx::Error> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT id FROM locations WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some((id,)) = row {
            Ok(id)
        } else {
            let res = sqlx::query("INSERT INTO locations (path, name) VALUES (?, ?)")
                .bind(path)
                .bind(name)
                .execute(&self.pool)
                .await?;
            Ok(res.last_insert_rowid())
        }
    }

    pub async fn get_images_needing_thumbnails(
        &self,
        limit: i32,
    ) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT id, path FROM images WHERE thumbnail_path IS NULL LIMIT ?")
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    pub async fn update_thumbnail_path(
        &self,
        image_id: i64,
        path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET thumbnail_path = ? WHERE id = ?")
            .bind(path)
            .bind(image_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear thumbnail path to trigger regeneration by the worker
    pub async fn clear_thumbnail_path(&self, image_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET thumbnail_path = NULL WHERE id = ?")
            .bind(image_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    /// Get all thumbnail paths for a location (for cleanup before deletion)
    pub async fn get_location_thumbnails(&self, location_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT thumbnail_path FROM images WHERE location_id = ? AND thumbnail_path IS NOT NULL"
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|(path,)| path).collect())
    }
    
    /// Delete a location and all its images
    pub async fn delete_location(&self, location_id: i64) -> Result<(), sqlx::Error> {
        // First delete all images for this location
        sqlx::query("DELETE FROM images WHERE location_id = ?")
            .bind(location_id)
            .execute(&self.pool)
            .await?;
        
        // Also delete any tags associated with those images
        sqlx::query("DELETE FROM image_tags WHERE image_id NOT IN (SELECT id FROM images)")
            .execute(&self.pool)
            .await?;
        
        // Then delete the location itself
        sqlx::query("DELETE FROM locations WHERE id = ?")
            .bind(location_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// Get all locations
    pub async fn get_locations(&self) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT id, path, name FROM locations ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }
}
