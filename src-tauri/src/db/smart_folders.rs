//! Smart folder management.
//!
//! Smart folders are essentially saved search queries that appear as
//! virtual folders in the UI.

use crate::db::models::SmartFolder;
use chrono::{DateTime, Utc};
use super::Db;

impl Db {
    /// Retrieves all saved smart folders.
    pub async fn get_smart_folders(&self) -> Result<Vec<SmartFolder>, sqlx::Error> {
        let rows = sqlx::query_as!(
            SmartFolder,
            "SELECT id as \"id!\", name, query_json, created_at as \"created_at!: DateTime<Utc>\" FROM smart_folders"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Saves a new smart folder.
    pub async fn save_smart_folder(&self, name: &str, query_json: &str) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(
            "INSERT INTO smart_folders (name, query_json) VALUES (?, ?)",
            name,
            query_json
        )
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    /// Updates an existing smart folder.
    pub async fn update_smart_folder(&self, id: i64, name: &str, query_json: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE smart_folders SET name = ?, query_json = ? WHERE id = ?",
            name,
            query_json,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes a smart folder.
    pub async fn delete_smart_folder(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM smart_folders WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
