//! Captures removal effects before folder cascades erase asset ownership.

use crate::core::{
    error::{AppError, AppResult},
    library::FolderUnlinkImpact,
};
use futures_util::TryStreamExt;
use sqlx::{Sqlite, Transaction};

const TREE_ASSETS: &str = r#"
    WITH RECURSIVE family AS (
        SELECT id FROM folders WHERE id = ?1
        UNION ALL
        SELECT folders.id FROM folders JOIN family ON folders.parent_id = family.id
    )
    SELECT assets.id FROM assets
    JOIN folders root ON root.id = ?1
    WHERE assets.folder_id IN (SELECT id FROM family)
       OR (assets.folder_id IS NULL
           AND SUBSTR(REPLACE(assets.path, ?2, '/'), 1,
               LENGTH(RTRIM(REPLACE(root.path, ?2, '/'), '/') || '/'))
               = (RTRIM(REPLACE(root.path, ?2, '/'), '/') || '/') COLLATE NOCASE)
"#;

/// Uses the same ownership predicate for review and the write-transaction recheck.
///
/// # Errors
/// Rejects absent or nested folders and propagates storage failures.
pub(crate) async fn unlink_impact(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<FolderUnlinkImpact> {
    let root_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM folders WHERE id = ? AND parent_id IS NULL)",
    )
    .bind(folder_id)
    .fetch_one(&mut **transaction)
    .await?;
    if !root_exists {
        return Err(AppError::NotFound("Monitored folder".into()));
    }

    let statement = format!(
        "WITH removed_assets AS ({TREE_ASSETS})
         SELECT json_array(field.asset_id, field.plugin_id, field.publisher_fingerprint,
                           field.field_id, field.schema_version, field.revision, field.value_json)
         FROM plugin_asset_data field JOIN removed_assets ON removed_assets.id = field.asset_id
         ORDER BY field.asset_id, field.plugin_id, field.publisher_fingerprint, field.field_id"
    );
    let mut rows = sqlx::query_scalar::<_, String>(&statement)
        .bind(folder_id)
        .bind(if cfg!(windows) { "\\" } else { "/" })
        .fetch(&mut **transaction);
    let mut revision = blake3::Hasher::new_derive_key("Mundam folder unlink annotations v1");
    revision.update(folder_id.as_bytes());
    revision.update(b"\0");
    let mut annotation_count = 0;
    while let Some(row) = rows.try_next().await? {
        revision.update(row.as_bytes());
        revision.update(b"\n");
        annotation_count += 1;
    }
    Ok(FolderUnlinkImpact {
        annotation_count,
        annotation_revision: revision.finalize().to_hex().to_string(),
    })
}

/// Asset identity retained until post-commit cache cleanup and event publication.
#[derive(sqlx::FromRow)]
pub(crate) struct RemovedFolderAsset {
    /// Stable identity used to locate generated cache variants.
    pub id: String,
}

/// Captures the complete asset set while recursive folder ownership still exists.
///
/// # Errors
/// Returns a database error if the removal manifest cannot be read.
pub(crate) async fn collect_assets(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<Vec<RemovedFolderAsset>> {
    Ok(sqlx::query_as(TREE_ASSETS)
        .bind(folder_id)
        .bind(if cfg!(windows) { "\\" } else { "/" })
        .fetch_all(&mut **transaction)
        .await?)
}

/// Deletes owned assets and legacy orphans physically inside the confirmed root.
///
/// # Errors
/// Returns a database error if asset or dependent-record deletion fails.
pub(crate) async fn delete_assets(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<()> {
    let group_selection = format!(
        "WITH removed_assets AS ({TREE_ASSETS})
        SELECT id FROM duplicate_groups
        WHERE canonical_asset_id IN (SELECT id FROM removed_assets)
           OR id IN (SELECT group_id FROM duplicate_candidates
                     WHERE asset_id IN (SELECT id FROM removed_assets))"
    );
    let affected_groups: Vec<String> = sqlx::query_scalar(&group_selection)
        .bind(folder_id)
        .bind(if cfg!(windows) { "\\" } else { "/" })
        .fetch_all(&mut **transaction)
        .await?;
    let statement = format!("DELETE FROM assets WHERE id IN (SELECT id FROM ({TREE_ASSETS}))");
    sqlx::query(&statement)
        .bind(folder_id)
        .bind(if cfg!(windows) { "\\" } else { "/" })
        .execute(&mut **transaction)
        .await?;
    super::duplicate_removal::reconcile_groups(transaction, &affected_groups).await?;
    Ok(())
}
