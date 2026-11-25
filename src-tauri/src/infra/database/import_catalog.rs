//! Scoped catalog reads and revision-bound pagination for the external importer.
use crate::core::{
    imports::catalog::{
        CatalogPage, CatalogPageRequest, CatalogReadGrant, FolderAvailability, ImportCatalog,
        ImportFolder, ImportTag,
    },
    AppError, AppResult,
};
use async_trait::async_trait;
use sqlx::{SqliteConnection, SqlitePool};

/// Validates assigned tag IDs without exposing the taxonomy to the caller.
///
/// # Errors
/// Returns serialization or catalog failures.
pub async fn contains_import_tags(pool: &SqlitePool, tag_ids: &[&String]) -> AppResult<bool> {
    let encoded = serde_json::to_string(tag_ids).map_err(serialization_error)?;
    let present = sqlx::query_scalar(
        "SELECT NOT EXISTS (SELECT 1 FROM json_each(?1) AS selected
         WHERE NOT EXISTS (SELECT 1 FROM tags WHERE tags.id = selected.value))",
    )
    .bind(encoded)
    .fetch_one(pool)
    .await?;
    Ok(present)
}

/// Reads external projections from the active catalog. The host supplies a private
/// cursor key shared by requests handled in the same service instance.
pub struct SqliteImportCatalog<'pool> {
    pool: &'pool SqlitePool,
    cursor_key: [u8; 32],
}

impl<'pool> SqliteImportCatalog<'pool> {
    /// Creates the read adapter with a host-generated cursor signing key.
    pub fn new(pool: &'pool SqlitePool, cursor_key: [u8; 32]) -> Self {
        Self { pool, cursor_key }
    }
}

#[async_trait]
impl ImportCatalog for SqliteImportCatalog<'_> {
    async fn list_folders(
        &self,
        grant: &CatalogReadGrant,
        parent_id: Option<&str>,
        request: CatalogPageRequest<'_>,
    ) -> AppResult<CatalogPage<ImportFolder>> {
        if !grant.can_read_folders {
            return Err(AppError::Forbidden(
                "Folder read permission required".into(),
            ));
        }
        let mut transaction = self.pool.begin().await?;
        let revision = revision(&mut transaction, grant, "folders").await?;
        let mut roots = grant.destinations.root_ids.clone();
        roots.sort_unstable();
        roots.dedup();
        let page = Pager::new(
            &self.cursor_key,
            request,
            &(grant, "folders", parent_id, revision, &roots),
        )?;
        let roots = serde_json::to_string(&roots).map_err(serialization_error)?;
        if let Some(parent) = parent_id {
            let visible: bool = sqlx::query_scalar(
                "WITH RECURSIVE lineage(id, parent_id) AS (
                    SELECT id, parent_id FROM folders WHERE id = ?1
                    UNION
                    SELECT folders.id, folders.parent_id FROM folders
                    JOIN lineage ON folders.id = lineage.parent_id
                )
                SELECT EXISTS (
                    SELECT 1 FROM lineage WHERE parent_id IS NULL
                    AND id IN (SELECT value FROM json_each(?2))
                )",
            )
            .bind(parent)
            .bind(&roots)
            .fetch_one(&mut *transaction)
            .await?;
            if !visible {
                return Err(AppError::NotFound("Import folder unavailable".into()));
            }
        }
        let rows: Vec<(String, Option<String>, String, bool)> = sqlx::query_as(
            "SELECT folder.id, folder.parent_id, folder.name,
                EXISTS (SELECT 1 FROM folders AS child WHERE child.parent_id = folder.id)
             FROM folders AS folder
             WHERE folder.parent_id IS ?1
               AND (?1 IS NOT NULL OR folder.id IN (SELECT value FROM json_each(?2)))
             ORDER BY folder.name, folder.id LIMIT ?3 OFFSET ?4",
        )
        .bind(parent_id)
        .bind(roots)
        .bind(page.limit + 1)
        .bind(page.offset)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        let (rows, next_cursor) = page.finish(rows);
        Ok(CatalogPage {
            items: rows
                .into_iter()
                .map(|(id, parent_id, name, has_children)| ImportFolder {
                    id,
                    parent_id,
                    name,
                    has_children,
                    availability: FolderAvailability::Unknown,
                })
                .collect(),
            next_cursor,
        })
    }

    async fn list_tags(
        &self,
        grant: &CatalogReadGrant,
        query: Option<&str>,
        request: CatalogPageRequest<'_>,
    ) -> AppResult<CatalogPage<ImportTag>> {
        if !grant.can_read_tags {
            return Err(AppError::Forbidden("Tag read permission required".into()));
        }
        let query = query.unwrap_or_default().trim();
        if query.len() > 256 || query.chars().any(char::is_control) {
            return Err(AppError::ValidationFailed("Invalid tag query".into()));
        }
        let mut transaction = self.pool.begin().await?;
        let revision = revision(&mut transaction, grant, "tags").await?;
        let page = Pager::new(
            &self.cursor_key,
            request,
            &(grant, "tags", query, revision, ()),
        )?;
        let rows: Vec<TagRow> = sqlx::query_as(
            "WITH RECURSIVE
             page AS (
                 SELECT id, parent_id, name, color, COALESCE(order_index, 0) AS sort_order
                 FROM tags WHERE instr(lower(name), lower(?1)) > 0
                 ORDER BY sort_order, name, id LIMIT ?2 OFFSET ?3
             ),
             ancestors(tag_id, parent_id, name, depth, visited) AS (
                 SELECT id, parent_id, name, 0, json_array(id) FROM page
                 UNION ALL
                 SELECT ancestors.tag_id, tags.parent_id, tags.name, ancestors.depth + 1,
                     json_insert(ancestors.visited, '$[#]', tags.id)
                 FROM ancestors JOIN tags ON tags.id = ancestors.parent_id
                 WHERE tags.id NOT IN (SELECT value FROM json_each(ancestors.visited))
             )
             SELECT page.*, (
                 SELECT json_group_array(name) FROM (
                     SELECT name FROM ancestors WHERE tag_id = page.id ORDER BY depth DESC
                 )
             ) AS labels
             FROM page ORDER BY sort_order, name, id",
        )
        .bind(query)
        .bind(page.limit + 1)
        .bind(page.offset)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        let (rows, next_cursor) = page.finish(rows);
        let items = rows
            .into_iter()
            .map(|row| {
                Ok(ImportTag {
                    id: row.id,
                    parent_id: row.parent_id,
                    name: row.name,
                    color: row.color,
                    order_index: row.sort_order,
                    labels: serde_json::from_str(&row.labels).map_err(serialization_error)?,
                })
            })
            .collect::<AppResult<_>>()?;
        Ok(CatalogPage { items, next_cursor })
    }
}

#[derive(sqlx::FromRow)]
struct TagRow {
    id: String,
    parent_id: Option<String>,
    name: String,
    color: Option<String>,
    sort_order: i64,
    labels: String,
}

async fn revision(
    connection: &mut SqliteConnection,
    grant: &CatalogReadGrant,
    kind: &str,
) -> AppResult<i64> {
    sqlx::query_scalar(
        "SELECT revision FROM import_catalog_revisions, import_preferences
         WHERE kind = ?1 AND singleton = 1 AND catalog_id = ?2",
    )
    .bind(kind)
    .bind(&grant.destinations.catalog_id)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| AppError::NotFound("Import catalog unavailable".into()))
}

/// A cursor ties its offset to the query and the revision observed in the read transaction.
struct Pager {
    signer: blake3::Hasher,
    limit: i64,
    offset: i64,
}

impl Pager {
    fn new<Filter: serde::Serialize, Scope: serde::Serialize>(
        key: &[u8; 32],
        request: CatalogPageRequest<'_>,
        (grant, kind, filter, revision, scope): &(&CatalogReadGrant, &str, Filter, i64, Scope),
    ) -> AppResult<Self> {
        let limit = i64::from(request.limit.unwrap_or(50));
        if !(1..=100).contains(&limit) {
            return Err(AppError::ValidationFailed(
                "Page limit must be between 1 and 100".into(),
            ));
        }
        let context = serde_json::to_vec(&(
            "mundam-import-catalog-v1",
            &grant.client_id,
            grant.grant_version,
            &grant.destinations.catalog_id,
            kind,
            filter,
            revision,
            scope,
        ))
        .map_err(serialization_error)?;
        let mut signer = blake3::Hasher::new_keyed(key);
        signer.update(&context);
        let mut page = Self {
            signer,
            limit,
            offset: 0,
        };
        if let Some(cursor) = request.cursor {
            page.offset = page.read_cursor(cursor).ok_or(AppError::CursorExpired)?;
        }
        Ok(page)
    }

    fn signature(&self, offset: i64) -> blake3::Hash {
        self.signer.clone().update(&offset.to_be_bytes()).finalize()
    }

    fn read_cursor(&self, cursor: &str) -> Option<i64> {
        if cursor.len() > 84 {
            return None;
        }
        let (offset, signature) = cursor.split_once('.')?;
        let offset: i64 = offset.parse().ok()?;
        let signature = blake3::Hash::from_hex(signature).ok()?;
        (offset > 0 && offset <= i64::MAX - 100 && self.signature(offset) == signature)
            .then_some(offset)
    }

    fn finish<Item>(&self, mut items: Vec<Item>) -> (Vec<Item>, Option<String>) {
        let cursor = (items.len() > self.limit as usize).then(|| {
            items.pop();
            let next_offset = self.offset + self.limit;
            format!("{next_offset}.{}", self.signature(next_offset))
        });
        (items, cursor)
    }
}

fn serialization_error(error: serde_json::Error) -> AppError {
    AppError::Internal(error.to_string())
}

#[cfg(test)]
mod tests;
