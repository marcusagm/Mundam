//! Recovery never turns missing context or an unsupported native operation into success.

use super::*;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

async fn pending(
    pool: &SqlitePool,
    identifier: &str,
    kind: &str,
    payload: serde_json::Value,
) -> AppResult<()> {
    sqlx::query("INSERT INTO asset_operations_log (id, operation_type, asset_id, payload, status, created_at) VALUES (?, ?, 'asset', ?, 'PENDING', CURRENT_TIMESTAMP)")
        .bind(identifier).bind(kind).bind(payload.to_string()).execute(pool).await?;
    Ok(())
}

#[tokio::test]
async fn supported_delete_replays_idempotently_before_and_after_its_native_effect() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let source = fixture.data.path().join("delete-me");
    tokio::fs::write(&source, b"temporary fixture").await?;
    for identifier in ["before-delete", "after-delete"] {
        pending(
            &fixture.pool,
            identifier,
            "DELETE_ASSET",
            serde_json::json!({"physical": true, "path": source}),
        )
        .await?;
    }
    let service = SagaRecoveryService::new(fixture.pool.clone());
    service.run_recovery().await?;
    service.run_recovery().await?;
    assert!(!tokio::fs::try_exists(source).await?);
    let completed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM asset_operations_log WHERE status = 'COMPLETED'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(completed, 2);
    Ok(())
}

#[tokio::test]
async fn ambiguous_intents_are_retained_as_failed_and_independent_recovery_continues() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    let original = fixture.data.path().join("original");
    tokio::fs::write(&original, b"preserved original").await?;
    for (identifier, kind, payload) in [
        (
            "trash",
            "MOVE_TO_TRASH",
            serde_json::json!({"path": original}),
        ),
        (
            "restore",
            "RESTORE_FROM_TRASH",
            serde_json::json!({"path": original}),
        ),
        (
            "no-path",
            "DELETE_ASSET",
            serde_json::json!({"physical": true}),
        ),
        (
            "no-flag",
            "DELETE_ASSET",
            serde_json::json!({"path": original}),
        ),
        (
            "relative",
            "DELETE_ASSET",
            serde_json::json!({"physical": true, "path": "relative"}),
        ),
        (
            "directory",
            "DELETE_ASSET",
            serde_json::json!({"physical": true, "path": fixture.data.path()}),
        ),
    ] {
        pending(&fixture.pool, identifier, kind, payload).await?;
    }
    pending(
        &fixture.pool,
        "logical",
        "DELETE_ASSET",
        serde_json::json!({"physical": false}),
    )
    .await?;
    let service = SagaRecoveryService::new(fixture.pool.clone());
    assert!(service.run_recovery().await.is_err());
    let failed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_operations_log WHERE status = 'FAILED' AND error_note IS NOT NULL").fetch_one(&fixture.pool).await?;
    assert_eq!(failed, 6);
    let completed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM asset_operations_log WHERE status = 'COMPLETED'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(completed, 1);
    service.run_recovery().await?;
    assert_eq!(tokio::fs::read(original).await?, b"preserved original");
    Ok(())
}
