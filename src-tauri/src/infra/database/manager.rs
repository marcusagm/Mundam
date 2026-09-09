use crate::core::error::AppResult;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::ConnectOptions;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// Manages the SQLite database connection pool and initialization.
#[derive(Clone, Debug)]
pub struct DbManager {
    pool: SqlitePool,
}

impl DbManager {
    /// Initializes the database connection pool and runs migrations.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file.
    ///
    /// # Errors
    /// Returns `AppError` if connection or migrations fail.
    pub async fn new(db_path: &Path) -> AppResult<Self> {
        let database_url = format!("sqlite:{}", db_path.to_string_lossy());

        // Configure optimized SQLite options
        let connection_options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(30))
            .log_slow_statements(log::LevelFilter::Warn, Duration::from_secs(10))
            .foreign_keys(true);
        // PRAGMA default_cache_size is usually set via SQL query if needed,
        // but many are covered by SQLx options.

        let pool = SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(20)
            .connect_with(connection_options)
            .await?;

        // Apply additional performance pragmas
        sqlx::query("PRAGMA default_cache_size = -2000;")
            .execute(&pool)
            .await?;

        // Run migrations
        // Note: For now we use the same migrations folder.
        // We will add the Sprint 1.3 migration soon.
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Returns a shared reference to the connection pool.
    ///
    /// # Returns
    ///
    /// * `&SqlitePool` - The connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Runs database maintenance operations (VACUUM and ANALYZE).
    pub async fn run_maintenance(&self) -> AppResult<()> {
        sqlx::query("VACUUM").execute(&self.pool).await?;
        sqlx::query("ANALYZE").execute(&self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_db_manager_initialization() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_mundam.db");

        let manager = DbManager::new(&db_path)
            .await
            .expect("Failed to initialize DbManager");

        // Verify pragmas
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode;")
            .fetch_one(manager.pool())
            .await
            .expect("Failed to query journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let synchronous: i32 = sqlx::query_scalar("PRAGMA synchronous;")
            .fetch_one(manager.pool())
            .await
            .expect("Failed to query synchronous");
        assert_eq!(synchronous, 1); // 1 = NORMAL

        // Verify migrations (one of the V2 tables should exist)
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='assets')",
        )
        .fetch_one(manager.pool())
        .await
        .expect("Failed to check if table exists");
        assert!(table_exists);
    }
}
