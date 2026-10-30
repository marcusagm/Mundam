//! Exercises recursive unmonitoring against the migrated schema and real Ledger.

use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::ledger::command::{CreateFolderPayload, LedgerCommand, RemoveFolderPayload};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::path_guard::LedgerPathGuard;
use crate::infra::database::ledger::SqliteAssetLedger;
use crate::infra::events::TokioEventBus;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

async fn folder(
    ledger: &SqliteAssetLedger,
    path: &Path,
    parent_id: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(ledger
        .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
            parent_id,
            name: path.to_string_lossy().into_owned(),
            path: path.to_path_buf(),
        }))
        .await?
        .id)
}

async fn asset(
    pool: &SqlitePool,
    identifier: &str,
    folder_id: Option<&str>,
    path: &Path,
) -> TestResult {
    sqlx::query(
        "INSERT INTO assets
            (id, name, path, state, format_type, family, file_size, folder_id, thumbnail_path)
        VALUES (?, ?, ?, 'IDLE', 'jpg', 'IMAGE', 8, ?, ?)",
    )
    .bind(identifier)
    .bind(identifier)
    .bind(path.to_string_lossy().as_ref())
    .bind(folder_id)
    .bind(path.to_string_lossy().as_ref())
    .execute(pool)
    .await?;
    Ok(())
}

fn removal(folder_id: &str) -> LedgerCommand {
    LedgerCommand::RemoveFolder(RemoveFolderPayload {
        folder_id: folder_id.to_owned(),
    })
}

#[tokio::test]
async fn folder_removal_deletes_entire_catalog_tree_and_generated_variants_only() -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let originals = tempfile::tempdir()?;
    let event_bus = Arc::new(TokioEventBus::new());
    let mut events = event_bus.subscribe();
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        event_bus,
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_path = originals.path().join("Library");
    let child_path = root_path.join("Child");
    let grandchild_path = child_path.join("Grandchild");
    tokio::fs::create_dir_all(&grandchild_path).await?;
    let root_id = folder(&ledger, &root_path, None).await?;
    let child_id = folder(&ledger, &child_path, Some(root_id.clone())).await?;
    let grandchild_id = folder(&ledger, &grandchild_path, Some(child_id.clone())).await?;
    let sibling_id = folder(&ledger, &originals.path().join("Other"), None).await?;

    for (identifier, folder_id, directory) in [
        ("root-asset", Some(root_id.as_str()), root_path.as_path()),
        ("child-asset", Some(child_id.as_str()), child_path.as_path()),
        (
            "deep-asset",
            Some(grandchild_id.as_str()),
            grandchild_path.as_path(),
        ),
        ("other-asset", Some(sibling_id.as_str()), originals.path()),
        ("unassigned-asset", None, originals.path()),
        ("orphan-asset", None, root_path.as_path()),
    ] {
        let original_path = directory.join(format!("{identifier}.jpg"));
        tokio::fs::write(&original_path, b"original").await?;
        asset(&pool, identifier, folder_id, &original_path).await?;
        for (cache_directory, extension) in [
            ("thumbnails", "webp"),
            ("thumbnails", "glb"),
            ("thumbnails", "gltf"),
            ("previews", "jpg"),
            ("previews", "png"),
            ("previews", "webp"),
            ("previews", "bin"),
        ] {
            let directory = app_data.path().join(cache_directory);
            tokio::fs::create_dir_all(&directory).await?;
            tokio::fs::write(
                directory.join(format!("{identifier}.{extension}")),
                b"cache",
            )
            .await?;
        }
    }
    sqlx::query("INSERT INTO tags (id, name) VALUES ('tag', 'Tag')")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO asset_tags (asset_id, tag_id) VALUES ('deep-asset', 'tag')")
        .execute(&pool)
        .await?;
    while events.try_recv().is_ok() {}

    ledger.execute(removal(&root_id)).await?;

    let remaining: Vec<String> = sqlx::query_scalar("SELECT id FROM assets ORDER BY id")
        .fetch_all(&pool)
        .await?;
    assert_eq!(remaining, ["other-asset", "unassigned-asset"]);
    let folders: Vec<String> = sqlx::query_scalar("SELECT id FROM folders")
        .fetch_all(&pool)
        .await?;
    assert_eq!(folders, [sibling_id]);
    let tag_links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_tags")
        .fetch_one(&pool)
        .await?;
    assert_eq!(tag_links, 0);
    for (identifier, directory) in [
        ("root-asset", root_path.as_path()),
        ("child-asset", child_path.as_path()),
        ("deep-asset", grandchild_path.as_path()),
    ] {
        assert_eq!(
            tokio::fs::read(directory.join(format!("{identifier}.jpg"))).await?,
            b"original"
        );
    }
    for directory_name in ["thumbnails", "previews"] {
        let mut entries = tokio::fs::read_dir(app_data.path().join(directory_name)).await?;
        while let Some(entry) = entries.next_entry().await? {
            let filename = entry.file_name().to_string_lossy().into_owned();
            assert!(
                filename.starts_with("other-asset.") || filename.starts_with("unassigned-asset.")
            );
        }
    }
    assert!(app_data.path().join("previews/other-asset.jpg").exists());
    match events.try_recv()? {
        DomainEvent::LibraryFoldersRemoved {
            folder_ids,
            removed_asset_count,
        } => {
            assert_eq!(folder_ids, [root_id]);
            assert_eq!(removed_asset_count, 4);
        }
        event => panic!("Expected consolidated removal, received {event:?}"),
    }
    assert!(events.try_recv().is_err());
    Ok(())
}

#[path = "folder_duplicate_removal_tests.rs"]
mod duplicate_tests;

#[tokio::test]
async fn folder_removal_rollback_preserves_rows_caches_and_events() -> TestResult {
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
    let root_id = folder(&ledger, Path::new("/Library"), None).await?;
    asset(
        &pool,
        "asset",
        Some(&root_id),
        Path::new("/Library/original.jpg"),
    )
    .await?;
    tokio::fs::create_dir_all(app_data.path().join("previews")).await?;
    let preview = app_data.path().join("previews/asset.jpg");
    tokio::fs::write(&preview, b"cache").await?;
    while events.try_recv().is_ok() {}

    let result = ledger
        .execute(LedgerCommand::Batch(vec![
            removal(&root_id),
            removal("missing"),
        ]))
        .await;
    assert!(result.is_err());
    let asset_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
        .fetch_one(&pool)
        .await?;
    assert_eq!(asset_count, 1);
    assert!(preview.exists());
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn folder_removal_accepts_empty_folders_and_missing_cache_directories() -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_id = folder(&ledger, Path::new("/Library"), None).await?;
    ledger.execute(removal(&root_id)).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn folder_removal_preserves_orphans_outside_literal_path_boundary() -> TestResult {
    let pool = database().await?;
    let app_data = tempfile::tempdir()?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        app_data.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let root_path = if cfg!(windows) {
        r"C:\Library_100%"
    } else {
        "/Library_100%"
    };
    let inside = if cfg!(windows) {
        "c:/Library_100%/Photos/photo.jpg"
    } else {
        "/Library_100%/Photos/photo.jpg"
    };
    let similarly_named = if cfg!(windows) {
        "C:/Library_100%Sibling/photo.jpg"
    } else {
        "/Library_100%Sibling/photo.jpg"
    };
    let wildcard_match = if cfg!(windows) {
        "C:/LibraryX100Extra/photo.jpg"
    } else {
        "/LibraryX100Extra/photo.jpg"
    };
    let root_id = folder(&ledger, Path::new(root_path), None).await?;
    for (identifier, path) in [
        ("inside", inside),
        ("sibling", similarly_named),
        ("wildcard", wildcard_match),
    ] {
        asset(&pool, identifier, None, Path::new(path)).await?;
    }

    ledger.execute(removal(&root_id)).await?;

    let remaining: Vec<String> = sqlx::query_scalar("SELECT id FROM assets ORDER BY id")
        .fetch_all(&pool)
        .await?;
    assert_eq!(remaining, ["sibling", "wildcard"]);
    Ok(())
}
