//! Read-only native identity lookup for safe watcher correlation.
use crate::core::{
    error::{AppError, AppResult},
    library::FileIdentity,
};

/// Verifies an observed destination against the catalog's durable object identity.
///
/// # Errors
/// Returns native IO or transactional lookup failures; absent identities are not proof.
pub(crate) async fn matches_destination(
    connection: &mut sqlx::SqliteConnection,
    entity: crate::core::library::LibraryEntity,
    identifier: &str,
    destination: &std::path::Path,
) -> AppResult<bool> {
    let identity = crate::infra::filesystem::identity::read(destination).await?;
    let source_query = match entity {
        crate::core::library::LibraryEntity::Asset => "SELECT path FROM assets WHERE id = ?",
        crate::core::library::LibraryEntity::Folder => "SELECT path FROM folders WHERE id = ?",
    };
    let Some(source) = sqlx::query_scalar::<_, String>(source_query)
        .bind(identifier)
        .fetch_optional(&mut *connection)
        .await?
    else {
        return Ok(false);
    };
    if tokio::fs::try_exists(&source).await?
        && !crate::infra::filesystem::identity::same(std::path::Path::new(&source), destination)
            .await?
    {
        return Ok(false);
    }
    let query = match entity {
        crate::core::library::LibraryEntity::Asset => "SELECT EXISTS(SELECT 1 FROM library_file_identities WHERE asset_id = ? AND device = ? AND object = ? AND birth IS ?)",
        crate::core::library::LibraryEntity::Folder => "SELECT EXISTS(SELECT 1 FROM library_folder_identities WHERE folder_id = ? AND device = ? AND object = ? AND birth IS ?)",
    };
    Ok(sqlx::query_scalar(query)
        .bind(identifier)
        .bind(identity.device.to_string())
        .bind(identity.object.to_string())
        .bind(identity.birth.map(|birth| birth.to_string()))
        .fetch_one(connection)
        .await?)
}

/// Reads identity without heuristics or mutation.
///
/// # Errors
/// Returns storage or invalid persisted identity errors.
pub(crate) async fn get(
    pool: &sqlx::SqlitePool,
    asset_id: &str,
) -> AppResult<Option<FileIdentity>> {
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT device, object, birth FROM library_file_identities WHERE asset_id = ?",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await?;
    row.map(|(device, object, birth)| {
        Ok(FileIdentity {
            birth: birth
                .map(|birth| {
                    birth
                        .parse()
                        .map_err(|_| AppError::Internal("Invalid identity generation".into()))
                })
                .transpose()?,
            device: device
                .parse()
                .map_err(|_| AppError::Internal("Invalid identity device".into()))?,
            object: object
                .parse()
                .map_err(|_| AppError::Internal("Invalid identity object".into()))?,
        })
    })
    .transpose()
}

/// Resolves a moved folder without transferring metadata between distinct native objects.
///
/// # Errors
/// Returns destination identity and catalog failures; absent/ambiguous candidates return None.
pub(crate) async fn folder_at_destination(
    pool: &sqlx::SqlitePool,
    path: &std::path::Path,
) -> AppResult<Option<String>> {
    let mut connection = pool.acquire().await?;
    folder_in_connection(&mut connection, path).await
}

/// Reads a native folder match using the caller's validation transaction.
///
/// # Errors
/// Returns native or catalog failures without creating metadata.
pub(crate) async fn folder_in_connection(
    connection: &mut sqlx::SqliteConnection,
    path: &std::path::Path,
) -> AppResult<Option<String>> {
    let identity = crate::infra::filesystem::identity::read(path).await?;
    let Some(birth) = identity.birth else {
        return Ok(None);
    };
    let rows: Vec<(String,String)> = sqlx::query_as("SELECT folders.id,folders.path FROM folders JOIN library_folder_identities observed ON folders.id = observed.folder_id WHERE observed.device = ? AND observed.object = ? AND observed.birth = ?")
        .bind(identity.device.to_string()).bind(identity.object.to_string()).bind(birth.to_string()).fetch_all(&mut *connection).await?;
    let mut missing = Vec::new();
    for (identifier, original) in rows {
        if !tokio::fs::try_exists(original).await? {
            missing.push(identifier);
        }
    }
    Ok(if missing.len() == 1 {
        missing.pop()
    } else {
        None
    })
}
