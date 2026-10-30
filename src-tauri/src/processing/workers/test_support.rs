//! Real schema and adapters used to validate processing recovery and Ledger integration.

use crate::core::formats::FormatRegistry;
use crate::core::path_guard::LedgerPathGuard;
use crate::infra::database::{ledger::SqliteAssetLedger, queries::SqliteAssetQueries};
use crate::infra::events::TokioEventBus;
use crate::infra::processing::SqliteProcessingRepository;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(crate) struct ProcessingFixture {
    pub pool: SqlitePool,
    pub registry: Arc<FormatRegistry>,
    pub ledger: Arc<SqliteAssetLedger>,
    pub queries: Arc<SqliteAssetQueries>,
    pub processing: Arc<SqliteProcessingRepository>,
    pub event_bus: Arc<TokioEventBus>,
    pub data: tempfile::TempDir,
}

impl ProcessingFixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let registry = Arc::new(FormatRegistry::new());
        let event_bus = Arc::new(TokioEventBus::new());
        let data = tempfile::tempdir()?;
        tokio::fs::create_dir(data.path().join("thumbnails")).await?;
        let ledger = Arc::new(SqliteAssetLedger::new(
            pool.clone(),
            event_bus.clone(),
            data.path().to_path_buf(),
            Arc::new(LedgerPathGuard::new()),
        ));
        let queries = Arc::new(SqliteAssetQueries::new(pool.clone(), registry.clone()));
        let processing = Arc::new(SqliteProcessingRepository::new(pool.clone()));
        Ok(Self {
            pool,
            registry,
            ledger,
            queries,
            processing,
            event_bus,
            data,
        })
    }

    pub async fn asset(&self, identifier: &str, thumbnail: Option<&str>) -> TestResult {
        let source = self.data.path().join(format!("{identifier}.png"));
        sqlx::query(
            "INSERT INTO assets (id, name, path, state, format_type, family, file_size, thumbnail_path)
             VALUES (?, ?, ?, 'Indexed', 'png', 'IMAGE', 8, ?)",
        ).bind(identifier).bind(identifier).bind(source.to_string_lossy().as_ref()).bind(thumbnail)
            .execute(&self.pool).await?;
        Ok(())
    }
}
