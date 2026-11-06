//! Recovery compatibility and application-port regression coverage.
use super::*;

#[tokio::test]
async fn committed_deletion_keeps_cache_cleanup_retryable_without_reporting_a_failed_write(
) -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/child", Some(&root)).await?;
    fixture.asset("kept", "root/child/kept.png", &child).await?;
    let invalid_cache = fixture.directory.path().join("app/thumbnails/kept.webp");
    tokio::fs::create_dir_all(&invalid_cache).await?;
    let receipt = fixture
        .ledger
        .execute_library(LibraryOperation::DeleteFolder { folder_id: child })
        .await?;
    assert!(receipt.path.join("kept.png").exists());
    assert!(receipt
        .warning
        .as_deref()
        .is_some_and(|warning| warning.contains("cleanup")));
    let pending: bool = sqlx::query_scalar(
        "SELECT cleanup_pending FROM library_operation_receipts WHERE operation_id = ?",
    )
    .bind(&receipt.operation_id)
    .fetch_one(&fixture.pool)
    .await?;
    assert!(pending);
    tokio::fs::remove_dir(invalid_cache).await?;
    retry_cleanup(&fixture.pool, &fixture.directory.path().join("app")).await?;
    let cleanup: (bool,String) = sqlx::query_as("SELECT cleanup_pending,cleanup_assets_json FROM library_operation_receipts WHERE operation_id = ?").bind(receipt.operation_id).fetch_one(&fixture.pool).await?;
    assert_eq!(cleanup, (false, "[]".into()));
    Ok(())
}

#[tokio::test]
async fn unicode_aliases_follow_the_live_volume_and_hardlinks_remain_distinct_entries() -> TestResult
{
    use crate::core::ledger::command::CreateAssetPayload;
    use crate::core::models::asset::AssetState;
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let composed = fixture.folder("root/caf\u{e9}", Some(&root)).await?;
    let decomposed = fixture.folder("root/cafe\u{301}", Some(&root)).await?;
    let first_directory = fixture.directory.path().join("root/caf\u{e9}");
    let second_directory = fixture.directory.path().join("root/cafe\u{301}");
    let aliases =
        crate::infra::filesystem::identity::same(&first_directory, &second_directory).await?;
    assert_eq!(composed == decomposed, aliases);
    let first = fixture.directory.path().join("root/first.png");
    let second = fixture.directory.path().join("root/second.png");
    tokio::fs::write(&first, b"shared").await?;
    tokio::fs::hard_link(&first, &second).await?;
    assert_eq!(
        crate::infra::filesystem::identity::read(&first).await?,
        crate::infra::filesystem::identity::read(&second).await?
    );
    assert!(!crate::infra::filesystem::identity::same(&first, &second).await?);
    let payload = |path| CreateAssetPayload {
        path,
        file_size: 6,
        format_type: "png".into(),
        family: "IMAGE".into(),
        state_init: AssetState::Indexed,
        folder_id: Some(root.clone()),
        created_at: None,
        modified_at: None,
    };
    let first = fixture
        .ledger
        .execute(LedgerCommand::CreateAsset(payload(first)))
        .await?;
    let second = fixture
        .ledger
        .execute(LedgerCommand::CreateAsset(payload(second)))
        .await?;
    assert_ne!(first.id, second.id);
    Ok(())
}

struct RecoverySettings;
#[async_trait::async_trait]
impl crate::core::settings::SettingsRepository for RecoverySettings {
    async fn load(&self) -> AppResult<crate::core::settings::AppSettings> {
        let mut settings = crate::core::settings::AppSettings::default();
        settings
            .extra
            .insert("folder_recovery_max_bytes".into(), serde_json::json!(1));
        Ok(settings)
    }
    async fn save(&self, _settings: &crate::core::settings::AppSettings) -> AppResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn persistent_history_records_restore_and_trusted_settings_bound_new_archives() -> TestResult
{
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/child", Some(&root)).await?;
    fixture.asset("kept", "root/child/kept.png", &child).await?;
    let limited = SqliteAssetLedger::new(
        fixture.pool.clone(),
        Arc::new(TokioEventBus::new()),
        fixture.directory.path().join("app"),
        fixture.guard.clone(),
    )
    .with_settings_repository(Arc::new(RecoverySettings));
    let error = limited
        .execute_library(LibraryOperation::DeleteFolder {
            folder_id: child.clone(),
        })
        .await
        .err()
        .ok_or("Quota should reject deletion")?;
    assert!(error.to_string().contains("budget exceeded"));
    assert!(fixture
        .directory
        .path()
        .join("root/child/kept.png")
        .exists());
    fixture
        .run(LibraryOperation::DeleteFolder {
            folder_id: child.clone(),
        })
        .await?;
    let history =
        crate::infra::database::query_handlers::library_recovery::page(&fixture.pool, 0).await?;
    assert_eq!(history.len(), 1);
    assert!(history[0].restorable);
    fixture
        .run(LibraryOperation::RestoreFolder { folder_id: child })
        .await?;
    let history =
        crate::infra::database::query_handlers::library_recovery::page(&fixture.pool, 0).await?;
    assert_eq!(history.len(), 1);
    assert!(!history[0].restorable);
    assert!(history[0].original_path.ends_with("child"));
    Ok(())
}

#[tokio::test]
async fn case_only_folder_rename_and_staged_rollback_preserve_original_spelling() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/lower", Some(&root)).await?;
    fixture
        .run(LibraryOperation::RenameFolder {
            folder_id: child,
            name: "LOWER".into(),
        })
        .await?;
    let names = std::fs::read_dir(fixture.directory.path().join("root"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(names.contains(&std::ffi::OsString::from("LOWER")));
    let source = fixture.directory.path().join("root/LOWER");
    let temporary = fixture.directory.path().join("root/temporary");
    let destination = fixture.directory.path().join("root/lower");
    let changes = DiskChanges {
        moves: vec![
            (source.clone(), temporary.clone()),
            (temporary.clone(), destination.clone()),
        ],
        ..Default::default()
    };
    let journal =
        journal::Journal::prepare(&fixture.directory.path().join("app"), &changes).await?;
    move_without_replace(&source, &temporary).await?;
    move_without_replace(&temporary, &destination).await?;
    journal.rollback().await?;
    let names = std::fs::read_dir(source.parent().ok_or("Missing parent")?)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(names.contains(&std::ffi::OsString::from("LOWER")));
    Ok(())
}

#[tokio::test]
async fn recovery_accepts_added_schema_defaults_skips_deleted_tags_and_rejects_corruption(
) -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/child", Some(&root)).await?;
    fixture.asset("kept", "root/child/kept.png", &child).await?;
    sqlx::query("INSERT INTO tags(id,name) VALUES ('tag','Tag')")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("INSERT INTO asset_tags(asset_id,tag_id) VALUES ('kept','tag')")
        .execute(&fixture.pool)
        .await?;
    let deleted = fixture
        .run(LibraryOperation::DeleteFolder {
            folder_id: child.clone(),
        })
        .await?;
    let snapshot: String =
        sqlx::query_scalar("SELECT snapshot_json FROM folder_recovery WHERE folder_id = ?")
            .bind(&child)
            .fetch_one(&fixture.pool)
            .await?;
    sqlx::query(
        "UPDATE folder_recovery SET snapshot_json = '{\"version\":999}' WHERE folder_id = ?",
    )
    .bind(&child)
    .execute(&fixture.pool)
    .await?;
    assert!(fixture
        .run(LibraryOperation::RestoreFolder {
            folder_id: child.clone()
        })
        .await
        .is_err());
    assert!(deleted.path.join("kept.png").exists());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM assets")
            .fetch_one(&fixture.pool)
            .await?,
        0
    );
    sqlx::query("UPDATE folder_recovery SET snapshot_json = ? WHERE folder_id = ?")
        .bind(snapshot)
        .bind(&child)
        .execute(&fixture.pool)
        .await?;
    sqlx::query("ALTER TABLE assets ADD COLUMN future_field TEXT NOT NULL DEFAULT 'compatible'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("DELETE FROM tags WHERE id = 'tag'")
        .execute(&fixture.pool)
        .await?;
    fixture
        .run(LibraryOperation::RestoreFolder { folder_id: child })
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT future_field FROM assets WHERE id = 'kept'")
            .fetch_one(&fixture.pool)
            .await?,
        "compatible"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM asset_tags")
            .fetch_one(&fixture.pool)
            .await?,
        0
    );
    Ok(())
}

#[derive(Default)]
struct RecordingObserver {
    roots: tokio::sync::Mutex<Vec<Vec<PathBuf>>>,
    fail_next: std::sync::atomic::AtomicBool,
}

#[async_trait::async_trait]
impl crate::core::library::RootObservation for RecordingObserver {
    async fn reconcile(&self, roots: Vec<PathBuf>) -> AppResult<()> {
        self.roots.lock().await.push(roots);
        if self
            .fail_next
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(AppError::Internal("Injected observation failure".into()));
        }
        Ok(())
    }
}

#[tokio::test]
async fn concurrent_root_renames_return_committed_receipts_and_observation_retries_latest_state(
) -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let registry = Arc::new(crate::core::formats::FormatRegistry::new());
    let queries =
        crate::infra::database::queries::SqliteAssetQueries::new(fixture.pool.clone(), registry);
    let observer = RecordingObserver::default();
    observer
        .fail_next
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (first, second) = tokio::join!(
        crate::feature::library::operations::execute(
            &fixture.ledger,
            &queries,
            &observer,
            LibraryOperation::RenameFolder {
                folder_id: root.clone(),
                name: "first".into()
            }
        ),
        crate::feature::library::operations::execute(
            &fixture.ledger,
            &queries,
            &observer,
            LibraryOperation::RenameFolder {
                folder_id: root,
                name: "second".into()
            }
        )
    );
    let outcomes = [first?, second?];
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| result.warning.is_some())
            .count(),
        1
    );
    let latest = outcomes
        .iter()
        .find(|result| result.path.is_dir())
        .ok_or("Missing final root")?;
    assert!(latest.warning.is_none());
    crate::feature::library::operations::reconcile(&queries, &observer).await?;
    let registrations = observer.roots.lock().await;
    assert_eq!(registrations.last(), Some(&vec![latest.path.clone()]));
    let serialized = serde_json::to_value(latest)?;
    assert!(serialized["operationId"]
        .as_str()
        .is_some_and(|identifier| !identifier.is_empty()));
    assert_eq!(serialized["entityType"], "folder");
    assert!(serialized.get("filename").is_none());
    Ok(())
}
