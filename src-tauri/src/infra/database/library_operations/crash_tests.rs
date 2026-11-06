use super::*;
use std::path::PathBuf;
type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn database(directory: &Path) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(directory.join("catalog.sqlite"))
        .create_if_missing(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[tokio::test]
#[ignore = "Executed explicitly by journal_survives_process_exit_at_durable_boundaries"]
async fn journal_crash_child() -> TestResult {
    let directory = PathBuf::from(std::env::var("MUNDAM_CRASH_TEST_DIRECTORY")?);
    let phase = std::env::var("MUNDAM_CRASH_TEST_PHASE")?;
    let pool = database(&directory).await?;
    let source = directory.join("source");
    let destination = directory.join("destination");
    tokio::fs::write(&source, b"original").await?;
    let changes = DiskChanges {
        moves: vec![(source.clone(), destination.clone())],
        ..Default::default()
    };
    let journal = journal::Journal::prepare(&directory, &changes).await?;
    let mut transaction = pool.begin().await?;
    sqlx::query("INSERT INTO library_operation_receipts(operation_id,entity_id,operation_kind) VALUES (?,'entity','renameAsset')")
        .bind(&journal.operation_id).execute(&mut *transaction).await?;
    if phase != "prepared" {
        move_without_replace(&source, &destination).await?;
    }
    if phase == "committed" {
        transaction.commit().await?;
    }
    std::process::exit(86);
}

#[tokio::test]
async fn journal_survives_process_exit_at_durable_boundaries() -> TestResult {
    for phase in ["prepared", "moved", "committed"] {
        let directory = tempfile::tempdir()?;
        let executable = std::env::current_exe()?;
        let child_directory = directory.path().to_path_buf();
        let status = tokio::task::spawn_blocking(move || {
            std::process::Command::new(executable)
                .args([
                    "--ignored",
                    "--exact",
                    "infra::database::library_operations::crash_tests::journal_crash_child",
                ])
                .env("MUNDAM_CRASH_TEST_DIRECTORY", child_directory)
                .env("MUNDAM_CRASH_TEST_PHASE", phase)
                .output()
        })
        .await??;
        assert_eq!(
            status.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let pool = database(directory.path()).await?;
        journal::recover(&pool, directory.path()).await?;
        journal::recover(&pool, directory.path()).await?;
        let expected = if phase == "committed" {
            "destination"
        } else {
            "source"
        };
        assert_eq!(
            tokio::fs::read(directory.path().join(expected)).await?,
            b"original"
        );
        pool.close().await;
    }
    Ok(())
}

#[tokio::test]
async fn failed_sqlite_commit_compensates_disk_and_preserves_original_error() -> TestResult {
    let directory = tempfile::tempdir()?;
    let pool = database(directory.path()).await?;
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    tokio::fs::write(&source, b"original").await?;
    let changes = DiskChanges {
        moves: vec![(source.clone(), destination.clone())],
        ..Default::default()
    };
    let journal = journal::Journal::prepare(directory.path(), &changes).await?;
    let mut transaction = pool.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "INSERT INTO library_file_identities(asset_id,device,object) VALUES ('missing','1','2')",
    )
    .execute(&mut *transaction)
    .await?;
    move_without_replace(&source, &destination).await?;
    let error = commit_or_repair(transaction, &pool, &journal, directory.path())
        .await
        .err()
        .ok_or("Expected failed commit")?;
    assert!(error.to_string().to_lowercase().contains("foreign key"));
    assert_eq!(tokio::fs::read(&source).await?, b"original");
    assert!(!destination.exists());
    Ok(())
}
