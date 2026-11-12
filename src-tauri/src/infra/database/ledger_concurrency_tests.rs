use super::*;
use crate::core::ledger::command::{CreateAssetPayload, CreateFolderPayload};
use crate::core::models::asset::AssetState;
use crate::core::path_guard::LedgerPathGuard;
use crate::infra::database::manager::DbManager;
use crate::infra::events::TokioEventBus;
use std::time::Duration;

#[tokio::test]
async fn wal_deferred_read_upgrade_reproduces_busy_snapshot(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("test.db")).await?;
    let pool = database.pool();
    sqlx::query("CREATE TABLE snapshot_test (value INTEGER NOT NULL)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO snapshot_test VALUES (0)")
        .execute(pool)
        .await?;
    let mut reader = pool.begin().await?;
    let _: i64 = sqlx::query_scalar("SELECT value FROM snapshot_test")
        .fetch_one(&mut *reader)
        .await?;
    sqlx::query("UPDATE snapshot_test SET value = 1")
        .execute(pool)
        .await?;
    let error = sqlx::query("UPDATE snapshot_test SET value = 2")
        .execute(&mut *reader)
        .await
        .err()
        .ok_or("a stale WAL snapshot must not be promoted to a writer")?;
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("517")
    );
    reader.rollback().await?;
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn batch_create_waits_for_existing_writer_then_commits(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("test.db")).await?;
    let pool = database.pool().clone();
    let source = directory.path().join("source.png");
    tokio::fs::write(&source, b"source").await?;
    let ledger = Arc::new(SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        directory.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    ));
    let mut writer = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("CREATE TABLE writer_test (value INTEGER)")
        .execute(&mut *writer)
        .await?;
    let mut batch = tokio::spawn(async move {
        ledger
            .execute(LedgerCommand::BatchCreate(vec![CreateAssetPayload {
                path: source,
                file_size: 6,
                format_type: "png".into(),
                family: "IMAGE".into(),
                state_init: AssetState::Indexed,
                folder_id: None,
                created_at: None,
                modified_at: None,
            }]))
            .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(100), &mut batch)
        .await
        .is_err());
    writer.commit().await?;
    tokio::time::timeout(Duration::from_secs(10), batch).await???;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE name = 'source.png'")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn identity_observation_waits_for_writer_and_persists(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("test.db")).await?;
    let pool = database.pool().clone();
    let source = directory.path().join("source.png");
    tokio::fs::write(&source, b"source").await?;
    let ledger = Arc::new(SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        directory.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    ));
    let asset = ledger
        .execute(LedgerCommand::BatchCreate(vec![CreateAssetPayload {
            path: source.clone(),
            file_size: 6,
            format_type: "png".into(),
            family: "IMAGE".into(),
            state_init: AssetState::Indexed,
            folder_id: None,
            created_at: None,
            modified_at: None,
        }]))
        .await?;
    sqlx::query("DELETE FROM library_file_identities WHERE asset_id = ?")
        .bind(&asset.id)
        .execute(&pool)
        .await?;
    let mut writer = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("UPDATE assets SET notes = 'concurrent' WHERE id = ?")
        .bind(&asset.id)
        .execute(&mut *writer)
        .await?;
    let mut observation =
        tokio::spawn(async move { ledger.observe_file_identities(vec![source]).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut observation)
            .await
            .is_err()
    );
    writer.commit().await?;
    tokio::time::timeout(Duration::from_secs(10), observation).await???;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM library_file_identities WHERE asset_id = ?")
            .bind(asset.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 1);
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn create_folder_waits_for_existing_writer_then_commits(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("test.db")).await?;
    let pool = database.pool().clone();
    let path = directory.path().join("folder");
    tokio::fs::create_dir(&path).await?;
    let ledger = Arc::new(SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        directory.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    ));
    let mut writer = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("CREATE TABLE writer_folder_test (value INTEGER)")
        .execute(&mut *writer)
        .await?;
    let mut create = tokio::spawn(async move {
        ledger
            .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
                name: "folder".into(),
                path,
                parent_id: None,
            }))
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut create)
            .await
            .is_err()
    );
    writer.commit().await?;
    let folder = tokio::time::timeout(Duration::from_secs(10), create).await???;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE id = ?")
        .bind(folder.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn identity_observation_does_not_assign_replaced_file_to_existing_asset(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("test.db")).await?;
    let pool = database.pool().clone();
    let source = directory.path().join("source.png");
    tokio::fs::write(&source, b"source").await?;
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        Arc::new(TokioEventBus::new()),
        directory.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let asset = ledger
        .execute(LedgerCommand::BatchCreate(vec![CreateAssetPayload {
            path: source.clone(),
            file_size: 6,
            format_type: "png".into(),
            family: "IMAGE".into(),
            state_init: AssetState::Indexed,
            folder_id: None,
            created_at: None,
            modified_at: None,
        }]))
        .await?;
    sqlx::query("DELETE FROM library_file_identities WHERE asset_id = ?")
        .bind(&asset.id)
        .execute(&pool)
        .await?;
    let original = crate::infra::filesystem::identity::read(&source).await?;
    tokio::fs::rename(&source, directory.path().join("original.png")).await?;
    tokio::fs::write(&source, b"replacement").await?;
    crate::infra::database::handlers::library::identities::remember(
        &pool,
        vec![(source, original)],
    )
    .await?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM library_file_identities WHERE asset_id = ?")
            .bind(asset.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 0);
    database.close().await;
    Ok(())
}
