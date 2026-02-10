//! Key-value application settings storage.

use serde_json::Value;
use super::Db;

impl Db {
    /// Retrieves a setting value by its key.
    pub async fn get_setting(&self, key: &str) -> Result<Option<Value>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM app_settings WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some((json_str,)) => {
                let value: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);
                Ok(Some(value))
            },
            None => Ok(None)
        }
    }

    /// Saves or updates a setting value.
    pub async fn set_setting(&self, key: &str, value: &Value) -> Result<(), sqlx::Error> {
        let json_str = serde_json::to_string(value).unwrap();

        sqlx::query(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(key)
        .bind(json_str)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
