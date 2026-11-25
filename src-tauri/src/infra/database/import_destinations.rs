//! SQLite-backed selection and scoped resolution of import destinations.
use crate::core::{
    error::{AppError, AppResult},
    imports::destinations::{DestinationGrant, ImportDestinations, ImportPreference},
};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::{io::ErrorKind, path::Path};

/// Borrows the same catalog pool used by the library.
pub struct SqliteImportDestinations<'pool>(pub &'pool SqlitePool);

#[async_trait]
impl ImportDestinations for SqliteImportDestinations<'_> {
    async fn preference(&self) -> AppResult<ImportPreference> {
        let (catalog_id, folder_id) = sqlx::query_as(
            "SELECT catalog_id, default_folder_id FROM import_preferences WHERE singleton = 1",
        )
        .fetch_one(self.0)
        .await?;
        Ok(ImportPreference {
            catalog_id,
            folder_id,
        })
    }

    async fn set_default(
        &self,
        catalog_id: &str,
        folder_id: Option<&str>,
    ) -> AppResult<ImportPreference> {
        let (catalog_id, folder_id) = sqlx::query_as(
            "UPDATE import_preferences SET default_folder_id = ?1
             WHERE singleton = 1 AND catalog_id = ?2
               AND (?1 IS NULL OR EXISTS (SELECT 1 FROM folders WHERE id = ?1))
             RETURNING catalog_id, default_folder_id",
        )
        .bind(folder_id)
        .bind(catalog_id)
        .fetch_optional(self.0)
        .await?
        .ok_or_else(|| AppError::NotFound("Indexed import folder".into()))?;
        Ok(ImportPreference {
            catalog_id,
            folder_id,
        })
    }

    async fn resolve(
        &self,
        grant: &DestinationGrant,
        requested: Option<&str>,
    ) -> AppResult<Option<String>> {
        let root_ids = serde_json::to_string(&grant.root_ids)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let destination: Option<(String, String, String)> = sqlx::query_as(
            "WITH RECURSIVE lineage(id, parent_id, path) AS (
                 SELECT folders.id, folders.parent_id, folders.path
                 FROM import_preferences JOIN folders
                   ON folders.id = COALESCE(?1, import_preferences.default_folder_id)
                 WHERE singleton = 1 AND catalog_id = ?2
                 UNION
                 SELECT folders.id, folders.parent_id, folders.path
                 FROM folders JOIN lineage ON folders.id = lineage.parent_id
             )
             SELECT destination.id, destination.path, root.path
             FROM import_preferences
             JOIN lineage AS destination ON destination.id = COALESCE(?1, default_folder_id)
             JOIN lineage AS root ON root.parent_id IS NULL
             WHERE root.id IN (SELECT value FROM json_each(?3))",
        )
        .bind(requested)
        .bind(&grant.catalog_id)
        .bind(root_ids)
        .fetch_optional(self.0)
        .await?;
        let Some((id, destination, root)) = destination else {
            return Ok(None);
        };
        let Some(root) = existing_directory(Path::new(&root)).await? else {
            return Ok(None);
        };
        let Some(destination) = existing_directory(Path::new(&destination)).await? else {
            return Ok(None);
        };
        Ok(destination.starts_with(root).then_some(id))
    }
}

async fn existing_directory(path: &Path) -> AppResult<Option<std::path::PathBuf>> {
    match tokio::fs::canonicalize(path).await {
        Ok(path)
            if tokio::fs::metadata(&path)
                .await
                .is_ok_and(|entry| entry.is_dir()) =>
        {
            Ok(Some(path))
        }
        Ok(_) => Ok(None),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::PermissionDenied
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
