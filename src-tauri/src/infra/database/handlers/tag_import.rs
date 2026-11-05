//! Merges and replaces taxonomy outlines within the ledger-owned transaction.

use super::{tag_hierarchy, tags_handler};
use crate::core::error::{AppError, AppResult};
use crate::core::ledger::command::{CreateTagPayload, ImportTagsPayload, UpdateTagPayload};
use crate::core::models::asset::Asset;
use sqlx::{Sqlite, Transaction};

/// Reuses matching tags to preserve associations and appends outline siblings in order.
///
/// # Errors
/// Returns validation errors for conflicting existing hierarchies or invalid
/// outlines, and database errors. The ledger rolls back the complete import.
pub async fn handle_import(
    transaction: &mut Transaction<'_, Sqlite>,
    payload: ImportTagsPayload,
) -> AppResult<Asset> {
    crate::core::ledger::tag_validation::validate_import(&payload)?;
    let audit_payload = serde_json::to_value(&payload)
        .map_err(|error| AppError::Internal(format!("Failed to serialize tag import: {error}")))?;
    if payload.replace_existing {
        sqlx::query("DELETE FROM asset_tags")
            .execute(&mut **transaction)
            .await?;
        sqlx::query("DELETE FROM tags")
            .execute(&mut **transaction)
            .await?;
    }
    let mut imported_ids: Vec<String> = Vec::new();
    let mut last_asset = None;
    for entry in payload.entries {
        let parent_id = entry.parent_index.map(|index| imported_ids[index].clone());
        let existing: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT id, parent_id FROM tags WHERE name = ?")
                .bind(entry.name.trim())
                .fetch_optional(&mut **transaction)
                .await?;
        let tag_id = if let Some((identifier, existing_parent)) = existing {
            if existing_parent != parent_id {
                return Err(AppError::ValidationFailed(format!(
                    "Tag '{}' already exists under a different parent.",
                    entry.name
                )));
            }
            identifier
        } else {
            let created = tags_handler::handle_create_tag(
                transaction,
                CreateTagPayload {
                    name: entry.name.trim().to_string(),
                    parent_id: parent_id.clone(),
                    color: entry.color.clone(),
                },
            )
            .await?;
            created.id
        };
        let next_order: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(order_index), -100) + 100 FROM tags WHERE parent_id IS ?",
        )
        .bind(&parent_id)
        .fetch_one(&mut **transaction)
        .await?;
        last_asset = Some(
            tags_handler::handle_update_tag(
                transaction,
                UpdateTagPayload {
                    id: tag_id.clone(),
                    name: None,
                    color: entry.color.clone(),
                    parent_id: None,
                    order_index: Some(next_order),
                    inherits_color: Some(entry.color.is_none()),
                },
            )
            .await?,
        );
        imported_ids.push(tag_id);
    }
    tag_hierarchy::refresh_colors(transaction).await?;
    super::shared::log_operation(
        transaction,
        "IMPORT_TAGS",
        "taxonomy",
        audit_payload,
        "COMPLETED",
        None,
    )
    .await?;
    last_asset.ok_or_else(|| AppError::ValidationFailed("Enter at least one tag.".into()))
}
