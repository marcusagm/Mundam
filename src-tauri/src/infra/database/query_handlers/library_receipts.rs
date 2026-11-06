//! Read-only commit proofs used by crash recovery and the cache outbox.
use crate::core::error::AppResult;
use sqlx::SqlitePool;

/// Determines the durable commit outcome without replaying a user intention.
///
/// # Errors
/// Returns catalog errors, preserving the journal when an outcome is uncertain.
pub(crate) async fn committed(pool: &SqlitePool, operation_id: &str) -> AppResult<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM library_operation_receipts WHERE operation_id = ?",
    )
    .bind(operation_id)
    .fetch_one(pool)
    .await?
        != 0)
}

/// Reads one bounded page of retryable cache cleanup effects.
///
/// # Errors
/// Returns database read failures without acknowledging any receipt.
pub(crate) async fn pending_cleanup(pool: &SqlitePool) -> AppResult<Vec<(String, String)>> {
    Ok(sqlx::query_as("SELECT operation_id, cleanup_assets_json FROM library_operation_receipts WHERE cleanup_pending = 1 LIMIT 32").fetch_all(pool).await?)
}
