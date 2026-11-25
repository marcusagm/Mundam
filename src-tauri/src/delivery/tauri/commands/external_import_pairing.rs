//! Trusted administrative commands for external importer consent.
use crate::{
    core::{
        imports::{
            destinations::ImportDestinations,
            pairing::{
                ImportPairing, ImportScope, PairedImporter, PairingApproval, PendingPairing,
            },
        },
        repository::AssetQueryHandler,
        AppResult,
    },
    delivery::{
        external_api::desktop::{DesktopImportService, ImportEndpoint},
        tauri::admission::IpcRequest,
    },
    infra::database::{
        import_destinations::SqliteImportDestinations, import_pairing::SqliteImportPairing,
        manager::DbManager,
    },
};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// Desktop-generated invitation and public HTTPS connection information.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopInvitation {
    version: u8,
    #[serde(flatten)]
    endpoint: ImportEndpoint,
    code: String,
    expires_at: i64,
}

#[derive(Serialize)]
struct ConsentRoot {
    id: String,
    name: String,
    path: String,
}

/// Snapshot of the active catalog and its importer consent requests.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPairingStatus {
    catalog_id: String,
    pending: Vec<PendingPairing>,
    clients: Vec<PairedImporter>,
    roots: Vec<ConsentRoot>,
}

/// Creates an invitation for the active HTTPS endpoint.
///
/// # Errors
/// Returns service unavailable or a catalog write failure.
#[tauri::command]
pub async fn create_external_import_invitation(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    database: State<'_, Arc<DbManager>>,
) -> AppResult<DesktopInvitation> {
    let (endpoint, server_key) = service.pairing_session().await?;
    let invitation = SqliteImportPairing::new(database.pool(), server_key)
        .invite()
        .await?;
    Ok(DesktopInvitation {
        version: 1,
        endpoint,
        code: invitation.code,
        expires_at: invitation.expires_at,
    })
}

/// Lists pending requests, current paired clients and roots available to grant.
///
/// # Errors
/// Returns service unavailable or a catalog read failure.
#[tauri::command]
pub async fn list_external_import_pairings(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    database: State<'_, Arc<DbManager>>,
    queries: State<'_, Arc<dyn AssetQueryHandler>>,
) -> AppResult<DesktopPairingStatus> {
    let (_, server_key) = service.pairing_session().await?;
    let pairing = SqliteImportPairing::new(database.pool(), server_key);
    let destinations = SqliteImportDestinations(database.pool());
    let (preference, pending, clients, roots) = tokio::try_join!(
        destinations.preference(),
        pairing.pending(),
        pairing.clients(),
        queries.list_folders(None),
    )?;
    let mut roots: Vec<_> = roots
        .into_iter()
        .map(|folder| ConsentRoot {
            id: folder.id,
            name: folder.name,
            path: folder.path.to_string_lossy().into_owned(),
        })
        .collect();
    roots.sort_unstable_by(|left, right| (&left.name, &left.id).cmp(&(&right.name, &right.id)));
    Ok(DesktopPairingStatus {
        catalog_id: preference.catalog_id,
        pending,
        clients,
        roots,
    })
}

/// Applies the user's selection of requested scopes and indexed roots.
///
/// # Errors
/// Rejects missing, stale or invalid approvals.
#[tauri::command]
pub async fn approve_external_import_pairing(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    database: State<'_, Arc<DbManager>>,
    request_id: String,
    catalog_id: String,
    scopes: Vec<ImportScope>,
    root_ids: Vec<String>,
) -> AppResult<()> {
    let (_, server_key) = service.pairing_session().await?;
    SqliteImportPairing::new(database.pool(), server_key)
        .approve(
            &request_id,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &scopes,
                root_ids: &root_ids,
            },
        )
        .await
}

/// Declines the pending request.
#[tauri::command]
pub async fn reject_external_import_pairing(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    database: State<'_, Arc<DbManager>>,
    request_id: String,
) -> AppResult<()> {
    let (_, server_key) = service.pairing_session().await?;
    SqliteImportPairing::new(database.pool(), server_key)
        .reject(&request_id)
        .await
}

/// Removes a client's access to the selected catalog.
#[tauri::command]
pub async fn revoke_external_import_client(
    _ipc: IpcRequest,
    service: State<'_, DesktopImportService>,
    database: State<'_, Arc<DbManager>>,
    catalog_id: String,
    client_id: String,
) -> AppResult<()> {
    let (_, server_key) = service.pairing_session().await?;
    SqliteImportPairing::new(database.pool(), server_key)
        .revoke(&catalog_id, &client_id)
        .await
}
