//! Windows resource containment for the helper process.

use std::{
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use tokio::process::Child;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};

const JOB_MEMORY_LIMIT: usize = 64 * 1024 * 1024;

pub(super) fn attach(process: &Child) -> io::Result<OwnedHandle> {
    let process = process
        .raw_handle()
        .ok_or_else(|| io::Error::other("missing process handle"))?;
    // The Child owns the process handle throughout this borrow.
    attach_handle(unsafe { BorrowedHandle::borrow_raw(process) })
}

/// Attaches an existing process handle to the probe's containment job.
///
/// # Errors
/// Propagates job creation, configuration and assignment errors from Windows.
pub fn attach_handle(process: BorrowedHandle<'_>) -> io::Result<OwnedHandle> {
    // An unnamed job receives no inheritable handle or security attributes.
    let raw_job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if raw_job.is_null() {
        return Err(io::Error::last_os_error());
    }
    // CreateJobObjectW supplied a new handle closed once when OwnedHandle drops.
    let job = unsafe { OwnedHandle::from_raw_handle(raw_job) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_JOB_MEMORY
        | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    limits.BasicLimitInformation.ActiveProcessLimit = 1;
    limits.JobMemoryLimit = JOB_MEMORY_LIMIT;
    // The information class requires the complete layout and byte size of this structure.
    let configured = unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if configured == 0 {
        return Err(io::Error::last_os_error());
    }
    // Both handles remain live during assignment. The child still waits for input.
    if unsafe { AssignProcessToJobObject(job.as_raw_handle(), process.as_raw_handle()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}
