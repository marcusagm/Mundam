use crate::core::error::AppResult;
use sqlx::SqlitePool;

/// Gets library statistics (total assets, folders, tags, size).
///
/// # Returns
///
/// * `Ok(LibraryStats)` with aggregated data.
/// * `Err(sqlx::Error)` on query failure.
pub async fn get_library_stats(pool: &SqlitePool, _registry: &crate::core::formats::registry::FormatRegistry) -> AppResult<crate::core::models::LibraryStats> {
    let stats_row = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM assets WHERE deleted_at IS NULL) as "total_assets!: i64",
            (SELECT COUNT(*) FROM folders) as "total_folders!: i64",
            (SELECT COUNT(*) FROM tags) as "total_tags!: i64",
            (SELECT COALESCE(SUM(file_size), 0) FROM assets WHERE deleted_at IS NULL) as "total_size_bytes!: i64",
            (SELECT COUNT(*) FROM assets WHERE deleted_at IS NULL AND id NOT IN (SELECT asset_id FROM asset_tags)) as "untagged_assets!: i64",
            (SELECT COUNT(*) FROM assets WHERE deleted_at IS NULL AND id IN (SELECT asset_id FROM asset_tags)) as "has_tags_assets!: i64",
            (SELECT COUNT(*) FROM assets WHERE deleted_at IS NULL AND is_favorite = 1) as "favorite_assets!: i64",
            (SELECT COUNT(*) FROM assets WHERE deleted_at IS NOT NULL) as "trash_assets!: i64",
            (SELECT COUNT(*) FROM smart_folders) as "smart_folders!: i64",
            (SELECT COUNT(*) FROM duplicate_candidates c JOIN duplicate_groups g ON c.group_id = g.id WHERE g.status = 'open') as "duplicate_assets!: i64"
        "#
    )
    .fetch_one(pool)
    .await?;

    let tag_counts_rows = sqlx::query!(
        r#"
        SELECT tag_id, COUNT(asset_id) as "count!: i64"
        FROM asset_tags
        WHERE asset_id IN (SELECT id FROM assets WHERE deleted_at IS NULL)
        GROUP BY tag_id
        "#
    )
    .fetch_all(pool)
    .await?;

    let tag_counts = tag_counts_rows
        .into_iter()
        .map(|r| crate::core::models::TagCount {
            tag_id: r.tag_id,
            count: r.count,
        })
        .collect();

    let folder_counts_rows = sqlx::query!(
        r#"
        SELECT folder_id as "folder_id: String", COUNT(id) as "count!: i64"
        FROM assets
        WHERE folder_id IS NOT NULL AND deleted_at IS NULL
        GROUP BY folder_id
        "#
    )
    .fetch_all(pool)
    .await?;

    let folder_counts: Vec<crate::core::models::FolderCount> = folder_counts_rows
        .into_iter()
        .filter_map(|r| {
            r.folder_id.map(|id| crate::core::models::FolderCount {
                folder_id: id,
                count: r.count,
            })
        })
        .collect();

    // Calculate recursive counts in-memory instead of slow recursive CTE
    let all_folders =
        sqlx::query!(r#"SELECT id as "id!", parent_id as "parent_id?" FROM folders"#)
            .fetch_all(pool)
            .await?;

    let mut parent_map: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    for row in all_folders {
        parent_map.insert(row.id, row.parent_id);
    }

    let mut recursive_counts: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for fc in &folder_counts {
        // Add to itself
        *recursive_counts.entry(fc.folder_id.clone()).or_insert(0) += fc.count;

        // Add to all parents
        let mut current_parent = parent_map.get(&fc.folder_id).cloned().flatten();
        while let Some(parent_id) = current_parent {
            *recursive_counts.entry(parent_id.clone()).or_insert(0) += fc.count;
            current_parent = parent_map.get(&parent_id).cloned().flatten();
        }
    }

    let folder_counts_recursive = recursive_counts
        .into_iter()
        .map(|(folder_id, count)| crate::core::models::FolderCount { folder_id, count })
        .collect();

    let family_stats_rows = sqlx::query(
        r#"
        SELECT 
            COALESCE(family, 'unknown') as family,
            COUNT(id) as count,
            COALESCE(SUM(file_size), 0) as total_size_bytes
        FROM assets
        WHERE deleted_at IS NULL
        GROUP BY family
        ORDER BY total_size_bytes DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let family_stats = family_stats_rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            crate::core::models::FamilyStat {
                family: row.get::<String, _>("family"),
                count: row.get::<i64, _>("count"),
                total_size_bytes: row.get::<i64, _>("total_size_bytes"),
            }
        })
        .collect();

    Ok(crate::core::models::LibraryStats {
        total_assets: stats_row.total_assets,
        total_folders: stats_row.total_folders,
        total_tags: stats_row.total_tags,
        total_size_bytes: stats_row.total_size_bytes,
        untagged_assets: stats_row.untagged_assets,
        has_tags_assets: stats_row.has_tags_assets,
        favorite_assets: stats_row.favorite_assets,
        trash_assets: stats_row.trash_assets,
        smart_folders: stats_row.smart_folders,
        duplicate_assets: stats_row.duplicate_assets,
        tag_counts,
        folder_counts,
        folder_counts_recursive: Some(folder_counts_recursive),
        family_stats,
    })
}
