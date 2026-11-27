//! NTFS publication uses handle-relative opens and an exclusive rename of the opened file.
use super::ImportParents;
use crate::core::library::FileIdentity;
use crate::infra::filesystem::identity::read_windows_handle;
use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, RawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr::null_mut,
};

const READ_ATTRIBUTES: u32 = 0x80;
const LIST_DIRECTORY: u32 = 1;
const SHARE_READ: u32 = 1;
const SHARE_DIRECTORY: u32 = SHARE_READ | 2;
const SYNCHRONIZE: u32 = 0x0010_0000;
const DELETE: u32 = 0x0001_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const GENERIC_READ: u32 = 0x8000_0000;
const DIRECTORY_FILE: u32 = 1;
const WRITE_THROUGH: u32 = 2;
const NON_DIRECTORY_FILE: u32 = 0x40;
const SYNCHRONOUS_NONALERT: u32 = 0x20;
const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const ATTR_REPARSE_POINT: u32 = 0x400;

pub(super) struct Directories {
    source: DirectoryChain,
    destination: DirectoryChain,
}

pub(in crate::infra::filesystem) struct DirectoryChain {
    leaf: File,
    _ancestors: Vec<File>,
}

impl Directories {
    pub(super) fn open(source: &Path, destination: &Path) -> io::Result<Self> {
        Ok(Self {
            source: DirectoryChain::open(source)?,
            destination: DirectoryChain::open(destination)?,
        })
    }

    pub(super) fn identities(&self) -> io::Result<ImportParents> {
        Ok(ImportParents {
            source: read_windows_handle(&self.source.leaf)?,
            destination: read_windows_handle(&self.destination.leaf)?,
        })
    }

    pub(super) fn move_file(
        &self,
        source: &OsStr,
        destination: &OsStr,
        expected: &FileIdentity,
        reverse: bool,
    ) -> io::Result<()> {
        let (source_parent, destination_parent) = if reverse {
            (&self.destination.leaf, &self.source.leaf)
        } else {
            (&self.source.leaf, &self.destination.leaf)
        };
        let source = open_relative(
            source_parent,
            source,
            READ_ATTRIBUTES | GENERIC_WRITE | DELETE,
            NON_DIRECTORY_FILE | WRITE_THROUGH,
            SHARE_READ,
        )?;
        if &read_windows_handle(&source)? != expected {
            return Err(invalid(
                "Import source identity changed before the native move",
            ));
        }
        rename_relative(&source, destination_parent, destination)?;
        source.sync_all()
    }
}

impl DirectoryChain {
    pub(in crate::infra::filesystem) fn open(path: &Path) -> io::Result<Self> {
        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Err(invalid("Import path has no Windows volume"));
        };
        let drive = match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => return Err(invalid("Import path must be on a local Windows drive")),
        };
        if !matches!(components.next(), Some(Component::RootDir)) {
            return Err(invalid("Import path must be on a local Windows drive"));
        }
        let mut root = PathBuf::from(prefix.as_os_str());
        root.push(Path::new(r"\"));
        let mut leaf = open_root(&root, drive)?;
        let mut ancestors = Vec::new();
        for component in components {
            let Component::Normal(name) = component else {
                return Err(invalid(
                    "Import path contains an ambiguous directory component",
                ));
            };
            let child = open_relative(
                &leaf,
                name,
                READ_ATTRIBUTES | LIST_DIRECTORY,
                DIRECTORY_FILE,
                SHARE_DIRECTORY,
            )?;
            verify_directory(&child)?;
            ancestors.push(leaf);
            leaf = child;
        }
        Ok(Self {
            leaf,
            _ancestors: ancestors,
        })
    }

    pub(in crate::infra::filesystem) fn identity(&self) -> io::Result<FileIdentity> {
        read_windows_handle(&self.leaf)
    }

    /// Opens one read-only regular file relative to the anchored directory.
    pub(in crate::infra::filesystem) fn open_regular_read(&self, name: &OsStr) -> io::Result<File> {
        let file = open_relative(
            &self.leaf,
            name,
            GENERIC_READ,
            NON_DIRECTORY_FILE,
            SHARE_READ,
        )?;
        let metadata = file.metadata()?;
        if metadata.is_file() && metadata.file_attributes() & ATTR_REPARSE_POINT == 0 {
            Ok(file)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected a regular cache file",
            ))
        }
    }

    pub(in crate::infra::filesystem) fn create_temporary(
        &self,
        name: &OsStr,
    ) -> io::Result<(File, FileIdentity)> {
        let file = relative_file(&self.leaf, name, |output, attributes, status| {
            // Counted name, rooted attributes and status buffer are valid for the native call.
            unsafe {
                NtCreateFile(
                    output,
                    GENERIC_WRITE | READ_ATTRIBUTES | SYNCHRONIZE,
                    attributes,
                    status,
                    null_mut(),
                    0x80,
                    0,
                    2,
                    SYNCHRONOUS_NONALERT | OPEN_REPARSE_POINT | NON_DIRECTORY_FILE | WRITE_THROUGH,
                    null_mut(),
                    0,
                )
            }
        })?;
        let identity = read_windows_handle(&file)?;
        Ok((file, identity))
    }

    pub(in crate::infra::filesystem) fn cleanup_temporary(
        &self,
        name: &OsStr,
        expected: Option<&FileIdentity>,
    ) -> io::Result<()> {
        let file = match open_relative(
            &self.leaf,
            name,
            READ_ATTRIBUTES | DELETE,
            NON_DIRECTORY_FILE,
            SHARE_READ,
        ) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if expected != Some(&read_windows_handle(&file)?) {
            return Err(invalid(
                "Destination-volume temporary ownership is unproven",
            ));
        }
        let mut delete = 1u8;
        let mut status = IoStatusBlock::default();
        // The checked file and one-byte FILE_DISPOSITION_INFORMATION stay alive for the call.
        check_status(unsafe {
            NtSetInformationFile(
                file.as_raw_handle(),
                &mut status,
                (&mut delete as *mut u8).cast(),
                1,
                13,
            )
        })
    }
}

fn open_root(path: &Path, drive_letter: u8) -> io::Result<File> {
    let path_wide = [u16::from(drive_letter), u16::from(b':'), 92, 0];
    // The absolute drive root is terminated and alive for the duration of the call.
    let drive = unsafe { GetDriveTypeW(path_wide.as_ptr()) };
    if !matches!(drive, 2 | 3) {
        return Err(invalid("Import requires a local Windows volume"));
    }
    let root = OpenOptions::new()
        .access_mode(READ_ATTRIBUTES | LIST_DIRECTORY)
        .share_mode(SHARE_DIRECTORY)
        .custom_flags(0x0200_0000 | OPEN_REPARSE_POINT)
        .open(path)?;
    verify_directory(&root)?;
    let mut filesystem = [0u16; 32];
    // The handle and output array remain valid, and unused output parameters are null.
    if unsafe {
        GetVolumeInformationByHandleW(
            root.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if !filesystem
        .iter()
        .copied()
        .take(5)
        .eq("NTFS".encode_utf16().chain(Some(0)))
    {
        return Err(invalid("Import publication requires NTFS on Windows"));
    }
    Ok(root)
}

fn verify_directory(directory: &File) -> io::Result<()> {
    let metadata = directory.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & ATTR_REPARSE_POINT != 0 {
        return Err(invalid(
            "Reparse points and non-directory entries cannot anchor an import",
        ));
    }
    Ok(())
}

fn component(name: &OsStr) -> io::Result<Vec<u16>> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(value)) if value == name)
        || components.next().is_some()
    {
        return Err(invalid("Expected a literal import name"));
    }
    let name: Vec<u16> = name.encode_wide().collect();
    if name.is_empty() || name.len() > 255 || name.contains(&0) || name.contains(&u16::from(b':')) {
        return Err(invalid("Invalid NTFS import name"));
    }
    Ok(name)
}

fn open_relative(
    parent: &File,
    name: &OsStr,
    access: u32,
    options: u32,
    share: u32,
) -> io::Result<File> {
    relative_file(parent, name, |output, attributes, status| {
        // Parent, counted name and writable outputs are valid for the synchronous call.
        unsafe {
            NtOpenFile(
                output,
                access | SYNCHRONIZE,
                attributes,
                status,
                share,
                options | SYNCHRONOUS_NONALERT | OPEN_REPARSE_POINT,
            )
        }
    })
}

fn relative_file(
    parent: &File,
    name: &OsStr,
    open: impl FnOnce(*mut RawHandle, *mut ObjectAttributes, *mut IoStatusBlock) -> i32,
) -> io::Result<File> {
    let mut name = component(name)?;
    let length = (name.len() * 2) as u16;
    let mut native_name = UnicodeString {
        length,
        maximum_length: length,
        buffer: name.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root: parent.as_raw_handle(),
        name: &mut native_name,
        attributes: 0x40, // OBJ_CASE_INSENSITIVE
        security_descriptor: null_mut(),
        security_qos: null_mut(),
    };
    let mut output = null_mut();
    let mut status = IoStatusBlock::default();
    let result = open(&mut output, &mut attributes, &mut status);
    check_status(result)?;
    if output.is_null() {
        return Err(io::Error::other(
            "Native import open did not return a handle",
        ));
    }
    // A successful native open transfers ownership of the returned handle to this file.
    Ok(unsafe { File::from_raw_handle(output) })
}

fn rename_relative(source: &File, parent: &File, name: &OsStr) -> io::Result<()> {
    let name = component(name)?;
    let mut information = RenameInformation {
        replace: 0,
        root: parent.as_raw_handle(),
        length: (name.len() * 2) as u32,
        name: [0; 256],
    };
    information.name[..name.len()].copy_from_slice(&name);
    let mut status = IoStatusBlock::default();
    // The source and parent handles, aligned request and status buffer remain valid synchronously.
    let result = unsafe {
        NtSetInformationFile(
            source.as_raw_handle(),
            &mut status,
            (&mut information as *mut RenameInformation).cast(),
            size_of::<RenameInformation>() as u32,
            10,
        )
    }; // FileRenameInformation
    check_status(result)
}

fn check_status(status: i32) -> io::Result<()> {
    if status == 0 {
        return Ok(());
    }
    // This API only converts a numeric NTSTATUS to the corresponding Win32 error code.
    Err(io::Error::from_raw_os_error(
        unsafe { RtlNtStatusToDosError(status) } as i32,
    ))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: RawHandle,
    name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: RawHandle,
    security_qos: RawHandle,
}

#[repr(C)]
#[derive(Default)]
struct IoStatusBlock {
    status: usize,
    information: usize,
}

#[repr(C)]
struct RenameInformation {
    replace: u32,
    root: RawHandle,
    length: u32,
    name: [u16; 256],
}

#[link(name = "ntdll")]
extern "system" {
    fn NtCreateFile(
        file: *mut RawHandle,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatusBlock,
        allocation: *mut i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        extended_attributes: RawHandle,
        extended_length: u32,
    ) -> i32;
    fn NtOpenFile(
        file: *mut RawHandle,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatusBlock,
        share: u32,
        options: u32,
    ) -> i32;
    fn NtSetInformationFile(
        file: RawHandle,
        status: *mut IoStatusBlock,
        information: RawHandle,
        length: u32,
        class: u32,
    ) -> i32;
    fn RtlNtStatusToDosError(status: i32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetDriveTypeW(path: *const u16) -> u32;
    fn GetVolumeInformationByHandleW(
        volume: RawHandle,
        name: *mut u16,
        name_len: u32,
        serial: *mut u32,
        max_component: *mut u32,
        flags: *mut u32,
        filesystem: *mut u16,
        filesystem_len: u32,
    ) -> i32;
}
