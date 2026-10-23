//! The OS primitives the engine deletes with. Nothing here follows a link or
//! opens a file's content: a link, FIFO, socket, device, or reparse point is
//! removed as the entry it is.

use sniff::filesystem::query::FileIdentity;

pub(super) use imp::*;

/// The filesystem part of an identity, which differs across a mount.
pub(super) fn volume(identity: &FileIdentity) -> u64 {
    match identity {
        FileIdentity::Unix { device, .. } => *device,
        FileIdentity::Windows { volume_serial, .. } => *volume_serial,
    }
}

#[cfg(unix)]
mod imp {
    use std::ffi::{CString, OsStr};
    use std::io;
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use cap_std::fs::{Dir, Metadata, MetadataExt};
    use sniff::filesystem::query::FileIdentity;

    /// A permission denial is first answered by making the directory
    /// writable: on Unix only the directory's mode decides whether an entry
    /// in it may be unlinked.
    pub(in crate::remove::delete) const FIX_DENIAL_BY_MODE: bool = true;

    pub(in crate::remove::delete) fn identity(metadata: &Metadata, _full: &Path) -> io::Result<FileIdentity> {
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(in crate::remove::delete) fn dir_identity(dir: &Dir, _full: &Path) -> io::Result<FileIdentity> {
        let metadata = dir.dir_metadata()?;
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(in crate::remove::delete) fn is_reparse_point(_metadata: &Metadata) -> bool {
        false
    }

    pub(in crate::remove::delete) fn is_volume_mount(_full: &Path) -> bool {
        false
    }

    /// `unlinkat` relative to `parent`: a FIFO, socket, or device is never
    /// opened, and a hard link loses only this name.
    pub(in crate::remove::delete) fn remove_file(parent: &Dir, name: &OsStr, _full: &Path, _identity: &FileIdentity) -> io::Result<()> {
        parent.remove_file(name)
    }

    pub(in crate::remove::delete) fn remove_dir(parent: &Dir, name: &OsStr, _full: &Path, _identity: &FileIdentity) -> io::Result<()> {
        parent.remove_dir(name)
    }

    /// Gives the owner full access to the open directory `dir`, through its
    /// handle. A directory has no other hard links, so no other name's
    /// permissions change.
    pub(in crate::remove::delete) fn make_writable(dir: &Dir) -> io::Result<()> {
        let mode = dir.dir_metadata()?.mode();
        chmod_handle(dir.as_raw_fd(), owner_full(mode))
    }

    /// Gives the owner full access to the directory `name` in `parent`, which
    /// must still be the one `metadata` describes, without following a link:
    /// a link swapped in is refused, never changed through.
    #[cfg(target_os = "linux")]
    pub(in crate::remove::delete) fn make_traversable(parent: &Dir, name: &OsStr, metadata: &Metadata) -> io::Result<()> {
        use std::os::fd::{FromRawFd, OwnedFd};
        let c_name = c_name(name)?;
        // `O_PATH` needs no permission on the directory itself, and with
        // `O_NOFOLLOW | O_DIRECTORY` a link fails rather than being opened.
        // SAFETY: the descriptor is `parent`'s and the name is NUL-terminated.
        let fd = unsafe {
            libc::openat(parent.as_raw_fd(), c_name.as_ptr(), libc::O_PATH | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `openat` just returned this descriptor, owned from here on.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: `stat` is writable for the call.
        if unsafe { libc::fstat(fd.as_raw_fd(), &mut stat) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if u64::from(stat.st_dev) != metadata.dev() || u64::from(stat.st_ino) != metadata.ino() {
            return Err(io::Error::other(super::super::Replaced));
        }
        chmod_handle(fd.as_raw_fd(), owner_full(metadata.mode()))
    }

    #[cfg(not(target_os = "linux"))]
    pub(in crate::remove::delete) fn make_traversable(parent: &Dir, name: &OsStr, metadata: &Metadata) -> io::Result<()> {
        let c_name = c_name(name)?;
        // SAFETY: the descriptor is `parent`'s and the name is NUL-terminated.
        let result = unsafe { libc::fchmodat(parent.as_raw_fd(), c_name.as_ptr(), owner_full(metadata.mode()), libc::AT_SYMLINK_NOFOLLOW) };
        if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }

    fn owner_full(mode: u32) -> libc::mode_t {
        ((mode & 0o7777) | 0o700) as libc::mode_t
    }

    fn c_name(name: &OsStr) -> io::Result<CString> {
        CString::new(name.as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a name holds a NUL byte"))
    }

    /// `fchmod` on `fd`. On Linux cap-std holds directories as `O_PATH`
    /// descriptors, which `fchmod` rejects (`EBADF`); the descriptor's
    /// `/proc/self/fd` entry reaches the same object.
    fn chmod_handle(fd: std::os::fd::RawFd, mode: libc::mode_t) -> io::Result<()> {
        // SAFETY: `fd` is open for the whole call.
        if unsafe { libc::fchmod(fd, mode) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        #[cfg(target_os = "linux")]
        if error.raw_os_error() == Some(libc::EBADF) {
            let proc_path = CString::new(format!("/proc/self/fd/{fd}")).expect("no NUL in a formatted path");
            // SAFETY: a NUL-terminated path.
            if unsafe { libc::chmod(proc_path.as_ptr(), mode) } == 0 {
                return Ok(());
            }
            return Err(io::Error::last_os_error());
        }
        Err(error)
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use cap_std::fs::{Dir, Metadata, MetadataExt};
    use sniff::filesystem::query::FileIdentity;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, DELETE, FILE_ATTRIBUTE_READONLY, FILE_BASIC_INFO,
        FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        FILE_DISPOSITION_INFO, FILE_DISPOSITION_INFO_EX, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES, FileBasicInfo,
        FileDispositionInfo, FileDispositionInfoEx, GetFileInformationByHandle, OPEN_EXISTING, SYNCHRONIZE,
        SetFileInformationByHandle,
    };

    use crate::remove::quarantine::rename;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

    /// Clearing the read-only attribute cannot overcome an access-control
    /// denial, and deletion already ignores that attribute, so a denial is
    /// never answered by changing permissions.
    pub(in crate::remove::delete) const FIX_DENIAL_BY_MODE: bool = false;

    /// By path, opening the entry itself: the 128-bit file ID, since the
    /// 64-bit index is not unique on ReFS.
    pub(in crate::remove::delete) fn identity(_metadata: &Metadata, full: &Path) -> io::Result<FileIdentity> {
        rename::identity(full)
    }

    pub(in crate::remove::delete) fn dir_identity(dir: &Dir, _full: &Path) -> io::Result<FileIdentity> {
        rename::handle_identity(dir.as_raw_handle())
    }

    /// A symlink, a junction, or any other reparse point (a cloud placeholder,
    /// a deduplicated file): removed as an entry, never traversed.
    pub(in crate::remove::delete) fn is_reparse_point(metadata: &Metadata) -> bool {
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    /// A junction to a volume (`\\?\Volume{…}\`) is a mounted filesystem.
    pub(in crate::remove::delete) fn is_volume_mount(full: &Path) -> bool {
        std::fs::read_link(full).is_ok_and(|target| {
            let bytes = target.as_os_str().as_encoded_bytes();
            bytes.starts_with(br"\\?\Volume{") || bytes.starts_with(br"\??\Volume{")
        })
    }

    pub(in crate::remove::delete) fn remove_file(_parent: &Dir, _name: &OsStr, full: &Path, identity: &FileIdentity) -> io::Result<()> {
        delete(full, identity)
    }

    pub(in crate::remove::delete) fn remove_dir(_parent: &Dir, _name: &OsStr, full: &Path, identity: &FileIdentity) -> io::Result<()> {
        delete(full, identity)
    }

    pub(in crate::remove::delete) fn make_writable(_dir: &Dir) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(ERROR_ACCESS_DENIED as i32))
    }

    pub(in crate::remove::delete) fn make_traversable(_parent: &Dir, _name: &OsStr, _metadata: &Metadata) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(ERROR_ACCESS_DENIED as i32))
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by `open` and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// `\\?\` form of an absolute path, so a deep `target` tree past
    /// `MAX_PATH` still opens, as `std::fs` does it.
    fn verbatim(full: &Path) -> io::Result<Vec<u16>> {
        let absolute = std::path::absolute(full)?;
        let text: Vec<u16> = absolute.as_os_str().encode_wide().map(|unit| if unit == u16::from(b'/') { u16::from(b'\\') } else { unit }).collect();
        let prefix: &[u16] = &[92, 92, 63, 92]; // \\?\
        let mut wide = Vec::with_capacity(text.len() + 8);
        if text.starts_with(prefix) {
            wide.extend_from_slice(&text);
        } else if text.starts_with(&[92, 92]) {
            wide.extend(r"\\?\UNC\".encode_utf16());
            wide.extend_from_slice(&text[2..]);
        } else {
            wide.extend_from_slice(prefix);
            wide.extend_from_slice(&text);
        }
        wide.push(0);
        Ok(wide)
    }

    /// Opens the entry at `full` itself (never a link's target) for
    /// deletion, sharing everything so the open blocks no other program.
    fn open(full: &Path) -> io::Result<OwnedHandle> {
        let path = verbatim(full)?;
        // SAFETY: `path` is NUL-terminated UTF-16.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                DELETE | FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE { Err(io::Error::last_os_error()) } else { Ok(OwnedHandle(handle)) }
    }

    /// Deletes the entry at `full`, which must still be the object with
    /// `identity`. POSIX semantics free the name at once even while another
    /// program holds a handle that allows deletion, and the read-only
    /// attribute is ignored rather than cleared, so another hard link's
    /// attributes never change. A filesystem without those flags falls back
    /// to a plain delete, clearing read-only only where no other name shares
    /// the attributes.
    fn delete(full: &Path, identity: &FileIdentity) -> io::Result<()> {
        let handle = open(full)?;
        if rename::handle_identity(handle.0)? != *identity {
            return Err(io::Error::other(crate::remove::delete::Replaced));
        }
        let info = FILE_DISPOSITION_INFO_EX {
            Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
        };
        // SAFETY: `info` is a valid `FILE_DISPOSITION_INFO_EX` for the call.
        let done = unsafe {
            SetFileInformationByHandle(handle.0, FileDispositionInfoEx, (&info as *const FILE_DISPOSITION_INFO_EX).cast(), size_of::<FILE_DISPOSITION_INFO_EX>() as u32)
        };
        if done != 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if !matches!(error.raw_os_error().map(|code| code as u32), Some(ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION)) {
            return Err(error);
        }
        match dispose(&handle) {
            Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) && clear_read_only(&handle)? => dispose(&handle),
            result => result,
        }
    }

    fn dispose(handle: &OwnedHandle) -> io::Result<()> {
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        // SAFETY: `info` is a valid `FILE_DISPOSITION_INFO` for the call.
        let done = unsafe {
            SetFileInformationByHandle(handle.0, FileDispositionInfo, (&info as *const FILE_DISPOSITION_INFO).cast(), size_of::<FILE_DISPOSITION_INFO>() as u32)
        };
        if done != 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }

    /// Clears read-only on an entry no other name shares; `false` when it was
    /// not read-only or another hard link shares it.
    fn clear_read_only(handle: &OwnedHandle) -> io::Result<bool> {
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: `info` is writable for the call.
        if unsafe { GetFileInformationByHandle(handle.0, &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.dwFileAttributes & FILE_ATTRIBUTE_READONLY == 0 || info.nNumberOfLinks > 1 {
            return Ok(false);
        }
        // Zero times leave them unchanged.
        let mut basic: FILE_BASIC_INFO = unsafe { std::mem::zeroed() };
        basic.FileAttributes = info.dwFileAttributes & !FILE_ATTRIBUTE_READONLY;
        // SAFETY: `basic` is a valid `FILE_BASIC_INFO` for the call.
        let done = unsafe { SetFileInformationByHandle(handle.0, FileBasicInfo, (&basic as *const FILE_BASIC_INFO).cast(), size_of::<FILE_BASIC_INFO>() as u32) };
        if done != 0 { Ok(true) } else { Err(io::Error::last_os_error()) }
    }
}
