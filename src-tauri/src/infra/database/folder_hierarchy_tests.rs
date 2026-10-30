//! Regression coverage for child-first library registration through the Ledger handler.

use super::adopt_orphaned_children;
use crate::core::ledger::command::CreateFolderPayload;
use crate::infra::database::handlers::folder_handler::handle_create_folder;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn database() -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::raw_sql(
        "CREATE TABLE folders (
            id TEXT PRIMARY KEY, parent_id TEXT REFERENCES folders(id),
            name TEXT NOT NULL, path TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );
        CREATE TABLE asset_operations_log (
            id TEXT PRIMARY KEY, operation_type TEXT, asset_id TEXT,
            payload TEXT, status TEXT, error_note TEXT, created_at TEXT
        );",
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}

async fn register(pool: &SqlitePool, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    register_with_parent(pool, path, None).await
}

async fn register_with_parent(
    pool: &SqlitePool,
    path: &Path,
    parent_id: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;
    let folder = handle_create_folder(
        &mut transaction,
        CreateFolderPayload {
            parent_id,
            name: path.to_string_lossy().into_owned(),
            path: path.to_path_buf(),
        },
    )
    .await?;
    transaction.commit().await?;
    Ok(folder.id)
}

async fn parent(pool: &SqlitePool, folder_id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT parent_id FROM folders WHERE id = ?")
        .bind(folder_id)
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn registering_physical_parent_adopts_existing_root_without_changing_identity() -> TestResult
{
    let pool = database().await?;
    let directory = tempfile::tempdir()?;
    let child_path = directory.path().join("Photos");
    std::fs::create_dir(&child_path)?;

    let child_id = register(&pool, &child_path).await?;
    let parent_id = register(&pool, directory.path()).await?;

    assert_eq!(parent(&pool, &child_id).await?, Some(parent_id.clone()));
    assert_eq!(parent(&pool, &parent_id).await?, None);
    assert_eq!(register(&pool, &child_path).await?, child_id);
    assert_eq!(parent(&pool, &child_id).await?, Some(parent_id));
    Ok(())
}

#[tokio::test]
async fn depth_ordered_scan_preserves_intermediate_folders_and_existing_subtrees() -> TestResult {
    let pool = database().await?;
    let root_path = Path::new("/Library");
    let middle_path = root_path.join("Projects");
    let child_path = middle_path.join("Photos");
    let child_id = register(&pool, &child_path).await?;
    let leaf_id = register(&pool, &child_path.join("Originals")).await?;
    adopt_orphaned_children(&pool, &child_id, &child_path.to_string_lossy()).await?;

    let root_id = register(&pool, root_path).await?;
    assert_eq!(parent(&pool, &child_id).await?, None);
    let middle_id = register_with_parent(&pool, &middle_path, Some(root_id.clone())).await?;

    assert_eq!(parent(&pool, &child_id).await?, Some(middle_id.clone()));
    assert_eq!(parent(&pool, &middle_id).await?, Some(root_id));
    assert_eq!(parent(&pool, &leaf_id).await?, Some(child_id));
    Ok(())
}

#[tokio::test]
async fn literal_names_do_not_adopt_similarly_named_siblings() -> TestResult {
    let pool = database().await?;
    let child_id = register(&pool, Path::new("/Library_100%/Photos")).await?;
    let sibling_id = register(&pool, Path::new("/LibraryX100Extra/Photos")).await?;
    let prefix_id = register(&pool, Path::new("/Library_100%Extra/Photos")).await?;
    let parent_id = register(&pool, Path::new("/Library_100%/")).await?;

    assert_eq!(parent(&pool, &child_id).await?, Some(parent_id));
    assert_eq!(parent(&pool, &sibling_id).await?, None);
    assert_eq!(parent(&pool, &prefix_id).await?, None);
    Ok(())
}

#[tokio::test]
async fn adoption_rolls_back_with_folder_creation() -> TestResult {
    let pool = database().await?;
    let child_id = register(&pool, Path::new("/Library/Photos")).await?;
    let mut transaction = pool.begin().await?;
    handle_create_folder(
        &mut transaction,
        CreateFolderPayload {
            parent_id: None,
            name: "Library".into(),
            path: "/Library".into(),
        },
    )
    .await?;
    transaction.rollback().await?;

    assert_eq!(parent(&pool, &child_id).await?, None);
    let folder_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders")
        .fetch_one(&pool)
        .await?;
    assert_eq!(folder_count, 1);
    Ok(())
}

#[cfg(windows)]
#[tokio::test]
async fn windows_accepts_mixed_separators_drive_roots_and_unc_paths() -> TestResult {
    let pool = database().await?;
    for (parent_path, child_path) in [
        (r"C:\Library", "c:/Library/Photos"),
        (r"D:\", r"D:\Photos"),
        (r"\\server\share\Library\", "//server/share/Library/Photos"),
        (r"\\?\E:\Library", r"\\?\E:\Library\Photos"),
    ] {
        let child_id = register(&pool, Path::new(child_path)).await?;
        let parent_id = register(&pool, Path::new(parent_path)).await?;
        assert_eq!(parent(&pool, &child_id).await?, Some(parent_id));
    }
    Ok(())
}

#[cfg(not(windows))]
#[tokio::test]
async fn unix_preserves_literal_backslashes_in_folder_names() -> TestResult {
    let pool = database().await?;
    let child_id = register(&pool, Path::new(r"/Library\Archive/Photos")).await?;
    register(&pool, Path::new("/Library")).await?;
    assert_eq!(parent(&pool, &child_id).await?, None);
    let parent_id = register(&pool, Path::new(r"/Library\Archive")).await?;
    assert_eq!(parent(&pool, &child_id).await?, Some(parent_id));
    Ok(())
}
