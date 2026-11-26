//! SQLite adapter for profile-wide trust and installed signed bytes.
//!
//! A single write transaction serializes admission with revocation. Keeping the
//! bounded package in its registry row makes metadata and bytes recover together.

use crate::core::{
    plugins::{
        InstalledPlugin, PluginActivationReview, PluginInstallRecord, PluginRegistry,
        PublisherAnchor, RegisteredPublisher,
    },
    AppError, AppResult,
};
use async_trait::async_trait;
use chrono::Utc;
use mundam_plugin_sdk::Permission;
use sqlx::{
    sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
    },
    Row, SqlitePool,
};
use std::{path::Path, time::Duration};

#[cfg(test)]
mod tests;

const MAX_INSTALLED_PACKAGES: i64 = 128;

/// Isolated profile data adapter, opened independently of the asset catalog.
pub struct SqlitePluginRegistry(SqlitePool);

impl SqlitePluginRegistry {
    /// Opens the registry and applies only plugin profile migrations.
    ///
    /// # Errors
    /// Returns filesystem, database or migration failures.
    pub async fn open(path: &Path) -> AppResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3));
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./plugin-migrations").run(&pool).await?;
        Ok(Self(pool))
    }
}

#[async_trait]
impl PluginRegistry for SqlitePluginRegistry {
    async fn trust_publisher(&self, anchor: &PublisherAnchor) -> AppResult<()> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let outcome = sqlx::query(
            "INSERT INTO plugin_publishers (namespace, fingerprint) VALUES (?1, ?2)
             ON CONFLICT DO NOTHING",
        )
        .bind(&anchor.namespace)
        .bind(anchor.fingerprint.as_slice())
        .execute(&mut *transaction)
        .await?;
        if outcome.rows_affected() == 0 {
            let identical = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM plugin_publishers
                 WHERE namespace = ?1 AND fingerprint = ?2 AND revoked_at IS NULL)",
            )
            .bind(&anchor.namespace)
            .bind(anchor.fingerprint.as_slice())
            .fetch_one(&mut *transaction)
            .await?;
            if !identical {
                return Err(AppError::ValidationFailed(
                    "Plugin publisher namespace or key is already registered or revoked".into(),
                ));
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn active_publisher(&self, fingerprint: [u8; 32]) -> AppResult<Option<PublisherAnchor>> {
        let namespace = sqlx::query_scalar::<_, String>(
            "SELECT namespace FROM plugin_publishers WHERE fingerprint = ? AND revoked_at IS NULL",
        )
        .bind(fingerprint.as_slice())
        .fetch_optional(&self.0)
        .await?;
        Ok(namespace.map(|namespace| PublisherAnchor {
            namespace,
            fingerprint,
        }))
    }

    async fn list_publishers(&self) -> AppResult<Vec<RegisteredPublisher>> {
        let rows = sqlx::query_as::<_, (String, Vec<u8>, bool)>(
            "SELECT namespace, fingerprint, revoked_at IS NOT NULL FROM plugin_publishers ORDER BY namespace",
        )
        .fetch_all(&self.0)
        .await?;
        rows.into_iter()
            .map(|(namespace, fingerprint, revoked)| {
                let fingerprint = fingerprint.try_into().map_err(|_| {
                    AppError::Internal("Invalid stored plugin publisher fingerprint".into())
                })?;
                Ok(RegisteredPublisher {
                    anchor: PublisherAnchor {
                        namespace,
                        fingerprint,
                    },
                    revoked,
                })
            })
            .collect()
    }

    async fn revoke_publisher(&self, anchor: &PublisherAnchor) -> AppResult<()> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let result = sqlx::query(
            "UPDATE plugin_publishers SET revoked_at = COALESCE(revoked_at, ?3)
             WHERE namespace = ?1 AND fingerprint = ?2",
        )
        .bind(&anchor.namespace)
        .bind(anchor.fingerprint.as_slice())
        .bind(Utc::now().timestamp())
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Plugin publisher was not registered".into(),
            ));
        }
        sqlx::query(
            "DELETE FROM plugin_catalog_activations WHERE plugin_id IN (
                SELECT id FROM installed_plugins WHERE publisher = ?1 AND publisher_fingerprint = ?2
             )",
        )
        .bind(&anchor.namespace)
        .bind(anchor.fingerprint.as_slice())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn install(&self, record: PluginInstallRecord<'_>) -> AppResult<InstalledPlugin> {
        let preview = record.preview;
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let trusted = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM plugin_publishers
             WHERE namespace = ?1 AND fingerprint = ?2 AND revoked_at IS NULL)",
        )
        .bind(&preview.publisher.namespace)
        .bind(preview.publisher.fingerprint.as_slice())
        .fetch_one(&mut *transaction)
        .await?;
        if !trusted {
            return Err(AppError::Forbidden(
                "Plugin publisher key is not trusted".into(),
            ));
        }
        let existing = sqlx::query(
            "SELECT id, publisher, version, name, 0 AS enabled, package_digest
             FROM installed_plugins WHERE id = ?",
        )
        .bind(&preview.plugin_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(row) = existing {
            if row.try_get::<Vec<u8>, _>("package_digest")? != preview.package_digest {
                return Err(AppError::ValidationFailed(
                    "A different package for this plugin is already installed".into(),
                ));
            }
            let installed = installed_projection(&row)?;
            transaction.commit().await?;
            return Ok(installed);
        }
        let installed = sqlx::query(
            "INSERT INTO installed_plugins (id, publisher, publisher_fingerprint, version, name, package_digest, package)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7 WHERE
                 (SELECT COUNT(*) FROM installed_plugins) < ?8
             RETURNING id, publisher, version, name, 0 AS enabled",
        )
        .bind(&preview.plugin_id)
        .bind(&preview.publisher.namespace)
        .bind(preview.publisher.fingerprint.as_slice())
        .bind(&preview.version)
        .bind(&preview.name)
        .bind(preview.package_digest.as_slice())
        .bind(record.bytes)
        .bind(MAX_INSTALLED_PACKAGES)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| AppError::ValidationFailed("Plugin installation limit reached".into()))?;
        let installed = installed_projection(&installed)?;
        transaction.commit().await?;
        Ok(installed)
    }

    async fn list_installed(&self, catalog_id: &str) -> AppResult<Vec<InstalledPlugin>> {
        sqlx::query(
            "SELECT plugin.id, plugin.publisher, plugin.version, plugin.name,
                    EXISTS (
                        SELECT 1 FROM plugin_catalog_activations AS approval
                        JOIN plugin_publishers AS publisher ON publisher.namespace = plugin.publisher
                            AND publisher.fingerprint = plugin.publisher_fingerprint
                        WHERE approval.catalog_id = ?1 AND approval.plugin_id = plugin.id
                          AND approval.package_digest = plugin.package_digest AND publisher.revoked_at IS NULL
                    ) AS enabled
             FROM installed_plugins AS plugin ORDER BY plugin.id",
        )
        .bind(catalog_id)
        .fetch_all(&self.0)
        .await?
        .iter()
        .map(installed_projection)
        .collect()
    }

    async fn installed_package(&self, plugin_id: &str) -> AppResult<Vec<u8>> {
        sqlx::query_scalar("SELECT package FROM installed_plugins WHERE id = ?")
            .bind(plugin_id)
            .fetch_optional(&self.0)
            .await?
            .ok_or_else(|| AppError::NotFound("Plugin is not installed".into()))
    }

    async fn enable(
        &self,
        review: &PluginActivationReview,
        permissions: &[Permission],
    ) -> AppResult<()> {
        let permissions = serde_json::to_string(permissions)
            .map_err(|_| AppError::Internal("Could not encode plugin permissions".into()))?;
        let written = sqlx::query(
            "INSERT INTO plugin_catalog_activations (catalog_id, plugin_id, package_digest, permissions)
             SELECT ?1, plugin.id, plugin.package_digest, ?4
             FROM installed_plugins AS plugin
             JOIN plugin_publishers AS publisher ON publisher.namespace = plugin.publisher
                 AND publisher.fingerprint = plugin.publisher_fingerprint
             WHERE plugin.id = ?2 AND plugin.package_digest = ?3 AND publisher.revoked_at IS NULL
             ON CONFLICT (catalog_id, plugin_id) DO UPDATE
             SET package_digest = excluded.package_digest, permissions = excluded.permissions",
        )
        .bind(&review.catalog_id)
        .bind(&review.plugin_id)
        .bind(review.package_digest.as_slice())
        .bind(permissions)
        .execute(&self.0)
        .await?;
        if written.rows_affected() == 0 {
            return Err(AppError::Forbidden(
                "Plugin package or publisher trust changed".into(),
            ));
        }
        Ok(())
    }

    async fn disable(&self, catalog_id: &str, plugin_id: &str) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM plugin_catalog_activations WHERE catalog_id = ?1 AND plugin_id = ?2",
        )
        .bind(catalog_id)
        .bind(plugin_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}

fn installed_projection(row: &SqliteRow) -> AppResult<InstalledPlugin> {
    let approved = row.try_get("enabled")?;
    Ok(InstalledPlugin {
        id: row.try_get("id")?,
        publisher: row.try_get("publisher")?,
        version: row.try_get("version")?,
        name: row.try_get("name")?,
        enabled: approved,
        approved,
    })
}
