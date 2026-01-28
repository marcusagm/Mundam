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

        // Initialize schema if tables don't exist
        
        // Run main schema
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


    pub async fn get_folder_by_path(&self, path: &str) -> Result<Option<i64>, sqlx::Error> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT id FROM folders WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn upsert_folder(
        &self,
        path: &str,
        name: &str,
        parent_id: Option<i64>,
        is_root: bool,
    ) -> Result<i64, sqlx::Error> {
        // Try to find existing
        if let Some(id) = self.get_folder_by_path(path).await? {
            // Update parent_id if it was NULL and now we know it
            // Only update is_root if true (don't demote a root to non-root automatically, 
            // though user logic might want that, for now let's assume we just ensure it exist)
            // Actually for "Is Root", if we are scanning a root, we set it.
            if is_root || parent_id.is_some() {
                 let mut query = "UPDATE folders SET ".to_string();
                 let mut updates = Vec::new();
                 if is_root { updates.push("is_root = 1"); }
                 if parent_id.is_some() { updates.push("parent_id = ?"); }
                 
                 query.push_str(&updates.join(", "));
                 query.push_str(" WHERE id = ?");
                 
                 let mut q = sqlx::query(&query);
                 if is_root { /* no bind needed for const */ }
                 if let Some(pid) = parent_id { q = q.bind(pid); }
                 q = q.bind(id);
                 q.execute(&self.pool).await?;
            }
            return Ok(id);
        }

        let res = sqlx::query(
            "INSERT INTO folders (path, name, parent_id, is_root) VALUES (?, ?, ?, ?)"
        )
        .bind(path)
        .bind(name)
        .bind(parent_id)
        .bind(is_root)
        .execute(&self.pool)
        .await?;
        
        Ok(res.last_insert_rowid())
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
    
    pub async fn get_location_thumbnails(&self, location_id: i64) -> Result<Vec<String>, sqlx::Error> {
        // Use CTE to get all descendants
        let rows: Vec<(String,)> = sqlx::query_as(
            "WITH RECURSIVE family AS (
                SELECT id FROM folders WHERE id = ?
                UNION ALL
                SELECT f.id FROM folders f JOIN family ON f.parent_id = family.id
             )
             SELECT thumbnail_path FROM images WHERE folder_id IN family AND thumbnail_path IS NOT NULL"
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|(path,)| path).collect())
    }
    
    pub async fn delete_folder(&self, folder_id: i64) -> Result<(), sqlx::Error> {
        // CASCADE delete in schema handles children folders and images
        sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(folder_id)
            .execute(&self.pool)
            .await?;
            
        // Clean up orphan tags? (Maybe later)
        Ok(())
    }

    pub async fn adopt_orphaned_children(&self, parent_id: i64, parent_path: &str) -> Result<(), sqlx::Error> {
        // Find existing ROOT folders that should differ to this new parent
        // Use standard separator '/' or platform specific? 
        // Paths in DB are full paths.
        // We assume standard path formatting logic (Rust to_string_lossy).
        // Let's use a LIKE query with safe binding.
        // We match parent_path + separator + %.
        // Note: Windows paths might use backslash. The `path` string format depends on how `PathBuf` stringifies.
        // On Mac it uses forward slash.
        
        let path_pattern = format!("{}/%", parent_path); // Append separator
        
        // Update any root folder that logicially belongs to this new parent
        sqlx::query(
            "UPDATE folders 
             SET parent_id = ?, is_root = 0 
             WHERE is_root = 1 
             AND path LIKE ?"
        )
        .bind(parent_id)
        .bind(path_pattern)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }


    pub async fn get_folder_hierarchy(&self) -> Result<Vec<(i64, Option<i64>, String, String, bool)>, sqlx::Error> {
        // Return id, parent_id, path, name, is_root
        let rows: Vec<(i64, Option<i64>, String, String, bool)> = sqlx::query_as(
            "SELECT id, parent_id, path, name, is_root FROM folders ORDER BY path"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    
    pub async fn get_folder_counts_recursive(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        // Recursive count of images in folders
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "WITH RECURSIVE folder_tree AS (
                -- Base case: Direct folder
                SELECT id as root_id, id as child_id
                FROM folders
                
                UNION ALL
                
                -- Recursive case: Child folders
                SELECT ft.root_id, f.id
                FROM folders f
                JOIN folder_tree ft ON f.parent_id = ft.child_id
            )
            SELECT ft.root_id as folder_id, COUNT(i.id) as count
            FROM folder_tree ft
            JOIN images i ON i.folder_id = ft.child_id
            GROUP BY ft.root_id"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn get_folder_counts_direct(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        // Direct image counts
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT folder_id, COUNT(*) as count FROM images GROUP BY folder_id"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn save_image(
        &self,
        folder_id: i64,
        img: &crate::indexer::metadata::ImageMetadata,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO images (folder_id, path, filename, width, height, size, created_at, modified_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                folder_id=excluded.folder_id,
                filename=excluded.filename,
                width=excluded.width,
                height=excluded.height,
                size=excluded.size,
                modified_at=excluded.modified_at"
        )
        .bind(folder_id)
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

