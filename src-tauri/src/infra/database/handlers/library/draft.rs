//! Empty-draft cancellation is an owned transition, never a recursive content deletion.
use super::{
    plan::DiskChanges,
    validation::{folder, invalid},
};
use crate::core::{error::AppResult, ledger::command::RemoveFolderPayload, models::asset::Asset};
use crate::infra::database::handlers::folder_handler;
use sqlx::{Sqlite, Transaction};

/// Cancels only an unchanged creation receipt with no physical or catalogued contents.
///
/// # Errors
/// Rejects replaced/renamed drafts, external contents, pending catalog items and storage failures.
pub(crate) async fn cancel(
    transaction: &mut Transaction<'_, Sqlite>,
    folder_id: String,
    creation_operation_id: String,
    changes: &mut DiskChanges,
) -> AppResult<Asset> {
    let identity_json: Option<String> = sqlx::query_scalar("SELECT created_identity_json FROM library_operation_receipts WHERE operation_id = ? AND entity_id = ? AND operation_kind = 'createFolder' AND confirmed_path = (SELECT path FROM folders WHERE id = entity_id)")
                .bind(creation_operation_id).bind(&folder_id).fetch_optional(&mut **transaction).await?.flatten();
    let expected: crate::core::library::FileIdentity = serde_json::from_str(
        &identity_json.ok_or_else(|| invalid("Creation ownership is unavailable"))?,
    )
    .map_err(|_| invalid("Invalid creation receipt"))?;
    let (path, _) = folder(transaction, &folder_id).await?;
    if crate::infra::filesystem::identity::read(&path).await? != expected {
        return Err(invalid("Draft folder has been replaced"));
    }
    let mut contents = tokio::fs::read_dir(&path).await?;
    if contents.next_entry().await?.is_some() {
        return Err(invalid(
            "Folder contains files. Confirm its name or delete it with Undo.",
        ));
    }
    let registered_contents: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM assets WHERE folder_id = ?) + (SELECT COUNT(*) FROM folders WHERE parent_id = ?)")
                .bind(&folder_id).bind(&folder_id).fetch_one(&mut **transaction).await?;
    if registered_contents != 0 {
        return Err(invalid(
            "Folder still has catalogued contents. Confirm its name or wait for reconciliation.",
        ));
    }
    changes.remove_empty = Some(path);
    folder_handler::handle_remove_folder(
        transaction,
        RemoveFolderPayload {
            folder_id,
            expected_path: None,
        },
    )
    .await
}
