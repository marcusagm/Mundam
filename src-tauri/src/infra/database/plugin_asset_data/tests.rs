use crate::core::{
    path_guard::LedgerPathGuard,
    plugins::asset_data::{
        PluginAssetDataLedger, PluginAssetFieldKey, PluginBooleanField, PluginBooleanWrite,
        PluginBooleanWriteOutcome,
    },
    AppError,
};
use crate::infra::{
    database::{ledger::SqliteAssetLedger, manager::DbManager},
    events::TokioEventBus,
};
use std::path::Path;
use std::sync::Arc;

async fn catalog(
    path: &Path,
) -> Result<(DbManager, SqliteAssetLedger), Box<dyn std::error::Error>> {
    let database = DbManager::new(path).await?;
    sqlx::query(
        "INSERT INTO assets (id, name, path, state, format_type, family, file_size)
         VALUES ('asset', 'image.png', 'image.png', 'IDLE', 'png', 'IMAGE', 1)",
    )
    .execute(database.pool())
    .await?;
    let ledger = SqliteAssetLedger::new(
        database.pool().clone(),
        Arc::new(TokioEventBus::new()),
        path.to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    Ok((database, ledger))
}

fn key(plugin: &str, signer: u8) -> PluginAssetFieldKey {
    PluginAssetFieldKey {
        asset_id: "asset".into(),
        plugin_id: plugin.into(),
        publisher_fingerprint: [signer; 32],
        field_id: "reviewed".into(),
    }
}

fn write(schema_version: u32, expected_revision: i64, value: bool) -> PluginBooleanWrite {
    PluginBooleanWrite {
        schema_version,
        expected_revision,
        value,
    }
}

#[tokio::test]
async fn plugin_data_purge_rejects_stale_reviews_and_rolls_back_all_scopes(
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::core::plugins::data_management::{StoredPluginDataCounts, StoredPluginOwner};
    let directory = tempfile::tempdir()?;
    let (database, ledger) = catalog(&directory.path().join("catalog.sqlite")).await?;
    let owner = StoredPluginOwner {
        plugin_id: "org.example.review".into(),
        publisher_fingerprint: "01".repeat(32),
    };
    let chosen = key(&owner.plugin_id, 1);
    let other = key(&owner.plugin_id, 2);
    ledger
        .write_plugin_boolean(&chosen, write(1, 0, true))
        .await?;
    ledger
        .write_plugin_boolean(&other, write(1, 0, true))
        .await?;
    sqlx::query("INSERT INTO folder_recovery (folder_id, original_path, archive_path, snapshot_json) VALUES ('folder', '/original', '/archive', '[]')")
        .execute(database.pool()).await?;
    sqlx::query("INSERT INTO folder_recovery_plugin_asset_data VALUES ('folder', 'archived', ?1, ?2, 'reviewed', 1, 1, 'false')")
        .bind(&owner.plugin_id).bind(&owner.publisher_fingerprint).execute(database.pool()).await?;
    sqlx::query("INSERT INTO plugin_derived_results VALUES ('asset', ?1, ?2, 'derived', zeroblob(32), 1, zeroblob(32), '{}')")
        .bind(&owner.plugin_id).bind([1u8; 32].as_slice()).execute(database.pool()).await?;
    let first = ledger.review_stored_plugin_data(&owner).await?;
    let expected = StoredPluginDataCounts {
        annotations: 1,
        recovery_annotations: 1,
        cached_results: 1,
    };
    assert_eq!(first.counts, expected);
    let inventory = ledger.list_stored_plugin_data().await?;
    assert_eq!(inventory.len(), 2);
    assert_eq!(inventory[0].counts, expected);
    ledger
        .write_plugin_boolean(&chosen, write(1, 1, false))
        .await?;
    assert!(
        !ledger
            .purge_stored_plugin_data(&owner, &first.revision)
            .await?
    );

    let refreshed = ledger.review_stored_plugin_data(&owner).await?;
    assert_eq!(refreshed.counts, expected);
    sqlx::query("CREATE TRIGGER reject_cache_purge BEFORE DELETE ON plugin_derived_results BEGIN SELECT RAISE(ABORT, 'test'); END")
        .execute(database.pool()).await?;
    assert!(ledger
        .purge_stored_plugin_data(&owner, &refreshed.revision)
        .await
        .is_err());
    assert_eq!(
        ledger.review_stored_plugin_data(&owner).await?.revision,
        refreshed.revision
    );
    sqlx::query("DROP TRIGGER reject_cache_purge")
        .execute(database.pool())
        .await?;
    assert!(
        ledger
            .purge_stored_plugin_data(&owner, &refreshed.revision)
            .await?
    );
    assert_eq!(
        ledger.review_stored_plugin_data(&owner).await?.counts,
        StoredPluginDataCounts::default()
    );
    assert_eq!(ledger.read_plugin_boolean(&chosen).await?, None);
    assert!(ledger.read_plugin_boolean(&other).await?.is_some());
    assert_eq!(ledger.list_stored_plugin_data().await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn plugin_asset_data_is_scoped_and_survives_trash_restore(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let (database, ledger) = catalog(&directory.path().join("catalog.sqlite")).await?;
    let (_, other_catalog) = catalog(&directory.path().join("other.sqlite")).await?;
    let owner = key("org.example.review", 1);
    assert_eq!(ledger.read_plugin_boolean(&owner).await?, None);
    let stored = PluginBooleanField {
        schema_version: 1,
        revision: 1,
        value: true,
    };
    assert_eq!(
        ledger
            .write_plugin_boolean(&owner, write(1, 0, true))
            .await?,
        PluginBooleanWriteOutcome::Stored(stored)
    );
    assert_eq!(
        ledger
            .read_plugin_boolean(&key("org.example.other", 1))
            .await?,
        None
    );
    assert_eq!(
        ledger
            .read_plugin_boolean(&key("org.example.review", 2))
            .await?,
        None
    );
    assert_eq!(other_catalog.read_plugin_boolean(&owner).await?, None);
    assert_eq!(
        ledger
            .write_plugin_boolean(&owner, write(2, 1, false))
            .await?,
        PluginBooleanWriteOutcome::SchemaMismatch { stored_version: 1 }
    );

    sqlx::query("UPDATE assets SET deleted_at = CURRENT_TIMESTAMP WHERE id = 'asset'")
        .execute(database.pool())
        .await?;
    assert!(matches!(
        ledger.read_plugin_boolean(&owner).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        ledger
            .write_plugin_boolean(&owner, write(1, 1, false))
            .await,
        Err(AppError::NotFound(_))
    ));
    sqlx::query("UPDATE assets SET deleted_at = NULL WHERE id = 'asset'")
        .execute(database.pool())
        .await?;
    assert_eq!(ledger.read_plugin_boolean(&owner).await?, Some(stored));
    sqlx::query("DELETE FROM assets WHERE id = 'asset'")
        .execute(database.pool())
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM plugin_asset_data")
            .fetch_one(database.pool())
            .await?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn plugin_asset_data_accepts_one_concurrent_writer_per_revision(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let (_, ledger) = catalog(&directory.path().join("catalog.sqlite")).await?;
    let owner = key("org.example.review", 1);
    let (first, second) = tokio::join!(
        ledger.write_plugin_boolean(&owner, write(1, 0, true)),
        ledger.write_plugin_boolean(&owner, write(1, 0, false))
    );
    let outcomes = [first?, second?];
    let stored = ledger
        .read_plugin_boolean(&owner)
        .await?
        .ok_or("field missing")?;
    assert_eq!(stored.revision, 1);
    assert!(outcomes.contains(&PluginBooleanWriteOutcome::Stored(stored)));
    assert!(outcomes.contains(&PluginBooleanWriteOutcome::Conflict {
        current_revision: 1
    }));
    assert!(matches!(
        ledger
            .write_plugin_boolean(&owner, write(1, 1, !stored.value))
            .await?,
        PluginBooleanWriteOutcome::Stored(PluginBooleanField { revision: 2, .. })
    ));
    Ok(())
}
