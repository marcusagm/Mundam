use super::matching_tests::{claim, fingerprint, prepare};
use crate::core::{
    ledger::port::TransactionalAssetLedger,
    matching::*,
    processing::{ProcessingKind, ProcessingRepository, ProcessingResult},
    repository::DuplicatesRepository,
};
use crate::infra::database::duplicates::SqliteDuplicatesRepository;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

#[tokio::test]
async fn processing_matching_recovers_manual_intent_and_expired_ownership_after_reopening(
) -> TestResult {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    let fixture = ProcessingFixture::new().await?;
    let options = SqliteConnectOptions::new()
        .filename(fixture.data.path().join("matching.db"))
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options.clone())
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let repository = SqliteDuplicatesRepository::new(pool.clone());
    repository.get_active_rule_set().await?;
    repository.request_matching("interrupted-manual").await?;
    let old = claim(&repository, MatchingKind::Exact).await?;
    sqlx::query("UPDATE duplicate_matching_jobs SET lease_expires_at = unixepoch() - 1")
        .execute(&pool)
        .await?;
    pool.close().await;
    let reopened = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    let first = SqliteDuplicatesRepository::new(reopened.clone());
    let second = SqliteDuplicatesRepository::new(reopened.clone());
    assert!(first.matching_status().await?.manual_pending);
    let (left, right) = tokio::join!(
        first.claim_matching(MatchingKind::Exact, false),
        second.claim_matching(MatchingKind::Exact, false)
    );
    let left = left?;
    let right = right?;
    assert_ne!(left.is_some(), right.is_some());
    let current = left.or(right).ok_or("No recovered comparison")?;
    assert_eq!(
        current.ticket.manual_run_id.as_deref(),
        Some("interrupted-manual")
    );
    assert_eq!(current.ticket.revision, old.ticket.revision);
    assert_ne!(current.ticket.lease_id, old.ticket.lease_id);
    assert!(!first.publish_matching(&old, Vec::new()).await?);
    assert!(first.publish_matching(&current, Vec::new()).await?);
    assert!(first
        .matching_status()
        .await?
        .completed
        .contains(&"exact_matching".to_string()));
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn processing_matching_intent_commits_and_rolls_back_with_its_fingerprint() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    for kind in [MatchingKind::Exact, MatchingKind::Visual] {
        let input = claim(&repository, kind).await?;
        assert!(repository.publish_matching(&input, Vec::new()).await?);
    }
    fixture.asset("new", None).await?;
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Fingerprint, "new")
        .await?
        .ok_or("No fingerprint attempt")?;
    sqlx::query("CREATE TRIGGER reject_fingerprint_completion BEFORE UPDATE ON processing_tasks WHEN NEW.kind = 'fingerprint' AND NEW.state = 'succeeded' BEGIN SELECT RAISE(ABORT, 'test rollback'); END")
        .execute(&fixture.pool).await?;
    assert!(fixture
        .ledger
        .commit_processing(
            &ticket,
            ProcessingResult::Fingerprint(fingerprint("new", "new bytes"))
        )
        .await
        .is_err());
    assert!(repository.get_fingerprint("new").await?.is_none());
    assert_eq!(repository.matching_status().await?.completed.len(), 2);
    sqlx::query("DROP TRIGGER reject_fingerprint_completion")
        .execute(&fixture.pool)
        .await?;
    assert!(
        fixture
            .ledger
            .commit_processing(
                &ticket,
                ProcessingResult::Fingerprint(fingerprint("new", "new bytes"))
            )
            .await?
    );
    assert_eq!(repository.matching_status().await?.pending, 2);
    Ok(())
}
