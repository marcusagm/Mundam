use super::{Fixture, TestResult};
use crate::core::{
    imports::{publication::PublishPreparedImport, ImportItem},
    ledger::{
        command::{CreateAssetPayload, LedgerCommand},
        port::TransactionalAssetLedger,
    },
    models::asset::AssetState,
};
use sha2::{Digest, Sha256};
use std::{io::Cursor, path::PathBuf};
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
