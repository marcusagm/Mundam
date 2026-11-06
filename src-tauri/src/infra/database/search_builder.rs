//! Dynamic SQL query builder for advanced asset search.
//!
//! Ported and adapted from the legacy search engine to support the new
//! hexagonal architecture and final table schema.

use crate::core::models::search::{LogicalOperator, SearchCriterion, SearchGroup, SearchItem};
use sqlx::{QueryBuilder, Sqlite};

/// Builds the WHERE clause for an advanced search query.
///
/// # Arguments
///
/// * `group` - The root search group.
/// * `query_builder` - The SQLx QueryBuilder to append to.
pub fn build_search_where_clause<'a>(
    group: &'a SearchGroup,
    query_builder: &mut QueryBuilder<'a, Sqlite>,
    registry: &crate::core::formats::registry::FormatRegistry,
) {
    query_builder.push(" (");

    let mut first = true;
    for item in &group.items {
        if !first {
            match group.logical_operator {
                LogicalOperator::And => {
                    query_builder.push(" AND ");
                }
                LogicalOperator::Or => {
                    query_builder.push(" OR ");
                }
            };
        }
        first = false;

        match item {
            SearchItem::Group(g) => build_search_where_clause(g, query_builder, registry),
            SearchItem::Criterion(c) => build_search_criterion_clause(c, query_builder, registry),
        }
    }

    if group.items.is_empty() {
        query_builder.push(" 1=1 ");
    }

    query_builder.push(") ");
}

/// Builds the WHERE clause for an advanced search query.
///
/// # Arguments
///
/// * `c` - The search criterion.
/// * `query_builder` - The SQLx QueryBuilder to append to.
fn build_search_criterion_clause<'a>(
    c: &'a SearchCriterion,
    query_builder: &mut QueryBuilder<'a, Sqlite>,
    registry: &crate::core::formats::registry::FormatRegistry,
) {
    match c.key.as_str() {
        "name" | "filename" | "path" | "format_type" | "format" | "extension" | "family"
        | "media_type" | "mediaType" | "notes" => {
            let col = match c.key.as_str() {
                "filename" => "name",
                "format" | "extension" => "format_type",
                "media_type" | "mediaType" => "family",
                "notes" => "notes",
                k => k,
            };

            let mut value_to_bind = c.value.as_str().unwrap_or("").to_string();

            // Special case for format/extension: map extension to format name if found
            if ["format", "extension"].contains(&c.key.as_str()) {
                let ext_lower = value_to_bind.to_lowercase();
                if let Some(sf) = registry.detect_by_extension(&ext_lower) {
                    value_to_bind = sf.name.to_string();
                }
            }

            query_builder.push(" a.");
            query_builder.push(col);
            match c.operator.as_str() {
                "contains" => {
                    query_builder.push(" LIKE ");
                    query_builder.push_bind(format!("%{}%", value_to_bind));
                }
                "not_contains" => {
                    query_builder.push(" NOT LIKE ");
                    query_builder.push_bind(format!("%{}%", value_to_bind));
                }
                "equals" | "eq" | "exact" => {
                    query_builder.push(" = ");
                    query_builder.push_bind(value_to_bind);
                }
                "starts_with" => {
                    query_builder.push(" LIKE ");
                    query_builder.push_bind(format!("{}%", value_to_bind));
                }
                "ends_with" => {
                    query_builder.push(" LIKE ");
                    query_builder.push_bind(format!("%{}", value_to_bind));
                }
                _ => {
                    query_builder.push(" = 1 ");
                }
            }
        }
        "file_size" | "size" | "rating" | "width" | "height" | "duration_secs" | "isFavorite" => {
            // Note: width/height/duration_secs are in v2_asset_metadata_envelope (m)
            // but we'll assume the caller joins correctly if they use these keys.
            let prefix = if ["width", "height", "duration_secs"].contains(&c.key.as_str()) {
                "m."
            } else {
                "a."
            };

            let key = match c.key.as_str() {
                "size" => "file_size",
                "isFavorite" => "rating",
                k => k,
            };

            query_builder.push(prefix);
            query_builder.push(key);

            if c.key == "isFavorite" {
                let is_fav = c.value.as_bool().unwrap_or(false);
                if is_fav {
                    query_builder.push(" > 0 ");
                } else {
                    query_builder.push(" = 0 ");
                }
                return;
            }

            match c.operator.as_str() {
                "gt" | "greater_than" => {
                    query_builder.push(" > ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                }
                "lt" | "less_than" => {
                    query_builder.push(" < ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                }
                "eq" | "equals" => {
                    query_builder.push(" = ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                }
                "gte" | "greater_than_equals" => {
                    query_builder.push(" >= ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                }
                "lte" | "less_than_equals" => {
                    query_builder.push(" <= ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                }
                "between" => {
                    if let Some(arr) = c.value.as_array() {
                        if arr.len() == 2 {
                            query_builder.push(" BETWEEN ");
                            query_builder.push_bind(arr[0].as_i64().unwrap_or(0));
                            query_builder.push(" AND ");
                            query_builder.push_bind(arr[1].as_i64().unwrap_or(0));
                        } else {
                            query_builder.push(" = 1 ");
                        }
                    } else {
                        query_builder.push(" = 1 ");
                    }
                }
                _ => {
                    query_builder.push(" = 1 ");
                }
            }
        }
        "created_at" | "updated_at" | "creationDate" | "modified_at" | "modifiedDate" | "added"
        | "added_at" | "date_added" => {
            let key = match c.key.as_str() {
                "creationDate" => "created_at",
                "modifiedDate" => "modified_at",
                "added" | "date_added" => "added_at",
                k => k,
            };

            query_builder.push(" a.");
            query_builder.push(key);

            match c.operator.as_str() {
                "gt" | "after" => {
                    query_builder.push(" > ");
                    query_builder.push_bind(c.value.as_str().unwrap_or(""));
                }
                "lt" | "before" => {
                    query_builder.push(" < ");
                    query_builder.push_bind(c.value.as_str().unwrap_or(""));
                }
                "between" => {
                    if let Some(arr) = c.value.as_array() {
                        if arr.len() == 2 {
                            query_builder.push(" BETWEEN ");
                            query_builder.push_bind(arr[0].as_str().unwrap_or(""));
                            query_builder.push(" AND ");
                            query_builder.push_bind(arr[1].as_str().unwrap_or(""));
                        } else {
                            query_builder.push(" = 1 ");
                        }
                    } else {
                        query_builder.push(" = 1 ");
                    }
                }
                _ => {
                    query_builder.push(" = 1 ");
                }
            }
        }
        "tags" | "tag" | "tag_id" => {
            // value can be a single tag name or an array of tag names
            match c.operator.as_str() {
                "contains" | "in" | "contains_any" => {
                    query_builder.push(" a.id IN (SELECT asset_id FROM asset_tags WHERE tag_id IN (SELECT id FROM tags WHERE ");
                    if let Some(tags) = c.value.as_array() {
                        query_builder.push(" (name IN (");
                        let mut first_name = true;
                        for tag in tags {
                            if let Some(tag_name_or_identifier) = tag.as_str() {
                                if !first_name {
                                    query_builder.push(", ");
                                }
                                query_builder.push_bind(tag_name_or_identifier);
                                first_name = false;
                            }
                        }
                        query_builder.push(") OR id IN (");
                        let mut first_id = true;
                        for tag in tags {
                            if let Some(tag_name_or_identifier) = tag.as_str() {
                                if !first_id {
                                    query_builder.push(", ");
                                }
                                query_builder.push_bind(tag_name_or_identifier);
                                first_id = false;
                            }
                        }
                        query_builder.push("))) ");
                    } else {
                        let val = c.value.as_str().unwrap_or("");
                        query_builder.push(" (name = ");
                        query_builder.push_bind(val);
                        query_builder.push(" OR id = ");
                        query_builder.push_bind(val);
                        query_builder.push(") ");
                    }
                    query_builder.push(")) ");
                }
                "not_contains" => {
                    query_builder.push(" a.id NOT IN (SELECT asset_id FROM asset_tags WHERE tag_id IN (SELECT id FROM tags WHERE ");
                    if let Some(tags) = c.value.as_array() {
                        query_builder.push(" (name IN (");
                        let mut first_name = true;
                        for tag in tags {
                            if let Some(tag_name_or_identifier) = tag.as_str() {
                                if !first_name {
                                    query_builder.push(", ");
                                }
                                query_builder.push_bind(tag_name_or_identifier);
                                first_name = false;
                            }
                        }
                        query_builder.push(") OR id IN (");
                        let mut first_id = true;
                        for tag in tags {
                            if let Some(tag_name_or_identifier) = tag.as_str() {
                                if !first_id {
                                    query_builder.push(", ");
                                }
                                query_builder.push_bind(tag_name_or_identifier);
                                first_id = false;
                            }
                        }
                        query_builder.push("))) ");
                    } else {
                        let val = c.value.as_str().unwrap_or("");
                        query_builder.push(" (name = ");
                        query_builder.push_bind(val);
                        query_builder.push(" OR id = ");
                        query_builder.push_bind(val);
                        query_builder.push(") ");
                    }
                    query_builder.push(")) ");
                }
                _ => {
                    query_builder.push(" 1=1 ");
                }
            }
        }
        "folder" => {
            let val = c.value.as_str().unwrap_or("");
            match c.operator.as_str() {
                "is" | "equals" => {
                    query_builder.push(" a.folder_id IN (SELECT id FROM folders WHERE path = ");
                    query_builder.push_bind(val);
                    query_builder.push(" OR id = ");
                    query_builder.push_bind(val);
                    query_builder.push(") ");
                }
                "in" | "recursive" => {
                    query_builder.push(" a.folder_id IN (WITH RECURSIVE subfolders AS (SELECT id FROM folders WHERE path = ");
                    query_builder.push_bind(val);
                    query_builder.push(" OR id = ");
                    query_builder.push_bind(val);
                    query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN subfolders s ON f.parent_id = s.id) SELECT id FROM subfolders) ");
                }
                _ => {
                    query_builder.push(" 1=1 ");
                }
            }
        }
        "color" => {
            // CIE-76 Euclidean distance in LAB space.
            // Value: { "hex": "#FF5733", "threshold": 25.0 }
            let hex_color = c
                .value
                .get("hex")
                .and_then(|v| v.as_str())
                .unwrap_or("#000000");
            let threshold = c
                .value
                .get("threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(25.0);

            match hex_to_lab(hex_color) {
                Ok((l, a, b)) => {
                    let threshold_squared = threshold * threshold;
                    query_builder.push(" a.id IN (SELECT DISTINCT asset_id FROM asset_colors WHERE ((lab_lightness - ");
                    query_builder.push_bind(l);
                    query_builder.push(") * (lab_lightness - ");
                    query_builder.push_bind(l);
                    query_builder.push(") + (lab_green_red - ");
                    query_builder.push_bind(a);
                    query_builder.push(") * (lab_green_red - ");
                    query_builder.push_bind(a);
                    query_builder.push(") + (lab_blue_yellow - ");
                    query_builder.push_bind(b);
                    query_builder.push(") * (lab_blue_yellow - ");
                    query_builder.push_bind(b);
                    query_builder.push(")) < ");
                    query_builder.push_bind(threshold_squared);
                    query_builder.push(") ");
                }
                Err(_) => {
                    query_builder.push(" 1=0 ");
                }
            }
        }
        _ => {
            query_builder.push(" 1=1 ");
        }
    }
}

/// Converts a hexadecimal color string to CIE-LAB components.
///
/// Supports formats: "#RRGGBB" and "RRGGBB".
///
/// # Errors
///
/// Returns an error if the hex string is invalid.
fn hex_to_lab(
    hex_color: &str,
) -> Result<(f64, f64, f64), Box<dyn std::error::Error + Send + Sync>> {
    let hex_trimmed = hex_color.trim_start_matches('#');
    if hex_trimmed.len() != 6 {
        return Err(format!("Invalid hex color length: {}", hex_color).into());
    }
    let red = u8::from_str_radix(&hex_trimmed[0..2], 16)?;
    let green = u8::from_str_radix(&hex_trimmed[2..4], 16)?;
    let blue = u8::from_str_radix(&hex_trimmed[4..6], 16)?;
    let srgb_color = palette::Srgb::new(
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
    );
    let lab_color: palette::Lab = palette::IntoColor::into_color(srgb_color);
    Ok((lab_color.l as f64, lab_color.a as f64, lab_color.b as f64))
}

/// Tests for the search builder.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::search::{LogicalOperator, SearchCriterion, SearchGroup, SearchItem};
    use serde_json::json;

    /// Builds a fully-populated `FormatRegistry` for test scenarios.
    fn build_test_registry() -> crate::core::formats::registry::FormatRegistry {
        use crate::processing::media::providers;

        let mut registry = crate::core::formats::registry::FormatRegistry::new();
        registry.register_batch(providers::collect_all_providers());
        registry.register_fallback(providers::fallback_provider());
        registry
    }

    /// Tests the build_simple_where_clause function.
    #[test]
    fn test_build_simple_where_clause() {
        let criterion = SearchCriterion {
            id: "1".to_string(),
            key: "name".to_string(),
            operator: "contains".to_string(),
            value: json!("test"),
        };

        let group = SearchGroup {
            id: "root".to_string(),
            logical_operator: LogicalOperator::And,
            items: vec![SearchItem::Criterion(criterion)],
        };

        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM assets WHERE ");
        let registry = build_test_registry();
        build_search_where_clause(&group, &mut query_builder, &registry);

        let sql = query_builder.into_sql();
        assert!(sql.contains("a.name LIKE ?"));
    }

    /// Tests the build_search_where_clause function with nested groups.
    #[test]
    fn test_build_nested_where_clause() {
        let c1 = SearchCriterion {
            id: "1".to_string(),
            key: "name".to_string(),
            operator: "contains".to_string(),
            value: json!("a"),
        };
        let c2 = SearchCriterion {
            id: "2".to_string(),
            key: "format_type".to_string(),
            operator: "eq".to_string(),
            value: json!("image/jpeg"),
        };

        let nested_group = SearchGroup {
            id: "nested".to_string(),
            logical_operator: LogicalOperator::Or,
            items: vec![SearchItem::Criterion(c1), SearchItem::Criterion(c2)],
        };

        let root_group = SearchGroup {
            id: "root".to_string(),
            logical_operator: LogicalOperator::And,
            items: vec![SearchItem::Group(nested_group)],
        };

        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM assets WHERE ");
        let registry = build_test_registry();
        build_search_where_clause(&root_group, &mut query_builder, &registry);

        let sql = query_builder.into_sql();
        assert!(sql.contains("a.name LIKE ?"));
        assert!(sql.contains("OR"));
        assert!(sql.contains("a.format_type = ?"));
    }

    /// Tests the build_search_where_clause function with color proximity.
    #[test]
    fn test_color_search_proximity_sql() {
        let criterion = SearchCriterion {
            id: "color1".to_string(),
            key: "color".to_string(),
            operator: "proximity".to_string(),
            value: json!({ "hex": "#FF0000", "threshold": 10.0 }),
        };

        let group = SearchGroup {
            id: "root".to_string(),
            logical_operator: LogicalOperator::And,
            items: vec![SearchItem::Criterion(criterion)],
        };

        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM assets WHERE ");
        let registry = build_test_registry();
        build_search_where_clause(&group, &mut query_builder, &registry);

        let sql = query_builder.into_sql();
        assert!(sql.contains(
            "a.id IN (SELECT DISTINCT asset_id FROM asset_colors WHERE ((lab_lightness - ?"
        ));
        assert!(sql.contains("< ?"));
    }
}
