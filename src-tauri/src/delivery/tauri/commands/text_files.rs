//! Text transfers that share shutdown admission with other application commands.

use crate::{
    core::error::{AppError, AppResult},
    delivery::tauri::admission::IpcRequest,
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_fs::FsExt;

/// Reuses the filesystem plugin's global denials along with native-dialog grants.
pub struct TextFileAccess(tauri::ipc::GlobalScope<FilesystemScopePath>);

#[derive(Debug)]
struct FilesystemScopePath {
    path: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ScopePathInput {
    Path(PathBuf),
    Object { path: PathBuf },
}

impl tauri::ipc::ScopeObject for FilesystemScopePath {
    type Error = tauri::Error;
    fn deserialize<R: Runtime>(
        app: &AppHandle<R>,
        raw: tauri::utils::acl::Value,
    ) -> Result<Self, Self::Error> {
        let path = match serde_json::from_value::<ScopePathInput>(raw.into())? {
            ScopePathInput::Path(path) | ScopePathInput::Object { path } => path,
        };
        match app.path().parse(path) {
            Ok(path) => Ok(Self { path: Some(path) }),
            Err(tauri::Error::UnknownPath) => Ok(Self { path: None }),
            Err(error) => Err(error),
        }
    }
}

impl<'de, R: Runtime> tauri::ipc::CommandArg<'de, R> for TextFileAccess {
    fn from_command(
        command: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        tauri::ipc::GlobalScope::from_command(tauri::ipc::CommandItem {
            plugin: Some("fs"),
            ..command
        })
        .map(Self)
    }
}

fn selected_path<R: Runtime>(
    app: &AppHandle<R>,
    path: String,
    access: &TextFileAccess,
) -> AppResult<PathBuf> {
    let path = PathBuf::from(path);
    let forbidden = tauri::fs::Scope::new(
        app,
        &tauri::utils::config::FsScope::Scope {
            allow: Vec::new(),
            deny: access
                .0
                .denies()
                .iter()
                .filter_map(|entry| entry.path.clone())
                .collect(),
            require_literal_leading_dot: None,
        },
    )?;
    if !path.is_absolute() || forbidden.is_forbidden(&path) || !app.fs_scope().is_allowed(&path) {
        return Err(AppError::Forbidden(
            "The text file is outside the authorized filesystem scope".into(),
        ));
    }
    Ok(path)
}

/// Imports a text file whose path was authorized by the native dialog.
/// # Errors
/// Rejects closed admission, unauthorized paths and filesystem failures.
#[tauri::command]
pub async fn read_text_file<R: Runtime>(
    _ipc: IpcRequest,
    access: TextFileAccess,
    app: AppHandle<R>,
    path: String,
) -> AppResult<String> {
    crate::feature::text_files::read(&selected_path(&app, path, &access)?).await
}

/// Exports text to a destination authorized by the native save dialog.
/// # Errors
/// Rejects closed admission, unauthorized paths and filesystem failures.
#[tauri::command]
pub async fn write_text_file<R: Runtime>(
    _ipc: IpcRequest,
    access: TextFileAccess,
    app: AppHandle<R>,
    path: String,
    contents: String,
) -> AppResult<()> {
    crate::feature::text_files::write(&selected_path(&app, path, &access)?, &contents).await
}
