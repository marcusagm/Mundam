use crate::core::error::AppResult;
use crate::core::models::AssetColor;
use sqlx::SqlitePool;

/// Retrieves all colors extracted for a specific asset.
pub async fn get_asset_colors(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    asset_id: &str,
) -> AppResult<Vec<crate::core::models::AssetColor>> {
    let rows = sqlx::query!(
        r#"
        SELECT id as "id!", hex_color, lab_lightness, lab_green_red, lab_blue_yellow, percentage, rank
        FROM asset_colors
        WHERE asset_id = ?
        ORDER BY rank ASC
        "#,
        asset_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| AssetColor {
            id: Some(r.id),
            hex_color: r.hex_color,
            lab_lightness: r.lab_lightness,
            lab_green_red: r.lab_green_red,
            lab_blue_yellow: r.lab_blue_yellow,
            percentage: r.percentage,
            rank: r.rank as i32,
        })
        .collect())
}

/// Returns thumbnail paths for all assets in a folder and its subfolders.
///
/// # Arguments
///
/// * `folder_id` - The ID of the folder to retrieve thumbnails for.
///
/// # Returns
///
/// * `Ok(Vec<String>)` if the thumbnails were found successfully.
/// * `Err(sqlx::Error)` if the thumbnails could not be found.
pub async fn get_folder_thumbnails(
    pool: &SqlitePool,
    _registry: &crate::core::formats::registry::FormatRegistry,
    folder_id: &str,
) -> AppResult<Vec<String>> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE family AS (
            SELECT id FROM folders WHERE id = ?
            UNION ALL
            SELECT f.id FROM folders f JOIN family ON f.parent_id = family.id
        )
        SELECT thumbnail_path as "thumbnail_path!" FROM assets
        WHERE folder_id IN family AND thumbnail_path IS NOT NULL
        "#,
        folder_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.thumbnail_path).collect())
}
