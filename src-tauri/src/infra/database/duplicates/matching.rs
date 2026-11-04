//! MatchingRepository facade; SQL stays in query and command adapters.

use super::SqliteDuplicatesRepository;
use crate::core::{duplicates::matching::*, AppResult};
use crate::infra::database::{handlers::matching as handlers, query_handlers::matching as queries};
use async_trait::async_trait;

#[async_trait]
impl MatchingRepository for SqliteDuplicatesRepository {
    async fn matching_status(&self) -> AppResult<MatchingStatus> {
        queries::matching_status(&self.pool).await
    }

    async fn request_matching(&self, run_id: &str) -> AppResult<()> {
        handlers::request_matching(&self.pool, run_id).await
    }

    async fn claim_matching(
        &self,
        kind: MatchingKind,
        automatic_enabled: bool,
    ) -> AppResult<Option<MatchingInput>> {
        handlers::claim_matching(&self.pool, kind, automatic_enabled).await
    }

    async fn renew_matching(&self, ticket: &MatchingTicket) -> AppResult<bool> {
        handlers::renew_matching(&self.pool, ticket).await
    }

    async fn publish_matching(
        &self,
        input: &MatchingInput,
        groups: Vec<MatchingGroup>,
    ) -> AppResult<bool> {
        handlers::publication::publish(&self.pool, input, groups).await
    }

    async fn release_matching(
        &self,
        ticket: &MatchingTicket,
        error: Option<String>,
    ) -> AppResult<()> {
        handlers::release_matching(&self.pool, ticket, error).await
    }

    async fn cancel_matching(&self, run_id: Option<&str>) -> AppResult<()> {
        handlers::cancel_matching(&self.pool, run_id).await
    }
}
