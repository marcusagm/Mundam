//! Real catalog/filesystem coverage for split watcher notifications and stale hints.
use super::*;
use crate::core::{
    formats::FormatRegistry, ledger::port::TransactionalAssetLedger, path_guard::LedgerPathGuard,
    repository::AssetQueryHandler,
};
use crate::infra::{
    database::{ledger::SqliteAssetLedger, queries::SqliteAssetQueries},
    events::TokioEventBus,
};

#[tokio::test]
async fn hierarchy_creation_preserves_parent_order_and_rejects_unowned_paths(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::processing::workers::test_support::ProcessingFixture::new().await?;
    let root_path = fixture.data.path().join("indexed");
    let leaf_path = root_path.join("parent/leaf");
    tokio::fs::create_dir_all(&leaf_path).await?;
    let root = fixture
        .ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            parent_id: None,
            name: "indexed".into(),
            path: root_path.clone(),
        }))
        .await?;
    let indexer = LibraryIndexer::new(
        fixture.queries.clone(),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.registry.clone(),
    );
    let leaf_id = indexer
        .ensure_folder_hierarchy(&leaf_path)
        .await?
        .ok_or("Expected an indexed leaf")?;
    let parent_id = fixture
        .queries
        .find_folder_by_path(&root_path.join("parent").to_string_lossy())
        .await?
        .ok_or("Expected an indexed parent")?;
    let leaf = fixture
        .queries
        .get_folder_by_id(&leaf_id)
        .await?
        .ok_or("Expected leaf metadata")?;
    assert_eq!(leaf.parent_id.as_deref(), Some(parent_id.as_str()));
    let parent = fixture
        .queries
        .get_folder_by_id(&parent_id)
        .await?
        .ok_or("Expected parent metadata")?;
    assert_eq!(parent.parent_id, Some(root.id));
    let count = fixture.queries.list_all_subfolders().await?.len();
    assert_eq!(
        indexer.ensure_folder_hierarchy(&leaf_path).await?,
        Some(leaf_id)
    );
    assert_eq!(fixture.queries.list_all_subfolders().await?.len(), count);

    let unowned = fixture.data.path().join("unowned/leaf");
    tokio::fs::create_dir_all(&unowned).await?;
    assert!(matches!(
        indexer.ensure_folder_hierarchy(&unowned).await,
        Err(crate::core::AppError::NotFound(_))
    ));
    assert_eq!(fixture.queries.list_all_subfolders().await?.len(), count);
    Ok(())
}

#[tokio::test]
async fn split_delete_create_preserves_identity_and_external_changes_ignore_ancestor_hints(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let bus = Arc::new(TokioEventBus::new());
    let guard = Arc::new(LedgerPathGuard::new());
    let ledger = Arc::new(SqliteAssetLedger::new(
        pool.clone(),
        bus.clone(),
        directory.path().join("app"),
        guard.clone(),
    ));
    let root_path = directory.path().join("root");
    tokio::fs::create_dir(&root_path).await?;
    let root = ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            parent_id: None,
            name: "root".into(),
            path: root_path.clone(),
        }))
        .await?;
    let old_path = root_path.join("old.png");
    tokio::fs::write(&old_path, b"original").await?;
    let original = ledger
        .execute(LedgerCommand::CreateAsset(CreateAssetPayload {
            path: old_path.clone(),
            file_size: 8,
            format_type: "png".into(),
            family: "IMAGE".into(),
            state_init: AssetState::Indexed,
            folder_id: Some(root.id),
            created_at: None,
            modified_at: None,
        }))
        .await?;
    sqlx::query("UPDATE assets SET notes = 'preserved' WHERE id = ?")
        .bind(&original.id)
        .execute(&pool)
        .await?;
    let mut registry = FormatRegistry::new();
    registry.register(Arc::new(
        crate::processing::media::providers::image::png::PngFormatProvider,
    ));
    let registry = Arc::new(registry);
    let queries = Arc::new(SqliteAssetQueries::new(pool.clone(), registry.clone()));
    let indexer =
        LibraryIndexer::new(queries.clone(), ledger, bus, registry).with_path_guard(guard.clone());
    let token = tokio_util::sync::CancellationToken::new();
    let moved_path = root_path.join("moved.png");
    tokio::fs::rename(&old_path, &moved_path).await?;
    guard.mark(&root_path);
    indexer
        .handle_path_deleted(old_path.to_string_lossy().into())
        .await;
    assert!(queries.get_by_id(&original.id).await?.is_some());
    indexer
        .handle_file_discovered(moved_path.to_string_lossy().into())
        .await;
    let recovered = queries
        .find_asset_by_path(&moved_path.to_string_lossy())
        .await?
        .ok_or("Move was not indexed")?;
    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.notes.as_deref(), Some("preserved"));
    let renamed_path = root_path.join("renamed.png");
    tokio::fs::rename(&moved_path, &renamed_path).await?;
    indexer
        .handle_path_renamed(
            moved_path.to_string_lossy().into(),
            renamed_path.to_string_lossy().into(),
            &token,
        )
        .await;
    assert_eq!(
        queries
            .find_asset_by_path(&renamed_path.to_string_lossy())
            .await?
            .ok_or("External rename lost")?
            .id,
        original.id
    );
    let external_path = root_path.join("external.png");
    tokio::fs::write(&external_path, b"external").await?;
    indexer
        .handle_file_discovered(external_path.to_string_lossy().into())
        .await;
    assert!(queries
        .find_asset_by_path(&external_path.to_string_lossy())
        .await?
        .is_some());
    indexer
        .handle_path_renamed(
            renamed_path.to_string_lossy().into(),
            external_path.to_string_lossy().into(),
            &token,
        )
        .await;
    let retained = queries
        .get_by_id(&original.id)
        .await?
        .ok_or("Original lost")?;
    assert_eq!(retained.path, renamed_path);
    assert_eq!(retained.notes.as_deref(), Some("preserved"));
    let independent = queries
        .find_asset_by_path(&external_path.to_string_lossy())
        .await?
        .ok_or("Independent discovery lost")?;
    assert_ne!(independent.id, original.id);
    assert!(independent.notes.as_deref().unwrap_or_default().is_empty());

    let source_folder = root_path.join("source");
    let unrelated_folder = root_path.join("unrelated");
    tokio::fs::create_dir(&source_folder).await?;
    tokio::fs::create_dir(&unrelated_folder).await?;
    let source_id = indexer
        .ensure_folder_hierarchy(&source_folder)
        .await?
        .ok_or("Source folder lost")?;
    indexer
        .handle_directory_rename(source_folder.clone(), unrelated_folder.clone(), &token)
        .await;
    assert_eq!(
        queries
            .get_folder_by_id(&source_id)
            .await?
            .ok_or("Folder identity lost")?
            .path,
        source_folder
    );
    let unrelated_id = queries
        .find_folder_by_path(&unrelated_folder.to_string_lossy())
        .await?
        .ok_or("Independent folder was not discovered")?;
    assert_ne!(unrelated_id, source_id);
    tokio::fs::remove_file(&external_path).await?;
    indexer
        .handle_path_deleted(external_path.to_string_lossy().into())
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(5200)).await;
    assert!(queries
        .find_asset_by_path(&external_path.to_string_lossy())
        .await?
        .is_none());
    assert!(queries.get_by_id(&original.id).await?.is_some());
    Ok(())
}
