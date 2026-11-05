//! SQLite processing port: delegates reads and mutations to their dedicated adapters.

use crate::core::{processing::*, AppResult};
use crate::infra::database::{
    handlers::processing as handlers, query_handlers::processing as queries,
};
use async_trait::async_trait;
use sqlx::SqlitePool;

pub(crate) mod selection;

/// Shares the application pool so asset removal cascades to pending work.
pub struct SqliteProcessingRepository {
    pool: SqlitePool,
}
impl SqliteProcessingRepository {
    /// Binds the durable queue adapter to the application database.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProcessingRepository for SqliteProcessingRepository {
    async fn tool_statuses(
        &self,
    ) -> AppResult<Vec<crate::core::processing::tools::ExtractionToolStatus>> {
        queries::tools::statuses(&self.pool).await
    }
    async fn record_tool_checks(
        &self,
        checks: Vec<crate::core::processing::tools::ExtractionToolCheck>,
    ) -> AppResult<()> {
        handlers::tools::record(&self.pool, checks).await
    }
    async fn diagnostics(
        &self,
        kind: ProcessingKind,
    ) -> AppResult<Vec<crate::core::processing::diagnostics::ProcessingDiagnostic>> {
        queries::diagnostics(&self.pool, kind).await
    }

    async fn retry(
        &self,
        kind: ProcessingKind,
        scope: crate::core::processing::diagnostics::ProcessingRetryScope,
    ) -> AppResult<crate::core::processing::diagnostics::ProcessingRetryCounts> {
        handlers::retry(&self.pool, kind, scope).await
    }

    async fn queue_counts(
        &self,
        kind: ProcessingKind,
        active_assets: &[String],
    ) -> AppResult<crate::core::processing::control::ProcessingQueueCounts> {
        queries::queue_counts(&self.pool, kind, active_assets).await
    }

    async fn thumbnail_statuses(
        &self,
        asset_ids: &[String],
    ) -> AppResult<Vec<ThumbnailProcessingStatus>> {
        queries::thumbnail_statuses(&self.pool, asset_ids).await
    }

    async fn pending(&self, kind: ProcessingKind, limit: u32) -> AppResult<Vec<String>> {
        queries::pending(&self.pool, kind, limit).await
    }

    async fn begin(
        &self,
        kind: ProcessingKind,
        asset_id: &str,
    ) -> AppResult<Option<ProcessingTicket>> {
        handlers::begin(&self.pool, kind, asset_id).await
    }

    async fn is_current(&self, ticket: &ProcessingTicket) -> AppResult<bool> {
        let mut transaction = self.pool.begin().await?;
        handlers::attempts::is_current(&mut transaction, ticket).await
    }

    async fn renew(&self, ticket: &ProcessingTicket) -> AppResult<bool> {
        handlers::attempts::renew(&self.pool, ticket).await
    }

    async fn release(&self, ticket: &ProcessingTicket) -> AppResult<()> {
        handlers::attempts::release(&self.pool, ticket).await
    }

    async fn preview_path(&self, asset_id: &str) -> AppResult<Option<String>> {
        queries::preview_path(&self.pool, asset_id).await
    }

    async fn finish(&self, ticket: &ProcessingTicket, outcome: ProcessingOutcome) -> AppResult<()> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        handlers::attempts::finish(&mut transaction, ticket, outcome).await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
