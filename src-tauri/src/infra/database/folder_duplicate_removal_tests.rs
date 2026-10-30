//! Validates bounded event volume and duplicate integrity at the removal transaction boundary.

use super::*;

async fn duplicate_group(pool: &SqlitePool, identifier: &str, members: &[&str]) -> TestResult {
    sqlx::query(
        "INSERT OR IGNORE INTO duplicate_rule_sets (id, name, created_at, updated_at)
         VALUES ('test-rule', 'Rule', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO duplicate_groups
            (id, rule_set_id, group_type, canonical_asset_id, confidence, status,
             candidate_count, created_at, updated_at)
         VALUES (?, 'test-rule', 'exact', ?, 1, 'open', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(identifier)
    .bind(members.first().copied())
    .bind(members.len() as i64)
    .execute(pool)
    .await?;
    for (index, member) in members.iter().enumerate() {
        sqlx::query(
            "INSERT INTO duplicate_candidates (group_id, asset_id, score, reasons)
             VALUES (?, ?, ?, '[]')",
        )
        .bind(identifier)
        .bind(member)
        .bind(index as f64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn folder_removal_large_library_emits_one_compact_event_without_lagging() -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let event_bus = Arc::new(TokioEventBus::new());
    let mut events = event_bus.subscribe();
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        event_bus,
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_id = folder(&ledger, Path::new("/Bulk"), None).await?;
    sqlx::query(
        "WITH RECURSIVE sequence(counter) AS (
            VALUES(1) UNION ALL SELECT counter + 1 FROM sequence WHERE counter < 3000
         ) INSERT INTO assets (id, name, path, state, format_type, family, file_size, folder_id)
         SELECT 'bulk-' || counter, 'photo', '/Bulk/' || counter, 'IDLE', 'jpg', 'IMAGE', 8, ? FROM sequence",
    ).bind(&root_id).execute(&pool).await?;
    while events.try_recv().is_ok() {}

    ledger.execute(removal(&root_id)).await?;

    let event = events.try_recv()?;
    match &event {
        DomainEvent::LibraryFoldersRemoved {
            folder_ids,
            removed_asset_count,
        } => {
            assert_eq!(folder_ids, &[root_id]);
            assert_eq!(*removed_asset_count, 3000);
        }
        event => panic!("Expected one removal summary, received {event:?}"),
    }
    assert!(serde_json::to_string(&event)?.len() < 512);
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn folder_removal_reconciles_duplicate_counts_and_canonical_members_before_notification(
) -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_id = folder(&ledger, Path::new("/Library"), None).await?;
    for identifier in ["removed-a", "removed-b", "kept-a", "kept-b", "kept-c"] {
        let folder_id = identifier
            .starts_with("removed")
            .then_some(root_id.as_str());
        asset(
            &pool,
            identifier,
            folder_id,
            &Path::new("/Images").join(identifier),
        )
        .await?;
        sqlx::query("INSERT INTO duplicate_fingerprints (asset_id, updated_at) VALUES (?, CURRENT_TIMESTAMP)")
            .bind(identifier).execute(&pool).await?;
    }
    duplicate_group(&pool, "empty", &["removed-a", "removed-b"]).await?;
    duplicate_group(&pool, "singleton", &["removed-a", "kept-a"]).await?;
    duplicate_group(&pool, "mixed", &["removed-a", "kept-a", "kept-b"]).await?;
    duplicate_group(&pool, "unrelated", &["kept-a", "kept-c"]).await?;

    ledger.execute(removal(&root_id)).await?;

    let groups: Vec<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT id, candidate_count, canonical_asset_id FROM duplicate_groups ORDER BY id",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        groups,
        vec![
            ("mixed".into(), 2, Some("kept-b".into())),
            ("unrelated".into(), 2, Some("kept-a".into())),
        ]
    );
    let fingerprints: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_fingerprints")
        .fetch_one(&pool)
        .await?;
    assert_eq!(fingerprints, 3);
    Ok(())
}

#[tokio::test]
async fn folder_removal_duplicate_reconciliation_rolls_back_with_assets() -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_id = folder(&ledger, Path::new("/Library"), None).await?;
    asset(
        &pool,
        "removed",
        Some(&root_id),
        Path::new("/Library/photo.jpg"),
    )
    .await?;
    asset(&pool, "kept", None, Path::new("/Other/photo.jpg")).await?;
    duplicate_group(&pool, "group", &["removed", "kept"]).await?;

    assert!(ledger
        .execute(LedgerCommand::Batch(vec![
            removal(&root_id),
            removal("missing")
        ]))
        .await
        .is_err());

    let group: (i64, String) = sqlx::query_as(
        "SELECT candidate_count, canonical_asset_id FROM duplicate_groups WHERE id = 'group'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(group, (2, "removed".into()));
    let candidates: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_candidates")
        .fetch_one(&pool)
        .await?;
    assert_eq!(candidates, 2);
    Ok(())
}

#[tokio::test]
async fn folder_removal_repair_migration_removes_legacy_empty_groups_and_repairs_membership(
) -> TestResult {
    let pool = database().await?;
    asset(&pool, "kept-a", None, Path::new("/Other/a.jpg")).await?;
    asset(&pool, "kept-b", None, Path::new("/Other/b.jpg")).await?;
    duplicate_group(&pool, "empty", &[]).await?;
    duplicate_group(&pool, "valid", &["kept-a", "kept-b"]).await?;
    duplicate_group(&pool, "history", &["kept-a"]).await?;
    sqlx::query("UPDATE duplicate_groups SET canonical_asset_id = 'missing', candidate_count = 99")
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE duplicate_groups SET status = 'resolved' WHERE id = 'history'")
        .execute(&pool)
        .await?;

    sqlx::raw_sql(include_str!(
        "../../../migrations/20261030000000_reconcile_duplicate_membership.sql"
    ))
    .execute(&pool)
    .await?;

    let groups: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT id, candidate_count, canonical_asset_id FROM duplicate_groups ORDER BY id",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        groups,
        vec![
            ("history".into(), 1, "kept-a".into()),
            ("valid".into(), 2, "kept-b".into()),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn folder_removal_duplicate_integrity_also_applies_to_individual_asset_deletion() -> TestResult
{
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    asset(&pool, "removed", None, Path::new("/Other/a.jpg")).await?;
    asset(&pool, "kept", None, Path::new("/Other/b.jpg")).await?;
    duplicate_group(&pool, "group", &["removed", "kept"]).await?;

    ledger
        .execute(LedgerCommand::DeleteAsset {
            asset_id: Some("removed".into()),
            path: None,
            physical_delete: false,
        })
        .await?;

    let groups: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM duplicate_groups")
        .fetch_one(&pool)
        .await?;
    assert_eq!(groups, 0);
    Ok(())
}
