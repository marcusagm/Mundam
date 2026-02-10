//! Folder and location hierarchy management.
//!
//! Handles the physical structure of the library, mapping filesystem paths
//! to database records and managing hierarchical relationships.

use super::Db;

impl Db {
    /// Retrieves the absolute filesystem path for a folder by its ID.
    pub async fn get_folder_path(&self, id: i64) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query!("SELECT path FROM folders WHERE id = ?", id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.path))
    }

    /// Finds a folder by its filesystem path.
    ///
    /// Includes a case-insensitive fallback specifically for macOS.
    pub async fn get_folder_by_path(&self, path: &str) -> Result<Option<i64>, sqlx::Error> {
        let path = path.trim_end_matches('/');
        let row = sqlx::query!("SELECT id as \"id!\" FROM folders WHERE path = ?", path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            Ok(Some(r.id))
        } else {
            // Case-insensitive fallback for move detections on case-insensitive filesystems
            let row_loose = sqlx::query!("SELECT id as \"id!\" FROM folders WHERE path = ? COLLATE NOCASE", path)
                .fetch_optional(&self.pool)
                .await?;
            Ok(row_loose.map(|r| r.id))
        }
    }

    /// Inserts or updates a folder record.
    ///
    /// # Arguments
    /// * `is_root` - If true, this folder is a top-level "Location" managed by the user.
    pub async fn upsert_folder(
        &self,
        path: &str,
        name: &str,
        parent_id: Option<i64>,
        is_root: bool,
    ) -> Result<i64, sqlx::Error> {
        let path = path.trim_end_matches('/');

        if let Some(id) = self.get_folder_by_path(path).await? {
            // Guard: Do not demote a root folder if it's already marked as such.
            let existing: Option<(bool, Option<i64>)> = sqlx::query_as("SELECT is_root, parent_id FROM folders WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

            if let Some((existing_is_root, _)) = existing {
                if existing_is_root && !is_root {
                    return Ok(id);
                }
            }

            // Update parent relationship if needed.
            if is_root || parent_id.is_some() {
                 let mut query = "UPDATE folders SET ".to_string();
                 let mut updates = Vec::new();
                 if is_root {
                     updates.push("is_root = 1");
                     updates.push("parent_id = NULL");
                 } else if parent_id.is_some() {
                     updates.push("parent_id = ?");
                 }

                 query.push_str(&updates.join(", "));
                 query.push_str(" WHERE id = ?");

                 let mut q = sqlx::query(&query);
                 if !is_root && parent_id.is_some() { q = q.bind(parent_id); }
                 q = q.bind(id);
                 q.execute(&self.pool).await?;
            }
            return Ok(id);
        }

        let res = sqlx::query!(
            "INSERT INTO folders (path, name, parent_id, is_root) VALUES (?, ?, ?, ?)",
            path, name, parent_id, is_root
        )
        .execute(&self.pool)
        .await;

        match res {
            Ok(r) => Ok(r.last_insert_rowid()),
            Err(e) => {
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("2067") { // Unique constraint
                         return self.get_folder_by_path(path).await?.ok_or(e);
                    }
                }
                Err(e)
            }
        }
    }

    /// Retrieves all thumbnail paths for images within a folder and all its descendants.
    pub async fn get_location_thumbnails(&self, location_id: i64) -> Result<Vec<String>, sqlx::Error> {
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

    /// Deletes a folder record. Images and child folders are handled by CASCADE.
    pub async fn delete_folder(&self, folder_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM folders WHERE id = ?", folder_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Finds unlinked root folders that should be children of a newly added parent folder.
    pub async fn adopt_orphaned_children(&self, parent_id: i64, parent_path: &str) -> Result<(), sqlx::Error> {
        let path_pattern = format!("{}/%", parent_path);

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

    /// Retrieves the entire folder hierarchy.
    ///
    /// Returns: Vec<(id, parent_id, path, name, is_root)>
    pub async fn get_folder_hierarchy(&self) -> Result<Vec<(i64, Option<i64>, String, String, bool)>, sqlx::Error> {
        let rows: Vec<(i64, Option<i64>, String, String, bool)> = sqlx::query_as(
            "SELECT id, parent_id, path, name, is_root FROM folders ORDER BY path"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Gets image counts for all folders, including files in subfolders.
    pub async fn get_folder_counts_recursive(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            "WITH RECURSIVE folder_tree AS (
                SELECT id as root_id, id as child_id
                FROM folders
                UNION ALL
                SELECT ft.root_id, f.id
                FROM folders f
                JOIN folder_tree ft ON f.parent_id = ft.child_id
            )
            SELECT ft.root_id as \"folder_id!\", COUNT(i.id) as \"count!\"
            FROM folder_tree ft
            LEFT JOIN images i ON i.folder_id = ft.child_id
            GROUP BY ft.root_id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.folder_id, r.count as i64)).collect())
    }

    /// Gets image counts for folders (direct children only).
    pub async fn get_folder_counts_direct(&self) -> Result<Vec<(i64, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT folder_id as \"folder_id!\", COUNT(*) as \"count!\" FROM images GROUP BY folder_id"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.folder_id, r.count as i64)).collect())
    }

    /// Ensures all parent folders exist for a given path.
    pub async fn ensure_folder_hierarchy(&self, path: &str) -> Result<i64, sqlx::Error> {
        let path = path.trim_end_matches('/');

        let existing = sqlx::query_as::<_, (i64, bool)>("SELECT id, is_root FROM folders WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;

        if let Some((id, is_root)) = existing {
            if is_root { return Ok(id); }
        }

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
                if parent_str.len() > 1 && parent_str != "/" {
                     Some(Box::pin(self.ensure_folder_hierarchy(&parent_str)).await?)
                } else {
                    None
                }
            }
        } else {
            None
        };

        self.upsert_folder(path, &name, parent_id, false).await
    }

    /// Renames a folder and recursively updates all paths for subfolders and images.
    pub async fn rename_folder(&self, old_path: &str, new_path: &str, new_name: &str) -> Result<bool, sqlx::Error> {
        let old_path = old_path.trim_end_matches('/');
        let new_path = new_path.trim_end_matches('/');

        let folder = self.get_folder_by_path(old_path).await?;

        if let Some(id) = folder {
            println!("DEBUG: DB - Renaming folder ID {} from '{}' to '{}'", id, old_path, new_path);

            let res = sqlx::query!("UPDATE folders SET path = ?, name = ? WHERE id = ?", new_path, new_name, id)
                .execute(&self.pool)
                .await;

            match res {
                Ok(_) => {},
                Err(e) => {
                    if let Some(db_err) = e.as_database_error() {
                        if db_err.code().as_deref() == Some("2067") { // Unique conflict (Already exists)
                            if let Some(target_id) = self.get_folder_by_path(new_path).await? {
                                // Merge logic: Move children to target
                                sqlx::query!("UPDATE images SET folder_id = ? WHERE folder_id = ?", target_id, id).execute(&self.pool).await?;
                                sqlx::query!("UPDATE folders SET parent_id = ? WHERE parent_id = ?", target_id, id).execute(&self.pool).await?;
                                sqlx::query!("DELETE FROM folders WHERE id = ?", id).execute(&self.pool).await?;
                            } else { return Err(e); }
                        } else { return Err(e); }
                    } else { return Err(e); }
                }
            }

            // Update sub-paths
            let old_prefix = format!("{}/", old_path);
            let old_len = old_path.len() as i32;

            sqlx::query(
                "UPDATE folders SET path = ? || SUBSTR(path, ? + 1) WHERE SUBSTR(path, 1, ? + 1) = ?"
            )
            .bind(new_path)
            .bind(old_len)
            .bind(old_len)
            .bind(&old_prefix)
            .execute(&self.pool)
            .await?;

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

    /// Lists all top-level root folders (Locations).
    pub async fn get_all_root_folders(&self) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let rows = sqlx::query!("SELECT id as \"id!\", path FROM folders WHERE is_root = 1 OR parent_id IS NULL")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| (r.id, r.path)).collect())
    }

    /// Finds all sub-folders belonging to a specific root location.
    pub async fn get_folders_under_root(&self, root_path: &str) -> Result<Vec<(i64, String)>, sqlx::Error> {
        let root_path = root_path.trim_end_matches('/');
        let pattern = format!("{}/%", root_path);
        let rows = sqlx::query!(
            "SELECT id as \"id!\", path FROM folders WHERE path = ? OR path LIKE ?",
            root_path,
            pattern
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.id, r.path)).collect())
    }
}
