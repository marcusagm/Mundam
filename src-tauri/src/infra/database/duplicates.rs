//! SQLite implementation of the `DuplicatesRepository` port.
//!
//! Acts as a thin facade that delegates read operations to
//! `query_handlers::duplicates_queries` and write operations to
//! `handlers::duplicates_handler`, following the same pattern used
//! by [`SqliteAssetQueries`](super::queries::SqliteAssetQueries)
//! and the Ledger handler modules.

use crate::core::error::AppResult;
use crate::core::models::{
    DuplicateCandidate, DuplicateFingerprint, DuplicateGroup, DuplicateResolution, DuplicateRuleSet,
};
use crate::core::repository::DuplicatesRepository;
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

/// SQLite implementation of the `DuplicatesRepository` port.
///
/// Holds only the connection pool and delegates all SQL logic to
/// stateless functions in `query_handlers::duplicates_queries` (reads)
/// and `handlers::duplicates_handler` (writes).
pub struct SqliteDuplicatesRepository {
    pool: Pool<Sqlite>,
}

impl SqliteDuplicatesRepository {
    /// Creates a new instance with the given connection pool.
    ///
    /// # Arguments
    /// * `pool` - The SQLite connection pool.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DuplicatesRepository for SqliteDuplicatesRepository {
    async fn save_fingerprint(&self, fingerprint: DuplicateFingerprint) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::save_fingerprint(&self.pool, fingerprint).await
    }

    async fn get_fingerprint(&self, asset_id: &str) -> AppResult<Option<DuplicateFingerprint>> {
        crate::infra::database::query_handlers::duplicates_queries::get_fingerprint(&self.pool, asset_id).await
    }

    async fn get_rule_sets(&self) -> AppResult<Vec<DuplicateRuleSet>> {
        crate::infra::database::query_handlers::duplicates_queries::get_rule_sets(&self.pool).await
    }

    async fn save_rule_set(&self, rule_set: DuplicateRuleSet) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::save_rule_set(&self.pool, rule_set).await
    }

    async fn save_group(&self, group: DuplicateGroup) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::save_group(&self.pool, group).await
    }

    async fn save_candidate(&self, candidate: DuplicateCandidate) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::save_candidate(&self.pool, candidate).await
    }

    async fn get_groups_by_status(&self, status: &str) -> AppResult<Vec<DuplicateGroup>> {
        crate::infra::database::query_handlers::duplicates_queries::get_groups_by_status(&self.pool, status).await
    }

    async fn get_group_candidates(&self, group_id: &str) -> AppResult<Vec<DuplicateCandidate>> {
        crate::infra::database::query_handlers::duplicates_queries::get_group_candidates(&self.pool, group_id).await
    }

    async fn save_resolution(&self, resolution: DuplicateResolution) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::save_resolution(&self.pool, resolution).await
    }

    async fn get_latest_resolution(&self, group_id: &str) -> AppResult<Option<DuplicateResolution>> {
        crate::infra::database::query_handlers::duplicates_queries::get_latest_resolution(&self.pool, group_id).await
    }

    async fn delete_resolution(&self, resolution_id: &str) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::delete_resolution(&self.pool, resolution_id).await
    }

    async fn update_group_status(&self, group_id: &str, status: &str) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::update_group_status(&self.pool, group_id, status).await
    }

    async fn run_exact_match_scan(&self, token: Option<tokio_util::sync::CancellationToken>) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::run_exact_match_scan(&self.pool, token).await
    }

    async fn backfill_missing_fingerprints(&self) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::backfill_missing_fingerprints(&self.pool).await
    }

    async fn run_visual_match_scan(&self, token: Option<tokio_util::sync::CancellationToken>) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::run_visual_match_scan(&self.pool, token).await
    }

    async fn rehash_pending_fingerprints(
        &self,
        token: Option<tokio_util::sync::CancellationToken>,
        progress_sender: Option<tokio::sync::mpsc::UnboundedSender<(usize, usize)>>,
    ) -> AppResult<usize> {
        crate::infra::database::handlers::duplicates_handler::rehash_pending_fingerprints(&self.pool, token, progress_sender).await
    }

    async fn delete_fingerprint(&self, asset_id: &str) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::delete_fingerprint(&self.pool, asset_id).await
    }

    async fn remove_candidate_from_groups(&self, asset_id: &str) -> AppResult<()> {
        crate::infra::database::handlers::duplicates_handler::remove_candidate_from_groups(&self.pool, asset_id).await
    }
}
