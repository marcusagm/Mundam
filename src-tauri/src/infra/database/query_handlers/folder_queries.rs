use crate::core::error::AppResult;
use crate::core::models::Folder;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

/// Lists folders in the database.
///
/// # Arguments
///
/// * `parent_id` - The ID of the parent folder.
///
/// # Returns
///
/// * `Ok(Vec<Folder>)` if the folders were found successfully.
/// * `Err(sqlx::Error)` if the folders could not be found.
pub async fn list_folders(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    parent_id: Option<String>,
) -> AppResult<Vec<crate::core::models::Folder>> {
    let rows = if let Some(parent) = parent_id {
        sqlx::query_as!(
            crate::infra::database::models::FolderDb,
            r#"SELECT id as "id!", parent_id, name as "name!", path as "path!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM folders WHERE parent_id = ?"#,
            parent
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            crate::infra::database::models::FolderDb,
            r#"SELECT id as "id!", parent_id, name as "name!", path as "path!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM folders WHERE parent_id IS NULL"#
        )
        .fetch_all(pool)
        .await?
    };

    Ok(rows.into_iter().map(Folder::from).collect())
}

/// Gets a folder by its ID.
///
/// # Arguments
///
/// * `id` - The ID of the folder to retrieve.
///
/// # Returns
///
/// * `Ok(Option<Folder>)` if the folder was found successfully.
/// * `Err(sqlx::Error)` if the folder could not be found.
pub async fn get_folder_by_id(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, id: &str) -> AppResult<Option<crate::core::models::Folder>> {
    let row = sqlx::query_as!(
        crate::infra::database::models::FolderDb,
        r#"SELECT id as "id!", parent_id, name as "name!", path as "path!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM folders WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Folder::from))
}

/// Lists all folders (entire hierarchy).
///
/// # Returns
///
/// * `Ok(Vec<Folder>)` if the folders were found successfully.
/// * `Err(sqlx::Error)` if the folders could not be found.
pub async fn list_all_subfolders(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<Folder>> {
    let rows = sqlx::query_as!(
        crate::infra::database::models::FolderDb,
        r#"SELECT id as "id!", parent_id, name as "name!", path as "path!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>" FROM folders ORDER BY path"#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Folder::from).collect())
}

/// Returns the asset counts for all folders (recursive).
///
/// # Returns
///
/// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
/// * `Err(sqlx::Error)` if the counts could not be found.
pub async fn get_subfolder_asset_counts(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<(String, i64)>> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE 
          hierarchy_tree(root_id, child_id) AS (
            SELECT id, id FROM folders
            UNION ALL
            SELECT ht.root_id, f.id
            FROM folders f
            JOIN hierarchy_tree ht ON f.parent_id = ht.child_id
          ),
          leaf_counts AS (
            SELECT folder_id, COUNT(*) as cnt FROM assets GROUP BY folder_id
          )
        SELECT ht.root_id as "folder_id!", SUM(COALESCE(lc.cnt, 0)) as "count!"
        FROM hierarchy_tree ht
        LEFT JOIN leaf_counts lc ON lc.folder_id = ht.child_id
        GROUP BY ht.root_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.folder_id, r.count)).collect())
}

/// Returns the asset counts for root locations.
///
/// # Returns
///
/// * `Ok(Vec<(String, i64)>)` if the counts were found successfully.
/// * `Err(sqlx::Error)` if the counts could not be found.
pub async fn get_location_root_counts(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<(String, i64)>> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE 
          hierarchy_tree(root_id, child_id) AS (
            SELECT id, id FROM folders WHERE parent_id IS NULL
            UNION ALL
            SELECT ht.root_id, f.id
            FROM folders f
            JOIN hierarchy_tree ht ON f.parent_id = ht.child_id
          ),
          leaf_counts AS (
            SELECT folder_id, COUNT(*) as cnt FROM assets GROUP BY folder_id
          )
        SELECT ht.root_id as "folder_id!", SUM(COALESCE(lc.cnt, 0)) as "count!"
        FROM hierarchy_tree ht
        LEFT JOIN leaf_counts lc ON lc.folder_id = ht.child_id
        GROUP BY ht.root_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.folder_id, r.count)).collect())
}

pub async fn find_folder_by_path(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, path: &str) -> AppResult<Option<String>> {
    let row = sqlx::query!(
        r#"SELECT id as "id!" FROM folders WHERE path = ? COLLATE NOCASE"#,
        path
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.id))
}

/// Keeps legacy repository callers consistent with transactional folder creation.
///
/// # Errors
/// Returns a database error if the hierarchy update cannot be persisted.
pub async fn adopt_orphaned_children(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, parent_id: &str, parent_path: &str) -> AppResult<()> {
    super::super::folder_hierarchy::adopt_orphaned_children(pool, parent_id, parent_path).await
}

