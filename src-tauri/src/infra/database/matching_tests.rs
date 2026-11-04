use crate::core::{matching::*, models::DuplicateFingerprint, repository::DuplicatesRepository};
use crate::infra::database::duplicates::SqliteDuplicatesRepository;
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};

pub(super) fn fingerprint(asset_id: &str, hash: &str) -> DuplicateFingerprint {
    DuplicateFingerprint {
        asset_id: asset_id.into(),
        content_hash: Some(blake3::hash(hash.as_bytes()).to_hex().to_string()),
        perceptual_hash: Some("ffffffffffffffff".into()),
        block_hash: None,
        thumb_hash: None,
        width: Some(32),
        height: Some(32),
        file_size: Some(8),
        mime_type: None,
        format_family: Some("IMAGE".into()),
        color_profile: None,
        orientation: None,
        fingerprint_version: 5,
        updated_at: chrono::Utc::now(),
    }
}

pub(super) async fn prepare(
    fixture: &ProcessingFixture,
) -> Result<SqliteDuplicatesRepository, Box<dyn std::error::Error>> {
    let repository = SqliteDuplicatesRepository::new(fixture.pool.clone());
    let mut policy = repository.get_active_rule_set().await?.ok_or("No policy")?;
    policy.consider_exact_match = true;
    policy.consider_visual_match = true;
    repository.save_rule_set(policy).await?;
    for asset_id in ["a", "b"] {
        fixture.asset(asset_id, None).await?;
        repository
            .save_fingerprint(fingerprint(asset_id, "same"))
            .await?;
    }
    Ok(repository)
}

pub(super) async fn claim(
    repository: &SqliteDuplicatesRepository,
    kind: MatchingKind,
) -> Result<MatchingInput, Box<dyn std::error::Error>> {
    repository
        .claim_matching(kind, true)
        .await?
        .ok_or_else(|| "No matching reservation".into())
}

fn calculate(input: &MatchingInput) -> Result<Vec<MatchingGroup>, Box<dyn std::error::Error>> {
    crate::feature::duplicates::matching::compare(
        input,
        &tokio_util::sync::CancellationToken::new(),
    )
    .ok_or_else(|| "Canceled unexpectedly".into())
}

#[tokio::test]
async fn processing_matching_coalesces_intent_and_fences_expiry_and_input_changes() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let second = SqliteDuplicatesRepository::new(fixture.pool.clone());
    let first = claim(&repository, MatchingKind::Exact).await?;
    assert!(second
        .claim_matching(MatchingKind::Exact, true)
        .await?
        .is_none());
    repository
        .save_fingerprint(fingerprint("b", "changed"))
        .await?;
    assert!(!repository.renew_matching(&first.ticket).await?);
    assert!(
        !repository
            .publish_matching(&first, calculate(&first)?)
            .await?
    );
    repository.release_matching(&first.ticket, None).await?;
    let old = claim(&second, MatchingKind::Exact).await?;
    assert!(old.ticket.revision > first.ticket.revision);
    sqlx::query("UPDATE duplicate_matching_jobs SET lease_expires_at = unixepoch() - 1 WHERE kind = 'exact_matching'")
        .execute(&fixture.pool).await?;
    assert!(!second.renew_matching(&old.ticket).await?);
    let current = claim(&repository, MatchingKind::Exact).await?;
    assert_eq!(current.ticket.revision, old.ticket.revision);
    assert_ne!(current.ticket.lease_id, old.ticket.lease_id);
    repository
        .release_matching(&old.ticket, Some("late failure".into()))
        .await?;
    assert!(repository.renew_matching(&current.ticket).await?);
    assert!(!repository.publish_matching(&old, calculate(&old)?).await?);
    assert!(
        repository
            .publish_matching(&current, calculate(&current)?)
            .await?
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_matching_jobs")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(count, 2);
    Ok(())
}

#[tokio::test]
async fn processing_matching_pins_policy_and_rejects_a_rule_change_before_publication() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let old = claim(&repository, MatchingKind::Exact).await?;
    let mut policy = old.policy.clone();
    policy.consider_exact_match = false;
    repository.save_rule_set(policy).await?;
    assert!(old.policy.consider_exact_match);
    assert!(!repository.publish_matching(&old, calculate(&old)?).await?);
    repository.release_matching(&old.ticket, None).await?;
    let current = claim(&repository, MatchingKind::Exact).await?;
    assert!(!current.policy.consider_exact_match);
    assert!(calculate(&current)?.is_empty());
    assert!(
        repository
            .publish_matching(&current, calculate(&current)?)
            .await?
    );
    assert!(repository
        .matching_status()
        .await?
        .completed
        .contains(&"exact_matching".to_string()));
    Ok(())
}

#[tokio::test]
async fn processing_matching_completion_failure_rolls_back_all_groups_and_candidates() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let input = claim(&repository, MatchingKind::Exact).await?;
    sqlx::query("CREATE TRIGGER reject_matching_completion BEFORE UPDATE ON duplicate_matching_jobs WHEN NEW.state = 'succeeded' BEGIN SELECT RAISE(ABORT, 'test rollback'); END")
        .execute(&fixture.pool).await?;
    assert!(repository
        .publish_matching(&input, calculate(&input)?)
        .await
        .is_err());
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM duplicate_groups), (SELECT COUNT(*) FROM duplicate_candidates)")
        .fetch_one(&fixture.pool).await?;
    assert_eq!(counts, (0, 0));
    assert_eq!(repository.matching_status().await?.publication_sequence, 0);
    assert!(repository.renew_matching(&input.ticket).await?);
    sqlx::query("DROP TRIGGER reject_matching_completion")
        .execute(&fixture.pool)
        .await?;
    assert!(
        repository
            .publish_matching(&input, calculate(&input)?)
            .await?
    );
    assert_eq!(repository.matching_status().await?.publication_sequence, 1);
    Ok(())
}

#[tokio::test]
async fn processing_matching_keeps_identity_selection_canonical_and_ignored_history() -> TestResult
{
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let input = claim(&repository, MatchingKind::Exact).await?;
    assert!(
        repository
            .publish_matching(&input, calculate(&input)?)
            .await?
    );
    let id: String = sqlx::query_scalar("SELECT id FROM duplicate_groups")
        .fetch_one(&fixture.pool)
        .await?;
    sqlx::query("UPDATE duplicate_groups SET canonical_asset_id = 'b'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE duplicate_candidates SET is_selected = 1 WHERE asset_id = 'a'")
        .execute(&fixture.pool)
        .await?;
    repository.request_matching("manual").await?;
    let repeat = claim(&repository, MatchingKind::Exact).await?;
    assert!(
        repository
            .publish_matching(&repeat, calculate(&repeat)?)
            .await?
    );
    let retained: (String, String, i64) = sqlx::query_as("SELECT id, canonical_asset_id, (SELECT is_selected FROM duplicate_candidates WHERE asset_id = 'a') FROM duplicate_groups")
        .fetch_one(&fixture.pool).await?;
    assert_eq!(retained, (id.clone(), "b".into(), 1));
    repository.update_group_status(&id, "ignored").await?;
    repository.request_matching("manual").await?;
    let ignored = claim(&repository, MatchingKind::Exact).await?;
    assert!(
        repository
            .publish_matching(&ignored, calculate(&ignored)?)
            .await?
    );
    let retained: (String, String, i64) =
        sqlx::query_as("SELECT id, status, candidate_count FROM duplicate_groups")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(retained, (id, "ignored".into(), 2));
    Ok(())
}

#[tokio::test]
async fn processing_matching_excludes_inactive_and_outdated_fingerprints_and_removes_empty_open_groups(
) -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let input = claim(&repository, MatchingKind::Exact).await?;
    assert!(
        repository
            .publish_matching(&input, calculate(&input)?)
            .await?
    );
    sqlx::query("UPDATE assets SET state = 'Offline' WHERE id = 'b'")
        .execute(&fixture.pool)
        .await?;
    let input = claim(&repository, MatchingKind::Exact).await?;
    assert_eq!(input.fingerprints.len(), 1);
    assert!(
        repository
            .publish_matching(&input, calculate(&input)?)
            .await?
    );
    let groups: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_groups")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(groups, 0);
    Ok(())
}

#[tokio::test]
async fn processing_matching_manual_cancel_retains_independent_automatic_intent() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    repository.request_matching("manual").await?;
    assert!(repository.request_matching("another").await.is_err());
    let input = repository
        .claim_matching(MatchingKind::Exact, false)
        .await?
        .ok_or("No manual claim")?;
    repository.cancel_matching(Some("another")).await?;
    assert!(repository.renew_matching(&input.ticket).await?);
    repository.cancel_matching(Some("manual")).await?;
    assert!(
        !repository
            .publish_matching(&input, calculate(&input)?)
            .await?
    );
    assert!(repository.matching_status().await?.manual_cancel_requested);
    repository.release_matching(&input.ticket, None).await?;
    assert!(!repository.matching_status().await?.manual_pending);
    assert!(repository
        .claim_matching(MatchingKind::Exact, false)
        .await?
        .is_none());
    assert!(repository
        .claim_matching(MatchingKind::Exact, true)
        .await?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn processing_matching_bounds_retries_and_reopens_work_on_new_inputs() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    for attempt in 0..3 {
        let input = claim(&repository, MatchingKind::Exact).await?;
        repository
            .release_matching(&input.ticket, Some("comparison storage failed".into()))
            .await?;
        if attempt < 2 {
            assert_eq!(repository.matching_status().await?.retry_wait, 1);
            assert!(repository
                .claim_matching(MatchingKind::Exact, true)
                .await?
                .is_none());
            sqlx::query("UPDATE duplicate_matching_jobs SET next_attempt_at = unixepoch() - 1")
                .execute(&fixture.pool)
                .await?;
        }
    }
    assert_eq!(repository.matching_status().await?.failed, 1);
    assert!(repository
        .claim_matching(MatchingKind::Exact, true)
        .await?
        .is_none());
    repository.save_fingerprint(fingerprint("b", "new")).await?;
    assert_eq!(repository.matching_status().await?.failed, 0);
    assert!(repository
        .claim_matching(MatchingKind::Exact, true)
        .await?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn processing_matching_visual_math_does_not_group_identical_content_twice() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = prepare(&fixture).await?;
    let identical = claim(&repository, MatchingKind::Visual).await?;
    assert!(calculate(&identical)?.is_empty());
    repository.release_matching(&identical.ticket, None).await?;
    repository
        .save_fingerprint(fingerprint("b", "different bytes"))
        .await?;
    let similar = claim(&repository, MatchingKind::Visual).await?;
    let groups = calculate(&similar)?;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].candidates.len(), 2);
    assert_eq!(
        groups[0].group_type,
        crate::core::models::DuplicateGroupType::Visual
    );
    let canceled = tokio_util::sync::CancellationToken::new();
    canceled.cancel();
    assert!(crate::feature::duplicates::matching::compare(&similar, &canceled).is_none());
    assert!(repository.publish_matching(&similar, groups).await?);
    Ok(())
}
