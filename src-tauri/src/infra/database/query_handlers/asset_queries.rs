use crate::core::error::AppResult;
use crate::core::models::{
    Asset, AssetFilter, AssetSummaryDto, PageParams,
};
use crate::infra::database::models::AssetSummaryDb;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

/// Finds all assets in the database.
///
/// # Returns
///
/// * `Ok(Vec<Asset>)` if the assets were found successfully.
/// * `Err(sqlx::Error)` if the assets could not be found.
pub async fn find_all(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<Asset>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            m.width as "width: i64",
            m.height as "height: i64",
            a.rating as "rating: i64",
            a.notes as "notes?",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>",
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| crate::infra::database::models::AssetDb {
            id: r.id,
            name: r.name,
            path: r.path,
            state: r.state,
            format_type: r.format_type,
            family: r.family,
            file_size: r.file_size,
            created_at: r.created_at,
            modified_at: r.modified_at,
            added_at: r.added_at,
            updated_at: r.updated_at,
            folder_id: r.folder_id,
            thumbnail_path: r.thumbnail_path,
            rating: r.rating,
            notes: r.notes,
            is_favorite: r.is_favorite,
            deleted_at: r.deleted_at,
            width: r.width,
            height: r.height,
            duration_secs: r.duration_secs,
            technical_payload: r.technical_payload,
            semantic_payload: r.semantic_payload,
            dominant_color: r.dominant_color,
        })
        .map(Into::into)
        .collect())
}

/// Gets an asset by its ID.
///
/// # Arguments
///
/// * `id` - The ID of the asset to retrieve.
///
/// # Returns
///
/// * `Ok(Option<Asset>)` if the asset was found successfully.
/// * `Err(sqlx::Error)` if the asset could not be found.
pub async fn get_by_id(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, id: &str) -> AppResult<Option<Asset>> {
    let row = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            a.rating as "rating: i64",
            a.notes as "notes?",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>",
            m.width as "width: i64",
            m.height as "height: i64",
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE a.id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| crate::infra::database::models::AssetDb {
            id: r.id,
            name: r.name,
            path: r.path,
            state: r.state,
            format_type: r.format_type,
            family: r.family,
            file_size: r.file_size,
            created_at: r.created_at,
            modified_at: r.modified_at,
            added_at: r.added_at,
            updated_at: r.updated_at,
            folder_id: r.folder_id,
            thumbnail_path: r.thumbnail_path,
            rating: r.rating,
            notes: r.notes,
            is_favorite: r.is_favorite,
            deleted_at: r.deleted_at,
            width: r.width,
            height: r.height,
            duration_secs: r.duration_secs,
            technical_payload: r.technical_payload,
            semantic_payload: r.semantic_payload,
            dominant_color: r.dominant_color,
        })
        .map(Into::into))
}

/// Lists assets with pagination and filtering.
///
/// # Arguments
///
/// * `filter` - The filter to apply to the assets.
/// * `page` - The pagination parameters.
///
/// # Returns
///
/// * `Ok(Vec<AssetSummaryDto>)` if the assets were found successfully.
/// * `Err(sqlx::Error)` if the assets could not be found.
pub async fn list_paginated(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    filter: AssetFilter,
    page: PageParams,
) -> AppResult<Vec<AssetSummaryDto>> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT 
            a.id as id, a.name as name, a.path as path, a.state as state, 
            a.format_type as format_type, a.family as family, 
            a.created_at as created_at, 
            a.modified_at as modified_at,
            a.added_at as added_at,
            a.updated_at as updated_at, 
            a.folder_id as folder_id, a.thumbnail_path as thumbnail_path, 
            a.file_size as file_size, 
            m.width as width, m.height as height, 
            a.rating as rating, a.notes as notes, a.is_favorite as is_favorite, a.deleted_at as deleted_at 
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE 1=1 
        "#,
    );

    if let Some(family) = filter.family {
        query_builder.push(" AND a.family = ");
        query_builder.push_bind(family);
    }

    if let Some(state) = filter.state {
        query_builder.push(" AND a.state = ");
        query_builder.push_bind(state.to_string());
    }

    if let Some(ref search) = filter.search_query {
        if !search.is_empty() {
            let lower = search.to_lowercase();
            let chars: Vec<char> = lower.chars().collect();
            if filter.search_fuzzy.unwrap_or(false) && chars.len() >= 3 {
                let mut trigrams = Vec::new();
                for i in 0..=chars.len() - 3 {
                    let tri: String = chars[i..i + 3].iter().collect();
                    trigrams.push(format!("\"{}\"", tri));
                }
                let match_expr = trigrams.join(" OR ");
                query_builder.push(
                    " AND a.rowid IN (SELECT rowid FROM assets_fts WHERE assets_fts MATCH ",
                );
                query_builder.push_bind(match_expr);
                query_builder.push(" ORDER BY bm25(assets_fts) LIMIT 500)");
            } else {
                query_builder.push(" AND (a.name LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR a.notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(")");
            }
        }
    }

    if let Some(folder_id) = filter.folder_id {
        if filter.recursive.unwrap_or(false) {
            query_builder.push(" AND a.folder_id IN (WITH RECURSIVE subfolders AS (SELECT id FROM folders WHERE id = ");
            query_builder.push_bind(folder_id);
            query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN subfolders ON f.parent_id = subfolders.id) SELECT id FROM subfolders)");
        } else {
            query_builder.push(" AND a.folder_id = ");
            query_builder.push_bind(folder_id);
        }
    }

    if let Some(tags) = filter.tags {
        if !tags.is_empty() {
            query_builder
                .push(" AND a.id IN (SELECT asset_id FROM asset_tags WHERE tag_id IN (");
            let mut first = true;
            for tag in tags {
                if !first {
                    query_builder.push(", ");
                }
                query_builder.push_bind(tag);
                first = false;
            }
            query_builder.push("))");
        }
    }

    if let Some(untagged) = filter.untagged {
        if untagged {
            query_builder.push(" AND a.id NOT IN (SELECT asset_id FROM asset_tags)");
        }
    }

    if let Some(true) = filter.trash_only {
        query_builder.push(" AND a.deleted_at IS NOT NULL");
    } else {
        query_builder.push(" AND a.deleted_at IS NULL");
    }

    if let Some(true) = filter.favorites_only {
        query_builder.push(" AND a.is_favorite = 1");
    }

    if let Some(true) = filter.has_tags {
        query_builder.push(" AND a.id IN (SELECT asset_id FROM asset_tags)");
    }


    // --- Sorting Logic ---
    let allowed_cols = [
        "filename",      // Frontend uses 'filename'
        "name",          // Backend fallback
        "created_at",
        "modified_at",
        "added_at",
        "size",          // Frontend uses 'size'
        "file_size",     // Backend fallback
        "format",        // Frontend uses 'format'
        "format_type",   // Backend fallback
        "rating",
    ];

    let sort_field_input = filter
        .sort_by
        .as_deref()
        .filter(|c| allowed_cols.contains(c))
        .unwrap_or("created_at");

    let final_sort_by = match sort_field_input {
        "filename" => "name",
        "format" => "format_type",
        "size" => "file_size",
        other => other,
    };

    let sort_order_input = filter
        .sort_order
        .as_deref()
        .filter(|o| *o == "asc" || *o == "desc")
        .unwrap_or("desc");

    let final_order = if sort_order_input == "asc" { "ASC" } else { "DESC" };

    query_builder.push(" ORDER BY a.");
    query_builder.push(final_sort_by);

    if ["name", "format_type"].contains(&final_sort_by) {
        query_builder.push(" COLLATE NOCASE ");
    }

    query_builder.push(" ");
    query_builder.push(final_order);

    // Always use secondary sort by name to ensure stable pagination
    if final_sort_by != "name" {
        query_builder.push(", a.name COLLATE NOCASE ASC");
    }

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

/// Retrieves a single asset by its unique ID.
///
/// # Arguments
///
/// * `id` - The ID of the asset to retrieve.
///
/// # Returns
///
/// * `Ok(Asset)` if the asset was found successfully.
/// * `Err(sqlx::Error)` if the asset could not be found.
pub async fn get_asset_by_id(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, id: &str) -> AppResult<Asset> {
    let row = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            a.rating as "rating: i64",
            a.notes as "notes?",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>",
            m.width as "width: i64",
            m.height as "height: i64",
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE a.id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::core::error::AppError::NotFound(id.to_string()))?;

    let asset_db = crate::infra::database::models::AssetDb {
        id: row.id,
        name: row.name,
        path: row.path,
        state: row.state,
        format_type: row.format_type,
        family: row.family,
        file_size: row.file_size,
        created_at: row.created_at,
        modified_at: row.modified_at,
        added_at: row.added_at,
        updated_at: row.updated_at,
        folder_id: row.folder_id,
        thumbnail_path: row.thumbnail_path,
        rating: row.rating,
        notes: row.notes,
        is_favorite: row.is_favorite,
        deleted_at: row.deleted_at,
        width: row.width,
        height: row.height,
        duration_secs: row.duration_secs,
        technical_payload: row.technical_payload,
        semantic_payload: row.semantic_payload,
        dominant_color: row.dominant_color,
    };

    Ok(asset_db.into())
}

pub async fn find_asset_by_path(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, path: &str) -> AppResult<Option<Asset>> {
    let row = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            a.rating as "rating: i64",
            a.notes as "notes?",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>",
            m.width as "width: i64",
            m.height as "height: i64",
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE a.path = ? COLLATE NOCASE
        "#,
        path
    )
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| crate::infra::database::models::AssetDb {
            id: r.id,
            name: r.name,
            path: r.path,
            state: r.state,
            format_type: r.format_type,
            family: r.family,
            file_size: r.file_size,
            created_at: r.created_at,
            modified_at: r.modified_at,
            added_at: r.added_at,
            updated_at: r.updated_at,
            folder_id: r.folder_id,
            thumbnail_path: r.thumbnail_path,
            rating: r.rating,
            notes: r.notes,
            is_favorite: r.is_favorite,
            deleted_at: r.deleted_at,
            width: r.width,
            height: r.height,
            duration_secs: r.duration_secs,
            technical_payload: r.technical_payload,
            semantic_payload: r.semantic_payload,
            dominant_color: r.dominant_color,
        })
        .map(Into::into))
}

pub async fn find_assets_by_size(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    size_bytes: u64,
    state: Option<crate::core::models::AssetState>,
) -> AppResult<Vec<Asset>> {
    let size_i64 = size_bytes as i64;
    let state_str = state.map(|s| s.to_string());

    let rows = sqlx::query!(
        r#"
        SELECT 
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!", 
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!", 
            a.created_at as "created_at: DateTime<Utc>", 
            a.modified_at as "modified_at: DateTime<Utc>", 
            a.added_at as "added_at: DateTime<Utc>", 
            a.updated_at as "updated_at: DateTime<Utc>", 
            a.folder_id as "folder_id?", a.thumbnail_path as "thumbnail_path?", 
            a.rating as "rating?", a.notes as "notes?", 
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>", 
            m.width as "width?", m.height as "height?", 
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE a.file_size = ? 
          AND (? IS NULL OR a.state = ?)
        "#,
        size_i64,
        state_str,
        state_str
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| crate::infra::database::models::AssetDb {
            id: r.id,
            name: r.name,
            path: r.path,
            state: r.state,
            format_type: r.format_type,
            family: r.family,
            file_size: r.file_size,
            created_at: r.created_at,
            modified_at: r.modified_at,
            added_at: r.added_at,
            updated_at: r.updated_at,
            folder_id: r.folder_id,
            thumbnail_path: r.thumbnail_path,
            rating: r.rating,
            notes: r.notes,
            is_favorite: r.is_favorite,
            deleted_at: r.deleted_at,
            width: r.width,
            height: r.height,
            duration_secs: r.duration_secs,
            technical_payload: r.technical_payload,
            semantic_payload: r.semantic_payload,
            dominant_color: r.dominant_color,
        })
        .map(Into::into)
        .collect())
}

/// Gets the total count of assets matching the specified filter.
///
/// # Arguments
///
/// * `filter` - The given filters.
///
/// # Returns
///
/// * `Ok(i64)` asset count.
/// * `Err(sqlx::Error)` on query failure.
pub async fn get_asset_count(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, filter: AssetFilter) -> AppResult<i64> {
    let mut query_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT COUNT(*) as count FROM assets WHERE 1=1 ");

    if let Some(family) = filter.family {
        query_builder.push(" AND family = ");
        query_builder.push_bind(family);
    }

    if let Some(state) = filter.state {
        query_builder.push(" AND state = ");
        query_builder.push_bind(state.to_string());
    }

    if let Some(ref search) = filter.search_query {
        if !search.is_empty() {
            let lower = search.to_lowercase();
            let chars: Vec<char> = lower.chars().collect();
            if filter.search_fuzzy.unwrap_or(false) && chars.len() >= 3 {
                let mut trigrams = Vec::new();
                for i in 0..=chars.len() - 3 {
                    let tri: String = chars[i..i + 3].iter().collect();
                    trigrams.push(format!("\"{}\"", tri));
                }
                let match_expr = trigrams.join(" OR ");
                query_builder.push(
                    " AND rowid IN (SELECT rowid FROM assets_fts WHERE assets_fts MATCH ",
                );
                query_builder.push_bind(match_expr);
                query_builder.push(" ORDER BY bm25(assets_fts) LIMIT 500)");
            } else {
                query_builder.push(" AND (name LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(")");
            }
        }
    }

    if let Some(folder_id) = filter.folder_id {
        if filter.recursive.unwrap_or(false) {
            query_builder.push(" AND folder_id IN (WITH RECURSIVE subfolders AS (SELECT id FROM folders WHERE id = ");
            query_builder.push_bind(folder_id);
            query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN subfolders ON f.parent_id = subfolders.id) SELECT id FROM subfolders)");
        } else {
            query_builder.push(" AND folder_id = ");
            query_builder.push_bind(folder_id);
        }
    }

    if let Some(tags) = filter.tags {
        if !tags.is_empty() {
            query_builder.push(" AND id IN (SELECT asset_id FROM asset_tags WHERE tag_id IN (");
            let mut first = true;
            for tag in tags {
                if !first {
                    query_builder.push(", ");
                }
                query_builder.push_bind(tag);
                first = false;
            }
            query_builder.push("))");
        }
    }

    if let Some(untagged) = filter.untagged {
        if untagged {
            query_builder.push(" AND id NOT IN (SELECT asset_id FROM asset_tags)");
        }
    }

    if let Some(true) = filter.trash_only {
        query_builder.push(" AND deleted_at IS NOT NULL");
    } else {
        query_builder.push(" AND deleted_at IS NULL");
    }

    if let Some(true) = filter.favorites_only {
        query_builder.push(" AND is_favorite = 1");
    }

    if let Some(true) = filter.has_tags {
        query_builder.push(" AND id IN (SELECT asset_id FROM asset_tags)");
    }


    let row = query_builder.build().fetch_one(pool).await?;
    let count: i64 = sqlx::Row::get(&row, 0);

    Ok(count)
}

pub async fn get_assets_needing_repair(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<Vec<Asset>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            a.id as "id!", a.name as "name!", a.path as "path!", a.state as "state!",
            a.format_type as "format_type!", a.family as "family!", a.file_size as "file_size!",
            a.created_at as "created_at: DateTime<Utc>",
            a.modified_at as "modified_at: DateTime<Utc>",
            a.added_at as "added_at: DateTime<Utc>",
            a.updated_at as "updated_at: DateTime<Utc>",
            a.folder_id as "folder_id?",
            a.thumbnail_path as "thumbnail_path?",
            a.rating as "rating: i64",
            a.notes as "notes?",
            a.is_favorite as "is_favorite: bool",
            a.deleted_at as "deleted_at?: DateTime<Utc>",
            m.width as "width: i64",
            m.height as "height: i64",
            m.duration_secs as "duration_secs: f64",
            m.technical_payload as "technical_payload: serde_json::Value",
            m.semantic_payload as "semantic_payload: serde_json::Value",
            a.dominant_color as "dominant_color: serde_json::Value"
        FROM assets a
        LEFT JOIN asset_metadata_envelope m ON a.id = m.asset_id
        WHERE a.format_type = 'unknown' OR a.thumbnail_path IS NULL
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| crate::infra::database::models::AssetDb {
            id: r.id,
            name: r.name,
            path: r.path,
            state: r.state,
            format_type: r.format_type,
            family: r.family,
            file_size: r.file_size,
            created_at: r.created_at,
            modified_at: r.modified_at,
            added_at: r.added_at,
            updated_at: r.updated_at,
            folder_id: r.folder_id,
            thumbnail_path: r.thumbnail_path,
            rating: r.rating,
            notes: r.notes,
            is_favorite: r.is_favorite,
            deleted_at: r.deleted_at,
            width: r.width,
            height: r.height,
            duration_secs: r.duration_secs,
            technical_payload: r.technical_payload,
            semantic_payload: r.semantic_payload,
            dominant_color: r.dominant_color,
        })
        .map(Into::into)
        .collect())
}

/// Retrieves a list of asset IDs that are missing thumbnails.
///
/// # Arguments
///
/// * `limit` - The maximum number of asset IDs to retrieve.
///
/// # Returns
///
/// * `Ok(Vec<String>)` if the asset IDs were found successfully.
/// * `Err(sqlx::Error)` if the asset IDs could not be found.
pub async fn get_assets_needing_thumbnails(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry, limit: u32) -> AppResult<Vec<String>> {
    let limit_i64 = limit as i64;
    let rows = sqlx::query!(
        r#"SELECT id as "id!" FROM assets WHERE thumbnail_path IS NULL AND state != 'Thumbnailed' LIMIT ?"#,
        limit_i64
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.id).collect::<Vec<String>>())
}

/// Retrieves a map of path -> (file_size, updated_at) for all assets under a root path.
///
/// # Arguments
///
/// * `root_path` - The root path to search for assets.
///
/// # Returns
///
/// * `Ok(HashMap<String, (i64, DateTime<Utc>)>)` if the assets were found successfully.
/// * `Err(sqlx::Error)` if the assets could not be found.
pub async fn get_all_files_comparison_data(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    root_path: &str,
) -> AppResult<std::collections::HashMap<String, (i64, DateTime<Utc>)>> {
    let pattern = format!("{}%", root_path);
    let rows = sqlx::query!(
        r#"SELECT path as "path!", file_size as "file_size!", modified_at as "modified_at!: DateTime<Utc>" FROM assets WHERE path LIKE ?"#,
        pattern
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.path, (r.file_size, r.modified_at)))
        .collect())
}

