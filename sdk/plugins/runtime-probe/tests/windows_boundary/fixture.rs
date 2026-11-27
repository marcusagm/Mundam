//! Verifies LPAC policy with temporary files, loopback and one dummy event.

use super::{
    identity::{Profile, checked, grant_fixture_read},
    launch::{self, Container},
};
use std::{
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    mem::size_of,
    net::{SocketAddr, TcpListener, TcpStream},
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    path::Path,
    process::Command,
    ptr,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{GetHandleInformation, STATUS_INVALID_HANDLE, WAIT_TIMEOUT},
    Networking::WinSock::{WSACleanup, WSADATA, WSAEACCES, WSASYSCALLFAILURE, WSAStartup},
    Security::{
        GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_INFORMATION_CLASS, TOKEN_QUERY,
        TokenCapabilities, TokenIsAppContainer,
    },
    System::Threading::{
        CREATE_NO_WINDOW, CreateEventW, GetCurrentProcess, OpenProcessToken, SetEvent,
        WaitForSingleObject,
    },
};

const SENTINEL_NAME: &str = "private-sentinel.txt";
const PACKAGE_SENTINEL_NAME: &str = "all-app-packages-sentinel.txt";
const SENTINEL_BYTES: &[u8] = b"synthetic boundary test marker";
type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn verify_boundary() -> TestResult {
    let scope = tempfile::Builder::new().prefix("mundam-lpac-").tempdir()?;
    let root = scope.path();
    let profile = Profile::create(root.file_name().ok_or("missing fixture name")?)?;
    let accessible = root.join("accessible");
    fs::create_dir(&accessible)?;
    let native_check = accessible.join("native-fixture.exe");
    let guest_helper = accessible.join("wasm-helper.exe");
    fs::copy(env::current_exe()?, &native_check)?;
    fs::copy(env!("CARGO_BIN_EXE_mundam-runtime-probe"), &guest_helper)?;
    for path in [&accessible, &native_check, &guest_helper] {
        profile.allow_read_execute(path)?;
    }
    let package_sentinel = accessible.join(PACKAGE_SENTINEL_NAME);
    fs::write(&package_sentinel, SENTINEL_BYTES)?;
    grant_fixture_read(&package_sentinel, "S-1-15-2-1")?;
    assert_eq!(
        launch::run(
            &profile,
            Container::Regular,
            &native_check,
            &accessible,
            &[
                OsStr::new("--fixture-acl-control"),
                package_sentinel.as_os_str()
            ],
            &[]
        )
        .and_then(launch::FixtureExit::success)
        .map_err(|error| format!("Regular AppContainer control: {error}"))?,
        b"ACL_CONTROL_OK\n"
    );

    let sentinel = root.join(SENTINEL_NAME);
    fs::write(&sentinel, SENTINEL_BYTES)?;
    assert_eq!(fs::read(&sentinel)?, SENTINEL_BYTES);
    OpenOptions::new().write(true).open(&sentinel)?;

    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    let positive_control = TcpStream::connect_timeout(&address, Duration::from_secs(1))?;
    let (connection, _) = listener.accept()?;
    drop(positive_control);
    drop(connection);
    let address = address.to_string();

    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        bInheritHandle: 1,
        ..Default::default()
    };
    // The deliberately inheritable event is unrelated to the helper's pipe list.
    let event = unsafe { CreateEventW(&attributes, 1, 0, ptr::null()) };
    if event.is_null() {
        return Err(io::Error::last_os_error().into());
    }
    let event = unsafe { OwnedHandle::from_raw_handle(event) };
    let event_id = (event.as_raw_handle() as usize).to_string();

    let response = launch::run(
        &profile,
        Container::Limited,
        &native_check,
        &accessible,
        &[
            OsStr::new("--fixture-child"),
            sentinel.as_os_str(),
            package_sentinel.as_os_str(),
            OsStr::new(&address),
            OsStr::new(&event_id),
        ],
        &[],
    )
    .map_err(|error| format!("LPAC native fixture: {error}"))?;
    assert!(
        response.status == 0 || response.status == STATUS_INVALID_HANDLE as u32,
        "LPAC native fixture exit {:#010x}: {}",
        response.status,
        String::from_utf8_lossy(&response.output)
    );
    let network = response
        .output
        .strip_prefix(b"LPAC_HANDLE_CHECK ")
        .ok_or_else(|| {
            format!(
                "missing handle checkpoint: {}",
                String::from_utf8_lossy(&response.output)
            )
        })?;
    assert!(matches!(
        network,
        b"connect-denied\n" | b"winsock-denied\n" | b"winsock-unavailable\n"
    ));
    assert_eq!(
        unsafe { WaitForSingleObject(event.as_raw_handle(), 0) },
        WAIT_TIMEOUT
    );
    assert_eq!(fs::read(&sentinel)?, SENTINEL_BYTES);
    checked(unsafe { SetEvent(event.as_raw_handle()) })?;
    eprintln!(
        "Verified zero capabilities, LPAC opt-out by ACL comparison, private file and loopback denial, child process denial and explicit handle inheritance"
    );
    eprintln!(
        "Network observation: {}",
        String::from_utf8_lossy(network).trim()
    );
    eprintln!("Handle observation: process exit {:#010x}", response.status);

    let module = wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 42))"#)?;
    let mut frame = (module.len() as u32).to_le_bytes().to_vec();
    frame.extend_from_slice(&module);
    assert_eq!(
        launch::run(
            &profile,
            Container::Limited,
            &guest_helper,
            &accessible,
            &[],
            &frame
        )?
        .success()?,
        [0, 42, 0, 0, 0]
    );
    eprintln!("Wasm helper executed through the same LPAC launcher");
    profile.clean()?;
    scope.close()?;
    Ok(())
}

pub(super) fn verify_child() -> TestResult {
    let mut args = env::args_os().skip(2);
    let sentinel = args.next().ok_or("missing sentinel path")?;
    let sentinel = Path::new(&sentinel);
    let package_sentinel = args.next().ok_or("missing AppContainer sentinel path")?;
    let package_sentinel = Path::new(&package_sentinel);
    let address: SocketAddr = args
        .next()
        .ok_or("missing loopback address")?
        .to_string_lossy()
        .parse()?;
    let event: usize = args
        .next()
        .ok_or("missing event ID")?
        .to_string_lossy()
        .parse()?;
    if sentinel.file_name() != Some(OsStr::new(SENTINEL_NAME))
        || package_sentinel.file_name() != Some(OsStr::new(PACKAGE_SENTINEL_NAME))
        || !address.ip().is_loopback()
    {
        return Err("invalid fixture resources".into());
    }
    verify_container_token()?;
    File::open(env::current_exe()?)?;
    check_denied(File::open(sentinel), "private read was allowed")?;
    check_denied(
        OpenOptions::new().write(true).open(sentinel),
        "private write was allowed",
    )?;
    check_denied(
        File::open(package_sentinel),
        "All Application Packages ACL was allowed",
    )?;
    let network = verify_loopback_denied(address)?;
    if let Ok(mut child) = Command::new(env::current_exe()?)
        .arg("--fixture-noop")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err("an LPAC fixture created a child process".into());
    }
    // Strict handle checks may terminate the process on an invalid handle. This
    // checkpoint proves the file, network and child checks succeeded beforehand.
    println!("LPAC_HANDLE_CHECK {network}");
    io::stdout().flush()?;
    let handle = event as *mut std::ffi::c_void;
    let mut flags = 0;
    // Query the numeric slot before calling any operation on it. A valid slot could
    // belong to another object. The parent checks its actual event independently.
    if unsafe { GetHandleInformation(handle, &mut flags) } != 0 {
        unsafe {
            SetEvent(handle);
        }
    }
    Ok(())
}

fn verify_loopback_denied(address: SocketAddr) -> Result<&'static str, Box<dyn std::error::Error>> {
    let mut data = WSADATA::default();
    // Inspect native initialization before using std, which assumes WSAStartup succeeds.
    match unsafe { WSAStartup(0x0202, &mut data) } {
        0 => {
            let connection = TcpStream::connect_timeout(&address, Duration::from_millis(400));
            // Balance the successful native initialization. std manages its own initialization.
            unsafe {
                WSACleanup();
            }
            check_denied(connection, "loopback was allowed")?;
            Ok("connect-denied")
        }
        WSAEACCES => Ok("winsock-denied"),
        WSASYSCALLFAILURE => Ok("winsock-unavailable"),
        error => Err(io::Error::from_raw_os_error(error).into()),
    }
}

pub(super) fn verify_acl_control() -> TestResult {
    let path = env::args_os().nth(2).ok_or("missing ACL control path")?;
    let path = Path::new(&path);
    if path.file_name() != Some(OsStr::new(PACKAGE_SENTINEL_NAME)) {
        return Err("invalid ACL control path".into());
    }
    verify_container_token()?;
    File::open(path)?;
    println!("ACL_CONTROL_OK");
    Ok(())
}

fn check_denied<T>(result: io::Result<T>, unexpected_success: &'static str) -> TestResult {
    assert_eq!(
        result.err().ok_or(unexpected_success)?.kind(),
        io::ErrorKind::PermissionDenied,
        "{unexpected_success}"
    );
    Ok(())
}

fn verify_container_token() -> TestResult {
    let mut token = ptr::null_mut();
    // The pseudo handle is valid for the current process, and OpenProcessToken returns an owned handle.
    checked(unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) })?;
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    assert_eq!(token_word(&token, TokenIsAppContainer)?, 1);
    assert_eq!(token_word(&token, TokenCapabilities)?, 0);
    Ok(())
}

fn token_word(token: &OwnedHandle, kind: TOKEN_INFORMATION_CLASS) -> io::Result<u32> {
    let mut bytes = 0;
    unsafe {
        GetTokenInformation(token.as_raw_handle(), kind, ptr::null_mut(), 0, &mut bytes);
    }
    if bytes < size_of::<u32>() as u32 {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
    checked(unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            kind,
            buffer.as_mut_ptr().cast(),
            bytes,
            &mut bytes,
        )
    })?;
    // The selected classes start with a DWORD flag or a TOKEN_GROUPS count.
    Ok(unsafe { buffer.as_ptr().cast::<u32>().read() })
}
