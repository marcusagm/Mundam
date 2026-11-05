//! Bounded executable discovery and capability probes, without asset paths or private output.

use crate::core::{
    processing::tools::{ExtractionTool, ExtractionToolProbe},
    AppResult,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::Manager;
use wait_timeout::ChildExt;

type LookupKey = (
    String,
    Option<PathBuf>,
    Option<std::ffi::OsString>,
    Option<std::ffi::OsString>,
);
type Cache = HashMap<LookupKey, (Instant, Option<PathBuf>)>;
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
const CACHE_TTL: Duration = Duration::from_secs(60);

fn key<R: tauri::Runtime>(handle: Option<&tauri::AppHandle<R>>, name: &str) -> LookupKey {
    (
        name.into(),
        handle.and_then(|handle| handle.path().resource_dir().ok()),
        std::env::var_os("MUNDAM_RESOURCE_DIR"),
        std::env::var_os("PATH"),
    )
}

/// Resolves cached discovery without running an unbounded PATH subprocess.
pub(super) fn resolve<R: tauri::Runtime>(
    handle: Option<&tauri::AppHandle<R>>,
    name: &str,
) -> Option<PathBuf> {
    let key = key(handle, name);
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some((checked, value)) = cache.get(&key) {
        if checked.elapsed() < CACHE_TTL {
            return value.clone();
        }
    }
    let value = locate(handle, name);
    if cache.len() >= 32 {
        cache.clear();
    }
    cache.insert(key, (Instant::now(), value.clone()));
    value
}

/// A failed launch invalidates discovery across webview/resource lookup contexts.
pub(super) fn unavailable(name: &str) {
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for (key, value) in cache.iter_mut() {
        if key.0 == name {
            *value = (Instant::now(), None);
        }
    }
}

/// Production adapter; each version process is timed, hidden and reaped.
pub struct BinaryToolProbe;
#[async_trait::async_trait]
impl ExtractionToolProbe for BinaryToolProbe {
    async fn available(&self, tool: ExtractionTool) -> AppResult<bool> {
        tokio::task::spawn_blocking(move || {
            let name = tool.as_str();
            {
                let mut cache = CACHE
                    .get_or_init(Default::default)
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                cache.retain(|key, _| key.0 != name);
            }
            let found = locate::<tauri::Wry>(None, name);
            let available = found
                .as_deref()
                .is_some_and(|path| probe_version(path, tool));
            let mut cache = CACHE
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            cache.insert(
                key::<tauri::Wry>(None, name),
                (Instant::now(), if available { found } else { None }),
            );
            available
        })
        .await
        .map_err(|error| {
            crate::core::AppError::Internal(format!("Tool probe task failed: {error}"))
        })
    }
}

fn probe_version(path: &std::path::Path, tool: ExtractionTool) -> bool {
    let mut command = Command::new(path);
    command
        .arg(if tool == ExtractionTool::Assimp {
            "version"
        } else {
            "-version"
        })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    match child.wait_timeout(Duration::from_secs(2)) {
        Ok(Some(status)) => status.success(),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    }
}

/// Finds a bundled or PATH executable without launching it.
fn locate<R: tauri::Runtime>(
    app_handle: Option<&tauri::AppHandle<R>>,
    name: &str,
) -> Option<PathBuf> {
    let binary_name = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    let platform_dir = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows-x64" // Bundled Windows target.
    } else {
        "linux"
    };

    // Prefer the resource directory established by bootstrap.
    let mut resource_dirs = Vec::new();
    if let Some(handle) = app_handle {
        if let Ok(resource_dir) = handle.path().resource_dir() {
            resource_dirs.push(resource_dir);
        }
    }
    if let Some(resource_dir) = std::env::var_os("MUNDAM_RESOURCE_DIR") {
        resource_dirs.push(PathBuf::from(resource_dir));
    }

    for resource_dir in resource_dirs {
        let resource_dir = resource_dir.join("resources");
        let direct_path = resource_dir.join(name).join(&binary_name);
        if direct_path.is_file() {
            return Some(direct_path);
        }

        let platform_path = resource_dir
            .join(name)
            .join(platform_dir)
            .join(&binary_name);
        if platform_path.is_file() {
            return Some(platform_path);
        }

        let bin_path = resource_dir
            .join(name)
            .join(platform_dir)
            .join("bin")
            .join(&binary_name);
        if bin_path.is_file() {
            return Some(bin_path);
        }
    }

    // Development builds also support project resources.
    #[cfg(debug_assertions)]
    {
        let resource_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        let direct_path = resource_dir.join(name).join(&binary_name);
        if direct_path.is_file() {
            return Some(direct_path);
        }

        let platform_path = resource_dir
            .join(name)
            .join(platform_dir)
            .join(&binary_name);
        if platform_path.is_file() {
            return Some(platform_path);
        }

        let bin_path = resource_dir
            .join(name)
            .join(platform_dir)
            .join("bin")
            .join(&binary_name);
        if bin_path.is_file() {
            return Some(bin_path);
        }
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            let candidate = directory.join(&binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
