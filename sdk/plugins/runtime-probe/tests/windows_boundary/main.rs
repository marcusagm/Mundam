//! Opt-in Windows boundary qualification using only synthetic resources.

#[cfg(windows)]
mod fixture;
#[cfg(windows)]
mod identity;
#[cfg(windows)]
mod launch;

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows LPAC fixture unavailable on this target");
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args_os()
        .nth(1)
        .as_deref()
        .and_then(std::ffi::OsStr::to_str)
    {
        Some("--fixture-child") => fixture::verify_child()?,
        Some("--fixture-acl-control") => fixture::verify_acl_control()?,
        Some("--fixture-noop") => {}
        _ => fixture::verify_boundary()?,
    }
    Ok(())
}
