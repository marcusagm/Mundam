use crate::core::{
    ledger::{command::LedgerCommand, port::TransactionalAssetLedger},
    plugins::derived_data::{
        PluginDerivedDataLedger, PluginDerivedResultKey, PluginPreviewSource,
        MAX_DERIVED_RESULT_BYTES,
    },
    processing::{ProcessingKind, ProcessingRepository, ProcessingResult, PublishedThumbnail},
    AppError,
};
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use serde_json::{json, Map, Value};

fn result_key() -> PluginDerivedResultKey {
    PluginDerivedResultKey {
        asset_id: "asset".into(),
        plugin_id: "org.example.histogram".into(),
        publisher_fingerprint: [1; 32],
        result_id: "histogram".into(),
        package_digest: [2; 32],
        schema_version: 1,
    }
}

fn histogram() -> Map<String, Value> {
    Map::from_iter([("red".into(), json!([0, 5, 9]))])
}

async fn publish(
    fixture: &ProcessingFixture,
) -> Result<PluginPreviewSource, Box<dyn std::error::Error>> {
    let ticket = fixture
        .processing
        .begin(ProcessingKind::Thumbnail, "asset")
        .await?
        .ok_or("Missing thumbnail attempt")?;
    let stored = fixture
        .ledger
        .commit_processing(
            &ticket,
            ProcessingResult::Thumbnail {
                thumbnail: Some(PublishedThumbnail {
                    path: format!("{}.{}.webp", ticket.asset_id, ticket.lease_id),
                    content_digest: [3; 32],
                }),
                preview_path: None,
                metadata: None,
                unsupported_reason: None,
            },
        )
        .await?;
    assert!(stored);
    fixture
        .ledger
        .plugin_preview_source("asset")
        .await?
        .ok_or_else(|| "Missing published source".into())
}

#[tokio::test]
async fn plugin_derived_result_is_scoped_to_catalog_owner_package_and_schema() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", None).await?;
    let source = publish(&fixture).await?.revision;
    let ledger = &fixture.ledger;
    let key = result_key();
    assert!(
        ledger
            .write_plugin_derived_result(&key, source, histogram())
            .await?
    );
    assert_eq!(
        ledger.read_plugin_derived_result(&key, source).await?,
        Some(histogram())
    );

    for other in [
        PluginDerivedResultKey {
            plugin_id: "org.example.other".into(),
            ..result_key()
        },
        PluginDerivedResultKey {
            publisher_fingerprint: [4; 32],
            ..result_key()
        },
        PluginDerivedResultKey {
            package_digest: [4; 32],
            ..result_key()
        },
        PluginDerivedResultKey {
            schema_version: 2,
            ..result_key()
        },
        PluginDerivedResultKey {
            result_id: "other".into(),
            ..result_key()
        },
    ] {
        assert_eq!(
            ledger.read_plugin_derived_result(&other, source).await?,
            None
        );
    }
    let other_catalog = ProcessingFixture::new().await?;
    other_catalog.asset("asset", None).await?;
    let other_source = publish(&other_catalog).await?.revision;
    assert_eq!(
        other_catalog
            .ledger
            .read_plugin_derived_result(&key, other_source)
            .await?,
        None
    );

    let oversized = Map::from_iter([(
        "label".into(),
        json!("ç".repeat(MAX_DERIVED_RESULT_BYTES / 2)),
    )]);
    assert!(matches!(
        ledger
            .write_plugin_derived_result(&key, source, oversized)
            .await,
        Err(AppError::ValidationFailed(_))
    ));
    assert_eq!(
        ledger.read_plugin_derived_result(&key, source).await?,
        Some(histogram())
    );
    Ok(())
}

#[tokio::test]
async fn plugin_derived_result_rejects_stale_completion_and_expires_with_asset() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    fixture.asset("asset", Some("legacy.webp")).await?;
    let ledger = &fixture.ledger;
    let key = result_key();
    assert_eq!(ledger.plugin_preview_source("asset").await?, None);
    ledger
        .execute(LedgerCommand::RegenerateThumbnail {
            asset_id: "asset".into(),
        })
        .await?;
    let first = publish(&fixture).await?;
    assert!(
        ledger
            .write_plugin_derived_result(&key, first.revision, histogram())
            .await?
    );

    sqlx::query("UPDATE assets SET modified_at = CURRENT_TIMESTAMP WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    assert_eq!(ledger.plugin_preview_source("asset").await?, None);
    assert_eq!(
        ledger
            .read_plugin_derived_result(&key, first.revision)
            .await?,
        None
    );
    assert!(
        !ledger
            .write_plugin_derived_result(&key, first.revision, histogram())
            .await?
    );

    let replacement = publish(&fixture).await?;
    assert_eq!(first.content_digest, replacement.content_digest);
    assert_ne!(first.revision, replacement.revision);
    assert_eq!(
        ledger
            .read_plugin_derived_result(&key, replacement.revision)
            .await?,
        None
    );
    assert!(
        ledger
            .write_plugin_derived_result(&key, replacement.revision, histogram())
            .await?
    );
    assert!(
        !ledger
            .write_plugin_derived_result(&key, first.revision, Map::new())
            .await?
    );
    assert_eq!(
        ledger
            .read_plugin_derived_result(&key, replacement.revision)
            .await?,
        Some(histogram())
    );

    sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    assert_eq!(
        ledger
            .read_plugin_derived_result(&key, replacement.revision)
            .await?,
        None
    );
    assert!(
        !ledger
            .write_plugin_derived_result(&key, replacement.revision, Map::new())
            .await?
    );
    sqlx::query("DELETE FROM assets WHERE id = 'asset'")
        .execute(&fixture.pool)
        .await?;
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_derived_results")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(remaining, 0);
    Ok(())
}
