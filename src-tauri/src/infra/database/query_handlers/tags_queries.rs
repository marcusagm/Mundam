use crate::core::error::AppResult;
use crate::core::models::Tag;
use sqlx::SqlitePool;

/// Lists tags in the database.
///
/// # Returns
///
/// * `Ok(Vec<Tag>)` if the tags were found successfully.
/// * `Err(sqlx::Error)` if the tags could not be found.
pub async fn list_tags(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<crate::core::models::Tag>> {
    let rows = sqlx::query_as!(
        crate::infra::database::models::TagDb,
        r#"SELECT id as "id!", name as "name!", color, parent_id, order_index as "order_index!" FROM tags ORDER BY order_index ASC, name ASC"#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Tag::from).collect())
}

/// Retrieves all tags associated with a specific asset.
///
/// # Arguments
///
/// * `asset_id` - The unique identifier of the asset.
///
/// # Returns
///
/// * `Ok(Vec<Tag>)` if the tags were found successfully.
/// * `Err(sqlx::Error)` if the query fails.
pub async fn get_tags_for_asset(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, asset_id: &str) -> AppResult<Vec<crate::core::models::Tag>> {
    let rows = sqlx::query_as!(
        crate::infra::database::models::TagDb,
        r#"SELECT t.id as "id!", t.name as "name!", t.color, t.parent_id, t.order_index as "order_index!"
           FROM tags t
           JOIN asset_tags at ON t.id = at.tag_id
           WHERE at.asset_id = ?
           ORDER BY t.order_index ASC, t.name ASC"#,
        asset_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Tag::from).collect())
}

/// Retrieves all tags associated with multiple assets in a single batch query.
///
/// Returns a map grouping tags by their asset unique identifier.
///
/// # Arguments
///
/// * `pool` - The SQLite database connection pool.
/// * `_registry` - The format registry reference.
/// * `asset_identifiers` - Slice of asset unique identifiers whose tags to retrieve.
///
/// # Returns
///
/// * `Ok(HashMap<String, Vec<Tag>>)` mapping each asset identifier to its associated tags.
/// * `Err(AppError)` if the query fails.
pub async fn get_tags_for_assets(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    asset_identifiers: &[String],
) -> AppResult<std::collections::HashMap<String, Vec<crate::core::models::Tag>>> {
    use std::collections::HashMap;

    if asset_identifiers.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct AssetTagRecord {
        asset_id: String,
        id: String,
        name: String,
        color: Option<String>,
        parent_id: Option<String>,
        order_index: i64,
    }

    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
        r#"
        SELECT
            at.asset_id as asset_id,
            t.id as id,
            t.name as name,
            t.color as color,
            t.parent_id as parent_id,
            t.order_index as order_index
        FROM tags t
        JOIN asset_tags at ON t.id = at.tag_id
        WHERE at.asset_id IN (
        "#,
    );

    let mut separated_clause = query_builder.separated(", ");
    for asset_identifier in asset_identifiers {
        separated_clause.push_bind(asset_identifier);
    }
    separated_clause.push_unseparated(") ORDER BY t.order_index ASC, t.name ASC");

    let asset_tag_records = query_builder
        .build_query_as::<AssetTagRecord>()
        .fetch_all(pool)
        .await?;

    let mut tags_by_asset_identifier: HashMap<String, Vec<crate::core::models::Tag>> =
        HashMap::new();
    for asset_tag_record in asset_tag_records {
        let tag = crate::core::models::Tag {
            id: asset_tag_record.id,
            name: asset_tag_record.name,
            color: asset_tag_record.color,
            parent_id: asset_tag_record.parent_id,
            order_index: asset_tag_record.order_index,
        };
        tags_by_asset_identifier
            .entry(asset_tag_record.asset_id)
            .or_default()
            .push(tag);
    }

    Ok(tags_by_asset_identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::registry::FormatRegistry;
    use crate::infra::database::manager::DbManager;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_get_tags_for_assets_empty() {
        let temp_directory = tempdir().expect("Failed to create temporary directory");
        let database_path = temp_directory.path().join("test_mundam.db");
        let database_manager = DbManager::new(&database_path)
            .await
            .expect("Failed to initialize DbManager");
        let format_registry = FormatRegistry::new();

        let tags_by_asset_identifier = get_tags_for_assets(
            database_manager.pool(),
            &format_registry,
            &[],
        )
        .await
        .expect("Query must succeed for empty asset list");
        assert!(tags_by_asset_identifier.is_empty());
    }

    #[tokio::test]
    async fn test_get_tags_for_assets_batch() {
        let temp_directory = tempdir().expect("Failed to create temporary directory");
        let database_path = temp_directory.path().join("test_mundam.db");
        let database_manager = DbManager::new(&database_path)
            .await
            .expect("Failed to initialize DbManager");
        let format_registry = FormatRegistry::new();

        sqlx::query(
            r#"
            INSERT INTO assets (id, name, path, state, format_type, family, file_size)
            VALUES
                ('test-asset-1', 'a.jpg', '/a.jpg', 'Idle', 'image/jpeg', 'Image', 100),
                ('test-asset-2', 'b.jpg', '/b.jpg', 'Idle', 'image/jpeg', 'Image', 200);

            INSERT INTO tags (id, name, order_index)
            VALUES
                ('test-tag-1', 'Nature', 0),
                ('test-tag-2', 'Landscape', 1);

            INSERT INTO asset_tags (asset_id, tag_id)
            VALUES
                ('test-asset-1', 'test-tag-1'),
                ('test-asset-1', 'test-tag-2'),
                ('test-asset-2', 'test-tag-1');
            "#,
        )
        .execute(database_manager.pool())
        .await
        .expect("Failed to seed tags");

        let requested_identifiers = vec!["test-asset-1".to_string(), "test-asset-2".to_string()];
        let tags_by_asset_identifier = get_tags_for_assets(
            database_manager.pool(),
            &format_registry,
            &requested_identifiers,
        )
        .await
        .expect("Query must succeed");

        let asset_one_tags = tags_by_asset_identifier
            .get("test-asset-1")
            .expect("test-asset-1 should have tags");
        assert_eq!(asset_one_tags.len(), 2);

        let asset_two_tags = tags_by_asset_identifier
            .get("test-asset-2")
            .expect("test-asset-2 should have tags");
        assert_eq!(asset_two_tags.len(), 1);
    }
}

