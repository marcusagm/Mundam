//! Temporary LPAC identity and ACLs on disposable fixture files.

use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        DACL_SECURITY_INFORMATION, FreeSid,
        Isolation::{CreateAppContainerProfile, DeleteAppContainerProfile},
        PROTECTED_DACL_SECURITY_INFORMATION, PSID, SetFileSecurityW,
    },
};

pub(super) fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

pub(super) fn checked(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(super) struct Profile {
    name: Vec<u16>,
    pub sid: PSID,
    cleaned: bool,
}

impl Profile {
    pub fn create(name: &OsStr) -> io::Result<Self> {
        let name = wide(name);
        let description = wide("Disposable Mundam confinement test");
        let mut sid = ptr::null_mut();
        // Inputs are NUL-terminated and the returned SID belongs to this profile.
        let result = unsafe {
            CreateAppContainerProfile(
                name.as_ptr(),
                name.as_ptr(),
                description.as_ptr(),
                ptr::null(),
                0,
                &mut sid,
            )
        };
        if result < 0 {
            return Err(io::Error::other(format!(
                "CreateAppContainerProfile HRESULT {result:#010x}"
            )));
        }
        Ok(Self {
            name,
            sid,
            cleaned: false,
        })
    }

    pub fn clean(mut self) -> io::Result<()> {
        // The launcher waits for all its processes before the fixture calls this.
        let result = unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
        if result < 0 {
            return Err(io::Error::other(format!(
                "DeleteAppContainerProfile HRESULT {result:#010x}"
            )));
        }
        self.cleaned = true;
        Ok(())
    }

    pub fn allow_read_execute(&self, path: &Path) -> io::Result<()> {
        let mut sid_string = ptr::null_mut();
        // Windows allocates an owned NUL-terminated SID string.
        checked(unsafe { ConvertSidToStringSidW(self.sid, &mut sid_string) })?;
        let mut length = 0;
        // The successful conversion guarantees a readable string through its NUL.
        unsafe {
            while *sid_string.add(length) != 0 {
                length += 1;
            }
        }
        let package_sid =
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(sid_string, length) });
        unsafe {
            LocalFree(sid_string.cast());
        }

        grant_fixture_read(path, &package_sid)
    }
}

/// Gives only the selected identity read and execute rights on one disposable path.
pub(super) fn grant_fixture_read(path: &Path, recipient: &str) -> io::Result<()> {
    let acl = wide(format!(
        "D:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;FA;;;OW)(A;;GRGX;;;{recipient})"
    ));
    let mut descriptor = ptr::null_mut();
    checked(unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            acl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    })?;
    let path = wide(path);
    // The DACL is copied to the fixture path before its buffer is released.
    let result = checked(unsafe {
        SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    });
    unsafe {
        LocalFree(descriptor);
    }
    result
}

impl Drop for Profile {
    fn drop(&mut self) {
        if !self.cleaned {
            // Best effort on an earlier test failure, scoped to the unique name we created.
            unsafe {
                DeleteAppContainerProfile(self.name.as_ptr());
            }
        }
        unsafe {
            FreeSid(self.sid);
        }
    }
}
