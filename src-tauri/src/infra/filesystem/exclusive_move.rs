//! Platform moves with atomic destination-exists protection.
use std::{io, path::Path};

/// Moves a file or directory without replacing anything at the destination.
///
/// # Errors
/// Returns the native error for collisions, permission errors or cross-volume moves.
pub async fn move_path(source: &Path, destination: &Path) -> io::Result<()> {
    let source = source.to_path_buf();
    let destination = destination.to_path_buf();
    tokio::task::spawn_blocking(move || {
        rename_exclusive(&source, &destination)?;
        #[cfg(unix)]
        for parent in [source.parent(), destination.parent()]
            .into_iter()
            .flatten()
        {
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    })
    .await
    .map_err(io::Error::other)?
}

#[cfg(windows)]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // WRITE_THROUGH requests durable completion; REPLACE_EXISTING is deliberately absent.
    // Both terminated buffers remain alive throughout the native call.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x8) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    // C strings remain alive; exclusive flags make the existence check atomic.
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn rename_exclusive(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Exclusive moves are unavailable on this platform",
    ))
}
