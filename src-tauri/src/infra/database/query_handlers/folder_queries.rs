use crate::core::error::AppResult;
use crate::core::models::Folder;
use sqlx::SqlitePool;

/// Lists folders in the database.
///
/// # Arguments
///
/// * `parent_id` - The ID of the parent folder.
///
/// # Errors
///
/// Returns database query failures; absent entities are returned without an error.
pub async fn list_folders(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    parent_id: Option<String>,
) -> AppResult<Vec<crate::core::models::Folder>> {
    let rows = if let Some(parent) = parent_id {
        sqlx::query_as::<_, crate::infra::database::models::FolderDb>("SELECT color, inherits_color, id, parent_id, name, path, created_at, updated_at FROM folders WHERE parent_id = ?").bind(parent)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, crate::infra::database::models::FolderDb>("SELECT color, inherits_color, id, parent_id, name, path, created_at, updated_at FROM folders WHERE parent_id IS NULL")
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
/// # Errors
///
/// Returns database query failures; absent entities are returned without an error.
pub async fn get_folder_by_id(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    id: &str,
) -> AppResult<Option<crate::core::models::Folder>> {
    let row = sqlx::query_as::<_, crate::infra::database::models::FolderDb>("SELECT color, inherits_color, id, parent_id, name, path, created_at, updated_at FROM folders WHERE id = ?").bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Folder::from))
}

/// Lists all folders (entire hierarchy).
///
/// # Errors
///
/// Returns database query failures; absent entities are returned without an error.
pub async fn list_all_subfolders(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
) -> AppResult<Vec<Folder>> {
    let rows = sqlx::query_as::<_, crate::infra::database::models::FolderDb>("SELECT color, inherits_color, id, parent_id, name, path, created_at, updated_at FROM folders ORDER BY path")
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Folder::from).collect())
}

/// Returns the asset counts for all folders (recursive).
///
/// # Errors
///
/// Returns database query failures; absent entities are returned without an error.
pub async fn get_subfolder_asset_counts(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
) -> AppResult<Vec<(String, i64)>> {
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
/// # Errors
///
/// Returns database query failures; absent entities are returned without an error.
pub async fn get_location_root_counts(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
) -> AppResult<Vec<(String, i64)>> {
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

/// Resolves exact paths and verified live aliases without assuming case-insensitive volumes.
///
/// # Errors
/// Returns catalog query failures; absent paths return None.
pub async fn find_folder_by_path(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    path: &str,
) -> AppResult<Option<String>> {
    let mut connection = pool.acquire().await?;
    super::path_identity::resolve(&mut connection, "folders", std::path::Path::new(path)).await
}
