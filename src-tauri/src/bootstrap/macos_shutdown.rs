//! Bridges AppKit termination requests to the shared asynchronous shutdown coordinator.
//!
//! Tao's macOS delegate does not implement `applicationShouldTerminate:`.
//! Without that callback, Dock Quit and the native application menu terminate
//! AppKit without sending Tauri's preventable `ExitRequested` event.

use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use std::ffi::CStr;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static APPLICATION: OnceLock<AppHandle> = OnceLock::new();

const TERMINATE_CANCEL: isize = 0;
const TERMINATE_NOW: isize = 1;

extern "C-unwind" fn should_terminate(
    _delegate: &AnyObject,
    _selector: Sel,
    _application: *mut AnyObject,
) -> isize {
    let Some(application) = APPLICATION.get() else {
        tracing::error!("macOS termination handler has no application handle");
        return TERMINATE_NOW;
    };
    match application
        .state::<crate::feature::shutdown::ShutdownService>()
        .exit_authorized()
    {
        Ok(true) => return TERMINATE_NOW,
        Ok(false) => {}
        Err(error) => {
            tracing::error!(%error, "Unable to check macOS termination authorization");
        }
    }
    tracing::info!("macOS application termination requested");
    super::shutdown::request(application);
    TERMINATE_CANCEL
}

fn delegate_subclass(
    parent: &'static AnyClass,
    name: &CStr,
) -> Result<&'static AnyClass, Box<dyn std::error::Error>> {
    let selector = sel!(applicationShouldTerminate:);
    if parent.instance_method(selector).is_some() {
        return Err("macOS delegate already handles application termination".into());
    }
    let mut subclass = ClassBuilder::new(name, parent)
        .ok_or("Unable to create macOS shutdown delegate subclass")?;
    // The callback matches AppKit's NSInteger-returning delegate method.
    unsafe {
        subclass.add_method(
            selector,
            should_terminate as extern "C-unwind" fn(_, _, _) -> _,
        );
    }
    Ok(subclass.register())
}

/// Installs a delegate subclass without replacing Tao's existing delegate object.
///
/// Setup runs on the macOS main thread, after Tao creates its delegate. The
/// subclass adds no instance variables and inherits all existing callbacks.
///
/// # Errors
/// Returns an error when the existing delegate cannot be safely extended.
pub fn install(application: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if APPLICATION.set(application.clone()).is_err() {
        return Err("macOS termination handler already installed".into());
    }

    // AppKit and its delegate are accessed only on the main thread during setup.
    unsafe {
        let native_application: *mut AnyObject =
            msg_send![class!(NSApplication), sharedApplication];
        if native_application.is_null() {
            return Err("macOS application is unavailable".into());
        }
        let delegate: *mut AnyObject = msg_send![native_application, delegate];
        if delegate.is_null() {
            return Err("macOS application delegate is unavailable".into());
        }
        let parent_pointer = objc2::ffi::object_getClass(delegate);
        if parent_pointer.is_null() {
            return Err("macOS application delegate has no class".into());
        }
        let parent = &*parent_pointer;
        let name = CStr::from_bytes_with_nul(b"MundamShutdownAppDelegate\0")?;
        let subclass = delegate_subclass(parent, name)?;
        // The subclass adds no instance variables and retains Tao's entire implementation.
        objc2::ffi::object_setClass(delegate, subclass);
    }

    tracing::info!("Installed macOS application termination handler");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subclass_preserves_parent_and_refuses_to_override_termination(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let parent = class!(NSObject);
        let name = CStr::from_bytes_with_nul(b"MundamShutdownTestDelegate\0")?;
        let subclass = delegate_subclass(parent, name)?;
        assert_eq!(subclass.superclass(), Some(parent));
        assert!(subclass
            .instance_method(sel!(applicationShouldTerminate:))
            .is_some());
        assert!(subclass.instance_method(sel!(description)).is_some());
        let duplicate_name = CStr::from_bytes_with_nul(b"MundamShutdownTestDuplicate\0")?;
        assert!(delegate_subclass(subclass, duplicate_name).is_err());
        Ok(())
    }
}
