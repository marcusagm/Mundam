//! Real on-disk restart coverage for all processing stages after incomplete shutdown.

use super::*;
use crate::processing::workers::test_support::TestResult;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
async fn processing_restart_preserves_commits_rolls_back_transactions_and_fences_expired_owners(
) -> TestResult {
    for kind in [
        ProcessingKind::Thumbnail,
        ProcessingKind::Colors,
        ProcessingKind::Fingerprint,
    ] {
        let directory = tempfile::tempdir()?;
        let original = directory.path().join("original.png");
        tokio::fs::write(&original, b"original media").await?;
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("recovery.db"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options.clone())
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size, notes, thumbnail_path) VALUES ('asset', 'asset', ?, 'Indexed', 'png', 'IMAGE', 14, 'committed', ?)")
            .bind(original.to_string_lossy().as_ref())
            .bind(if kind == ProcessingKind::Colors { Some("published.webp") } else { None })
            .execute(&pool).await?;
        let repository = SqliteProcessingRepository::new(pool.clone());
        let old = repository
            .begin(kind, "asset")
            .await?
            .ok_or("No initial lease")?;
        let mut interrupted = pool.begin().await?;
        sqlx::query("UPDATE assets SET notes = 'uncommitted' WHERE id = 'asset'")
            .execute(&mut *interrupted)
            .await?;
        drop(interrupted);
        pool.close().await;

        let reopened = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&reopened)
            .await?;
        assert_eq!(integrity, "ok");
        let notes: String = sqlx::query_scalar("SELECT notes FROM assets WHERE id = 'asset'")
            .fetch_one(&reopened)
            .await?;
        assert_eq!(notes, "committed");
        let first = SqliteProcessingRepository::new(reopened.clone());
        let second = SqliteProcessingRepository::new(reopened.clone());
        assert!(first.begin(kind, "asset").await?.is_none());
        sqlx::query("UPDATE processing_tasks SET lease_expires_at = unixepoch() - 1")
            .execute(&reopened)
            .await?;
        let (left, right) = tokio::join!(first.begin(kind, "asset"), second.begin(kind, "asset"));
        let (left, right) = (left?, right?);
        assert_ne!(left.is_some(), right.is_some());
        let current = left.or(right).ok_or("No recovered lease")?;
        assert_ne!(old.lease_id, current.lease_id);
        assert_eq!(old.generation, current.generation);
        assert!(!first.renew(&old).await?);
        first.release(&old).await?;
        first
            .finish(&old, ProcessingOutcome::Failed("obsolete owner".into()))
            .await?;
        assert!(first.is_current(&current).await?);
        let outcome: (String, i64) =
            sqlx::query_as("SELECT state, attempts FROM processing_tasks WHERE kind = ?")
                .bind(kind.as_str())
                .fetch_one(&reopened)
                .await?;
        assert_eq!(outcome, ("pending".into(), 0));
        assert_eq!(tokio::fs::read(&original).await?, b"original media");
        first.release(&current).await?;
        assert_eq!(first.pending(kind, 1).await?, ["asset"]);
        reopened.close().await;
    }
    Ok(())
}
