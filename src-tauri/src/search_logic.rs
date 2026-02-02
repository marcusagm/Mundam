use serde::{Deserialize, Serialize};

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
            SearchItem::Criterion(c) => build_criterion_clause(c, query_builder),
            SearchItem::Group(g) => build_where_clause(g, query_builder),
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
                        // Use FTS5 Trigram MATCH
                        query_builder.push(" i.id IN (SELECT rowid FROM images_fts WHERE ");
                        query_builder.push(&c.key);
                        query_builder.push(" MATCH ");
                        // Trigram EXACT match on the string finds it as substring
                        query_builder.push_bind(format!("\"{}\"", c.value.as_str().unwrap_or("")));
                        query_builder.push(") ");
                    } else {
                        // Fallback for 'format'
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
                            // Use LIKE to match things like "heif(av1)" when searching for "heif"
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
                _ => { 
                    println!("WARN: Unknown operator '{}' for key '{}'", c.operator, c.key);
                    query_builder.push(" 1=1 "); 
                },
            }
        },
        "size" | "width" | "height" | "rating" => {
            query_builder.push(" i.");
            query_builder.push(&c.key);
            match c.operator.as_str() {
                "gt" => {
                    query_builder.push(" > ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
                "lt" => {
                    query_builder.push(" < ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
                "eq" => {
                    query_builder.push(" = ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
                "gte" => {
                    query_builder.push(" >= ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
                "lte" => {
                    query_builder.push(" <= ");
                    query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                },
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
                },
                _ => { query_builder.push(" = 1 "); },
            }
        },
        "added_at" | "created_at" | "modified_at" => {
            query_builder.push(" i.");
            query_builder.push(&c.key);
            
            // In SQLite dates are strings, so comparison works if in correct format.
            // Frontend sends DD/MM/YYYY, we should probably convert to YYYY-MM-DD on frontend or backend.
            // Assuming DD/MM/YYYY for now and trying to convert.
            let raw_val = c.value.as_str().unwrap_or("");
            
            // Try to normalize date.
            // If it matches DD/MM/YYYY, convert to YYYY-MM-DD
            // If it matches YYYY-MM-DD, keep it.
            // Simplified logic: Check for '/'
            let final_val = if raw_val.contains('/') {
                 let parts: Vec<&str> = raw_val.split('/').collect();
                 if parts.len() == 3 {
                    format!("{}-{}-{}", parts[2], parts[1], parts[0])
                 } else {
                    raw_val.to_string()
                 }
            } else {
                raw_val.to_string()
            };

            match c.operator.as_str() {
                "before" => {
                    query_builder.push(" < ");
                    query_builder.push_bind(final_val);
                },
                "after" => {
                    query_builder.push(" > ");
                    query_builder.push_bind(final_val);
                },
                "on" => {
                    query_builder.push(" LIKE ");
                    query_builder.push_bind(format!("{}%", final_val));
                },
                "between" => {
                    if let Some(arr) = c.value.as_array() {
                        if arr.len() == 2 {
                            let v1 = arr[0].as_str().unwrap_or("");
                            let v2 = arr[1].as_str().unwrap_or("");
                            
                            let parts1: Vec<&str> = v1.split('/').collect();
                            let parts2: Vec<&str> = v2.split('/').collect();
                            
                            let f1 = if parts1.len() == 3 { format!("{}-{}-{}", parts1[2], parts1[1], parts1[0]) } else { v1.to_string() };
                            let f2 = if parts2.len() == 3 { format!("{}-{}-{}", parts2[2], parts2[1], parts2[0]) } else { v2.to_string() };

                            query_builder.push(" BETWEEN ");
                            query_builder.push_bind(f1);
                            query_builder.push(" AND ");
                            query_builder.push_bind(format!("{} 23:59:59", f2));
                        } else {
                            query_builder.push(" = 1 ");
                        }
                    } else {
                        query_builder.push(" = 1 ");
                    }
                },
                _ => { query_builder.push(" = 1 "); },
            }
        },
        "tags" => {
            let tag_id = c.value.as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .or_else(|| c.value.as_i64());

            match c.operator.as_str() {
                "contains" | "contains_any" => {
                    if let Some(id) = tag_id {
                        query_builder.push(" i.id IN (SELECT image_id FROM image_tags WHERE tag_id = ");
                        query_builder.push_bind(id);
                        query_builder.push(") ");
                    } else {
                        query_builder.push(" 1=1 ");
                    }
                },
                "not_contains" => {
                    if let Some(id) = tag_id {
                        query_builder.push(" i.id NOT IN (SELECT image_id FROM image_tags WHERE tag_id = ");
                        query_builder.push_bind(id);
                        query_builder.push(") ");
                    } else {
                        query_builder.push(" 1=1 ");
                    }
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
                     // Recursive with depth limit to prevent infinite loops (max 50)
                     query_builder.push(" i.folder_id IN (WITH RECURSIVE subfolders AS (SELECT id, 0 as depth FROM folders WHERE id = ");
                     query_builder.push_bind(c.value.as_i64().unwrap_or(0));
                     query_builder.push(" UNION ALL SELECT f.id, s.depth + 1 FROM folders f JOIN subfolders s ON f.parent_id = s.id WHERE s.depth < 50) SELECT id FROM subfolders) ");
                },
                _ => { query_builder.push(" 1=1 "); },
            }
        },
        _ => { 
            println!("WARN: Unknown search criteria key: '{}'", c.key);
            query_builder.push(" 1=1 "); 
        },
    }
}
