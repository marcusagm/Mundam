use crate::core::error::AppResult;
use crate::core::models::{
    AssetSummaryDto, PageParams,
};
use crate::infra::database::models::AssetSummaryDb;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

/// Searches for assets based on the provided criteria.
///
/// # Arguments
///
/// * `criteria` - The search criteria.
/// * `page` - The pagination parameters.
///
/// # Returns
///
/// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
/// * `Err(sqlx::Error)` if the assets could not be found.
pub async fn search_assets(
    pool: &SqlitePool,
    registry: &crate::core::formats::registry::FormatRegistry,
    criteria: crate::core::models::SearchCriteria,
    page: PageParams,
) -> AppResult<Vec<AssetSummaryDto>> {
    use crate::infra::database::search_builder::build_search_where_clause;

    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT DISTINCT
            a.id as id, a.name as name, a.path as path, a.state as state, 
            a.format_type as format_type, a.family as family, 
            a.created_at as created_at, 
            a.modified_at as modified_at,
            a.added_at as added_at,
            a.updated_at as updated_at, 
            a.folder_id as folder_id, a.thumbnail_path as thumbnail_path, 
            a.file_size as file_size, 
            m.width as width, m.height as height, 
            a.rating as rating,             a.notes as notes,
                        a.is_favorite as is_favorite, a.deleted_at as deleted_at
                    FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE 1=1 AND 
        "#,
    );

    build_search_where_clause(&criteria.root_group, &mut query_builder, registry);

    // Ordering as per project standard
    query_builder.push(" ORDER BY a.created_at DESC, a.name ASC ");

    // Pagination
    query_builder.push(" LIMIT ");
    query_builder.push_bind(page.limit() as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(page.offset() as i64);

    let rows = query_builder
        .build_query_as::<AssetSummaryDb>()
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(AssetSummaryDto::from).collect())
}

pub async fn get_search_count(
    pool: &SqlitePool,
    registry: &crate::core::formats::registry::FormatRegistry,
    criteria: crate::core::models::SearchCriteria,
) -> AppResult<i64> {
    use crate::infra::database::search_builder::build_search_where_clause;

    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT COUNT(DISTINCT a.id)
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE 1=1 AND
        "#,
    );
    build_search_where_clause(&criteria.root_group, &mut query_builder, registry);

    let row = query_builder.build().fetch_one(pool).await?;
    let count: i64 = sqlx::Row::get(&row, 0);

    Ok(count)
}

/// Lists all smart folders.
///
/// # Returns
///
/// * `Ok(Vec<SmartFolder>)` if successful.
/// * `Err(sqlx::Error)` on query failure.
pub async fn list_smart_folders(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<crate::core::models::SmartFolder>> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", name as "name!", query_json as "query_json!", created_at as "created_at?: chrono::DateTime<chrono::Utc>", updated_at as "updated_at?: chrono::DateTime<chrono::Utc>" FROM smart_folders ORDER BY name ASC"#
    )
    .fetch_all(pool)
    .await?;

    let mut folders = Vec::new();
    for r in rows {
        folders.push(crate::core::models::SmartFolder {
            id: r.id,
            name: r.name,
            query_json: r.query_json,
            created_at: r.created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: r.updated_at.unwrap_or_else(chrono::Utc::now),
        });
    }

    Ok(folders)
}

