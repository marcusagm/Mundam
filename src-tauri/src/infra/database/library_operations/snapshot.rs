//! Captures user-authored catalog data; generated processing and duplicate indexes are rebuilt.
use crate::core::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

const TABLES: [&str; 5] = [
    "folders",
    "assets",
    "asset_metadata_envelope",
    "asset_tags",
    "asset_colors",
];

/// Schema-aware rows retain nullable fields without manually duplicating every asset column.
#[derive(Serialize, Deserialize)]
pub(super) struct TableSnapshot {
    table: String,
    columns: Vec<String>,
    rows: Vec<serde_json::Value>,
}

async fn columns(transaction: &mut Transaction<'_, Sqlite>, table: &str) -> AppResult<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
            .bind(table)
            .fetch_all(&mut **transaction)
            .await?,
    )
}

/// Retains catalog rows before cascading deletion removes their ownership links.
///
/// # Errors
/// Returns database or serialization errors before any original file is moved.
pub(super) async fn capture(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: &str,
) -> AppResult<String> {
    let folders: Vec<String> = sqlx::query_scalar("WITH RECURSIVE tree(id) AS (SELECT id FROM folders WHERE id = ? UNION ALL SELECT folders.id FROM folders JOIN tree ON folders.parent_id = tree.id) SELECT id FROM tree")
        .bind(folder_id).fetch_all(&mut **transaction).await?;
    let assets: Vec<String> =
        crate::infra::database::handlers::folder_removal::collect_assets(transaction, folder_id)
            .await?
            .into_iter()
            .map(|asset| asset.id)
            .collect();
    let mut snapshot = Vec::new();
    for table in TABLES {
        let columns = columns(transaction, table).await?;
        let json_fields = columns
            .iter()
            .map(|column| format!("'{column}', \"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let identifier_column = if table == "folders" || table == "assets" {
            "id"
        } else {
            "asset_id"
        };
        let identifiers = if table == "folders" {
            &folders
        } else {
            &assets
        };
        let query = format!("SELECT json_object({json_fields}) FROM {table} WHERE {identifier_column} IN (SELECT value FROM json_each(?))");
        let rows: Vec<String> = sqlx::query_scalar(&query)
            .bind(serde_json::to_string(identifiers).map_err(json_error)?)
            .fetch_all(&mut **transaction)
            .await?;
        snapshot.push(TableSnapshot {
            table: table.into(),
            columns,
            rows: rows
                .into_iter()
                .map(|row| serde_json::from_str(&row).map_err(json_error))
                .collect::<AppResult<_>>()?,
        });
    }
    serde_json::to_string(&snapshot).map_err(json_error)
}

/// Reinstates stable identities while leaving generated processing work to the workers.
///
/// # Errors
/// Rejects incompatible schemas, corrupt snapshots and constraint conflicts.
pub(super) async fn restore(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &str,
) -> AppResult<Vec<(String, String, String)>> {
    let tables: Vec<TableSnapshot> = serde_json::from_str(snapshot).map_err(json_error)?;
    let mut restored_assets = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        if TABLES.get(index).copied() != Some(table.table.as_str())
            || columns(transaction, &table.table).await? != table.columns
        {
            return Err(AppError::ValidationFailed(
                "Folder recovery schema does not match this library".into(),
            ));
        }
        let column_names = table
            .columns
            .iter()
            .map(|column| format!("\"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let values = table
            .columns
            .iter()
            .map(|column| format!("json_extract(value, '$.{column}')"))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT INTO {} ({column_names}) SELECT {values} FROM json_each(?)",
            table.table
        );
        sqlx::query(&query)
            .bind(serde_json::to_string(&table.rows).map_err(json_error)?)
            .execute(&mut **transaction)
            .await?;
        if table.table == "assets" {
            for row in &table.rows {
                let field = |name: &str| {
                    row.get(name)
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                restored_assets.push((field("id"), field("path"), field("format_type")));
            }
        }
    }
    if tables.len() != TABLES.len() {
        return Err(AppError::ValidationFailed(
            "Incomplete folder recovery snapshot".into(),
        ));
    }
    for (identifier, _, _) in &restored_assets {
        sqlx::query("UPDATE assets SET thumbnail_path = NULL WHERE id = ?")
            .bind(identifier)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(restored_assets)
}

fn json_error(error: serde_json::Error) -> AppError {
    AppError::Internal(format!("Folder recovery data: {error}"))
}
