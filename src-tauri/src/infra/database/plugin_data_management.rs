//! Catalog queries and fenced erasure for durable and regenerable plugin data.

use crate::core::{
    plugins::data_management::{
        StoredPluginData, StoredPluginDataCounts, StoredPluginDataReview, StoredPluginOwner,
    },
    AppError, AppResult,
};
use futures_util::TryStreamExt;
use sqlx::SqliteConnection;

const STORED_DATA: &str = "
    SELECT 1 AS kind, '' AS recovery_id, asset_id, plugin_id, publisher_fingerprint AS fingerprint,
           field_id AS item_id, schema_version, CAST(revision AS TEXT) AS revision, value_json
    FROM plugin_asset_data
    UNION ALL
    SELECT 2, folder_id, asset_id, plugin_id, publisher_fingerprint, field_id,
           schema_version, CAST(revision AS TEXT), value_json
    FROM folder_recovery_plugin_asset_data
    UNION ALL
    SELECT 3, '', asset_id, plugin_id, lower(hex(publisher_fingerprint)), result_id,
           schema_version, hex(package_digest) || hex(source_revision), value_json
    FROM plugin_derived_results";

pub(super) async fn inventory(
    connection: &mut SqliteConnection,
) -> AppResult<Vec<StoredPluginData>> {
    let statement = format!(
        "SELECT plugin_id, fingerprint, SUM(kind = 1), SUM(kind = 2), SUM(kind = 3)
         FROM ({STORED_DATA}) GROUP BY plugin_id, fingerprint ORDER BY plugin_id, fingerprint"
    );
    let rows = sqlx::query_as::<_, (String, String, i64, i64, i64)>(&statement)
        .fetch_all(connection)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                plugin_id,
                publisher_fingerprint,
                annotations,
                recovery_annotations,
                cached_results,
            )| {
                StoredPluginData {
                    owner: StoredPluginOwner {
                        plugin_id,
                        publisher_fingerprint,
                    },
                    counts: StoredPluginDataCounts {
                        annotations: annotations.unsigned_abs(),
                        recovery_annotations: recovery_annotations.unsigned_abs(),
                        cached_results: cached_results.unsigned_abs(),
                    },
                }
            },
        )
        .collect())
}

pub(super) async fn review(
    connection: &mut SqliteConnection,
    owner: &StoredPluginOwner,
) -> AppResult<StoredPluginDataReview> {
    let catalog_id: String =
        sqlx::query_scalar("SELECT catalog_id FROM import_preferences WHERE singleton = 1")
            .fetch_one(&mut *connection)
            .await?;
    let statement = format!(
        "SELECT kind, json_array(kind, recovery_id, asset_id, item_id, schema_version, revision, value_json)
         FROM ({STORED_DATA}) WHERE plugin_id = ? AND fingerprint = ?
         ORDER BY kind, recovery_id, asset_id, item_id"
    );
    let mut entries = sqlx::query_as::<_, (i32, String)>(&statement)
        .bind(&owner.plugin_id)
        .bind(&owner.publisher_fingerprint)
        .fetch(connection);
    let mut digest = blake3::Hasher::new_derive_key("Mundam stored plugin data review v1");
    for identity_part in [
        catalog_id.as_str(),
        &owner.plugin_id,
        &owner.publisher_fingerprint,
    ] {
        digest.update(identity_part.as_bytes());
        digest.update(b"\0");
    }
    let mut counts = StoredPluginDataCounts::default();
    while let Some((kind, entry)) = entries.try_next().await? {
        match kind {
            1 => counts.annotations += 1,
            2 => counts.recovery_annotations += 1,
            3 => counts.cached_results += 1,
            _ => {
                return Err(AppError::Internal(
                    "Unknown catalog plugin data category".into(),
                ))
            }
        }
        digest.update(entry.as_bytes());
        digest.update(b"\n");
    }
    Ok(StoredPluginDataReview {
        counts,
        revision: digest.finalize().to_hex().to_string(),
    })
}

pub(super) async fn purge(
    connection: &mut SqliteConnection,
    owner: &StoredPluginOwner,
    expected_revision: &str,
) -> AppResult<bool> {
    let current = review(connection, owner).await?;
    if current.revision != expected_revision || current.counts == StoredPluginDataCounts::default()
    {
        return Ok(false);
    }
    for statement in [
        "DELETE FROM plugin_asset_data WHERE plugin_id = ? AND publisher_fingerprint = ?",
        "DELETE FROM folder_recovery_plugin_asset_data WHERE plugin_id = ? AND publisher_fingerprint = ?",
        "DELETE FROM plugin_derived_results WHERE plugin_id = ? AND lower(hex(publisher_fingerprint)) = ?",
    ] {
        sqlx::query(statement).bind(&owner.plugin_id).bind(&owner.publisher_fingerprint)
            .execute(&mut *connection).await?;
    }
    Ok(true)
}
