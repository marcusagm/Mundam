use crate::core::error::{AppError, AppResult};
use sqlx::SqlitePool;

use tokio::fs;
use tracing::{info, warn};

/// Service responsible for recovering and completing pending Sagas (Outbox pattern).
///
/// Replays supported durable intents after a commit interrupted before its filesystem step.
/// Missing or unsupported recovery context is retained as a diagnostic failure;
/// it is never treated as proof that an unimplemented filesystem action completed.
pub struct SagaRecoveryService {
    pool: SqlitePool,
}

impl SagaRecoveryService {
    /// Creates a new instance of the SagaRecoveryService.
    ///
    /// # Arguments
    ///
    /// * `pool` - The SQLite database connection pool.
    ///
    /// # Returns
    ///
    /// A new instance of `SagaRecoveryService`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Runs the recovery process. Should be called during application startup.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if database queries fail or if there are critical
    /// issues executing the pending operations.
    pub async fn run_recovery(&self) -> AppResult<()> {
        info!("SagaRecovery: Starting recovery of pending operations...");

        // Find all pending operations ordered by oldest first
        let pending_operations = sqlx::query!(
            r#"
            SELECT 
                id as "id!", 
                operation_type as "operation_type!", 
                asset_id as "asset_id!", 
                payload as "payload!", 
                status, 
                error_note, 
                created_at
            FROM asset_operations_log
            WHERE status = 'PENDING'
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        if pending_operations.is_empty() {
            info!("SagaRecovery: No pending operations found. System is clean.");
            return Ok(());
        }

        info!(
            "SagaRecovery: Found {} pending operations. Processing...",
            pending_operations.len()
        );

        let mut failed_operations = 0;
        for operation in pending_operations {
            match self
                .process_operation(
                    &operation.id,
                    &operation.operation_type,
                    &operation.asset_id,
                    &operation.payload,
                )
                .await
            {
                Ok(_) => {
                    info!(
                        "SagaRecovery: Operation {} ({}) completed successfully.",
                        operation.id, operation.operation_type
                    );
                    self.mark_completed(&operation.id).await?;
                }
                Err(e) => {
                    failed_operations += 1;
                    warn!(
                        "SagaRecovery: Failed to process operation {} ({}): {}",
                        operation.id, operation.operation_type, e
                    );
                    self.mark_failed(&operation.id, &e.to_string()).await?;
                }
            }
        }

        info!("SagaRecovery: Recovery process finished.");
        if failed_operations > 0 {
            return Err(AppError::Internal(format!(
                "Saga recovery requires repair for {failed_operations} operation(s)"
            )));
        }
        Ok(())
    }

    /// Processes a single pending saga operation.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The unique identifier of the operation log.
    /// * `operation_type` - The string representing the operation type (e.g., "DELETE_ASSET").
    /// * `_asset_id` - The unique identifier of the asset associated with the operation.
    /// * `payload_string` - The JSON payload string containing operation context.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if payload parsing fails or if the actual recovery steps fail.
    async fn process_operation(
        &self,
        operation_id: &str,
        operation_type: &str,
        _asset_id: &str,
        payload_string: &str,
    ) -> AppResult<()> {
        let payload: serde_json::Value = serde_json::from_str(payload_string).map_err(|error| {
            AppError::Internal(format!(
                "Failed to parse payload for operation {}: {}",
                operation_id, error
            ))
        })?;

        match operation_type {
            "DELETE_ASSET" => {
                let physical_delete = payload
                    .get("physical")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| {
                        AppError::ValidationFailed(
                            "Delete recovery is missing its physical flag".into(),
                        )
                    })?;
                if physical_delete {
                    let path = payload
                        .get("path")
                        .and_then(|value| value.as_str())
                        .filter(|path| !path.is_empty())
                        .ok_or_else(|| {
                            AppError::ValidationFailed(
                                "Physical delete recovery is missing its path".into(),
                            )
                        })?;
                    if !std::path::Path::new(path).is_absolute() {
                        return Err(AppError::ValidationFailed(
                            "Physical delete recovery requires an absolute path".into(),
                        ));
                    }
                    match fs::remove_file(path).await {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(AppError::Io(error)),
                    }
                }
            }
            _ => {
                return Err(AppError::ValidationFailed(format!(
                    "Unsupported saga recovery operation: {operation_type}"
                )));
            }
        }

        Ok(())
    }

    /// Marks a saga operation as successfully completed.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The unique identifier of the operation log.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the database update fails.
    async fn mark_completed(&self, operation_id: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE asset_operations_log SET status = 'COMPLETED', error_note = NULL WHERE id = ?",
            operation_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks a saga operation as failed.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The unique identifier of the operation log.
    /// * `error_message` - The error string that caused the failure.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the database update fails.
    async fn mark_failed(&self, operation_id: &str, error_message: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE asset_operations_log SET status = 'FAILED', error_note = ? WHERE id = ?",
            error_message,
            operation_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "saga_recovery_tests.rs"]
mod tests;
