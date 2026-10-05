use crate::core::error::AppResult;
use crate::core::models::{DuplicateCandidate, DuplicateGroup, DuplicateResolutionAction, MetadataMergePayload};
use crate::feature::duplicates::commands::DuplicateCommandService;
use crate::feature::duplicates::queries::DuplicateQueryService;
use tauri::State;

/// Retrieves duplicate groups by status.
#[tauri::command]
pub async fn get_duplicate_groups(
    status: String,
    query_service: State<'_, DuplicateQueryService>,
) -> AppResult<Vec<DuplicateGroup>> {
    query_service.get_groups_by_status(&status).await
}

/// Retrieves candidates for a duplicate group.
#[tauri::command]
pub async fn get_duplicate_candidates(
    group_id: String,
    query_service: State<'_, DuplicateQueryService>,
) -> AppResult<Vec<DuplicateCandidate>> {
    query_service.get_group_candidates(&group_id).await
}

/// Resolves a duplicate group with a given action.
///
/// Delegates to `DuplicateCommandService::resolve_group`, which handles
/// marking non-kept assets as trashed via the Asset Ledger. The Ledger's
/// Saga post-commit step then handles the physical file move to the trash
/// directory, eliminating the need for any filesystem logic here.
///
/// # Arguments
///
/// * `group_id` - The unique ID of the duplicate group.
/// * `action` - The resolution action string (e.g. "custom_selection", "ignore").
/// * `kept_asset_ids` - Optional list of asset IDs to keep.
///
/// # Errors
///
/// Returns `AppError::ValidationFailed` if the action string is invalid.
#[tauri::command]
pub async fn resolve_duplicate_group(
    group_id: String,
    action: String,
    kept_asset_ids: Option<Vec<String>>,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    use std::str::FromStr;
    let parsed_action = DuplicateResolutionAction::from_str(&action)
        .map_err(|_| crate::core::error::AppError::ValidationFailed(format!("Invalid action: {}", action)))?;

    command_service
        .resolve_group(&group_id, parsed_action, kept_asset_ids)
        .await
}

/// Dispara uma varredura por duplicados no repositório.
#[tauri::command]
pub async fn start_duplicate_scan(
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.start_duplicate_scan().await?;
    Ok(())
}

/// Cancela a varredura de duplicados.
#[tauri::command]
pub async fn cancel_duplicate_scan(
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.cancel_duplicate_scan();
    Ok(())
}

/// Applies a user-confirmed metadata merge to the kept assets of a duplicate group.
///
/// This command is invoked after the user has reviewed the `MetadataMergeModal`
/// and confirmed their field-by-field merge decisions. It delegates the actual
/// ledger mutations to `DuplicateCommandService::apply_metadata_merge`.
///
/// # Arguments
/// * `kept_asset_ids` - The IDs of the assets that will receive the merged metadata.
/// * `merge_payload` - The user-confirmed merge decisions from the UI modal.
#[tauri::command]
pub async fn apply_duplicate_metadata_merge(
    kept_asset_ids: Vec<String>,
    merge_payload: MetadataMergePayload,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service
        .apply_metadata_merge(&kept_asset_ids, merge_payload)
        .await
}

/// Updates a duplicate rule set (e.g. from the settings modal).
#[tauri::command]
pub async fn update_duplicate_rule_set(
    rule_set: crate::core::models::DuplicateRuleSet,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.update_rule_set(rule_set).await
}


