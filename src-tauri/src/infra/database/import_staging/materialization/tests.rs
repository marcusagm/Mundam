use super::*;
use crate::core::imports::ImportItem;

#[tokio::test]
async fn reconstructs_ordered_parts_and_rejects_incomplete_cancelled_or_corrupt_staging(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    let database = workspace.path().join("import-staging.sqlite");
    let staging = SqliteImportStaging::open(&database).await?;
    let owner = UploadOwner {
        catalog_id: "catalog",
        client_id: "client",
    };
    let other = UploadOwner {
        catalog_id: "other-catalog",
        client_id: "client",
    };
    let first = vec![42; PART_BYTES];
    let last = b"last part";
    let manifest = ImportManifest {
        folder_id: None,
        tag_ids: vec![],
        items: vec![
            ImportItem {
                item_id: "first".into(),
                file_name: "asset.png".into(),
                byte_length: (first.len() + last.len()) as u64,
                source_url: None,
                tag_ids: vec!["label".into()],
            },
            ImportItem {
                item_id: "second".into(),
                file_name: "second.png".into(),
                byte_length: last.len() as u64,
                source_url: None,
                tag_ids: vec![],
            },
        ],
    };
    let batch = staging
        .reserve(
            &owner,
            "request",
            &serde_json::to_string(&manifest)?,
            &manifest,
            "indexed",
        )
        .await?;
    staging
        .store(
            &owner,
            &batch.batch_id,
            "first",
            1,
            &format!("{:x}", Sha256::digest(last)),
            last,
        )
        .await?;
    let mut output = Vec::new();
    assert!(matches!(
        staging
            .materialize_into(&owner, &batch.batch_id, "first", &mut output)
            .await,
        Err(UploadError::Incomplete)
    ));
    assert!(output.is_empty());
    staging
        .store(
            &owner,
            &batch.batch_id,
            "first",
            0,
            &format!("{:x}", Sha256::digest(&first)),
            &first,
        )
        .await?;
    assert!(matches!(
        staging.sealed_manifest(&owner, &batch.batch_id).await,
        Err(UploadError::Incomplete)
    ));
    staging
        .store(
            &owner,
            &batch.batch_id,
            "second",
            0,
            &format!("{:x}", Sha256::digest(last)),
            last,
        )
        .await?;
    drop(staging);

    let staging = SqliteImportStaging::open(&database).await?;
    let sealed = staging
        .materialize_into(&owner, &batch.batch_id, "first", &mut output)
        .await?;
    assert_eq!(sealed.folder_id, "indexed");
    assert_eq!(sealed.item.tag_ids, ["label"]);
    assert_eq!(output[..first.len()], first);
    assert_eq!(&output[first.len()..], last);
    assert_eq!(sealed.sha256, <[u8; 32]>::from(Sha256::digest(&output)));
    output.clear();
    assert!(matches!(
        staging
            .materialize_into(&other, &batch.batch_id, "first", &mut output)
            .await,
        Err(UploadError::Missing)
    ));
    assert!(output.is_empty());

    sqlx::query("UPDATE import_parts SET content = zeroblob(length(content)) WHERE batch_id = ? AND item_id = 'first' AND part_number = 1")
        .bind(&batch.batch_id).execute(&staging.0).await?;
    assert!(matches!(
        staging
            .materialize_into(&owner, &batch.batch_id, "first", &mut output)
            .await,
        Err(UploadError::StagingCorrupt)
    ));
    assert_eq!(output, first);
    staging.cancel(&owner, &batch.batch_id).await?;
    assert!(matches!(
        staging.sealed_manifest(&owner, &batch.batch_id).await,
        Err(UploadError::Cancelled)
    ));
    Ok(())
}
