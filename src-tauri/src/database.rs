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

        Ok(Self { pool })
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
}
