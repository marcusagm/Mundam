//! Main-window adapter for local plugin settings and the native package picker.

use crate::{
    bootstrap::AppDirectories,
    core::{
        imports::destinations::ImportDestinations,
        plugins::{
            asset_data::{
                PluginAssetDataLedger, PluginBooleanEdit, PluginInspectorEditResult,
                PluginInspectorFieldState, PluginSectionTarget,
            },
            data_management::{StoredPluginData, StoredPluginDataReview, StoredPluginOwner},
            presentation::PluginPresentation,
            InstalledPlugin, PluginPackageChange, PluginReplacementKind,
        },
        AppError, AppResult,
    },
    delivery::tauri::{
        admission::IpcRequest,
        plugin_admin::{
            fingerprint_hex, publisher_anchor, read_selected_package, DesktopPluginAdmin,
        },
    },
    infra::database::{
        import_destinations::SqliteImportDestinations, manager::DbManager,
        plugin_export::SqlitePluginAnnotationReader,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use mundam_plugin_sdk::{Permission, Target};
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    ipc::{CommandArg, CommandItem, InvokeError},
    AppHandle, Runtime, State, WebviewWindow,
};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

pub(crate) struct PluginSettingsRequest {
    _admission: IpcRequest,
}

impl<'de, R: Runtime> CommandArg<'de, R> for PluginSettingsRequest {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        if command.message.webview_ref().label() != "main" {
            return Err(
                AppError::Forbidden("Plugin settings require the main window".into()).into(),
            );
        }
        IpcRequest::from_command(command).map(|admission| Self {
            _admission: admission,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginSettings {
    safe_mode: bool,
    publishers: Vec<PublisherDetails>,
    installed: Vec<InstalledDetails>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivationDetails {
    review_id: Uuid,
    name: String,
    version: String,
    required_permissions: Vec<Permission>,
    optional_permissions: Vec<Permission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experimental_worker_id: Option<String>,
}

#[derive(Serialize)]
struct PublisherDetails {
    namespace: String,
    fingerprint: String,
    revoked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledDetails {
    id: String,
    publisher: String,
    version: String,
    name: String,
    enabled: bool,
    approved: bool,
    package_revision: String,
    rollback_version: Option<String>,
}

impl From<InstalledPlugin> for InstalledDetails {
    fn from(plugin: InstalledPlugin) -> Self {
        Self {
            id: plugin.id,
            publisher: plugin.publisher,
            version: plugin.version,
            name: plugin.name,
            enabled: plugin.enabled,
            approved: plugin.approved,
            package_revision: plugin.package_revision,
            rollback_version: plugin.rollback_version,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewDetails {
    review_id: Option<Uuid>,
    plugin_id: String,
    name: String,
    description: String,
    version: String,
    license: String,
    publisher: String,
    publisher_fingerprint: String,
    icon_base64: String,
    targets: Vec<Target>,
    required_permissions: Vec<Permission>,
    optional_permissions: Vec<Permission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experimental_worker_id: Option<String>,
    compatible: bool,
    replacement: Option<ReplacementDetails>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplacementDetails {
    kind: PluginReplacementKind,
    current_version: String,
    added_permissions: Vec<Permission>,
    removed_permissions: Vec<Permission>,
}

/// Reads local publisher and package inventory for Settings.
#[tauri::command]
pub(crate) async fn list_plugin_settings(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
) -> AppResult<PluginSettings> {
    let service = admin.service(&directories.app_data).await?;
    let catalog = active_catalog(&database).await?;
    let (publishers, installed) =
        tokio::try_join!(service.list_publishers(), service.list_installed(&catalog))?;
    Ok(PluginSettings {
        safe_mode: service.safe_mode(),
        publishers: publishers
            .into_iter()
            .map(|publisher| PublisherDetails {
                namespace: publisher.anchor.namespace,
                fingerprint: fingerprint_hex(&publisher.anchor.fingerprint),
                revoked: publisher.revoked,
            })
            .collect(),
        installed: installed.into_iter().map(InstalledDetails::from).collect(),
    })
}

async fn active_catalog(database: &DbManager) -> AppResult<String> {
    Ok(SqliteImportDestinations(database.pool())
        .preference()
        .await?
        .catalog_id)
}

/// Lists all plugin identities with retained data in the active catalog.
/// # Errors
/// Propagates admission and catalog read failures.
#[tauri::command]
pub(crate) async fn list_stored_plugin_data(
    _ipc: PluginSettingsRequest,
    ledger: State<'_, Arc<dyn PluginAssetDataLedger>>,
) -> AppResult<Vec<StoredPluginData>> {
    ledger.list_stored_plugin_data().await
}

/// Returns the impact and catalog revision for one exact signing identity.
/// # Errors
/// Propagates admission and catalog read failures.
#[tauri::command]
pub(crate) async fn review_stored_plugin_data(
    _ipc: PluginSettingsRequest,
    ledger: State<'_, Arc<dyn PluginAssetDataLedger>>,
    owner: StoredPluginOwner,
) -> AppResult<StoredPluginDataReview> {
    ledger.review_stored_plugin_data(&owner).await
}

/// Removes the reviewed rows if the data still matches the user's confirmation.
/// # Errors
/// Propagates admission and catalog write failures.
#[tauri::command]
pub(crate) async fn purge_stored_plugin_data(
    _ipc: PluginSettingsRequest,
    ledger: State<'_, Arc<dyn PluginAssetDataLedger>>,
    owner: StoredPluginOwner,
    revision: String,
) -> AppResult<bool> {
    ledger.purge_stored_plugin_data(&owner, &revision).await
}

/// Exports the active catalog's saved annotations to a user-authorized JSON file.
///
/// # Errors
/// Rejects calls outside Settings admission or filesystem scope and propagates export failures.
#[tauri::command]
pub(crate) async fn export_plugin_annotations<R: Runtime>(
    _ipc: PluginSettingsRequest,
    access: super::text_files::TextFileAccess,
    app: AppHandle<R>,
    database: State<'_, Arc<DbManager>>,
    path: String,
) -> AppResult<u64> {
    let destination = super::text_files::selected_path(&app, path, &access)?;
    crate::feature::plugins::export::annotations(
        &SqlitePluginAnnotationReader(database.pool()),
        &active_catalog(&database).await?,
        &destination,
    )
    .await
}

/// Loads the catalog's approved presentation definitions through the main-window boundary.
#[tauri::command]
pub(crate) async fn get_plugin_presentation(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
) -> AppResult<PluginPresentation> {
    admin
        .service(&directories.app_data)
        .await?
        .presentation(&active_catalog(&database).await?)
        .await
}

/// Loads values for one section rendered by the main window's Inspector.
#[tauri::command]
pub(crate) async fn get_plugin_inspector_values(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
    ledger: State<'_, Arc<dyn PluginAssetDataLedger>>,
    target: PluginSectionTarget,
) -> AppResult<Vec<PluginInspectorFieldState>> {
    admin
        .service(&directories.app_data)
        .await?
        .read_inspector_values(
            ledger.inner().as_ref(),
            &active_catalog(&database).await?,
            target,
        )
        .await
}

/// Saves one edit from the main window's authorized Inspector.
#[tauri::command]
pub(crate) async fn save_plugin_inspector_value(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
    ledger: State<'_, Arc<dyn PluginAssetDataLedger>>,
    edit: PluginBooleanEdit,
) -> AppResult<PluginInspectorEditResult> {
    admin
        .service(&directories.app_data)
        .await?
        .save_inspector_value(
            ledger.inner().as_ref(),
            &active_catalog(&database).await?,
            edit,
        )
        .await
}

/// Prepares a one-time permission review for an installed package in the active catalog.
#[tauri::command]
pub(crate) async fn review_plugin_activation(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
    plugin_id: String,
) -> AppResult<ActivationDetails> {
    let service = admin.service(&directories.app_data).await?;
    let (
        review_id,
        name,
        version,
        required_permissions,
        optional_permissions,
        experimental_worker_id,
    ) = admin
        .review_activation(service, active_catalog(&database).await?, &plugin_id)
        .await?;
    Ok(ActivationDetails {
        review_id,
        name,
        version,
        required_permissions,
        optional_permissions,
        experimental_worker_id,
    })
}

/// Consumes the reviewed grant for this catalog.
#[tauri::command]
pub(crate) async fn enable_reviewed_plugin(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
    review_id: Uuid,
    optional_permissions: Vec<Permission>,
) -> AppResult<()> {
    let service = admin.service(&directories.app_data).await?;
    admin
        .enable(
            service,
            &active_catalog(&database).await?,
            review_id,
            optional_permissions,
        )
        .await
}

/// Revokes a plugin grant for the current catalog.
#[tauri::command]
pub(crate) async fn disable_installed_plugin(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    database: State<'_, Arc<DbManager>>,
    plugin_id: String,
) -> AppResult<()> {
    let service = admin.service(&directories.app_data).await?;
    service
        .disable(&active_catalog(&database).await?, &plugin_id)
        .await
}

/// Records a publisher independently confirmed in Settings.
#[tauri::command]
pub(crate) async fn trust_plugin_publisher(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    namespace: String,
    fingerprint: String,
) -> AppResult<()> {
    let anchor = publisher_anchor(namespace, &fingerprint)?;
    admin
        .service(&directories.app_data)
        .await?
        .trust_publisher(anchor)
        .await
}

/// Revokes the exact key selected in Settings.
#[tauri::command]
pub(crate) async fn revoke_plugin_publisher(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    namespace: String,
    fingerprint: String,
) -> AppResult<()> {
    let anchor = publisher_anchor(namespace, &fingerprint)?;
    admin
        .service(&directories.app_data)
        .await?
        .revoke_publisher(anchor)
        .await
}

/// Opens a native file chooser. The renderer never submits a filesystem path.
#[tauri::command]
pub(crate) async fn choose_plugin_package(
    _ipc: PluginSettingsRequest,
    window: WebviewWindow,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
) -> AppResult<Option<ReviewDetails>> {
    let _selection = admin
        .selection
        .try_lock()
        .map_err(|_| AppError::ValidationFailed("A plugin file chooser is already open.".into()))?;
    admin.clear_review().await;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose a Mundam plugin")
        .add_filter("Mundam plugin", &["mundam-plugin"])
        .pick_file(move |selection| {
            let _ = sender.send(selection);
        });
    let Some(selection) = receiver.await.map_err(|_| AppError::Cancelled)? else {
        return Ok(None);
    };
    let path = selection
        .into_path()
        .map_err(|_| AppError::ValidationFailed("Choose a local plugin package.".into()))?;
    let bytes = read_selected_package(&path).await?;
    let service = admin.service(&directories.app_data).await?;
    let (change, review_id) = admin.review(service, bytes).await?;
    Ok(Some(review_details(change, review_id)))
}

fn review_details(change: PluginPackageChange, review_id: Option<Uuid>) -> ReviewDetails {
    let preview = change.preview;
    ReviewDetails {
        review_id,
        plugin_id: preview.plugin_id,
        name: preview.name,
        description: preview.description,
        version: preview.version,
        license: preview.license,
        publisher: preview.publisher.namespace,
        publisher_fingerprint: fingerprint_hex(&preview.publisher.fingerprint),
        icon_base64: STANDARD.encode(&preview.icon_png),
        targets: preview.targets,
        required_permissions: preview.required_permissions,
        optional_permissions: preview.optional_permissions,
        experimental_worker_id: preview.experimental_worker_id,
        compatible: preview.compatible,
        replacement: change.replacement.map(|replacement| ReplacementDetails {
            kind: replacement.kind,
            current_version: replacement.current_version,
            added_permissions: replacement.added_permissions,
            removed_permissions: replacement.removed_permissions,
        }),
    }
}

/// Offers the retained package for an explicit rollback review.
#[tauri::command]
pub(crate) async fn review_plugin_rollback(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    plugin_id: String,
) -> AppResult<ReviewDetails> {
    let _selection = admin.selection.try_lock().map_err(|_| {
        AppError::ValidationFailed("A plugin package is already being reviewed".into())
    })?;
    admin.clear_review().await;
    let service = admin.service(&directories.app_data).await?;
    let (change, id) = admin.review_rollback(service, &plugin_id).await?;
    Ok(review_details(change, id))
}

/// Removes the exact registry package confirmed in Settings across all catalogs.
#[tauri::command]
pub(crate) async fn remove_installed_plugin(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    plugin_id: String,
    package_revision: String,
) -> AppResult<()> {
    admin
        .service(&directories.app_data)
        .await?
        .remove_package(&plugin_id, &package_revision)
        .await
}

/// Consumes the one-time review and installs or replaces the verified snapshot.
#[tauri::command]
pub(crate) async fn install_reviewed_plugin(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    directories: State<'_, AppDirectories>,
    review_id: Uuid,
) -> AppResult<InstalledDetails> {
    let service = admin.service(&directories.app_data).await?;
    admin.install(service, review_id).await.map(Into::into)
}

/// Clears a cancelled review without affecting a subsequent review.
#[tauri::command]
pub(crate) async fn discard_plugin_review(
    _ipc: PluginSettingsRequest,
    admin: State<'_, DesktopPluginAdmin>,
    review_id: Uuid,
) -> AppResult<()> {
    admin.discard(review_id).await;
    Ok(())
}
