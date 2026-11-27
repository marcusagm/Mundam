//! Starts a suspended test process with LPAC and explicit standard handles.

use super::identity::{Profile, checked, wide};
use std::{
    env,
    ffi::{OsStr, OsString, c_void},
    fs::File,
    io::{self, Read, Write},
    mem::size_of,
    os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle},
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{HANDLE, HANDLE_FLAG_INHERIT, SetHandleInformation, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Security::SECURITY_CAPABILITIES,
    System::{
        Pipes::CreatePipe,
        Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW,
            TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
        },
        WindowsProgramming::PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT,
    },
};

struct Attributes {
    _buffer: Vec<u128>,
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
}

impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0;
        // The null call asks Windows for the variable-length storage requirement.
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 3, 0, &mut bytes);
        }
        if bytes == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut buffer = vec![0u128; bytes.div_ceil(size_of::<u128>())];
        let list = buffer.as_mut_ptr().cast();
        checked(unsafe { InitializeProcThreadAttributeList(list, 3, 0, &mut bytes) })?;
        Ok(Self {
            _buffer: buffer,
            list,
        })
    }

    fn add<T>(&mut self, attribute: u32, value: &T) -> io::Result<()> {
        // The caller owns each input through CreateProcessW and destruction of the list.
        checked(unsafe {
            UpdateProcThreadAttribute(
                self.list,
                0,
                attribute as usize,
                ptr::from_ref(value).cast::<c_void>(),
                size_of::<T>(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.list);
        }
    }
}

fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    // Default pipe handles are non-inheritable. Only child ends are opted in later.
    checked(unsafe { CreatePipe(&mut read, &mut write, ptr::null(), 4096) })?;
    Ok(unsafe {
        (
            OwnedHandle::from_raw_handle(read),
            OwnedHandle::from_raw_handle(write),
        )
    })
}

fn command_line(executable: &Path, arguments: &[&OsStr]) -> io::Result<Vec<u16>> {
    let mut command = OsString::new();
    for argument in std::iter::once(executable.as_os_str()).chain(arguments.iter().copied()) {
        // Fixtures use paths to files and numeric values, with room for spaces in paths.
        if argument.to_string_lossy().contains(['\0', '"'])
            || argument.to_string_lossy().ends_with('\\')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid fixture argument",
            ));
        }
        if !command.is_empty() {
            command.push(" ");
        }
        command.push("\"");
        command.push(argument);
        command.push("\"");
    }
    Ok(wide(command))
}

fn environment_block() -> io::Result<Vec<u16>> {
    let mut block = Vec::new();
    // Userenv needs the local app-data root to construct the AppContainer environment.
    // Permit only Windows system/profile paths. No search path or application variables.
    for name in ["LOCALAPPDATA", "SystemRoot"] {
        let value = env::var_os(name)
            .filter(|value| Path::new(value).is_absolute())
            .ok_or_else(|| io::Error::other(format!("missing Windows environment path {name}")))?;
        let mut entry = OsString::from(name);
        entry.push("=");
        entry.push(value);
        block.extend(wide(entry));
    }
    block.push(0);
    Ok(block)
}

#[derive(PartialEq, Eq)]
pub(super) enum Container {
    Regular,
    Limited,
}

pub(super) struct FixtureExit {
    pub status: u32,
    pub output: Vec<u8>,
}

impl FixtureExit {
    pub fn success(self) -> io::Result<Vec<u8>> {
        if self.status != 0 {
            return Err(io::Error::other(format!(
                "container fixture failed (exit {:#010x}): {}",
                self.status,
                String::from_utf8_lossy(&self.output),
            )));
        }
        Ok(self.output)
    }
}

pub(super) fn run(
    profile: &Profile,
    container: Container,
    executable: &Path,
    directory: &Path,
    arguments: &[&OsStr],
    request: &[u8],
) -> io::Result<FixtureExit> {
    if request.len() > 1024 || !executable.is_absolute() || !directory.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid fixture request",
        ));
    }
    let (child_input, host_input) = pipe()?;
    let (host_output, child_output) = pipe()?;
    let inherited: [HANDLE; 2] = [child_input.as_raw_handle(), child_output.as_raw_handle()];
    for handle in inherited {
        checked(unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) })?;
    }

    let capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: profile.sid,
        ..Default::default()
    };
    let policy = PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
    let mut attributes = Attributes::new()?;
    attributes.add(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, &capabilities)?;
    if container == Container::Limited {
        attributes.add(
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
            &policy,
        )?;
    }
    attributes.add(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &inherited)?;
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = inherited[0];
    startup.StartupInfo.hStdOutput = inherited[1];
    startup.StartupInfo.hStdError = inherited[1];
    startup.lpAttributeList = attributes.list;
    let mut process = PROCESS_INFORMATION::default();
    let mut command = command_line(executable, arguments)?;
    let mut environment = environment_block()?;
    // The process starts suspended. All attributes, handles and strings remain valid for this call.
    checked(unsafe {
        CreateProcessW(
            wide(executable).as_ptr(),
            command.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            CREATE_SUSPENDED
                | CREATE_NO_WINDOW
                | CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_mut_ptr().cast(),
            wide(directory).as_ptr(),
            &startup.StartupInfo,
            &mut process,
        )
    })
    .map_err(|error| io::Error::other(format!("CreateProcessW: {error}")))?;
    // A successful creation transfers these two handles to the launcher.
    let (process, thread) = unsafe {
        (
            OwnedHandle::from_raw_handle(process.hProcess),
            OwnedHandle::from_raw_handle(process.hThread),
        )
    };
    drop(child_input);
    drop(child_output);
    let job = match mundam_runtime_probe::job::attach_handle(process.as_handle()) {
        Ok(job) => job,
        Err(error) => {
            terminate(&process);
            return Err(error);
        }
    };

    let result = exchange(&process, &thread, host_input, host_output, request);
    if result.is_err() {
        terminate(&process);
    }
    drop(job);
    result
}

fn exchange(
    process: &OwnedHandle,
    thread: &OwnedHandle,
    host_input: OwnedHandle,
    host_output: OwnedHandle,
    request: &[u8],
) -> io::Result<FixtureExit> {
    if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
        return Err(io::Error::last_os_error());
    }
    // Only a bounded fixture frame is written into the 4096-byte OS pipe.
    File::from(host_input).write_all(request)?;
    match unsafe { WaitForSingleObject(process.as_raw_handle(), 2000) } {
        WAIT_OBJECT_0 => {}
        WAIT_TIMEOUT => {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "LPAC fixture deadline",
            ));
        }
        _ => return Err(io::Error::last_os_error()),
    }
    let mut exit = 0;
    checked(unsafe { GetExitCodeProcess(process.as_raw_handle(), &mut exit) })?;
    let mut reply = Vec::new();
    File::from(host_output).take(1025).read_to_end(&mut reply)?;
    if reply.len() > 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversized fixture reply",
        ));
    }
    Ok(FixtureExit {
        status: exit,
        output: reply,
    })
}

fn terminate(process: &OwnedHandle) {
    // The handle is owned by this invocation and refers only to its disposable process.
    unsafe {
        TerminateProcess(process.as_raw_handle(), 1);
        WaitForSingleObject(process.as_raw_handle(), 2000);
    }
}
