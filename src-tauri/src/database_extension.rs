use crate::indexer::metadata::ImageMetadata;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

// ... existing code ...

    pub async fn rename_folder(&self, old_path: &str, new_path: &str, new_name: &str) -> Result<bool, sqlx::Error> {
        // 1. Find the folder
        let folder = self.get_folder_by_path(old_path).await?;
        
        if let Some(id) = folder {
            // 2. Update the folder itself
            sqlx::query("UPDATE folders SET path = ?, name = ? WHERE id = ?")
                .bind(new_path)
                .bind(new_name)
                .bind(id)
                .execute(&self.pool)
                .await?;
                
            // 3. Update all children folders (Prefix update)
            // SQLITE lacks handy "startswith" update without recursion or multiple queries usually,
            // but we can use string manipulation.
            // "UPDATE folders SET path = REPLACE(path, old_path, new_path) WHERE path LIKE old_path/%"
            // Note: This matches old_path + /...
            
            let old_pattern = format!("{}/%", old_path);
            
            // We need to be careful with REPLACE logic to only replace the prefix.
            // Since we filter by LIKE prefix, it should be safe to replace the first occurrence.
            
            // For Folders
            // Note: SQLite REPLACE(string, pattern, replacement) replaces ALL occurrences.
            // We only want to replace the START.
            // `new_path || SUBSTR(path, LENGTH(old_path) + 1)`
            
            let old_len = old_path.len() as i32;
            
            sqlx::query(
                "UPDATE folders 
                 SET path = ? || SUBSTR(path, ? + 1) 
                 WHERE path LIKE ?"
            )
            .bind(new_path)
            .bind(old_len)
            .bind(&old_pattern)
            .execute(&self.pool)
            .await?;
            
            // For Images
            sqlx::query(
                "UPDATE images 
                 SET path = ? || SUBSTR(path, ? + 1) 
                 WHERE path LIKE ?"
            )
            .bind(new_path)
            .bind(old_len)
            .bind(&old_pattern)
            .execute(&self.pool)
            .await?;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
