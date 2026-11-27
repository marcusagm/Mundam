use super::*;
use crate::{
    core::plugins::export::PluginAnnotationExport,
    infra::database::{manager::DbManager, plugin_export::SqlitePluginAnnotationReader},
};
use futures_util::stream::{self, BoxStream};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn plugin_annotation_export_keeps_durable_values_and_excludes_private_paths_and_cache(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    sqlx::query(
        "INSERT INTO assets (id, name, path, state, format_type, family, file_size, deleted_at)
         VALUES ('asset-a', 'Review.png', '/private/Review.png', 'IDLE', 'png', 'IMAGE', 1, NULL),
                ('asset-b', 'Trash.png', '/private/Trash.png', 'Trashed', 'png', 'IMAGE', 1, CURRENT_TIMESTAMP)",
    ).execute(database.pool()).await?;
    let reader = SqlitePluginAnnotationReader(database.pool());
    let destination = directory.path().join("annotations.json");
    assert_eq!(annotations(&reader, "catalog", &destination).await?, 0);
    let empty: Value = serde_json::from_slice(&tokio::fs::read(&destination).await?)?;
    assert_eq!(
        empty,
        json!({"formatVersion": 1, "catalogId": "catalog", "fields": []})
    );

    let publisher = "ab".repeat(32);
    sqlx::query(
        "INSERT INTO plugin_asset_data (asset_id, plugin_id, publisher_fingerprint, field_id, schema_version, revision, value_json)
         VALUES ('asset-b', 'org.example.removed', ?1, 'reviewed', 1, 1, 'false'),
                ('asset-a', 'org.example.removed', ?1, 'reviewed', 2, 9007199254740993, 'true')",
    ).bind(&publisher).execute(database.pool()).await?;
    sqlx::query(
        "INSERT INTO plugin_derived_results (asset_id, plugin_id, publisher_fingerprint, result_id, package_digest, schema_version, source_revision, value_json)
         VALUES ('asset-a', 'org.example.removed', zeroblob(32), 'private-cache', zeroblob(32), 1, zeroblob(32), '{}')",
    ).execute(database.pool()).await?;
    assert_eq!(annotations(&reader, "catalog", &destination).await?, 2);
    let content = tokio::fs::read_to_string(&destination).await?;
    assert!(!content.contains("/private/"));
    assert!(!content.contains("private-cache"));
    let document: Value = serde_json::from_str(&content)?;
    assert_eq!(
        document["fields"][0],
        json!({
            "assetId": "asset-a", "assetName": "Review.png", "pluginId": "org.example.removed",
            "publisherFingerprint": publisher, "fieldId": "reviewed", "schemaVersion": 2,
            "revision": "9007199254740993", "value": true
        })
    );
    assert_eq!(document["fields"][1]["assetId"], "asset-b");
    assert_eq!(document["fields"][1]["value"], false);
    Ok(())
}

#[tokio::test]
async fn plugin_annotation_export_preserves_destination_after_snapshot_failure() -> TestResult {
    struct InterruptedRead;
    impl PluginAnnotationReader for InterruptedRead {
        fn annotations(&self) -> BoxStream<'_, AppResult<PluginAnnotationExport>> {
            Box::pin(stream::iter([
                Ok(PluginAnnotationExport {
                    asset_id: "asset".into(),
                    asset_name: "Review.png".into(),
                    plugin_id: "org.example.review".into(),
                    publisher_fingerprint: "ab".repeat(32),
                    field_id: "reviewed".into(),
                    schema_version: 1,
                    revision: "1".into(),
                    value: json!(true),
                }),
                Err(AppError::Internal("Snapshot interrupted".into())),
            ]))
        }
    }
    let directory = tempfile::tempdir()?;
    let destination = directory.path().join("annotations.json");
    tokio::fs::write(&destination, "previous export").await?;
    assert!(annotations(&InterruptedRead, "catalog", &destination)
        .await
        .is_err());
    assert_eq!(
        tokio::fs::read_to_string(&destination).await?,
        "previous export"
    );
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
    Ok(())
}
