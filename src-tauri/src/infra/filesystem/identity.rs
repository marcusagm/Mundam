//! Native object identity shared by journal recovery, path resolution and watcher correlation.
use crate::core::{error::AppResult, library::FileIdentity};
use std::{io, path::Path};

/// Reads identity without blocking Tokio or following a symbolic-link leaf.
///
/// # Errors
/// Returns IO errors for missing paths, unsupported entries or failed native calls.
pub async fn read(path: &Path) -> AppResult<FileIdentity> {
    let path = path.to_path_buf();
    Ok(tokio::task::spawn_blocking(move || read_sync(&path))
        .await
        .map_err(|error| io::Error::other(error.to_string()))??)
}

/// Reads identity inside an existing blocking filesystem task.
///
/// # Errors
/// Returns the native filesystem error; symbolic-link leaves are rejected.
pub fn read_sync(path: &Path) -> io::Result<FileIdentity> {
    let metadata = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other(
                "Reparse points cannot identify managed entries",
            ));
        }
    }
    if metadata.file_type().is_symlink() {
        return Err(io::Error::other(
            "Symbolic links do not identify managed entries",
        ));
    }
    native_identity(path)
}

#[cfg(unix)]
fn native_identity(path: &Path) -> io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        object: metadata.ino(),
        birth: metadata
            .created()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_nanos()).ok()),
    })
}

#[cfg(windows)]
fn native_identity(path: &Path) -> io::Result<FileIdentity> {
    use std::{
        fs::OpenOptions,
        os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    };
    #[repr(C)]
    #[derive(Default)]
    struct Information {
        attributes: u32,
        creation: [u32; 2],
        access: [u32; 2],
        write: [u32; 2],
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(
            handle: *mut std::ffi::c_void,
            information: *mut Information,
        ) -> i32;
    }
    let file = OpenOptions::new()
        .access_mode(0x80)
        .share_mode(7)
        .custom_flags(0x02000000 | 0x00200000)
        .open(path)?;
    let mut information = Information::default();
    // The owned handle and initialized output buffer remain valid throughout the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(FileIdentity {
        device: information.volume as u64,
        object: ((information.index_high as u64) << 32) | information.index_low as u64,
        birth: Some(((information.creation[1] as u64) << 32) | information.creation[0] as u64),
    })
}

/// Proves one directory entry across spelling aliases, while distinguishing hardlinks.
///
/// # Errors
/// Propagates native errors except a missing candidate, which is not the same object.
pub async fn same(source: &Path, candidate: &Path) -> AppResult<bool> {
    let candidate_identity = match read(candidate).await {
        Ok(identity) => identity,
        Err(crate::core::error::AppError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(false)
        }
        Err(error) => return Err(error),
    };
    if read(source).await? != candidate_identity {
        return Ok(false);
    }
    if source == candidate {
        return Ok(true);
    }
    let (Some(source_parent), Some(candidate_parent)) = (source.parent(), candidate.parent())
    else {
        return Ok(false);
    };
    if read(source_parent).await? != read(candidate_parent).await? {
        return Ok(false);
    }
    if source.file_name() == candidate.file_name() {
        return Ok(true);
    }
    // Separate hardlink directory entries are distinct assets even when object IDs agree.
    let mut source_spelling_exists = false;
    let mut candidate_spelling_exists = false;
    let mut entries = tokio::fs::read_dir(source_parent).await?;
    while let Some(entry) = entries.next_entry().await? {
        source_spelling_exists |= Some(entry.file_name().as_os_str()) == source.file_name();
        candidate_spelling_exists |= Some(entry.file_name().as_os_str()) == candidate.file_name();
    }
    Ok(!(source_spelling_exists && candidate_spelling_exists))
}
