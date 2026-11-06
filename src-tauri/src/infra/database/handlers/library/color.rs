//! Folder presentation edits remain independent of disk availability and hierarchy discovery.
use crate::core::{
    error::{AppError, AppResult},
    models::asset::Asset,
};
use crate::infra::database::handlers::shared;
use sqlx::{Sqlite, Transaction};

fn invalid(message: &str) -> AppError {
    AppError::ValidationFailed(message.into())
}

/// Updates only presentation fields and records the matching audit intention.
///
/// # Errors
/// Returns missing folders, invalid hexadecimal colors and persistence failures.
pub(crate) async fn update(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: String,
    color: Option<String>,
    inherits_color: bool,
) -> AppResult<Asset> {
    if let Some(value) = &color {
        if value.len() != 7
            || !value.starts_with('#')
            || !value[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(invalid("Use a hexadecimal color such as #5588cc"));
        }
    }
    let (path, name): (String, String) =
        sqlx::query_as("SELECT path, name FROM folders WHERE id = ?")
            .bind(&folder_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| AppError::NotFound("Folder not found".into()))?;
    sqlx::query("UPDATE folders SET color = ?, inherits_color = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(&color).bind(inherits_color).bind(&folder_id).execute(&mut **transaction).await?;
    shared::log_operation(
        transaction,
        "FOLDER_COLOR",
        &folder_id,
        serde_json::json!({ "color": color, "inheritsColor": inherits_color }),
        "COMPLETED",
        None,
    )
    .await?;
    Ok(super::projection::folder(
        folder_id,
        name,
        path.into(),
        None,
    ))
}
