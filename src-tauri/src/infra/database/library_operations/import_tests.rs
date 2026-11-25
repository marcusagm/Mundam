use super::super::{journal, move_without_replace, DiskChanges};
use super::{Fixture, TestResult};
use crate::core::{
    imports::{
        pairing::{ImportPairing, ImportScope, IssuedCredential, PairingApproval},
        publication::{PublicationAuthority, PublishPreparedImport},
        staging::{UploadError, UploadOwner, UploadState, LIMITS},
        ImportItem, ImportManifest,
    },
    ledger::{
        command::{CreateAssetPayload, LedgerCommand},
        port::TransactionalAssetLedger,
    },
    models::asset::AssetState,
};
use crate::infra::database::{
    import_pairing::SqliteImportPairing, import_staging::SqliteImportStaging,
};
use sha2::{Digest, Sha256};
use std::{io::Cursor, path::PathBuf, sync::Arc};
use uuid::Uuid;

fn png() -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(3, 2).write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

async fn prepare(
    fixture: &Fixture,
    folder_id: String,
    bytes: &[u8],
) -> Result<(PublishPreparedImport, PathBuf), Box<dyn std::error::Error>> {
    let prepared_id = Uuid::new_v4();
    let directory = fixture.directory.path().join("app").join("import-prepared");
    tokio::fs::create_dir_all(&directory).await?;
    let path = directory.join(prepared_id.to_string());
    tokio::fs::write(&path, bytes).await?;
    Ok((
        PublishPreparedImport {
            authority: PublicationAuthority::Host,
            attempt_id: None,
            batch_id: Uuid::new_v4(),
            prepared_id,
            folder_id,
            item: ImportItem {
                item_id: "one".into(),
                file_name: "cover.png".into(),
                byte_length: bytes.len() as u64,
                source_url: Some(" https://example.com/gallery ".into()),
                tag_ids: Vec::new(),
            },
            sha256: Sha256::digest(bytes).into(),
        },
        path,
    ))
}

#[tokio::test]
async fn publishes_exclusively_and_keeps_the_receipt_after_deletion() -> TestResult {
    let fixture = Fixture::new().await?;
    let folder_id = fixture.folder("root", None).await?;
    let folder = fixture.directory.path().join("root");
    let original = folder.join("cover.png");
    tokio::fs::write(&original, b"original").await?;
    sqlx::query("INSERT INTO tags (id, name) VALUES ('tag', 'Label')")
        .execute(&fixture.pool)
        .await?;
    let bytes = png()?;
    let (mut command, prepared) = prepare(&fixture, folder_id.clone(), &bytes).await?;
    command.item.tag_ids = vec!["tag".into()];

    let (first, repeated) = tokio::join!(
        fixture.ledger.publish_prepared_import(command.clone()),
        fixture.ledger.publish_prepared_import(command.clone()),
    );
    let first = first?;
    assert_eq!(repeated?, first);
    assert_eq!(first.file_name, "cover (1).png");
    let published_path = folder.join(&first.file_name);
    assert_eq!(tokio::fs::read(&original).await?, b"original");
    assert_eq!(tokio::fs::read(&published_path).await?, bytes);
    assert!(!prepared.try_exists()?);

    let (source, width, height, tag): (String, i64, i64, String) = sqlx::query_as(
        "SELECT source_url, width, height, tag_id FROM assets
         JOIN asset_metadata_envelope ON asset_metadata_envelope.asset_id = assets.id
         JOIN asset_tags ON asset_tags.asset_id = assets.id WHERE assets.id = ? AND folder_id = ?",
    )
    .bind(&first.asset_id)
    .bind(&folder_id)
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(
        (source.as_str(), width, height, tag.as_str()),
        ("https://example.com/gallery", 3, 2, "tag")
    );
    let mut changed = command.clone();
    changed.item.file_name = "another.png".into();
    assert!(fixture
        .ledger
        .publish_prepared_import(changed)
        .await
        .is_err());

    let observed = fixture
        .ledger
        .execute(LedgerCommand::CreateAsset(CreateAssetPayload {
            path: published_path.clone(),
            format_type: "png".into(),
            family: "IMAGE".into(),
            file_size: bytes.len() as u64,
            folder_id: Some(folder_id),
            state_init: AssetState::Indexed,
            created_at: None,
            modified_at: None,
        }))
        .await?;
    assert_eq!(observed.id, first.asset_id);
    let (source, tag): (String, String) = sqlx::query_as(
        "SELECT source_url, tag_id FROM assets JOIN asset_tags ON asset_id = id WHERE id = ?",
    )
    .bind(&first.asset_id)
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(
        (source.as_str(), tag.as_str()),
        ("https://example.com/gallery", "tag")
    );

    tokio::fs::remove_file(&published_path).await?;
    sqlx::query("DELETE FROM assets WHERE id = ?")
        .bind(&first.asset_id)
        .execute(&fixture.pool)
        .await?;
    assert_eq!(
        fixture.ledger.publish_prepared_import(command).await?,
        first
    );
    assert!(!published_path.try_exists()?);
    Ok(())
}

#[tokio::test]
async fn invalid_content_or_missing_tags_keeps_staging_intact() -> TestResult {
    let fixture = Fixture::new().await?;
    let folder = fixture.folder("root", None).await?;
    let (wrong_type, wrong_type_path) = prepare(&fixture, folder.clone(), b"not a PNG").await?;
    let bytes = png()?;
    let (mut missing_tag, prepared_path) = prepare(&fixture, folder, &bytes).await?;
    let mut wrong_digest = missing_tag.clone();
    wrong_digest.sha256[0] ^= 1;
    missing_tag.item.tag_ids = vec!["missing".into()];
    for command in [wrong_type, wrong_digest, missing_tag] {
        assert!(matches!(
            fixture.ledger.publish_prepared_import(command).await,
            Err(crate::core::AppError::ValidationFailed(_))
        ));
    }
    assert_eq!(tokio::fs::read(wrong_type_path).await?, b"not a PNG");
    assert_eq!(tokio::fs::read(prepared_path).await?, bytes);
    let recorded: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM assets) + (SELECT COUNT(*) FROM import_publication_receipts)",
    )
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(recorded, 0);
    assert!(tokio::fs::read_dir(fixture.directory.path().join("root"))
        .await?
        .next_entry()
        .await?
        .is_none());
    Ok(())
}

#[tokio::test]
async fn replaced_destination_requires_fresh_catalog_identity() -> TestResult {
    let fixture = Fixture::new().await?;
    let folder = fixture.folder("root", None).await?;
    let (command, prepared) = prepare(&fixture, folder, &png()?).await?;
    let destination = fixture.directory.path().join("root");
    tokio::fs::rename(&destination, fixture.directory.path().join("old-root")).await?;
    tokio::fs::create_dir(&destination).await?;
    assert!(matches!(
        fixture.ledger.publish_prepared_import(command).await,
        Err(crate::core::AppError::ValidationFailed(_))
    ));
    assert!(prepared.try_exists()?);
    assert!(!destination.join("cover.png").try_exists()?);
    Ok(())
}

#[tokio::test]
async fn sealed_staging_resumes_after_uncommitted_native_move() -> TestResult {
    let fixture = Fixture::new().await?;
    let destination_id = fixture.folder("root", None).await?;
    let bytes = png()?;
    let manifest = ImportManifest {
        folder_id: None,
        tag_ids: vec![],
        items: vec![ImportItem {
            item_id: "cover".into(),
            file_name: "cover.png".into(),
            byte_length: bytes.len() as u64,
            source_url: Some("https://example.com/gallery".into()),
            tag_ids: vec![],
        }],
    };
    let catalog_id: String = sqlx::query_scalar("SELECT catalog_id FROM import_preferences")
        .fetch_one(&fixture.pool)
        .await?;
    let owner = UploadOwner {
        catalog_id: &catalog_id,
        client_id: "browser",
    };
    let staging_path = fixture.directory.path().join("staging.sqlite");
    let staging = SqliteImportStaging::open(&staging_path).await?;
    let batch = staging
        .reserve(
            &owner,
            "retry",
            &serde_json::to_string(&manifest)?,
            &manifest,
            &destination_id,
        )
        .await?;
    staging
        .store(
            &owner,
            &batch.batch_id,
            "cover",
            0,
            &format!("{:x}", Sha256::digest(&bytes)),
            &bytes,
        )
        .await?;

    let prepared_id = Uuid::new_v4();
    let app_data = fixture.directory.path().join("app");
    let directory = app_data.join("import-prepared");
    tokio::fs::create_dir_all(&directory).await?;
    let prepared = directory.join(prepared_id.to_string());
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&prepared)
        .await?;
    let sealed = staging
        .materialize_into(&owner, &batch.batch_id, "cover", &mut file)
        .await?;
    let interrupted_attempt = staging
        .claim_publication(&owner, sealed.batch_id, "cover")
        .await?;
    file.sync_all().await?;
    drop(file);
    drop(staging);

    let destination = fixture.directory.path().join("root").join("cover.png");
    let interrupted = journal::Journal::prepare(
        &app_data,
        &DiskChanges {
            moves: vec![(prepared.clone(), destination.clone())],
            ..Default::default()
        },
    )
    .await?;
    move_without_replace(&prepared, &destination).await?;
    assert!(!prepared.try_exists()?);

    let reopened = SqliteImportStaging::open(&staging_path).await?;
    assert!(matches!(
        reopened.cancel(&owner, &batch.batch_id).await,
        Err(UploadError::PublicationPending)
    ));
    assert!(reopened
        .reconcile_publication(&owner, sealed.batch_id, &app_data, &fixture.ledger)
        .await?
        .is_none());
    assert!(prepared.try_exists()?);
    assert!(!destination.try_exists()?);
    let ready = reopened.sealed_manifest(&owner, &batch.batch_id).await?;
    assert_eq!(ready.folder_id.as_deref(), Some(destination_id.as_str()));
    let command = PublishPreparedImport {
        authority: PublicationAuthority::Host,
        attempt_id: None,
        batch_id: sealed.batch_id,
        prepared_id,
        folder_id: sealed.folder_id,
        item: sealed.item,
        sha256: sealed.sha256,
    };
    let ledger = Arc::new(fixture.ledger);
    let attempt = reopened
        .claim_publication(&owner, command.batch_id, "cover")
        .await?;
    assert!(matches!(
        reopened
            .commit_prepared(&owner, interrupted_attempt, ledger.clone(), command.clone())
            .await,
        Err(UploadError::Missing)
    ));
    let result = reopened
        .commit_prepared(&owner, attempt, ledger.clone(), command.clone())
        .await?;
    assert_eq!(ledger.publish_prepared_import(command).await?, result);
    let receipt = reopened
        .receipt(&owner, &batch.batch_id, None, None)
        .await?;
    assert!(matches!(receipt.state, UploadState::Committed));
    assert_eq!(
        receipt.items[0].asset_id.as_deref(),
        Some(result.asset_id.as_str())
    );
    assert!(receipt.confirmed_parts.is_empty());
    assert!(matches!(
        reopened.cancel(&owner, &batch.batch_id).await?.state,
        UploadState::Committed
    ));
    assert_eq!(result.file_name, "cover.png");
    assert_eq!(tokio::fs::read(&destination).await?, bytes);
    assert!(!prepared.try_exists()?);
    let journal_path = app_data
        .join("library-journal")
        .join(format!("{}.json", interrupted.operation_id));
    assert!(!journal_path.try_exists()?);
    Ok(())
}

#[tokio::test]
async fn reconciles_committed_item_after_staging_restart_and_preserves_it_on_cancel() -> TestResult
{
    let fixture = Fixture::new().await?;
    let folder_id = fixture.folder("root", None).await?;
    let bytes = png()?;
    let (mut command, prepared) = prepare(&fixture, folder_id.clone(), &bytes).await?;
    let mut remaining_item = command.item.clone();
    remaining_item.item_id = "later".into();
    remaining_item.file_name = "later.png".into();
    let manifest = ImportManifest {
        folder_id: Some(folder_id.clone()),
        tag_ids: vec![],
        items: vec![command.item.clone(), remaining_item],
    }
    .normalize(&LIMITS)?;
    command.item = manifest.items[0].clone();
    let catalog_id: String = sqlx::query_scalar("SELECT catalog_id FROM import_preferences")
        .fetch_one(&fixture.pool)
        .await?;
    let owner = UploadOwner {
        catalog_id: &catalog_id,
        client_id: "browser",
    };
    let database = fixture.directory.path().join("restart.sqlite");
    let app_data = fixture.directory.path().join("app");
    let staging = SqliteImportStaging::open(&database).await?;
    let batch = staging
        .reserve(
            &owner,
            "restart",
            &serde_json::to_string(&manifest)?,
            &manifest,
            &folder_id,
        )
        .await?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    for item in &manifest.items {
        staging
            .store(&owner, &batch.batch_id, &item.item_id, 0, &digest, &bytes)
            .await?;
    }
    command.batch_id = Uuid::parse_str(&batch.batch_id)?;
    let attempt = staging
        .claim_publication(&owner, command.batch_id, &command.item.item_id)
        .await?;
    assert!(matches!(
        staging
            .claim_publication(&owner, command.batch_id, "later")
            .await,
        Err(UploadError::PublicationPending)
    ));
    let pending = staging.receipt(&owner, &batch.batch_id, None, None).await?;
    assert!(matches!(pending.state, UploadState::Publishing));
    command.attempt_id = Some(attempt.attempt_id);
    let proof = fixture.ledger.publish_prepared_import(command).await?;
    assert!(!prepared.try_exists()?);
    // The catalog committed. Drop the staging pool before acknowledging its result there.
    drop(staging);

    let staging = SqliteImportStaging::open(&database).await?;
    let stranger = UploadOwner {
        catalog_id: &catalog_id,
        client_id: "stranger",
    };
    assert!(matches!(
        staging
            .reconcile_publication(&stranger, attempt.batch_id, &app_data, &fixture.ledger)
            .await,
        Err(UploadError::Missing)
    ));
    let other_catalog = Fixture::new().await?;
    assert!(staging
        .reconcile_publication(&owner, attempt.batch_id, &app_data, &other_catalog.ledger)
        .await
        .is_err());
    assert!(matches!(
        staging
            .receipt(&owner, &batch.batch_id, None, None)
            .await?
            .state,
        UploadState::Publishing
    ));
    assert_eq!(
        staging
            .reconcile_publication(&owner, attempt.batch_id, &app_data, &fixture.ledger)
            .await?,
        Some(proof)
    );
    let after = staging.cancel(&owner, &batch.batch_id).await?;
    assert!(matches!(after.state, UploadState::Cancelled));
    assert!(matches!(after.items[0].state, UploadState::Committed));
    assert!(after.items[0].asset_id.is_some());
    assert!(matches!(after.items[1].state, UploadState::Cancelled));
    assert!(after.confirmed_parts.is_empty());
    assert!(matches!(
        staging
            .claim_publication(&owner, attempt.batch_id, "later")
            .await,
        Err(UploadError::Cancelled)
    ));
    Ok(())
}

#[tokio::test]
async fn managed_temporaries_recover_selectively_and_publish_with_full_sync() -> TestResult {
    let fixture = Fixture::new().await?;
    let folder_id = fixture.folder("root", None).await?;
    let app_data = fixture.directory.path().join("app");
    tokio::fs::create_dir_all(&app_data).await?;
    let catalog_id: String = sqlx::query_scalar("SELECT catalog_id FROM import_preferences")
        .fetch_one(&fixture.pool)
        .await?;
    let owner = UploadOwner {
        catalog_id: &catalog_id,
        client_id: "browser",
    };
    let bytes = png()?;
    let manifest = ImportManifest {
        folder_id: None,
        tag_ids: vec![],
        items: vec![ImportItem {
            item_id: "cover".into(),
            file_name: "cover.png".into(),
            byte_length: bytes.len() as u64,
            source_url: None,
            tag_ids: vec![],
        }],
    };
    let storage = app_data.join("staging.sqlite");
    let staging = SqliteImportStaging::open(&storage).await?;
    let batch = staging
        .reserve(
            &owner,
            "managed",
            &serde_json::to_string(&manifest)?,
            &manifest,
            &folder_id,
        )
        .await?;
    let batch_id = Uuid::parse_str(&batch.batch_id)?;
    staging
        .store(
            &owner,
            &batch.batch_id,
            "cover",
            0,
            &format!("{:x}", Sha256::digest(&bytes)),
            &bytes,
        )
        .await?;
    let attempt = staging.claim_publication(&owner, batch_id, "cover").await?;
    use crate::infra::filesystem::prepared_imports;
    let abandoned = prepared_imports::path(&app_data, attempt.attempt_id);
    tokio::fs::create_dir_all(abandoned.parent().ok_or("temporary has no directory")?).await?;
    tokio::fs::write(&abandoned, b"partial output").await?;
    let unrelated = prepared_imports::path(&app_data, Uuid::new_v4());
    tokio::fs::write(&unrelated, b"keep unrelated file").await?;
    let moved = fixture.directory.path().join("root").join("cover.png");
    journal::Journal::prepare_import(
        &app_data,
        &DiskChanges {
            moves: vec![(abandoned.clone(), moved.clone())],
            ..Default::default()
        },
        b"partial output".len() as u64,
        Sha256::digest(b"partial output").into(),
        crate::infra::filesystem::import_move::ImportDirectories::open(&abandoned, &moved)
            .await?
            .parents,
    )
    .await?;
    move_without_replace(&abandoned, &moved).await?;
    drop(staging);

    let staging = SqliteImportStaging::open(&storage).await?;
    staging
        .recover_for_catalog(&catalog_id, &app_data, &fixture.ledger)
        .await?;
    assert!(!abandoned.try_exists()?);
    assert!(!moved.try_exists()?);
    assert_eq!(tokio::fs::read(&unrelated).await?, b"keep unrelated file");

    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&fixture.pool)
        .await?;
    sqlx::query(
        "CREATE TRIGGER require_full_for_import BEFORE INSERT ON import_publication_receipts
         WHEN (SELECT synchronous FROM pragma_synchronous) != 2
         BEGIN SELECT RAISE(ABORT, 'publication writer must use FULL'); END",
    )
    .execute(&fixture.pool)
    .await?;
    let ledger = Arc::new(fixture.ledger);
    let outcome = staging
        .publish_item(
            &owner,
            batch_id,
            "cover",
            PublicationAuthority::Host,
            &app_data,
            ledger,
        )
        .await?;
    let mut files =
        tokio::fs::read_dir(unrelated.parent().ok_or("temporary has no directory")?).await?;
    assert_eq!(
        files
            .next_entry()
            .await?
            .ok_or("unrelated temporary is missing")?
            .path(),
        unrelated
    );
    assert!(files.next_entry().await?.is_none());
    assert_eq!(outcome.file_name, "cover.png");
    assert_eq!(
        tokio::fs::read(fixture.directory.path().join("root").join("cover.png")).await?,
        bytes
    );
    let sync: i64 = sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(sync, 1);
    let recorded_birth: Option<String> =
        sqlx::query_scalar("SELECT birth FROM library_file_identities WHERE asset_id = ?")
            .bind(&outcome.asset_id)
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(
        recorded_birth,
        crate::infra::filesystem::identity::read(&moved)
            .await?
            .birth
            .map(|birth| birth.to_string()),
    );
    Ok(())
}

#[tokio::test]
async fn import_recovery_preserves_replaced_directory_and_can_retry() -> TestResult {
    let fixture = Fixture::new().await?;
    let root = fixture.folder("root", None).await?;
    let bytes = png()?;
    let (command, prepared) = prepare(&fixture, root, &bytes).await?;
    let app_data = fixture.directory.path().join("app");
    let destination_folder = fixture.directory.path().join("root");
    let destination = destination_folder.join("cover.png");
    let renamed = fixture.directory.path().join("prior-root");
    let source_identity = crate::infra::filesystem::identity::read(&prepared).await?;
    let guard =
        crate::infra::filesystem::import_move::ImportDirectories::open(&prepared, &destination)
            .await?;
    journal::Journal::prepare_import(
        &app_data,
        &DiskChanges {
            moves: vec![(prepared.clone(), destination.clone())],
            ..Default::default()
        },
        bytes.len() as u64,
        command.sha256,
        guard.parents.clone(),
    )
    .await?;
    guard
        .move_file(&prepared, &destination, &source_identity, false)
        .await?;
    drop(guard);

    tokio::fs::rename(&destination_folder, &renamed).await?;
    tokio::fs::create_dir(&destination_folder).await?;
    tokio::fs::write(&destination, b"independent data").await?;
    assert!(journal::recover(&fixture.pool, &app_data).await.is_err());
    assert_eq!(tokio::fs::read(&destination).await?, b"independent data");
    assert_eq!(tokio::fs::read(renamed.join("cover.png")).await?, bytes);
    assert!(!prepared.try_exists()?);

    tokio::fs::remove_file(&destination).await?;
    tokio::fs::remove_dir(&destination_folder).await?;
    tokio::fs::rename(&renamed, &destination_folder).await?;
    journal::recover(&fixture.pool, &app_data).await?;
    assert_eq!(tokio::fs::read(prepared).await?, bytes);
    assert!(!destination.try_exists()?);
    Ok(())
}

async fn pair(
    pairing: &SqliteImportPairing<'_>,
    catalog_id: &str,
    root_id: &str,
    scopes: &[ImportScope],
) -> Result<IssuedCredential, Box<dyn std::error::Error>> {
    let invitation = pairing.invite().await?;
    let request = pairing.request(&invitation.code, "Browser", scopes).await?;
    pairing
        .approve(
            &request.request_id,
            PairingApproval {
                catalog_id,
                scopes,
                root_ids: &[root_id.to_string()],
            },
        )
        .await?;
    Ok(pairing.redeem(&request.request_id, &request.secret).await?)
}

#[tokio::test]
async fn paired_publication_honors_rotation_revocation_and_receipt_ownership() -> TestResult {
    let fixture = Fixture::new().await?;
    let root_id = fixture.folder("root", None).await?;
    let catalog_id: String = sqlx::query_scalar("SELECT catalog_id FROM import_preferences")
        .fetch_one(&fixture.pool)
        .await?;
    let pairing = SqliteImportPairing::new(&fixture.pool, [42; 32]);
    let original = pair(
        &pairing,
        &catalog_id,
        &root_id,
        &[ImportScope::ImportAssets],
    )
    .await?;
    let other = pair(
        &pairing,
        &catalog_id,
        &root_id,
        &[ImportScope::ImportAssets],
    )
    .await?;
    let original_client = pairing.authenticate(&original.bearer).await?;
    let (mut command, prepared) = prepare(&fixture, root_id.clone(), &png()?).await?;
    command.authority = PublicationAuthority::Paired(original_client.publication_consent());

    let rotated = pairing.rotate(&original.bearer).await?;
    assert!(matches!(
        fixture
            .ledger
            .publish_prepared_import(command.clone())
            .await,
        Err(crate::core::AppError::Forbidden(_))
    ));
    assert!(prepared.try_exists()?);
    let client = pairing.authenticate(&rotated.bearer).await?;
    command.authority = PublicationAuthority::Paired(client.publication_consent());
    let receipt = fixture
        .ledger
        .publish_prepared_import(command.clone())
        .await?;
    assert_eq!(receipt.file_name, "cover.png");

    let stranger = pairing.authenticate(&other.bearer).await?;
    let mut foreign = command.clone();
    foreign.authority = PublicationAuthority::Paired(stranger.publication_consent());
    assert!(fixture
        .ledger
        .publish_prepared_import(foreign)
        .await
        .is_err());

    let renewed = pairing.rotate(&rotated.bearer).await?;
    let client = pairing.authenticate(&renewed.bearer).await?;
    command.authority = PublicationAuthority::Paired(client.publication_consent());
    assert_eq!(
        fixture
            .ledger
            .publish_prepared_import(command.clone())
            .await?,
        receipt
    );

    let (mut pending, pending_file) = prepare(&fixture, root_id, &png()?).await?;
    pending.authority = PublicationAuthority::Paired(client.publication_consent());
    pairing.revoke(&catalog_id, &original.client_id).await?;
    for denied in [command, pending] {
        assert!(matches!(
            fixture.ledger.publish_prepared_import(denied).await,
            Err(crate::core::AppError::Forbidden(_))
        ));
    }
    assert!(pending_file.try_exists()?);
    let published: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_publication_receipts")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(published, 1);
    Ok(())
}

#[tokio::test]
async fn paired_publication_checks_live_root_tag_scope_and_expiration() -> TestResult {
    let fixture = Fixture::new().await?;
    let root_id = fixture.folder("root", None).await?;
    let other_root = fixture.folder("other", None).await?;
    let catalog_id: String = sqlx::query_scalar("SELECT catalog_id FROM import_preferences")
        .fetch_one(&fixture.pool)
        .await?;
    let pairing = SqliteImportPairing::new(&fixture.pool, [42; 32]);
    let credential = pair(
        &pairing,
        &catalog_id,
        &root_id,
        &[ImportScope::ImportAssets],
    )
    .await?;
    let client = pairing.authenticate(&credential.bearer).await?;
    let (mut command, prepared) = prepare(&fixture, root_id.clone(), &png()?).await?;
    command.authority = PublicationAuthority::Paired(client.publication_consent());

    let mut changed_root = command.clone();
    changed_root.folder_id = other_root;
    let mut needs_tag_scope = command.clone();
    needs_tag_scope.item.tag_ids.push("label".into());
    for denied in [changed_root, needs_tag_scope] {
        assert!(matches!(
            fixture.ledger.publish_prepared_import(denied).await,
            Err(crate::core::AppError::Forbidden(_))
        ));
    }
    sqlx::query("UPDATE import_clients SET expires_at = unixepoch() WHERE id = ?")
        .bind(&credential.client_id)
        .execute(&fixture.pool)
        .await?;
    assert!(matches!(
        fixture.ledger.publish_prepared_import(command).await,
        Err(crate::core::AppError::Forbidden(_))
    ));
    assert!(prepared.try_exists()?);
    let records: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(records, 0);
    Ok(())
}
