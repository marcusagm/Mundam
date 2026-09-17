use crate::core::error::AppResult;
use crate::core::models::{DuplicateCandidate, DuplicateGroup, DuplicateResolutionAction};
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
#[tauri::command]
pub async fn resolve_duplicate_group(
    app_handle: tauri::AppHandle,
    group_id: String,
    action: String,
    kept_asset_ids: Option<Vec<String>>,
    command_service: State<'_, DuplicateCommandService>,
    query_service: State<'_, DuplicateQueryService>,
    asset_query: State<'_, crate::feature::assets::queries::AssetQueryService>,
) -> AppResult<()> {
    use std::str::FromStr;
    use tauri::Manager;
    let parsed_action = DuplicateResolutionAction::from_str(&action)
        .map_err(|_| crate::core::error::AppError::ValidationFailed(format!("Invalid action: {}", action)))?;

    let candidates = query_service.get_group_candidates(&group_id).await.unwrap_or_default();

    command_service
        .resolve_group(&group_id, parsed_action.clone(), kept_asset_ids.clone())
        .await?;

    if matches!(parsed_action, DuplicateResolutionAction::CustomSelection) {
        if let Some(kept_ids) = &kept_asset_ids {
            let dirs = app_handle.state::<crate::bootstrap::AppDirectories>();
            let trash_dir = crate::core::trash::trash_directory(&dirs.app_data);
            if !trash_dir.exists() {
                std::fs::create_dir_all(&trash_dir).ok();
            }

            for candidate in candidates {
                if !kept_ids.contains(&candidate.asset_id) {
                    if let Ok(Some(asset)) = asset_query.get_asset(&candidate.asset_id).await {
                        if let Some(deleted_at) = asset.deleted_at {
                            if let Some(trash_path) = crate::core::trash::build_trash_path(
                                &dirs.app_data, &candidate.asset_id, &asset.path, &deleted_at,
                            ) {
                                if asset.path.exists() {
                                    let _ = tokio::fs::rename(&asset.path, &trash_path).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
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
