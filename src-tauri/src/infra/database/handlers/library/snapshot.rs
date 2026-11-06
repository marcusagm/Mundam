//! Versioned, bounded snapshots preserve authored catalog data across folder Undo and schema additions.
use crate::core::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

const TABLES: [&str; 7] = [
    "folders",
    "assets",
    "asset_metadata_envelope",
    "asset_tags",
    "asset_colors",
    "library_folder_identities",
    "library_file_identities",
];
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTITIES: usize = 100_000;
const PAGE_SIZE: usize = 256;

#[derive(Serialize, Deserialize)]
struct TableSnapshot {
    table: String,
    columns: Vec<String>,
    rows: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    tables: Vec<TableSnapshot>,
}

fn invalid(message: &str) -> AppError {
    AppError::ValidationFailed(message.into())
}
fn json_error(error: serde_json::Error) -> AppError {
    invalid(&format!("Invalid folder recovery data: {error}"))
}

async fn columns(transaction: &mut Transaction<'_, Sqlite>, table: &str) -> AppResult<Vec<String>> {
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
            .bind(table)
            .fetch_all(&mut **transaction)
            .await?;
    if columns.iter().any(|column| {
        column.is_empty()
            || !column
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
    }) {
        return Err(invalid("Recovery requires supported schema column names"));
    }
    Ok(columns)
}

/// Captures bounded pages before cascading deletion removes ownership links.
///
/// # Errors
/// Rejects oversized or invalid catalog data before moving original files.
pub(crate) async fn capture(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<String> {
    let folders: Vec<String> = sqlx::query_scalar("WITH RECURSIVE tree(id) AS (SELECT id FROM folders WHERE id = ? UNION SELECT folders.id FROM folders JOIN tree ON folders.parent_id = tree.id) SELECT id FROM tree LIMIT 100001").bind(folder_id).fetch_all(&mut **transaction).await?;
    let assets: Vec<String> = sqlx::query_scalar("WITH RECURSIVE tree(id) AS (SELECT id FROM folders WHERE id = ? UNION SELECT folders.id FROM folders JOIN tree ON folders.parent_id = tree.id) SELECT id FROM assets WHERE folder_id IN (SELECT id FROM tree) LIMIT 100001").bind(folder_id).fetch_all(&mut **transaction).await?;
    if folders.len().saturating_add(assets.len()) > MAX_ENTITIES {
        return Err(invalid(
            "Folder exceeds the 100000-entity recovery budget. Delete smaller subfolders.",
        ));
    }
    let mut tables = Vec::new();
    let mut captured_bytes = 0usize;
    for table in TABLES {
        let columns = columns(transaction, table).await?;
        let fields = columns
            .iter()
            .map(|column| format!("'{column}', \"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let identifier_column = if table == "folders" || table == "assets" {
            "id"
        } else if table == "library_folder_identities" {
            "folder_id"
        } else {
            "asset_id"
        };
        let identifiers = if table == "folders" || table == "library_folder_identities" {
            &folders
        } else {
            &assets
        };
        let identifiers = serde_json::to_string(identifiers).map_err(json_error)?;
        let selection =
            format!("FROM {table} WHERE {identifier_column} IN (SELECT value FROM json_each(?))");
        let estimated: i64 = sqlx::query_scalar(&format!(
            "SELECT COALESCE(SUM(length(CAST(json_object({fields}) AS BLOB))),0) {selection}"
        ))
        .bind(&identifiers)
        .fetch_one(&mut **transaction)
        .await?;
        captured_bytes = captured_bytes.saturating_add(estimated as usize);
        if captured_bytes > MAX_SNAPSHOT_BYTES {
            return Err(invalid(
                "Recovery metadata exceeds 64 MiB. Delete smaller subfolders.",
            ));
        }
        let query =
            format!("SELECT json_object({fields}) {selection} ORDER BY rowid LIMIT 256 OFFSET ?");
        let mut rows = Vec::new();
        loop {
            let page: Vec<String> = sqlx::query_scalar(&query)
                .bind(&identifiers)
                .bind(rows.len() as i64)
                .fetch_all(&mut **transaction)
                .await?;
            let count = page.len();
            for row in page {
                let row = serde_json::from_str(&row).map_err(json_error)?;
                validate_row(table, &row)?;
                rows.push(row);
            }
            if count < PAGE_SIZE {
                break;
            }
        }
        tables.push(TableSnapshot {
            table: table.into(),
            columns,
            rows,
        });
    }
    let serialized = serde_json::to_string(&Snapshot { version: 1, tables }).map_err(json_error)?;
    if serialized.len() > MAX_SNAPSHOT_BYTES {
        return Err(invalid("Encoded recovery metadata exceeds 64 MiB"));
    }
    Ok(serialized)
}

/// Restores stable identities with defaults for additive schema migrations.
///
/// Legacy v0 arrays convert to v1 tables. Removed tags are skipped; missing parents,
/// unknown versions and removed schema fields retain the archive for explicit repair.
///
/// # Errors
/// Returns validation or storage failures; the caller rolls back all restored tables.
pub(crate) async fn restore(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &str,
) -> AppResult<Vec<(String, String, String)>> {
    if snapshot.len() > MAX_SNAPSHOT_BYTES * 2 {
        return Err(invalid("Recovery snapshot exceeds the supported budget"));
    }
    let value: serde_json::Value = serde_json::from_str(snapshot).map_err(json_error)?;
    let tables: Vec<TableSnapshot> = if value.is_array() {
        serde_json::from_value(value).map_err(json_error)?
    } else {
        let envelope: Snapshot = serde_json::from_value(value).map_err(json_error)?;
        if envelope.version != 1 {
            return Err(invalid("Unsupported folder recovery version"));
        }
        envelope.tables
    };
    if tables.len() != TABLES.len() && tables.len() != 5 {
        return Err(invalid("Incomplete folder recovery snapshot"));
    }
    for (index, table) in tables.iter().enumerate() {
        if TABLES[index] != table.table {
            return Err(invalid("Unsupported folder recovery table order"));
        }
        let current = columns(transaction, &table.table).await?;
        let mut unique = std::collections::HashSet::new();
        if table.columns.is_empty()
            || table
                .columns
                .iter()
                .any(|column| !current.contains(column) || !unique.insert(column))
        {
            return Err(invalid(
                "Recovery requires a schema migration or has invalid columns",
            ));
        }
        for row in &table.rows {
            validate_row(&table.table, row)?;
        }
    }
    // Parent rows can belong to later pages; constraint validation remains mandatory at commit.
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut **transaction)
        .await?;
    let mut restored = Vec::new();
    for table in tables {
        let column_names = table
            .columns
            .iter()
            .map(|column| format!("\"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let values = table
            .columns
            .iter()
            .map(|column| format!("json_extract(value,'$.{column}')"))
            .collect::<Vec<_>>()
            .join(", ");
        let filter = if table.table == "asset_tags" {
            " WHERE json_extract(value,'$.tag_id') IN (SELECT id FROM tags)"
        } else {
            ""
        };
        let query = format!(
            "INSERT INTO {} ({column_names}) SELECT {values} FROM json_each(?){filter}",
            table.table
        );
        for page in table.rows.chunks(PAGE_SIZE) {
            sqlx::query(&query)
                .bind(serde_json::to_string(page).map_err(json_error)?)
                .execute(&mut **transaction)
                .await?;
        }
        if table.table == "assets" {
            for row in table.rows {
                restored.push((
                    required(&row, "id")?.into(),
                    required(&row, "path")?.into(),
                    required(&row, "format_type")?.into(),
                ));
            }
        }
    }
    let identifiers: Vec<_> = restored
        .iter()
        .map(|(identifier, _, _)| identifier)
        .collect();
    sqlx::query(
        "UPDATE assets SET thumbnail_path = NULL WHERE id IN (SELECT value FROM json_each(?))",
    )
    .bind(serde_json::to_string(&identifiers).map_err(json_error)?)
    .execute(&mut **transaction)
    .await?;
    Ok(restored)
}

fn validate_row(table: &str, row: &serde_json::Value) -> AppResult<()> {
    required(
        row,
        if table == "folders" || table == "assets" {
            "id"
        } else if table == "library_folder_identities" {
            "folder_id"
        } else {
            "asset_id"
        },
    )?;
    if table == "folders" || table == "assets" {
        required(row, "path")?;
        required(row, "name")?;
    }
    if table == "assets" {
        required(row, "format_type")?;
    }
    if table == "asset_tags" {
        required(row, "tag_id")?;
    }
    if table.starts_with("library_") {
        for field in ["device", "object"] {
            required(row, field)?
                .parse::<u64>()
                .map_err(|_| invalid("Invalid native identity in recovery data"))?;
        }
        if let Some(birth) = row.get("birth").and_then(serde_json::Value::as_str) {
            birth
                .parse::<u64>()
                .map_err(|_| invalid("Invalid native identity generation"))?;
        }
    }
    Ok(())
}

fn required<'value>(row: &'value serde_json::Value, field: &str) -> AppResult<&'value str> {
    row.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(&format!("Recovery row requires nonempty {field}")))
}
