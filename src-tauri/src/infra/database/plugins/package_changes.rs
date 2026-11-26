//! Atomic replacement of the current package, its rollback slot and its approvals.

use super::{installed_projection, stored_digest};
use crate::core::{
    plugins::{
        InstalledPlugin, PluginInstallRecord, PluginReplacement, PluginReplacementKind,
        PublisherAnchor, StoredPluginPackage,
    },
    AppError, AppResult,
};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub(super) async fn current_package(
    pool: &SqlitePool,
    plugin_id: &str,
) -> AppResult<Option<StoredPluginPackage>> {
    let row = sqlx::query(
        "SELECT version, publisher, publisher_fingerprint, package_digest, package_revision, package
         FROM installed_plugins WHERE id = ?",
    )
    .bind(plugin_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(StoredPluginPackage {
            version: row.try_get("version")?,
            package_digest: stored_digest(&row, "package_digest")?,
            package_revision: row.try_get("package_revision")?,
            publisher: PublisherAnchor {
                namespace: row.try_get("publisher")?,
                fingerprint: stored_digest(&row, "publisher_fingerprint")?,
            },
            bytes: row.try_get("package")?,
        })
    })
    .transpose()
}

pub(super) async fn rollback_package(
    pool: &SqlitePool,
    plugin_id: &str,
) -> AppResult<Option<(Vec<u8>, [u8; 32])>> {
    let row = sqlx::query(
        "SELECT package_digest, package FROM plugin_rollback_packages WHERE plugin_id = ?",
    )
    .bind(plugin_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok((
            row.try_get("package")?,
            stored_digest(&row, "package_digest")?,
        ))
    })
    .transpose()
}

pub(super) async fn replace(
    pool: &SqlitePool,
    record: PluginInstallRecord<'_>,
    replacement: &PluginReplacement,
) -> AppResult<InstalledPlugin> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let preview = record.preview;

    let rollback_version = match replacement.kind {
        PluginReplacementKind::Update => {
            sqlx::query(
                "INSERT INTO plugin_rollback_packages (plugin_id, version, package_digest, package)
                 SELECT id, version, package_digest, package FROM installed_plugins
                 WHERE id = ?1 AND package_digest = ?2 AND package_revision = ?3
                 ON CONFLICT (plugin_id) DO UPDATE SET version = excluded.version,
                    package_digest = excluded.package_digest, package = excluded.package",
            )
            .bind(&preview.plugin_id)
            .bind(replacement.current_digest.as_slice())
            .bind(&replacement.current_revision)
            .execute(&mut *transaction)
            .await?;
            Some(replacement.current_version.as_str())
        }
        PluginReplacementKind::Rollback => {
            let consumed = sqlx::query(
                "DELETE FROM plugin_rollback_packages WHERE plugin_id = ?1 AND package_digest = ?2",
            )
            .bind(&preview.plugin_id)
            .bind(preview.package_digest.as_slice())
            .execute(&mut *transaction)
            .await?;
            if consumed.rows_affected() == 0 {
                return Err(AppError::ValidationFailed(
                    "The previous plugin package changed after review".into(),
                ));
            }
            None
        }
    };
    let replaced = sqlx::query(
        "UPDATE installed_plugins
         SET version = ?5, name = ?6, package_digest = ?7, package = ?8, package_revision = ?10
         WHERE id = ?1 AND package_digest = ?2 AND publisher = ?3 AND publisher_fingerprint = ?4
           AND package_revision = ?9
           AND EXISTS (SELECT 1 FROM plugin_publishers
                       WHERE namespace = ?3 AND fingerprint = ?4 AND revoked_at IS NULL)
         RETURNING id, publisher, version, name, 0 AS enabled, package_revision, ?11 AS rollback_version",
    )
    .bind(&preview.plugin_id)
    .bind(replacement.current_digest.as_slice())
    .bind(&preview.publisher.namespace)
    .bind(preview.publisher.fingerprint.as_slice())
    .bind(&preview.version)
    .bind(&preview.name)
    .bind(preview.package_digest.as_slice())
    .bind(record.bytes)
    .bind(&replacement.current_revision)
    .bind(Uuid::new_v4().simple().to_string())
    .bind(rollback_version)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::Forbidden("The installed plugin or publisher trust changed after review".into()))?;
    let installed = installed_projection(&replaced)?;
    sqlx::query("DELETE FROM plugin_catalog_activations WHERE plugin_id = ?")
        .bind(&preview.plugin_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(installed)
}
