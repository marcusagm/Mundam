//! Read-only access to generated thumbnails by validated literal cache names.

use crate::core::{
    plugins::preview_access::{PluginPreviewStorage, MAX_PLUGIN_PREVIEW_BYTES},
    AppError, AppResult,
};
use async_trait::async_trait;
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Reads native cache files from a host-selected directory.
pub struct NativePluginPreviews {
    directory: PathBuf,
}

impl NativePluginPreviews {
    /// Binds the adapter to the thumbnail directory selected by the host.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

#[async_trait]
impl PluginPreviewStorage for NativePluginPreviews {
    async fn read_thumbnail(&self, file_name: &str) -> AppResult<Vec<u8>> {
        if !self.directory.is_absolute() || !valid_cache_name(file_name) {
            return Err(AppError::ValidationFailed(
                "Invalid plugin preview cache entry".into(),
            ));
        }
        let directory = self.directory.clone();
        let file_name = file_name.to_owned();
        tokio::task::spawn_blocking(move || {
            let file = open_cache_entry(&directory, OsStr::new(&file_name))?;
            read_bounded(file)
        })
        .await
        .map_err(|_| AppError::Internal("Plugin preview read task failed".into()))?
    }
}

fn valid_cache_name(file_name: &str) -> bool {
    let Some((asset, publication)) = file_name
        .strip_suffix(".webp")
        .and_then(|stem| stem.split_once('.'))
    else {
        return false;
    };
    !asset.is_empty()
        && asset.len() <= 128
        && asset
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        && uuid::Uuid::parse_str(publication)
            .is_ok_and(|value| value.hyphenated().to_string() == publication)
}

fn read_bounded(file: File) -> AppResult<Vec<u8>> {
    let maximum = MAX_PLUGIN_PREVIEW_BYTES as u64;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(AppError::InvalidProcessingOutput);
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PLUGIN_PREVIEW_BYTES {
        return Err(AppError::InvalidProcessingOutput);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn open_cache_entry(directory: &Path, file_name: &OsStr) -> io::Result<File> {
    super::import_move::windows::DirectoryChain::open(directory)?.open_regular_read(file_name)
}

#[cfg(unix)]
fn open_cache_entry(directory: &Path, file_name: &OsStr) -> io::Result<File> {
    use std::{
        ffi::CString,
        fs::OpenOptions,
        os::fd::{AsRawFd, FromRawFd},
        os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    let root = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(directory)?;
    let name = CString::new(file_name.as_bytes())?;
    // The live directory descriptor and NUL-terminated literal name stay valid through openat.
    // O_NONBLOCK prevents opening an unexpected FIFO from delaying verification.
    let descriptor = unsafe {
        libc::openat(
            root.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    // A successful openat transfers ownership of a new descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(not(any(unix, windows)))]
fn open_cache_entry(_directory: &Path, _file_name: &OsStr) -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Plugin previews require a native cache reader",
    ))
}
