//! Database abstraction layer for Mundam.
//!
//! This module handles the connection pool, schema initialization, and
//! provides a central entry point for all database operations.

pub mod models;
pub mod images;
pub mod folders;
pub mod tags;
pub mod smart_folders;
pub mod settings;
pub mod search;

use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

/// The main database handle, wrapping a SQLite connection pool.
///
/// This struct is shared across the application via Tauri's state management.
pub struct Db {
    /// The underlying SQLite connection pool.
    pub pool: SqlitePool,
}

impl Db {
    /// Creates a new database instance or opens an existing one.
    ///
    /// This function also initializes the database with the required schema
    /// and runs any pending migrations.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path where the SQLite database file should be located.
    ///
    /// # Errors
    ///
    /// Returns a `sqlx::Error` if the connection fails or if migrations fail to run.
    pub async fn new(path: PathBuf) -> Result<Self, sqlx::Error> {
        use sqlx::sqlite::SqliteConnectOptions;
        use sqlx::Executor;
        use std::str::FromStr;

        let url = format!("sqlite:{}", path.to_string_lossy());
        let options = SqliteConnectOptions::from_str(&url)?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Optimize SQLite performance for concurrent read-heavy workloads
        pool.execute("PRAGMA journal_mode = WAL").await?;
        pool.execute("PRAGMA synchronous = NORMAL").await?;

        // Initialize schema and run migrations from the /migrations directory
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Returns a reference to the underlying connection pool.
    pub fn inner(&self) -> &SqlitePool {
        &self.pool
    }

    /// Performs routine database maintenance missions.
    ///
    /// Runs `VACUUM` to reclaim space and `ANALYZE` to update query planner statistics.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the maintenance queries fail.
    pub async fn run_maintenance(&self) -> Result<(), sqlx::Error> {
        println!("DEBUG: DB - Running Maintenance (VACUUM + ANALYZE)");
        sqlx::query("VACUUM").execute(&self.pool).await?;
        sqlx::query("ANALYZE").execute(&self.pool).await?;
        Ok(())
    }
}
