//! Dynamic search query builder for the image library.
//!
//! This module converts complex, nested search criteria from the frontend
//! into optimized SQLite queries using `sqlx::QueryBuilder`.

use serde::{Deserialize, Serialize};
use crate::db::models::ImageMetadata;
use super::Db;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchCriterion {
    pub id: String,
    pub key: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SearchItem {
    Group(SearchGroup),
    Criterion(SearchCriterion),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchGroup {
    pub id: String,
    pub logical_operator: LogicalOperator,
    pub items: Vec<SearchItem>,
}

impl Db {
    /// Retrieves a paginated and filtered list of images based on various criteria.
    #[allow(clippy::too_many_arguments)] // Deep filtering naturally requires many parameters
    pub async fn get_images_filtered(
        &self,
        limit: i32,
        offset: i32,
        tag_ids: Vec<i64>,
        match_all: bool,
        untagged: Option<bool>,
        folder_id: Option<i64>,
        recursive: bool,
        sort_by: Option<String>,
        sort_order: Option<String>,
        advanced_query: Option<String>,
        search_query: Option<String>,
    ) -> Result<Vec<ImageMetadata>, sqlx::Error> {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "WITH RECURSIVE target_folders AS (
               SELECT id FROM folders WHERE id = "
        );

        if let Some(fid) = folder_id {
            query_builder.push_bind(fid);
            if recursive {
                query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN target_folders tf ON f.parent_id = tf.id");
            }
        } else {
             query_builder.push(" -1 ");
        }

        query_builder.push(") SELECT DISTINCT i.id, i.path, i.filename, i.width, i.height, i.size, i.thumbnail_path, i.format, i.rating, i.notes, i.created_at, i.modified_at, i.added_at FROM images i ");

        if !tag_ids.is_empty() {
            query_builder.push(" JOIN image_tags it ON i.id = it.image_id ");
        }

        query_builder.push(" WHERE 1=1 ");

        let parsed_group = advanced_query.as_ref().and_then(|q| serde_json::from_str::<SearchGroup>(q).ok());
        if let Some(ref group) = parsed_group {
            query_builder.push(" AND ");
            build_where_clause(group, &mut query_builder);
        }

        if let Some(search) = search_query {
            if !search.is_empty() {
                query_builder.push(" AND (i.filename LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR i.notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(") ");
            }
        }

        if let Some(fid) = folder_id {
            if recursive {
                query_builder.push(" AND i.folder_id IN target_folders ");
            } else {
                query_builder.push(" AND i.folder_id = ");
                query_builder.push_bind(fid);
            }
        }

        if untagged == Some(true) {
            query_builder.push(" AND i.id NOT IN (SELECT DISTINCT image_id FROM image_tags) ");
        }

        if !tag_ids.is_empty() {
            query_builder.push(" AND it.tag_id IN (");
            let mut separated = query_builder.separated(", ");
            for id in &tag_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(") ");

            if match_all {
                query_builder.push(" GROUP BY i.id HAVING COUNT(DISTINCT it.tag_id) = ");
                query_builder.push_bind(tag_ids.len() as i32);
            }
        }

        // Sorting Logic
        let allowed_cols = ["filename", "created_at", "modified_at", "added_at", "size", "format", "rating"];
        let final_sort_by = sort_by.as_deref().filter(|c| allowed_cols.contains(c)).unwrap_or("id");
        let final_order = sort_order.as_deref().filter(|o| *o == "asc" || *o == "desc").unwrap_or("desc");

        query_builder.push(" ORDER BY (");
        query_builder.push(final_sort_by);
        query_builder.push(" IS NULL) ASC, ");
        query_builder.push(final_sort_by);

        if ["filename", "format"].contains(&final_sort_by) {
            query_builder.push(" COLLATE NOCASE ");
        }
        query_builder.push(" ");
        query_builder.push(final_order);

        if final_sort_by != "filename" {
            query_builder.push(", filename COLLATE NOCASE ASC");
        }

        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let images = query_builder.build_query_as::<ImageMetadata>().fetch_all(&self.pool).await?;
        Ok(images)
    }

    /// Gets the total count of images matching the search criteria.
    #[allow(clippy::too_many_arguments)]
    pub async fn get_image_count_filtered(
        &self,
        tag_ids: Vec<i64>,
        match_all: bool,
        untagged: Option<bool>,
        folder_id: Option<i64>,
        recursive: bool,
        advanced_query: Option<String>,
        search_query: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "WITH RECURSIVE target_folders AS (
               SELECT id FROM folders WHERE id = "
        );

        if let Some(fid) = folder_id {
            query_builder.push_bind(fid);
            if recursive {
                query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN target_folders tf ON f.parent_id = tf.id");
            }
        } else {
             query_builder.push(" -1 ");
        }

        query_builder.push(") SELECT DISTINCT i.id FROM images i ");

        if !tag_ids.is_empty() {
            query_builder.push(" JOIN image_tags it ON i.id = it.image_id ");
        }

        query_builder.push(" WHERE 1=1 ");

        let parsed_group = advanced_query.as_ref().and_then(|q| serde_json::from_str::<SearchGroup>(q).ok());
        if let Some(ref group) = parsed_group {
            query_builder.push(" AND ");
            build_where_clause(group, &mut query_builder);
        }

        if let Some(search) = search_query {
            if !search.is_empty() {
                query_builder.push(" AND (i.filename LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR i.notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(") ");
            }
        }

        if let Some(fid) = folder_id {
            if recursive {
                query_builder.push(" AND i.folder_id IN target_folders ");
            } else {
                query_builder.push(" AND i.folder_id = ");
                query_builder.push_bind(fid);
            }
        }

        if untagged == Some(true) {
            query_builder.push(" AND i.id NOT IN (SELECT DISTINCT image_id FROM image_tags) ");
        }

        if !tag_ids.is_empty() {
            query_builder.push(" AND it.tag_id IN (");
            let mut separated = query_builder.separated(", ");
            for id in &tag_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(") ");

            if match_all {
                query_builder.push(" GROUP BY i.id HAVING COUNT(DISTINCT it.tag_id) = ");
                query_builder.push_bind(tag_ids.len() as i32);
            }
        }

        // Fetch only IDs to count rows (most efficient way to count DISTINCT with HAVING in SQLx builder)
        let rows = query_builder.build_query_as::<(i64,)>().fetch_all(&self.pool).await?;
        Ok(rows.len() as i64)
    }
}

pub fn build_where_clause<'a>(group: &'a SearchGroup, query_builder: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>) {
    query_builder.push(" (");

    let mut first = true;
    for item in &group.items {
        if !first {
            match group.logical_operator {
                LogicalOperator::And => { query_builder.push(" AND "); },
                LogicalOperator::Or => { query_builder.push(" OR "); },
            };
        }
        first = false;

        match item {
            SearchItem::Group(g) => build_where_clause(g, query_builder),
            SearchItem::Criterion(c) => build_criterion_clause(c, query_builder),
        }
    }

    if group.items.is_empty() {
        query_builder.push(" 1=1 ");
    }

    query_builder.push(") ");
}

fn build_criterion_clause<'a>(c: &'a SearchCriterion, query_builder: &mut sqlx::QueryBuilder<'a, sqlx::Sqlite>) {
    match c.key.as_str() {
        "filename" | "notes" | "format" => {
            let is_fts_target = c.key == "filename" || c.key == "notes";

            match c.operator.as_str() {
                "contains" => {
                    if is_fts_target {
                        query_builder.push(" i.id IN (SELECT rowid FROM images_fts WHERE ");
                        query_builder.push(&c.key);
                        query_builder.push(" MATCH ");
                        query_builder.push_bind(format!("\"{}\"", c.value.as_str().unwrap_or("")));
                        query_builder.push(") ");
                    } else {
                        query_builder.push(" i.");
                        query_builder.push(&c.key);
                        query_builder.push(" LIKE ");
                        query_builder.push_bind(format!("%{}%", c.value.as_str().unwrap_or("")));
                    }
                },
                "not_contains" => {
                     if is_fts_target {
                        query_builder.push(" i.id NOT IN (SELECT rowid FROM images_fts WHERE ");
                        query_builder.push(&c.key);
                        query_builder.push(" MATCH ");
                        query_builder.push_bind(format!("\"{}\"", c.value.as_str().unwrap_or("")));
                        query_builder.push(") ");
                     } else {
                        query_builder.push(" i.");
                        query_builder.push(&c.key);
                        query_builder.push(" NOT LIKE ");
                        query_builder.push_bind(format!("%{}%", c.value.as_str().unwrap_or("")));
                     }
                },
                "equals" | "eq" => {
                    if c.key == "format" {
                        let val = c.value.as_str().unwrap_or("").to_lowercase();
                        if val == "jpg" || val == "jpeg" {
                            query_builder.push(" (i.format = 'jpg' OR i.format = 'jpeg') ");
                        } else {
                            query_builder.push(" i.format LIKE ");
                            query_builder.push_bind(format!("%{}%", val));
                        }
                    } else if c.key == "filename" {
                        let val = c.value.as_str().unwrap_or("");
                        query_builder.push(" (i.filename = ");
                        query_builder.push_bind(val);
                        query_builder.push(" OR i.filename LIKE ");
                        query_builder.push_bind(format!("{}.%", val));
                        query_builder.push(") ");
                    } else {
                        query_builder.push(" i.");
                        query_builder.push(&c.key);
                        query_builder.push(" = ");
                        query_builder.push_bind(c.value.as_str().unwrap_or(""));
                    }
                },
                "starts_with" => {
                    query_builder.push(" i.");
                    query_builder.push(&c.key);
                    query_builder.push(" LIKE ");
                    query_builder.push_bind(format!("{}%", c.value.as_str().unwrap_or("")));
                },
                "ends_with" => {
                    if c.key == "filename" {
                        query_builder.push(" (i.filename LIKE ");
                        query_builder.push_bind(format!("%{}", c.value.as_str().unwrap_or("")));
                        query_builder.push(" OR i.filename LIKE ");
                        query_builder.push_bind(format!("%{}.%", c.value.as_str().unwrap_or("")));
                        query_builder.push(") ");
                    } else {
                        query_builder.push(" i.");
                        query_builder.push(&c.key);
                        query_builder.push(" LIKE ");
                        query_builder.push_bind(format!("%{}", c.value.as_str().unwrap_or("")));
                    }
                },
                _ => { query_builder.push(" 1=1 "); },
            }
        },
        "size" | "width" | "height" | "rating" => {
            query_builder.push(" i.");
            query_builder.push(&c.key);
            match c.operator.as_str() {
                "gt" => { query_builder.push(" > "); query_builder.push_bind(c.value.as_i64().unwrap_or(0)); },
                "lt" => { query_builder.push(" < "); query_builder.push_bind(c.value.as_i64().unwrap_or(0)); },
                "eq" => { query_builder.push(" = "); query_builder.push_bind(c.value.as_i64().unwrap_or(0)); },
                "gte" => { query_builder.push(" >= "); query_builder.push_bind(c.value.as_i64().unwrap_or(0)); },
                "lte" => { query_builder.push(" <= "); query_builder.push_bind(c.value.as_i64().unwrap_or(0)); },
                "between" => {
                    if let Some(arr) = c.value.as_array() {
                        if arr.len() == 2 {
                            query_builder.push(" BETWEEN ");
                            query_builder.push_bind(arr[0].as_i64().unwrap_or(0));
                            query_builder.push(" AND ");
                            query_builder.push_bind(arr[1].as_i64().unwrap_or(0));
                        } else { query_builder.push(" = 1 "); }
                    } else { query_builder.push(" = 1 "); }
                },
                _ => { query_builder.push(" = 1 "); },
            }
        },
        "added_at" | "created_at" | "modified_at" => {
            query_builder.push(" i.");
            query_builder.push(&c.key);
            let val = c.value.as_str().unwrap_or("");
            match c.operator.as_str() {
                "before" => { query_builder.push(" < "); query_builder.push_bind(val); },
                "after" => { query_builder.push(" > "); query_builder.push_bind(val); },
                "on" => { query_builder.push(" LIKE "); query_builder.push_bind(format!("{}%", val)); },
                "between" => {
                    if let Some(arr) = c.value.as_array() {
                        if arr.len() == 2 {
                            let v1 = arr[0].as_str().unwrap_or("");
                            let v2 = arr[1].as_str().unwrap_or("");
                            query_builder.push(" BETWEEN ");
                            query_builder.push_bind(v1);
                            query_builder.push(" AND ");
                            let v2_final = if v2.len() == 10 { format!("{} 23:59:59", v2) } else { v2.to_string() };
                            query_builder.push_bind(v2_final);
                        } else { query_builder.push(" = 1 "); }
                    } else { query_builder.push(" = 1 "); }
                },
                _ => { query_builder.push(" = 1 "); },
            }
        },
        "tags" => {
            let tag_id = c.value.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| c.value.as_i64());
            match c.operator.as_str() {
                "contains" | "contains_any" => {
                    if let Some(id) = tag_id {
                        query_builder.push(" i.id IN (SELECT image_id FROM image_tags WHERE tag_id = ");
                        query_builder.push_bind(id);
                        query_builder.push(") ");
                    } else { query_builder.push(" 1=1 "); }
                },
                "not_contains" => {
                    if let Some(id) = tag_id {
                        query_builder.push(" i.id NOT IN (SELECT image_id FROM image_tags WHERE tag_id = ");
                        query_builder.push_bind(id);
                        query_builder.push(") ");
                    } else { query_builder.push(" 1=1 "); }
                },
                _ => { query_builder.push(" 1=1 "); },
            }
        },
        "folder" => {
            match c.operator.as_str() {
                "is" => {
                    query_builder.push(" i.folder_id = ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
                "in" => {
                     query_builder.push(" i.folder_id IN (WITH RECURSIVE subfolders AS (SELECT id, 0 as depth FROM folders WHERE id = ");
                     query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                     query_builder.push(" UNION ALL SELECT f.id, s.depth + 1 FROM folders f JOIN subfolders s ON f.parent_id = s.id WHERE s.depth < 50) SELECT id FROM subfolders) ");
                },
                _ => { query_builder.push(" 1=1 "); },
            }
        },
        _ => { query_builder.push(" 1=1 "); },
    }
}
