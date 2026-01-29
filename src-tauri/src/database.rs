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


    pub async fn get_folder_path(&self, id: i64) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as("SELECT path FROM folders WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn get_folder_by_path(&self, path: &str) -> Result<Option<i64>, sqlx::Error> {
        let path = path.trim_end_matches('/');
        let row: Option<(i64,)> = sqlx::query_as("SELECT id FROM folders WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;
        
        if row.is_some() {
            Ok(row.map(|r| r.0))
        } else {
            // MacOS case-insensitivity fallback
            let row_loose: Option<(i64,)> = sqlx::query_as("SELECT id FROM folders WHERE path = ? COLLATE NOCASE")
                .bind(path)
                .fetch_optional(&self.pool)
                .await?;
            Ok(row_loose.map(|r| r.0))
        }
    }

    pub async fn upsert_folder(
        &self,
        path: &str,
        name: &str,
        parent_id: Option<i64>,
        is_root: bool,
    ) -> Result<i64, sqlx::Error> {
        let path = path.trim_end_matches('/');
        
        // Try to find existing
        if let Some(id) = self.get_folder_by_path(path).await? {
            // PROTECTION: If this folder is already marked as a root and the new call 
            // is trying to treat it as a child (is_root=false), do NOT demote it.
            let existing: Option<(bool, Option<i64>)> = sqlx::query_as("SELECT is_root, parent_id FROM folders WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

            if let Some((existing_is_root, _)) = existing {
                if existing_is_root && !is_root {
                    return Ok(id);
                }
            }

            // Update parent_id if it was NULL or if we are setting root
            if is_root || parent_id.is_some() {
                 let mut query = "UPDATE folders SET ".to_string();
                 let mut updates = Vec::new();
                 if is_root { updates.push("is_root = 1"); updates.push("parent_id = NULL"); }
                 else if parent_id.is_some() { updates.push("parent_id = ?"); }
                 
                 query.push_str(&updates.join(", "));
                 query.push_str(" WHERE id = ?");
                 
                 let mut q = sqlx::query(&query);
                 if !is_root && parent_id.is_some() { q = q.bind(parent_id); }
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
        .await;
        
        match res {
            Ok(r) => Ok(r.last_insert_rowid()),
            Err(e) => {
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("2067") {
                         return self.get_folder_by_path(path).await?.ok_or(e);
                    }
                }
                Err(e)
            }
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
            LEFT JOIN images i ON i.folder_id = ft.child_id
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

    pub async fn ensure_folder_hierarchy(&self, path: &str) -> Result<i64, sqlx::Error> {
        let path = path.trim_end_matches('/');
        
        // 1. Check if this path is already a known root. If so, stop climbing.
        let existing = sqlx::query_as::<_, (i64, bool)>("SELECT id, is_root FROM folders WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some((id, is_root)) = existing {
            if is_root { return Ok(id); }
        }

        // 2. Find parent context.
        let path_obj = std::path::Path::new(path);
        let name = path_obj.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let parent_id = if let Some(parent) = path_obj.parent() {
            let parent_str = parent.to_string_lossy();
            
            if let Some(pid) = self.get_folder_by_path(&parent_str).await? {
                Some(pid)
            } else {
                // Parent not found. Recursively ensure parent.
                if parent_str.len() > 1 && parent_str != "/" {
                     Some(Box::pin(self.ensure_folder_hierarchy(&parent_str)).await?)
                } else {
                    None 
                }
            }
        } else {
            None
        };

        // 3. Create this folder (linked to parent)
        self.upsert_folder(path, &name, parent_id, false).await
    }
    pub async fn save_image(
        &self,
        folder_id: i64,
        img: &crate::indexer::metadata::ImageMetadata,
    ) -> Result<(i64, Option<i64>, bool), sqlx::Error> {
        // 1. Check if path already exists
        let existing: Option<(i64, i64)> = sqlx::query_as("SELECT id, folder_id FROM images WHERE path = ?")
            .bind(&img.path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some((id, old_fid)) = existing {
            // Standard Update
            sqlx::query(
                "UPDATE images SET 
                    folder_id = ?, filename = ?, width = ?, height = ?, size = ?, modified_at = ? 
                 WHERE path = ?"
            )
            .bind(folder_id)
            .bind(&img.filename)
            .bind(img.width)
            .bind(img.height)
            .bind(img.size)
            .bind(img.modified_at)
            .bind(&img.path)
            .execute(&self.pool)
            .await?;
            
            let old_fid_if_changed = if old_fid != folder_id { Some(old_fid) } else { None };
            return Ok((id, old_fid_if_changed, false));
        }

        // 2. Not in path. Could be a cross-root MOVE.
        let candidates: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT id, folder_id, path FROM images WHERE size = ? AND created_at = ?"
        )
        .bind(img.size)
        .bind(img.created_at)
        .fetch_all(&self.pool)
        .await?;

        for (id, old_fid, old_path) in candidates {
            if !std::path::Path::new(&old_path).exists() {
                println!("DEBUG: DB - Adopting 'lost' image record {} for new path: {}", id, img.path);
                sqlx::query(
                    "UPDATE images SET 
                        path = ?, folder_id = ?, filename = ?, modified_at = ? 
                     WHERE id = ?"
                )
                .bind(&img.path)
                .bind(folder_id)
                .bind(&img.filename)
                .bind(img.modified_at)
                .bind(id)
                .execute(&self.pool)
                .await?;
                return Ok((id, Some(old_fid), false));
            }
        }

        // 3. True New File
        let res = sqlx::query(
            "INSERT INTO images (folder_id, path, filename, width, height, size, created_at, modified_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(folder_id)
        .bind(&img.path)
        .bind(&img.filename)
        .bind(img.width)
        .bind(img.height)
        .bind(img.size)
        .bind(img.created_at)
        .bind(img.modified_at)
        .execute(&self.pool)
        .await?;
        
        Ok((res.last_insert_rowid(), None, true))
    }

    /// Retrieve context (folder, tags) for an image before deletion
    pub async fn get_image_context(
        &self,
        path: &str
    ) -> Result<Option<(i64, i64, Vec<i64>)>, sqlx::Error> {
        // Returns (image_id, folder_id, vec<tag_id>)
        // We first get image ID and folder ID
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT id, folder_id FROM images WHERE path = ?"
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((image_id, folder_id)) = row {
            // Get tags
            let tags: Vec<(i64,)> = sqlx::query_as(
                "SELECT tag_id FROM image_tags WHERE image_id = ?"
            )
            .bind(image_id)
            .fetch_all(&self.pool)
            .await?;

            let tag_ids = tags.into_iter().map(|(id,)| id).collect();
            Ok(Some((image_id, folder_id, tag_ids)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_file_comparison_data(
        &self,
        path: &str
    ) -> Result<Option<(i64, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
        let row: Option<(i64, String)> = sqlx::query_as(
            "SELECT size, created_at FROM images WHERE path = ?"
        )
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

    pub async fn delete_image_by_path_returning_context(
        &self,
        path: &str
    ) -> Result<Option<(i64, i64, Vec<i64>)>, sqlx::Error> {
        // 1. Get Context
        let context = self.get_image_context(path).await?;

        if let Some((image_id, _, _)) = context {
            // 2. Delete
            sqlx::query("DELETE FROM images WHERE id = ?")
                .bind(image_id)
                .execute(&self.pool)
                .await?;
            
            // Note: image_tags are cascaded if schema is correct, 
            // but we kept the IDs in memory to return
        }
        
        Ok(context)
    }

    pub async fn rename_image(
        &self,
        old_path: &str,
        new_path: &str,
        new_filename: &str,
        new_folder_id: i64
    ) -> Result<Option<(crate::indexer::metadata::ImageMetadata, i64)>, sqlx::Error> {
        // 1. Get existing image
        let row: Option<(i64, i64, i32, i32, i64, String, String, Option<String>, i32, Option<String>)> = sqlx::query_as(
            "SELECT id, folder_id, width, height, size, created_at, modified_at, thumbnail_path, rating, notes FROM images WHERE path = ?"
        )
        .bind(old_path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, old_folder_id, w, h, s, c_at, m_at, thumb, rating, notes)) = row {
            // 2. Update Path, Filename, Folder
            sqlx::query(
                "UPDATE images SET path = ?, filename = ?, folder_id = ?, modified_at = ? WHERE id = ?"
            )
            .bind(new_path)
            .bind(new_filename)
            .bind(new_folder_id)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;

            // Parse timestamps
            let created_dt = chrono::DateTime::parse_from_rfc3339(&c_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let modified_dt = chrono::DateTime::parse_from_rfc3339(&m_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            // Return updated metadata for frontend
            use crate::indexer::metadata::ImageMetadata;
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
                format: std::path::Path::new(new_filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                added_at: None,
            }, old_folder_id)))
        } else {
            Ok(None)
        }
    }
    pub async fn rename_folder(&self, old_path: &str, new_path: &str, new_name: &str) -> Result<bool, sqlx::Error> {
        let old_path = old_path.trim_end_matches('/');
        let new_path = new_path.trim_end_matches('/');
        
        // 1. Find the folder
        let folder = self.get_folder_by_path(old_path).await?;
        
        if let Some(id) = folder {
            println!("DEBUG: DB - Renaming folder ID {} from '{}' to '{}'", id, old_path, new_path);
            
            // 2. Try to Update the folder itself
            let res = sqlx::query("UPDATE folders SET path = ?, name = ? WHERE id = ?")
                .bind(new_path)
                .bind(new_name)
                .bind(id)
                .execute(&self.pool)
                .await;

            match res {
                Ok(_) => {
                    // Success, proceed to update children paths
                },
                Err(e) => {
                    if let Some(db_err) = e.as_database_error() {
                        if db_err.code().as_deref() == Some("2067") {
                            println!("DEBUG: DB - Merge detected for rename {} -> {}", old_path, new_path);
                            if let Some(target_id) = self.get_folder_by_path(new_path).await? {
                                // Move valid children
                                sqlx::query("UPDATE images SET folder_id = ? WHERE folder_id = ?").bind(target_id).bind(id).execute(&self.pool).await?;
                                sqlx::query("UPDATE folders SET parent_id = ? WHERE parent_id = ?").bind(target_id).bind(id).execute(&self.pool).await?;
                                sqlx::query("DELETE FROM folders WHERE id = ?").bind(id).execute(&self.pool).await?;
                            } else { return Err(e); }
                        } else { return Err(e); }
                    } else { return Err(e); }
                }
            }
                
            // 3. Update all children paths using SUBSTR for safety
            let old_prefix = format!("{}/", old_path);
            let old_len = old_path.len() as i32;
            
            // Update child folders
            sqlx::query(
                "UPDATE folders SET path = ? || SUBSTR(path, ? + 1) WHERE SUBSTR(path, 1, ? + 1) = ?"
            )
            .bind(new_path)
            .bind(old_len)
            .bind(old_len)
            .bind(&old_prefix)
            .execute(&self.pool)
            .await?;
            
            // Update child images
            sqlx::query(
                "UPDATE images SET path = ? || SUBSTR(path, ? + 1) WHERE SUBSTR(path, 1, ? + 1) = ?"
            )
            .bind(new_path)
            .bind(old_len)
            .bind(old_len)
            .bind(&old_prefix)
            .execute(&self.pool)
            .await?;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn get_all_root_folders(&self) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, path FROM folders WHERE is_root = 1 OR parent_id IS NULL"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_folders_under_root(&self, root_path: &str) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let root_path = root_path.trim_end_matches('/');
        let pattern = format!("{}/%", root_path);
        let rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, path FROM folders WHERE path = ? OR path LIKE ?"
        )
        .bind(root_path)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }
}

