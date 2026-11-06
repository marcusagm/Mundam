//! Durable object identities prevent size/timestamp based metadata transfers.
use crate::core::{error::AppResult, library::FileIdentity};
use std::path::PathBuf;

/// Records a bounded batch of observations against exact catalog paths.
///
/// # Errors
/// Returns database errors without changing unrelated asset ownership.
pub(crate) async fn remember(
    pool: &sqlx::SqlitePool,
    identities: Vec<(PathBuf, FileIdentity)>,
) -> AppResult<()> {
    for chunk in identities.chunks(256) {
        let mut transaction = pool.begin().await?;
        for (path, identity) in chunk {
            if crate::infra::filesystem::identity::read(path)
                .await
                .ok()
                .as_ref()
                != Some(identity)
            {
                continue;
            }
            if let Some(identifier) =
                crate::infra::database::handlers::shared::resolve_asset_id_robust(
                    &mut transaction,
                    path,
                )
                .await?
            {
                sqlx::query("INSERT INTO library_file_identities(asset_id,device,object,birth) VALUES (?,?,?,?) ON CONFLICT(asset_id) DO UPDATE SET device=excluded.device,object=excluded.object,birth=excluded.birth")
                    .bind(identifier).bind(identity.device.to_string()).bind(identity.object.to_string()).bind(identity.birth.map(|birth| birth.to_string())).execute(&mut *transaction).await?;
            } else if let Some(identifier) =
                crate::infra::database::handlers::shared::resolve_folder_id_robust(
                    &mut transaction,
                    path,
                )
                .await?
            {
                folder(&mut transaction, &identifier, identity).await?;
            }
        }
        transaction.commit().await?;
    }
    Ok(())
}

/// Records a folder's native ownership in the same transaction as its catalog row.
///
/// # Errors
/// Returns persistence failures before the enclosing operation commits.
pub(crate) async fn folder(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    identifier: &str,
    identity: &FileIdentity,
) -> AppResult<()> {
    sqlx::query("INSERT INTO library_folder_identities(folder_id,device,object,birth) VALUES (?,?,?,?) ON CONFLICT(folder_id) DO UPDATE SET device=excluded.device,object=excluded.object,birth=excluded.birth")
        .bind(identifier).bind(identity.device.to_string()).bind(identity.object.to_string()).bind(identity.birth.map(|birth| birth.to_string())).execute(&mut **transaction).await?;
    Ok(())
}

/// Seeds legacy/Undo entries before observation without holding a writer during native reads.
///
/// # Errors
/// Returns catalog failures. Missing physical entries remain unseeded for the next reconciliation.
pub(crate) async fn seed_missing(pool: &sqlx::SqlitePool) -> AppResult<()> {
    for (table, identities, identifier_column) in [
        ("folders", "library_folder_identities", "folder_id"),
        ("assets", "library_file_identities", "asset_id"),
    ] {
        let mut cursor = String::new();
        loop {
            let rows: Vec<(String,String)> = sqlx::query_as(&format!("SELECT catalog.id,catalog.path FROM {table} catalog LEFT JOIN {identities} observed ON catalog.id = observed.{identifier_column} WHERE observed.{identifier_column} IS NULL AND catalog.id > ? ORDER BY catalog.id LIMIT 256")).bind(&cursor).fetch_all(pool).await?;
            let count = rows.len();
            if let Some((identifier, _)) = rows.last() {
                cursor = identifier.clone();
            }
            let mut observations = Vec::new();
            for (_, path) in rows {
                if let Ok(identity) =
                    crate::infra::filesystem::identity::read(std::path::Path::new(&path)).await
                {
                    observations.push((PathBuf::from(path), identity));
                }
            }
            remember(pool, observations).await?;
            if count < 256 {
                break;
            }
        }
    }
    Ok(())
}

/// Reconstructs optional native identity tables for legacy Undo snapshots before exposure.
///
/// # Errors
/// Rejects paths outside the archived tree and propagates native/catalog failures.
/// Missing files retain their authored metadata without a fabricated identity.
pub(crate) async fn restore_from_archive(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    root_id: &str,
    original_root: &std::path::Path,
    archive: &std::path::Path,
) -> AppResult<()> {
    for (table, identities, identifier_column) in [
        ("folders", "library_folder_identities", "folder_id"),
        ("assets", "library_file_identities", "asset_id"),
    ] {
        let scope = if table == "folders" {
            "catalog.id"
        } else {
            "catalog.folder_id"
        };
        let statement = format!("WITH RECURSIVE tree(id) AS (SELECT id FROM folders WHERE id = ? UNION SELECT folders.id FROM folders JOIN tree ON folders.parent_id = tree.id) SELECT catalog.id,catalog.path FROM {table} catalog LEFT JOIN {identities} observed ON catalog.id = observed.{identifier_column} WHERE {scope} IN (SELECT id FROM tree) AND observed.{identifier_column} IS NULL AND catalog.id > ? ORDER BY catalog.id LIMIT 256");
        let mut cursor = String::new();
        loop {
            let rows: Vec<(String, String)> = sqlx::query_as(&statement)
                .bind(root_id)
                .bind(&cursor)
                .fetch_all(&mut **transaction)
                .await?;
            let count = rows.len();
            if let Some((identifier, _)) = rows.last() {
                cursor = identifier.clone();
            }
            for (identifier, original) in rows {
                let relative = std::path::Path::new(&original)
                    .strip_prefix(original_root)
                    .map_err(|_| {
                        crate::core::error::AppError::ValidationFailed(
                            "Recovery metadata is outside the archived tree".into(),
                        )
                    })?;
                if relative.components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                }) {
                    return Err(crate::core::error::AppError::ValidationFailed(
                        "Recovery metadata escapes archive storage".into(),
                    ));
                }
                let identity =
                    match crate::infra::filesystem::identity::read(&archive.join(relative)).await {
                        Ok(identity) => identity,
                        Err(crate::core::error::AppError::Io(error))
                            if error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            continue
                        }
                        Err(error) => return Err(error),
                    };
                sqlx::query(&format!("INSERT INTO {identities}({identifier_column},device,object,birth) VALUES (?,?,?,?)"))
                    .bind(identifier).bind(identity.device.to_string()).bind(identity.object.to_string()).bind(identity.birth.map(|birth| birth.to_string())).execute(&mut **transaction).await?;
            }
            if count < 256 {
                break;
            }
        }
    }
    Ok(())
}
