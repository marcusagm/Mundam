use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query_json: String,
    pub created_at: String,
}

use crate::database::Db;

impl Db {
    pub async fn get_smart_folders(&self) -> Result<Vec<SmartFolder>, sqlx::Error> {
        let rows = sqlx::query_as::<_, SmartFolder>(
            "SELECT id, name, query_json, created_at FROM smart_folders"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn save_smart_folder(&self, name: &str, query: &str) -> Result<i64, sqlx::Error> {
        let res = sqlx::query("INSERT INTO smart_folders (name, query_json) VALUES (?, ?)")
            .bind(name)
            .bind(query)
            .execute(&self.pool)
            .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn update_smart_folder(&self, id: i64, name: &str, query: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE smart_folders SET name = ?, query_json = ? WHERE id = ?")
            .bind(name)
            .bind(query)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_smart_folder(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM smart_folders WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
