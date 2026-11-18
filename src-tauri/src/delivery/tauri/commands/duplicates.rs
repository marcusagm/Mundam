use crate::core::error::AppResult;
use crate::core::models::{
    DuplicateCandidate, DuplicateGroup, DuplicateResolutionAction, MetadataMergePayload,
};
use crate::delivery::tauri::admission::IpcRequest;
use crate::feature::duplicates::commands::{DuplicateCommandService, DuplicateScanStatus};
use crate::feature::duplicates::queries::DuplicateQueryService;
use tauri::State;

/// Retrieves duplicate groups by status.
#[tauri::command]
pub async fn get_duplicate_groups(
    _ipc: IpcRequest,
    status: String,
    query_service: State<'_, DuplicateQueryService>,
) -> AppResult<Vec<DuplicateGroup>> {
    query_service.get_groups_by_status(&status).await
}

/// Retrieves candidates for a duplicate group.
#[tauri::command]
pub async fn get_duplicate_candidates(
    _ipc: IpcRequest,
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
    _ipc: IpcRequest,
    group_id: String,
    action: String,
    kept_asset_ids: Option<Vec<String>>,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    use std::str::FromStr;
    let parsed_action = DuplicateResolutionAction::from_str(&action).map_err(|_| {
        crate::core::error::AppError::ValidationFailed(format!("Invalid action: {}", action))
    })?;

    command_service
        .resolve_group(&group_id, parsed_action, kept_asset_ids)
        .await
}

/// Reverts a duplicate group resolution.
///
/// Restores any candidate assets sent to the trash during group resolution,
/// deletes the historical resolution record, and reverts the group status back to `open`.
///
/// # Arguments
/// * `group_id` - The unique ID of the duplicate group to revert.
/// * `command_service` - The duplicate command service state.
///
/// # Errors
/// Returns `AppError::Database` if database updates or restorations fail.
#[tauri::command]
pub async fn undo_duplicate_resolution(
    _ipc: IpcRequest,
    group_id: String,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.undo_duplicate_resolution(&group_id).await
}

/// Dispara uma varredura por duplicados no repositório em segundo plano.
#[tauri::command]
pub async fn start_duplicate_scan(
    _ipc: IpcRequest,
    command_service: State<'_, DuplicateCommandService>,
    lifecycle: State<'_, std::sync::Arc<crate::lifecycle::LifecycleRegistry>>,
) -> AppResult<()> {
    let service_clone = command_service.inner().clone();
    let cancellation = lifecycle.child_token();
    if !lifecycle.spawn_unique(format!("manual_duplicate_scan:{}", uuid::Uuid::new_v4()), async move {
        if let Err(scan_error) = service_clone.start_duplicate_scan_with_token(cancellation.clone()).await {
            if cancellation.is_cancelled() {
                tracing::debug!(%scan_error, "Background duplicate scan stopped during shutdown");
            } else {
                tracing::error!(%scan_error, "Background duplicate scan error");
            }
        }
    }) {
        return Err(crate::core::AppError::Cancelled);
    }
    Ok(())
}

/// Cancela a varredura de duplicados.
#[tauri::command]
pub async fn cancel_duplicate_scan(
    _ipc: IpcRequest,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.cancel_duplicate_scan().await?;
    Ok(())
}

/// Retrieves the current runtime status of the duplicate scan.
///
/// # Arguments
/// * `command_service` - The duplicate command service state.
///
/// # Errors
/// Returns `AppResult<DuplicateScanStatus>` representing current scanning state.
#[tauri::command]
pub async fn get_duplicate_scan_status(
    _ipc: IpcRequest,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<DuplicateScanStatus> {
    Ok(command_service.get_scan_status())
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
    _ipc: IpcRequest,
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
    _ipc: IpcRequest,
    rule_set: crate::core::models::DuplicateRuleSet,
    command_service: State<'_, DuplicateCommandService>,
) -> AppResult<()> {
    command_service.update_rule_set(rule_set).await
}

/// Retrieves the most recently active duplicate rule set configuration.
///
/// # Arguments
/// * `query_service` - State containing the duplicate query service.
///
/// # Errors
/// Returns `AppError::Database` if the query fails.
#[tauri::command]
pub async fn get_active_duplicate_rule_set(
    _ipc: IpcRequest,
    query_service: State<'_, DuplicateQueryService>,
) -> AppResult<Option<crate::core::models::DuplicateRuleSet>> {
    query_service.get_active_rule_set().await
}

/// Erases all existing duplicate index tables and triggers a full rescan.
///
/// # Arguments
/// * `command_service` - State containing the duplicate command service.
///
/// # Errors
/// Returns `AppError::Database` if the index reset or rescan fails.
#[tauri::command]
pub async fn rebuild_duplicate_index(
    _ipc: IpcRequest,
    command_service: State<'_, DuplicateCommandService>,
    lifecycle: State<'_, std::sync::Arc<crate::lifecycle::LifecycleRegistry>>,
) -> AppResult<()> {
    command_service
        .rebuild_duplicate_index_with_token(lifecycle.child_token())
        .await
}
