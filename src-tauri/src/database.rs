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

        // Manual migrations for existing databases BEFORE schema
        // Step 1: Create subfolders table if not exists (needed before adding FK to images)
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS subfolders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                location_id INTEGER NOT NULL,
                parent_id INTEGER,
                relative_path TEXT NOT NULL,
                name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_id) REFERENCES subfolders(id) ON DELETE CASCADE,
                UNIQUE(location_id, relative_path)
            )"
        )
        .execute(&pool)
        .await;

        // Step 2: Add columns to images table (ignore if already exist)
        let _ = sqlx::query("ALTER TABLE images ADD COLUMN rating INTEGER DEFAULT 0")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE images ADD COLUMN notes TEXT")
            .execute(&pool)
            .await;
        // Add subfolder_id WITHOUT foreign key constraint (SQLite limitation with ALTER)
        let _ = sqlx::query("ALTER TABLE images ADD COLUMN subfolder_id INTEGER")
            .execute(&pool)
            .await;

        // Step 3: Create indices if they don't exist
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_subfolder ON images(subfolder_id)")
            .execute(&pool)
            .await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_subfolders_location ON subfolders(location_id)")
            .execute(&pool)
            .await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_subfolders_parent ON subfolders(parent_id)")
            .execute(&pool)
            .await;

        // Run main schema for new databases (tables already exist will be skipped)
        let schema = include_str!("schema.sql");
        pool.execute(schema).await?;

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
    
    /// Get or create a subfolder
    pub async fn get_or_create_subfolder(
        &self,
        location_id: i64,
        relative_path: &str,
        name: &str,
        parent_id: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        // Try to find existing
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM subfolders WHERE location_id = ? AND relative_path = ?"
        )
        .bind(location_id)
        .bind(relative_path)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some((id,)) = row {
            Ok(id)
        } else {
            let res = sqlx::query(
                "INSERT INTO subfolders (location_id, parent_id, relative_path, name) VALUES (?, ?, ?, ?)"
            )
            .bind(location_id)
            .bind(parent_id)
            .bind(relative_path)
            .bind(name)
            .execute(&self.pool)
            .await?;
            Ok(res.last_insert_rowid())
        }
    }
    
    /// Get all subfolders for a location
    pub async fn get_subfolders(&self, location_id: i64) -> Result<Vec<(i64, Option<i64>, String, String)>, sqlx::Error> {
        let rows: Vec<(i64, Option<i64>, String, String)> = sqlx::query_as(
            "SELECT id, parent_id, relative_path, name FROM subfolders WHERE location_id = ? ORDER BY relative_path"
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }
    
    /// Get all subfolders for all locations (for tree building)
    pub async fn get_all_subfolders(&self) -> Result<Vec<(i64, i64, Option<i64>, String, String)>, sqlx::Error> {
        let rows: Vec<(i64, i64, Option<i64>, String, String)> = sqlx::query_as(
            "SELECT id, location_id, parent_id, relative_path, name FROM subfolders ORDER BY location_id, relative_path"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }
    
    /// Count images per subfolder
    pub async fn get_subfolder_counts(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT subfolder_id, COUNT(*) as count FROM images WHERE subfolder_id IS NOT NULL GROUP BY subfolder_id"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    /// Get image counts per location ROOT (images with no subfolder)
    pub async fn get_location_root_counts(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT location_id, COUNT(*) as count FROM images WHERE subfolder_id IS NULL GROUP BY location_id"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }
    
    /// Save image with subfolder_id
    pub async fn save_image_with_subfolder(
        &self,
        location_id: i64,
        subfolder_id: Option<i64>,
        img: &crate::indexer::metadata::ImageMetadata,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO images (location_id, subfolder_id, path, filename, width, height, size, created_at, modified_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                location_id=excluded.location_id,
                subfolder_id=excluded.subfolder_id,
                filename=excluded.filename,
                width=excluded.width,
                height=excluded.height,
                size=excluded.size,
                modified_at=excluded.modified_at"
        )
        .bind(location_id)
        .bind(subfolder_id)
        .bind(&img.path)
        .bind(&img.filename)
        .bind(img.width.map(|w| w as i32))
        .bind(img.height.map(|h| h as i32))
        .bind(img.size as i64)
        .bind(img.created_at)
        .bind(img.modified_at)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

