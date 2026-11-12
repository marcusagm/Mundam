use super::*;
use crate::core::error::AppError;
use crate::core::events::DomainEvent;
use crate::core::models::asset::Asset;
use crate::processing::workers::test_support::ProcessingFixture;
use async_trait::async_trait;

#[tokio::test]
async fn cancelling_an_active_scan_does_not_prune_or_publish_completion_or_failure(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let root_path = fixture.data.path().join("root");
    tokio::fs::create_dir(&root_path).await?;
    let root = fixture
        .ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            name: "root".into(),
            path: root_path.clone(),
            parent_id: None,
        }))
        .await?;
    let stale_path = root_path.join("missing.png");
    sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size, folder_id, modified_at) VALUES ('stale', 'missing.png', ?, 'Indexed', 'png', 'IMAGE', 1, ?, ?)")
        .bind(stale_path.to_string_lossy().as_ref())
        .bind(&root.id)
        .bind(chrono::Utc::now())
        .execute(&fixture.pool)
        .await?;
    for index in 0..10_000 {
        tokio::fs::write(root_path.join(format!("{index}.bin")), b"data").await?;
    }

    let token = tokio_util::sync::CancellationToken::new();
    let mut events = fixture.event_bus.subscribe();
    let indexer = Arc::new(LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        Arc::new(crate::core::formats::FormatRegistry::new()),
    ));
    let scan_token = token.clone();
    let scan_root = root_path.clone();
    let scan_folder_id = root.id.clone();
    let scan = tokio::spawn(async move {
        indexer
            .scan_directory_with_token(scan_root, Some(scan_folder_id), scan_token)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if matches!(events.recv().await?, DomainEvent::ScanStarted { .. }) {
                break;
            }
        }
        Ok::<(), tokio::sync::broadcast::error::RecvError>(())
    })
    .await??;
    token.cancel();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(10), scan).await??,
        Err(AppError::Cancelled)
    ));
    assert!(fixture.queries.get_by_id("stale").await?.is_some());
    assert!(matches!(
        events.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    Ok(())
}

#[tokio::test]
async fn aborting_fanout_cancels_the_producer_and_classification_tasks(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let root_path = fixture.data.path().join("root");
    tokio::fs::create_dir(&root_path).await?;
    for index in 0..10 {
        tokio::fs::write(root_path.join(format!("{index}.png")), b"image").await?;
    }
    let entries = WalkDir::new(&root_path)
        .min_depth(1)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let indexer = Arc::new(LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        Arc::new(crate::core::formats::FormatRegistry::new()),
    ));
    let folders = Arc::new(RwLock::new(HashMap::new()));
    let locked_folders = folders.write().await;
    let task_folders = folders.clone();
    let task_token = tokio_util::sync::CancellationToken::new();
    let token = task_token.clone();
    let task = tokio::spawn(async move {
        indexer
            .fanout_classify_and_persist(entries, &HashMap::new(), &task_folders, &None, 10, &token)
            .await
    });

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while Arc::strong_count(&folders) < 5 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while Arc::strong_count(&folders) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    drop(locked_folders);
    Ok(())
}

struct FailingBatchLedger {
    inner: Arc<dyn TransactionalAssetLedger>,
}

#[async_trait]
impl TransactionalAssetLedger for FailingBatchLedger {
    async fn execute(&self, command: LedgerCommand) -> AppResult<Asset> {
        if matches!(command, LedgerCommand::BatchCreate(_)) {
            return Err(AppError::Internal("injected batch failure".into()));
        }
        self.inner.execute(command).await
    }
}

#[tokio::test]
async fn failed_batches_do_not_prune_or_publish_completion_and_can_be_retried(
) -> Result<(), Box<dyn std::error::Error>> {
    for file_count in [1, 101] {
        let fixture = ProcessingFixture::new().await?;
        let root_path = fixture.data.path().join("root");
        tokio::fs::create_dir(&root_path).await?;
        let root = fixture
            .ledger
            .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
                name: "root".into(),
                path: root_path.clone(),
                parent_id: None,
            }))
            .await?;
        let stale_path = root_path.join("missing.png");
        sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size, folder_id, modified_at) VALUES ('stale', 'missing.png', ?, 'Indexed', 'png', 'IMAGE', 1, ?, ?)")
            .bind(stale_path.to_string_lossy().as_ref())
            .bind(&root.id)
            .bind(chrono::Utc::now())
            .execute(&fixture.pool)
            .await?;
        for index in 0..file_count {
            tokio::fs::write(root_path.join(format!("{index}.png")), b"image").await?;
        }
        let mut registry = crate::core::formats::FormatRegistry::new();
        registry.register(Arc::new(
            crate::processing::media::providers::image::png::PngFormatProvider,
        ));
        let registry = Arc::new(registry);
        let mut events = fixture.event_bus.subscribe();
        let failing = Arc::new(FailingBatchLedger {
            inner: fixture.ledger.clone(),
        });
        let indexer = LibraryIndexer::new(
            fixture.queries.clone(),
            failing,
            fixture.event_bus.clone(),
            registry.clone(),
        );
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            indexer.scan_directory(root_path.clone(), Some(root.id.clone())),
        )
        .await?;
        assert!(
            matches!(&result, Err(AppError::Internal(message)) if message == "injected batch failure"),
            "unexpected scan result: {result:?}"
        );
        assert!(fixture.queries.get_by_id("stale").await?.is_some());
        let mut failures = 0;
        while let Ok(event) = events.try_recv() {
            if let DomainEvent::ScanFailed { library_id } = &event {
                assert_eq!(library_id, &root.id);
                failures += 1;
            }
            assert!(!matches!(event, DomainEvent::ScanCompleted { .. }));
            assert!(!matches!(
                event,
                DomainEvent::ScanProgress { processed, total, .. } if processed == total
            ));
        }
        assert_eq!(failures, 1);

        let retry = LibraryIndexer::new(
            fixture.queries.clone(),
            fixture.ledger.clone(),
            fixture.event_bus.clone(),
            registry,
        );
        retry.scan_directory(root_path, Some(root.id)).await?;
        let inserted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE id != 'stale'")
            .fetch_one(&fixture.pool)
            .await?;
        assert_eq!(inserted, file_count);
        let mut completed = false;
        let mut final_progress = false;
        while let Ok(event) = events.try_recv() {
            match event {
                DomainEvent::ScanCompleted { .. } => completed = true,
                DomainEvent::ScanProgress {
                    processed, total, ..
                } if processed == total => final_progress = true,
                _ => {}
            }
        }
        assert!(completed);
        assert!(final_progress);
    }
    Ok(())
}

struct FailingIdentityLedger {
    inner: Arc<dyn TransactionalAssetLedger>,
}

#[async_trait]
impl TransactionalAssetLedger for FailingIdentityLedger {
    async fn execute(&self, command: LedgerCommand) -> AppResult<Asset> {
        self.inner.execute(command).await
    }

    async fn observe_file_identities(&self, _paths: Vec<PathBuf>) -> AppResult<()> {
        Err(AppError::Internal("injected identity failure".into()))
    }
}

#[tokio::test]
async fn identity_failure_emits_failure_instead_of_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let root_path = fixture.data.path().join("root");
    tokio::fs::create_dir(&root_path).await?;
    let root = fixture
        .ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            name: "root".into(),
            path: root_path.clone(),
            parent_id: None,
        }))
        .await?;
    let stale_path = root_path.join("missing.png");
    sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size, folder_id, modified_at) VALUES ('stale', 'missing.png', ?, 'Indexed', 'png', 'IMAGE', 1, ?, ?)")
                    .bind(stale_path.to_string_lossy().as_ref())
                    .bind(&root.id)
                    .bind(chrono::Utc::now())
                    .execute(&fixture.pool)
                    .await?;
    tokio::fs::write(root_path.join("new.png"), b"image").await?;
    let mut registry = crate::core::formats::FormatRegistry::new();
    registry.register(Arc::new(
        crate::processing::media::providers::image::png::PngFormatProvider,
    ));
    let registry = Arc::new(registry);
    let mut events = fixture.event_bus.subscribe();
    let indexer = LibraryIndexer::new(
        fixture.queries.clone(),
        Arc::new(FailingIdentityLedger {
            inner: fixture.ledger.clone(),
        }),
        fixture.event_bus.clone(),
        registry.clone(),
    );
    let result = indexer.scan_directory(root_path.clone(), None).await;
    assert!(
        matches!(result, Err(AppError::Internal(message)) if message == "injected identity failure")
    );
    assert!(fixture.queries.get_by_id("stale").await?.is_some());
    let mut failures = 0;
    while let Ok(event) = events.try_recv() {
        match event {
            DomainEvent::ScanFailed { library_id } => {
                assert_eq!(library_id, root.id);
                failures += 1;
            }
            DomainEvent::ScanCompleted { .. } => return Err("unexpected scan completion".into()),
            _ => {}
        }
    }
    assert_eq!(failures, 1);
    let retry = LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        registry,
    );
    retry.scan_directory(root_path, Some(root.id)).await?;
    let mut completed = false;
    while let Ok(event) = events.try_recv() {
        completed |= matches!(event, DomainEvent::ScanCompleted { .. });
    }
    assert!(completed);
    Ok(())
}

#[tokio::test]
async fn inaccessible_scan_root_emits_failure_without_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let mut events = fixture.event_bus.subscribe();
    let indexer = LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.registry.clone(),
    );
    let missing = fixture.data.path().join("missing");
    assert!(indexer.scan_directory(missing, None).await.is_err());
    let mut started = 0;
    let mut failed = 0;
    while let Ok(event) = events.try_recv() {
        match event {
            DomainEvent::ScanStarted { .. } => started += 1,
            DomainEvent::ScanFailed { library_id } => {
                assert_eq!(library_id, "root");
                failed += 1;
            }
            DomainEvent::ScanCompleted { .. } => return Err("unexpected scan completion".into()),
            _ => {}
        }
    }
    assert_eq!(started, 1);
    assert_eq!(failed, 1);
    Ok(())
}

#[tokio::test]
async fn failure_resolving_root_is_reported_before_scan_started(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let mut events = fixture.event_bus.subscribe();
    let indexer = LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.registry.clone(),
    );
    fixture.pool.close().await;
    assert!(indexer
        .scan_directory(fixture.data.path().to_path_buf(), None)
        .await
        .is_err());
    let mut failed = 0;
    while let Ok(event) = events.try_recv() {
        match event {
            DomainEvent::ScanStarted { .. } | DomainEvent::ScanCompleted { .. } => {
                return Err("unexpected scan lifecycle event".into());
            }
            DomainEvent::ScanFailed { library_id } => {
                assert_eq!(library_id, "root");
                failed += 1;
            }
            _ => {}
        }
    }
    assert_eq!(failed, 1);
    Ok(())
}

#[tokio::test]
async fn repeated_scan_respects_unicode_aliases_and_distinct_entries(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = ProcessingFixture::new().await?;
    let root_path = fixture.data.path().join("root");
    tokio::fs::create_dir(&root_path).await?;
    let root = fixture
        .ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            name: "root".into(),
            path: root_path.clone(),
            parent_id: None,
        }))
        .await?;
    let composed_folder = root_path.join("caf\u{e9}");
    let decomposed_folder = root_path.join("cafe\u{301}");
    tokio::fs::create_dir(&composed_folder).await?;
    let folder_is_alias =
        crate::infra::filesystem::identity::same(&composed_folder, &decomposed_folder).await?;
    let composed_file = composed_folder.join("r\u{e9}sum\u{e9}.png");
    let decomposed_file = composed_folder.join("re\u{301}sume\u{301}.png");
    tokio::fs::write(&composed_file, b"one").await?;
    let file_is_alias =
        crate::infra::filesystem::identity::same(&composed_file, &decomposed_file).await?;
    if !file_is_alias {
        tokio::fs::write(&decomposed_file, b"two").await?;
    }

    let mut registry = crate::core::formats::FormatRegistry::new();
    registry.register(Arc::new(
        crate::processing::media::providers::image::png::PngFormatProvider,
    ));
    let indexer = LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        Arc::new(registry),
    );
    indexer
        .scan_directory(root_path.clone(), Some(root.id.clone()))
        .await?;

    if folder_is_alias {
        sqlx::query("UPDATE folders SET path = ? WHERE parent_id = ?")
            .bind(composed_folder.to_string_lossy().as_ref())
            .bind(&root.id)
            .execute(&fixture.pool)
            .await?;
    }
    if file_is_alias {
        sqlx::query("UPDATE assets SET path = ?")
            .bind(composed_file.to_string_lossy().as_ref())
            .execute(&fixture.pool)
            .await?;
    }
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM asset_operations_log WHERE operation_type = 'CREATE_ASSET'",
    )
    .fetch_one(&fixture.pool)
    .await?;
    indexer.scan_directory(root_path, Some(root.id)).await?;
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM asset_operations_log WHERE operation_type = 'CREATE_ASSET'",
    )
    .fetch_one(&fixture.pool)
    .await?;
    let assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE deleted_at IS NULL")
        .fetch_one(&fixture.pool)
        .await?;
    let folders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(before, if file_is_alias { 1 } else { 2 });
    assert_eq!(after, before, "unchanged files must not be reindexed");
    assert_eq!(assets, before);
    assert_eq!(folders, 2, "observed folders must not be pruned");
    Ok(())
}
