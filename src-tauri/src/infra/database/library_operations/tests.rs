use super::*;
use crate::core::ledger::{
    command::{CreateFolderPayload, LedgerCommand},
    port::TransactionalAssetLedger,
};
use crate::infra::{database::ledger::SqliteAssetLedger, events::TokioEventBus};
use std::path::PathBuf;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "regression_tests.rs"]
mod regression;

struct Fixture {
    directory: tempfile::TempDir,
    pool: SqlitePool,
    ledger: SqliteAssetLedger,
    guard: Arc<LedgerPathGuard>,
}

impl Fixture {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let guard = Arc::new(LedgerPathGuard::new());
        let ledger = SqliteAssetLedger::new(
            pool.clone(),
            Arc::new(TokioEventBus::new()),
            directory.path().join("app"),
            guard.clone(),
        );
        Ok(Self {
            directory,
            pool,
            ledger,
            guard,
        })
    }

    async fn folder(
        &self,
        name: &str,
        parent: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = self.directory.path().join(name);
        tokio::fs::create_dir_all(&path).await?;
        Ok(self
            .ledger
            .execute(LedgerCommand::CreateFolder(CreateFolderPayload {
                name: name.into(),
                path,
                parent_id: parent.map(String::from),
            }))
            .await?
            .id)
    }

    async fn asset(&self, identifier: &str, name: &str, parent: &str) -> TestResult {
        let path = self.directory.path().join(name);
        tokio::fs::write(&path, identifier.as_bytes()).await?;
        sqlx::query("INSERT INTO assets (id, name, path, state, format_type, family, file_size, folder_id) VALUES (?, ?, ?, 'IDLE', 'png', 'IMAGE', 4, ?)")
            .bind(identifier).bind(path.file_name().unwrap_or_default().to_string_lossy().as_ref()).bind(path.to_string_lossy().as_ref()).bind(parent).execute(&self.pool).await?;
        Ok(())
    }

    async fn run(&self, operation: LibraryOperation) -> AppResult<Asset> {
        self.ledger
            .execute(LedgerCommand::LibraryOperation(operation))
            .await
    }
}

#[tokio::test]
async fn rename_preserves_bytes_identity_notes_rating_and_extension() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    fixture.asset("asset", "root/before.png", &root).await?;
    sqlx::query("UPDATE assets SET notes = 'keep me', rating = 4 WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    fixture
        .run(LibraryOperation::RenameAsset {
            asset_id: "asset".into(),
            name: "after".into(),
        })
        .await?;
    assert_eq!(
        tokio::fs::read(fixture.directory.path().join("root/after.png")).await?,
        b"asset"
    );
    assert!(!fixture.directory.path().join("root/before.png").exists());
    let row: (String, String, i64) =
        sqlx::query_as("SELECT name, notes, rating FROM assets WHERE id = 'asset'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(row, ("after.png".into(), "keep me".into(), 4));
    Ok(())
}

#[tokio::test]
async fn collision_and_invalid_names_preserve_both_files() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    fixture.asset("first", "root/first.png", &root).await?;
    fixture.asset("second", "root/second.png", &root).await?;
    for name in ["second", "../outside", "CON", "trailing."] {
        assert!(fixture
            .run(LibraryOperation::RenameAsset {
                asset_id: "first".into(),
                name: name.into()
            })
            .await
            .is_err());
    }
    assert_eq!(
        tokio::fs::read(fixture.directory.path().join("root/first.png")).await?,
        b"first"
    );
    assert_eq!(
        tokio::fs::read(fixture.directory.path().join("root/second.png")).await?,
        b"second"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM assets")
            .fetch_one(&fixture.pool)
            .await?,
        2
    );
    Ok(())
}

#[tokio::test]
async fn subtree_move_updates_exact_paths_and_parent_without_touching_similar_names() -> TestResult
{
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let source = fixture.folder("root/100%_art", Some(&root)).await?;
    let child = fixture.folder("root/100%_art/child", Some(&source)).await?;
    let similar = fixture.folder("root/100%_artwork", Some(&root)).await?;
    let destination = fixture.folder("root/destination", Some(&root)).await?;
    fixture
        .asset("inside", "root/100%_art/child/file.png", &child)
        .await?;
    fixture
        .asset("outside", "root/100%_artwork/file.png", &similar)
        .await?;
    fixture
        .run(LibraryOperation::MoveFolder {
            folder_id: source.clone(),
            destination_id: destination.clone(),
        })
        .await?;
    let parent: String = sqlx::query_scalar("SELECT parent_id FROM folders WHERE id = ?")
        .bind(source)
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(parent, destination);
    let path: String = sqlx::query_scalar("SELECT path FROM assets WHERE id = 'inside'")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(
        PathBuf::from(path),
        fixture
            .directory
            .path()
            .join("root/destination/100%_art/child/file.png")
    );
    assert!(fixture
        .directory
        .path()
        .join("root/100%_artwork/file.png")
        .exists());
    assert!(fixture
        .directory
        .path()
        .join("root/destination/100%_art/child/file.png")
        .exists());
    Ok(())
}

#[tokio::test]
async fn missing_source_rejects_batch_before_any_move_and_rolls_back_database() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let target = fixture.folder("root/target", Some(&root)).await?;
    fixture.asset("first", "root/first.png", &root).await?;
    fixture.asset("missing", "root/missing.png", &root).await?;
    tokio::fs::remove_file(fixture.directory.path().join("root/missing.png")).await?;
    assert!(fixture
        .run(LibraryOperation::MoveAssets {
            asset_ids: vec!["first".into(), "missing".into()],
            destination_id: target
        })
        .await
        .is_err());
    assert!(fixture.directory.path().join("root/first.png").exists());
    assert!(!fixture
        .directory
        .path()
        .join("root/target/first.png")
        .exists());
    let parent: String = sqlx::query_scalar("SELECT folder_id FROM assets WHERE id = 'first'")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(parent, root);
    Ok(())
}

#[tokio::test]
async fn create_color_cycle_validation_and_recoverable_deletion() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let created = fixture
        .run(LibraryOperation::CreateFolder {
            parent_id: root.clone(),
            name: "child".into(),
        })
        .await?;
    fixture
        .run(LibraryOperation::FolderColor {
            folder_id: created.id.clone(),
            color: Some("#112233".into()),
            inherits_color: false,
        })
        .await?;
    let color: String = sqlx::query_scalar("SELECT color FROM folders WHERE id = ?")
        .bind(&created.id)
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(color, "#112233");
    assert!(fixture
        .run(LibraryOperation::MoveFolder {
            folder_id: created.id.clone(),
            destination_id: created.id.clone()
        })
        .await
        .is_err());
    fixture
        .asset("kept", "root/child/kept.png", &created.id)
        .await?;
    fixture
        .run(LibraryOperation::DeleteFolder {
            folder_id: created.id,
        })
        .await?;
    assert!(!fixture.directory.path().join("root/child").exists());
    let mut archive =
        tokio::fs::read_dir(fixture.directory.path().join("app/deleted-folders")).await?;
    let entry = archive
        .next_entry()
        .await?
        .ok_or("Missing recovery folder")?;
    assert_eq!(
        tokio::fs::read(entry.path().join("kept.png")).await?,
        b"kept"
    );
    Ok(())
}

#[tokio::test]
async fn exclusive_move_never_replaces_existing_file() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    tokio::fs::write(&source, b"source").await?;
    tokio::fs::write(&destination, b"destination").await?;
    assert!(move_without_replace(&source, &destination).await.is_err());
    assert_eq!(tokio::fs::read(source).await?, b"source");
    assert_eq!(tokio::fs::read(destination).await?, b"destination");
    Ok(())
}

#[tokio::test]
async fn linked_root_can_be_renamed_and_nested_without_losing_its_assets() -> TestResult {
    let fixture = Fixture::new().await?;
    let source = fixture.folder("source-root", None).await?;
    let destination = fixture.folder("destination-root", None).await?;
    fixture
        .asset("kept", "source-root/kept.png", &source)
        .await?;
    fixture
        .run(LibraryOperation::RenameFolder {
            folder_id: source.clone(),
            name: "renamed-root".into(),
        })
        .await?;
    fixture
        .run(LibraryOperation::MoveFolder {
            folder_id: source.clone(),
            destination_id: destination.clone(),
        })
        .await?;
    let parent: String = sqlx::query_scalar("SELECT parent_id FROM folders WHERE id = ?")
        .bind(&source)
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(parent, destination);
    assert_eq!(
        tokio::fs::read(
            fixture
                .directory
                .path()
                .join("destination-root/renamed-root/kept.png")
        )
        .await?,
        b"kept"
    );
    let owner: String = sqlx::query_scalar("SELECT folder_id FROM assets WHERE id = 'kept'")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(owner, source);
    Ok(())
}

#[tokio::test]
async fn undo_restores_files_folder_colors_and_asset_metadata_with_original_ids() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/child", Some(&root)).await?;
    fixture.asset("kept", "root/child/kept.png", &child).await?;
    sqlx::query("UPDATE folders SET color = '#112233', inherits_color = 0 WHERE id = ?")
        .bind(&child)
        .execute(&fixture.pool)
        .await?;
    sqlx::query("UPDATE assets SET notes = 'Original notes', rating = 5 WHERE id = 'kept'")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("INSERT INTO tags (id, name) VALUES ('tag', 'Preserved')")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("INSERT INTO asset_tags (asset_id, tag_id) VALUES ('kept', 'tag')")
        .execute(&fixture.pool)
        .await?;
    fixture
        .run(LibraryOperation::DeleteFolder {
            folder_id: child.clone(),
        })
        .await?;
    fixture
        .run(LibraryOperation::RestoreFolder {
            folder_id: child.clone(),
        })
        .await?;
    assert_eq!(
        tokio::fs::read(fixture.directory.path().join("root/child/kept.png")).await?,
        b"kept"
    );
    let metadata: (String, i64, String) =
        sqlx::query_as("SELECT notes, rating, folder_id FROM assets WHERE id = 'kept'")
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(metadata, ("Original notes".into(), 5, child.clone()));
    let color: String = sqlx::query_scalar("SELECT color FROM folders WHERE id = ?")
        .bind(&child)
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(color, "#112233");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset_tags WHERE asset_id = 'kept' AND tag_id = 'tag'"
        )
        .fetch_one(&fixture.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folder_recovery")
            .fetch_one(&fixture.pool)
            .await?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn undo_collision_preserves_the_recovery_copy_and_allows_retry() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let child = fixture.folder("root/child", Some(&root)).await?;
    fixture.asset("kept", "root/child/kept.png", &child).await?;
    let deleted = fixture
        .run(LibraryOperation::DeleteFolder {
            folder_id: child.clone(),
        })
        .await?;
    let original = fixture.directory.path().join("root/child");
    tokio::fs::create_dir(&original).await?;
    assert!(fixture
        .run(LibraryOperation::RestoreFolder {
            folder_id: child.clone()
        })
        .await
        .is_err());
    assert_eq!(
        tokio::fs::read(deleted.path.join("kept.png")).await?,
        b"kept"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folder_recovery")
            .fetch_one(&fixture.pool)
            .await?,
        1
    );
    tokio::fs::remove_dir(original).await?;
    fixture
        .run(LibraryOperation::RestoreFolder { folder_id: child })
        .await?;
    Ok(())
}

#[tokio::test]
async fn created_folder_does_not_suppress_files_copied_into_it() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let created = fixture
        .run(LibraryOperation::CreateFolder {
            parent_id: root,
            name: "child".into(),
        })
        .await?;
    assert!(!fixture
        .guard
        .is_marked(&PathBuf::from(&created.path).join("copied.png")));
    Ok(())
}

#[tokio::test]
async fn boot_recovery_reverses_uncommitted_disk_moves_idempotently() -> TestResult {
    let fixture = Fixture::new().await?;
    let source = fixture.directory.path().join("source");
    let destination = fixture.directory.path().join("destination");
    tokio::fs::write(&source, b"original").await?;
    let changes = DiskChanges {
        moves: vec![(source.clone(), destination.clone())],
        ..Default::default()
    };
    let app_data = fixture.directory.path().join("app");
    let _journal = journal::Journal::prepare(&app_data, &changes).await?;
    move_without_replace(&source, &destination).await?;
    journal::recover(&fixture.pool, &app_data).await?;
    journal::recover(&fixture.pool, &app_data).await?;
    assert_eq!(tokio::fs::read(&source).await?, b"original");
    assert!(!destination.exists());
    Ok(())
}

#[tokio::test]
async fn rollback_attempts_independent_moves_after_conflict_and_retains_repair_manifest(
) -> TestResult {
    let fixture = Fixture::new().await?;
    let app_data = fixture.directory.path().join("app");
    let mut changes = DiskChanges::default();
    for name in ["first", "second", "third"] {
        let source = fixture.directory.path().join(name);
        tokio::fs::write(&source, name.as_bytes()).await?;
        changes.moves.push((
            source,
            fixture.directory.path().join(format!("moved-{name}")),
        ));
    }
    let journal = journal::Journal::prepare(&app_data, &changes).await?;
    for (source, destination) in &changes.moves {
        move_without_replace(source, destination).await?;
    }
    tokio::fs::write(&changes.moves[1].0, b"external").await?;
    let error = journal
        .rollback()
        .await
        .err()
        .ok_or("Expected repair conflict")?;
    assert!(error.to_string().contains("second"));
    assert_eq!(tokio::fs::read(&changes.moves[0].0).await?, b"first");
    assert_eq!(tokio::fs::read(&changes.moves[2].0).await?, b"third");
    assert_eq!(tokio::fs::read(&changes.moves[1].0).await?, b"external");
    assert_eq!(tokio::fs::read(&changes.moves[1].1).await?, b"second");
    assert!(app_data
        .join("library-journal")
        .join(format!("{}.json", journal.operation_id))
        .exists());
    Ok(())
}

#[tokio::test]
async fn folder_color_works_offline_without_adopting_children() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    tokio::fs::remove_dir(fixture.directory.path().join("root")).await?;
    fixture
        .run(LibraryOperation::FolderColor {
            folder_id: root.clone(),
            color: Some("#123abc".into()),
            inherits_color: false,
        })
        .await?;
    let color: String = sqlx::query_scalar("SELECT color FROM folders WHERE id = ?")
        .bind(root)
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(color, "#123abc");
    assert!(!fixture.directory.path().join("root").exists());
    Ok(())
}

#[tokio::test]
async fn draft_cancel_rejects_external_content_and_preserves_receipt_for_retry() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let receipt = fixture
        .ledger
        .execute_library(LibraryOperation::CreateFolder {
            parent_id: root,
            name: "draft".into(),
        })
        .await?;
    let contents = receipt.path.join("external.txt");
    tokio::fs::write(&contents, b"keep").await?;
    let cancel = LibraryOperation::CancelFolderDraft {
        folder_id: receipt.id.clone(),
        creation_operation_id: receipt.operation_id,
    };
    assert!(fixture.run(cancel.clone()).await.is_err());
    assert_eq!(tokio::fs::read(&contents).await?, b"keep");
    tokio::fs::remove_file(contents).await?;
    fixture.run(cancel).await?;
    assert!(!receipt.path.exists());
    Ok(())
}

#[tokio::test]
async fn native_identity_recovers_move_but_never_transfers_metadata_to_same_sized_file(
) -> TestResult {
    use crate::core::{ledger::command::CreateAssetPayload, models::asset::AssetState};
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    fixture.asset("stable", "root/old.png", &root).await?;
    let old = fixture.directory.path().join("root/old.png");
    fixture
        .ledger
        .observe_file_identities(vec![old.clone()])
        .await?;
    let moved = old.with_file_name("moved.png");
    tokio::fs::rename(&old, &moved).await?;
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
    let recovered = fixture
        .ledger
        .execute(LedgerCommand::CreateAsset(payload(moved)))
        .await?;
    assert_eq!(recovered.id, "stable");
    let unrelated = old.with_file_name("other.png");
    tokio::fs::write(&unrelated, b"stable").await?;
    let created = fixture
        .ledger
        .execute(LedgerCommand::CreateAsset(payload(unrelated)))
        .await?;
    assert_ne!(created.id, "stable");
    Ok(())
}
