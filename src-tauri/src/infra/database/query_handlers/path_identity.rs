//! Exact catalog resolution with live directory-entry proofs for case, Unicode and separator aliases.
use crate::core::error::AppResult;
use crate::core::path_guard::spelling_key;
use sqlx::SqliteConnection;
use std::path::Path;

/// Resolves cleanup candidates only when both observed and catalog paths are missing.
///
/// This is not an identity proof and must never drive rename, move or creation.
/// Deferred deletion also fences the captured catalog path; live entries and ambiguity are kept.
///
/// # Errors
/// Returns catalog or parent-resolution failures.
pub(crate) async fn missing_asset(
    connection: &mut SqliteConnection,
    path: &Path,
) -> AppResult<Option<String>> {
    if tokio::fs::try_exists(path).await.unwrap_or(true) {
        return Ok(None);
    }
    if let Some(identifier) = resolve(connection, "assets", path).await? {
        return Ok(Some(identifier));
    }
    let Some(parent) = path.parent() else {
        return Ok(None);
    };
    let Some(parent_id) = resolve(connection, "folders", parent).await? else {
        return Ok(None);
    };
    let Some(filename) = path.file_name() else {
        return Ok(None);
    };
    let key = spelling_key(Path::new(filename));
    let mut candidates = Vec::new();
    let mut offset = 0i64;
    loop {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id,path FROM assets WHERE folder_id = ? ORDER BY id LIMIT 256 OFFSET ?",
        )
        .bind(&parent_id)
        .bind(offset)
        .fetch_all(&mut *connection)
        .await?;
        let count = rows.len();
        for (identifier, candidate) in rows {
            let candidate = Path::new(&candidate);
            if candidate
                .file_name()
                .is_some_and(|name| spelling_key(Path::new(name)) == key)
                && !tokio::fs::try_exists(candidate).await.unwrap_or(true)
            {
                candidates.push(identifier);
            }
            if candidates.len() > 1 {
                return Ok(None);
            }
        }
        if count < 256 {
            break;
        }
        offset += count as i64;
    }
    Ok(candidates.pop())
}

/// Resolves exact paths first, then verifies aliases within the resolved parent scope.
///
/// Offline aliases are not guessed. Sibling reads are paged and native proofs distinguish
/// case-sensitive entries and hardlinks even when their comparison keys happen to match.
///
/// # Errors
/// Returns catalog lookup failures; inaccessible aliases are not identity proofs.
pub(crate) async fn resolve(
    connection: &mut SqliteConnection,
    table: &str,
    path: &Path,
) -> AppResult<Option<String>> {
    let table = match table {
        "folders" => "folders",
        "assets" => "assets",
        _ => return Ok(None),
    };
    let exact: Option<String> =
        sqlx::query_scalar(&format!("SELECT id FROM {table} WHERE path = ?"))
            .bind(path.to_string_lossy().as_ref())
            .fetch_optional(&mut *connection)
            .await?;
    if exact.is_some() {
        return Ok(exact);
    }
    let parent = if let Some(parent) = path.parent() {
        Box::pin(resolve(connection, "folders", parent)).await?
    } else {
        None
    };
    if parent.is_none() && table == "assets" {
        return Ok(None);
    }
    let parent_column = if table == "folders" {
        "parent_id"
    } else {
        "folder_id"
    };
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let scope = format!("{parent_column} IS ?");
    let name_filter = if parent.is_some() && filename.is_ascii() {
        " AND name = ? COLLATE NOCASE"
    } else {
        ""
    };
    let query = format!(
        "SELECT id,path FROM {table} WHERE {scope}{name_filter} ORDER BY rowid LIMIT 256 OFFSET ?"
    );
    let key = spelling_key(path);
    let mut offset = 0i64;
    loop {
        let mut query = sqlx::query_as::<_, (String, String)>(&query).bind(&parent);
        if !name_filter.is_empty() {
            query = query.bind(&filename);
        }
        let candidates = query.bind(offset).fetch_all(&mut *connection).await?;
        let count = candidates.len();
        for (identifier, candidate) in candidates {
            let candidate = Path::new(&candidate);
            let matches = if parent.is_some() {
                candidate.file_name().is_some_and(|name| {
                    spelling_key(Path::new(name)) == spelling_key(Path::new(&filename))
                })
            } else {
                spelling_key(candidate) == key
            };
            if matches
                && crate::infra::filesystem::identity::same(path, candidate)
                    .await
                    .unwrap_or(false)
            {
                return Ok(Some(identifier));
            }
        }
        if count < 256 {
            return Ok(None);
        }
        offset += count as i64;
    }
}
