//! An atomic rename that never replaces an existing destination.
//!
//! `std::fs::rename` may replace what is already at the destination (it
//! replaced an existing empty directory on every OS tested, and a file on
//! Windows), so the quarantine move uses each OS's no-replacement primitive:
//!
//! | OS | Primitive |
//! | --- | --- |
//! | Linux | `renameat2(RENAME_NOREPLACE)` |
//! | macOS | `renamex_np(RENAME_EXCL)` |
//! | Windows | `SetFileInformationByHandle(FileRenameInfo)` with `ReplaceIfExists = FALSE`, on a handle opened with `DELETE` access |
//!
//! None of them follows a link in the last component of either path, and none
//! needs special privileges. A failure is classified into a [`RenameFailure`]
//! so the caller can map it to an exit code without reading message text.

use std::io;
use std::path::Path;

use sniff::filesystem::query::FileIdentity;

/// Why a [`rename_no_replace`] failed. In every case nothing was moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameFailure {
    /// Something already exists at the destination, and it was left alone.
    Collision,
    /// The filesystem cannot rename without replacement, as on WSL drvfs,
    /// which returns `EINVAL` for every `RENAME_NOREPLACE` move. An `EEXIST`
    /// from such a filesystem still means [`Collision`](Self::Collision): the
    /// kernel checks the destination before the filesystem rejects the flag.
    Unsupported,
    /// The destination is on another filesystem or volume.
    CrossDevice,
    /// This user may not move the source: a Unix permission denial, or on
    /// Windows a denial while opening the source for `DELETE`.
    Permission,
    /// Windows only: the source opened for `DELETE`, but the rename was
    /// denied. An open file, a handle on a subdirectory, or a process whose
    /// working directory is inside the tree all cause this, and Windows gives
    /// them the same code as an access-control denial, so a rare permission
    /// problem is reported here too. Either way nothing moved.
    Sharing,
    /// Any other I/O failure, including a missing source.
    Other,
}

/// A failed [`rename_no_replace`]: its classification and the OS error.
#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {source}")]
pub struct RenameError {
    pub kind: RenameFailure,
    #[source]
    pub source: io::Error,
}

/// The step of the Windows rename that failed; on Unix the move is one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Opening the source with `DELETE` access (Windows only).
    #[cfg_attr(not(windows), allow(dead_code))]
    Open,
    /// The rename call itself.
    Rename,
}

/// Moves `from` to `to` in one atomic step, failing rather than replacing
/// anything that exists at `to`, including an empty directory or a dangling
/// link. The moved object keeps its [`identity`].
///
/// ## Errors
///
/// A [`RenameError`] whose `kind` says why. `from` is untouched whenever this
/// returns an error.
pub fn rename_no_replace(from: &Path, to: &Path) -> Result<(), RenameError> {
    platform::rename_no_replace(from, to).map_err(|(stage, source)| RenameError {
        kind: classify(stage, &source),
        source,
    })
}

/// The identity of the object at `path`, without following a final link:
/// device and inode on Unix, volume serial and 128-bit file ID on Windows
/// (the legacy 64-bit index is not unique on ReFS).
///
/// ## Errors
///
/// The error from reading the object's metadata.
pub fn identity(path: &Path) -> io::Result<FileIdentity> {
    platform::identity(path)
}

/// The [`identity`] of the object an open handle refers to.
///
/// ## Errors
///
/// The error from reading the handle's file ID.
#[cfg(windows)]
pub(crate) fn handle_identity(handle: std::os::windows::io::RawHandle) -> io::Result<FileIdentity> {
    platform::handle_identity(handle)
}

/// Maps an OS error from `stage` to a [`RenameFailure`].
pub(crate) fn classify(stage: Stage, error: &io::Error) -> RenameFailure {
    let Some(code) = error.raw_os_error() else {
        return RenameFailure::Other;
    };
    platform::classify(stage, code)
}

#[cfg(unix)]
mod unix_common {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use super::{FileIdentity, RenameFailure, Stage};

    pub(super) fn c_path(path: &Path) -> io::Result<CString> {
        CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))
    }

    pub(super) fn identity(path: &Path) -> io::Result<FileIdentity> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::symlink_metadata(path)?;
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(super) fn classify(_stage: Stage, code: i32) -> RenameFailure {
        match code {
            libc::EEXIST | libc::ENOTEMPTY => RenameFailure::Collision,
            // `EINVAL` is also the answer for moving a directory into itself,
            // which a sibling destination never is. Linux before 3.15 has no
            // `renameat2` (`ENOSYS`); macOS filesystems without exclusive
            // rename answer `ENOTSUP`.
            libc::EINVAL | libc::ENOSYS | libc::ENOTSUP => RenameFailure::Unsupported,
            libc::EXDEV => RenameFailure::CrossDevice,
            libc::EACCES | libc::EPERM => RenameFailure::Permission,
            _ => RenameFailure::Other,
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::io;
    use std::path::Path;

    use super::Stage;
    pub(super) use super::unix_common::{classify, identity};
    use super::unix_common::c_path;

    pub(super) fn rename_no_replace(from: &Path, to: &Path) -> Result<(), (Stage, io::Error)> {
        let from = c_path(from).map_err(|error| (Stage::Rename, error))?;
        let to = c_path(to).map_err(|error| (Stage::Rename, error))?;
        // The raw syscall rather than libc's wrapper, which older glibc and
        // musl builds do not export.
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result == 0 { Ok(()) } else { Err((Stage::Rename, io::Error::last_os_error())) }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::io;
    use std::path::Path;

    use super::Stage;
    pub(super) use super::unix_common::{classify, identity};
    use super::unix_common::c_path;

    pub(super) fn rename_no_replace(from: &Path, to: &Path) -> Result<(), (Stage, io::Error)> {
        let from = c_path(from).map_err(|error| (Stage::Rename, error))?;
        let to = c_path(to).map_err(|error| (Stage::Rename, error))?;
        let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        if result == 0 { Ok(()) } else { Err((Stage::Rename, io::Error::last_os_error())) }
    }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
mod platform {
    use std::io;
    use std::path::Path;

    use super::Stage;
    pub(super) use super::unix_common::{classify, identity};

    /// No no-replacement rename is wired up for this Unix; refusing keeps the
    /// contract rather than falling back to a rename that may replace.
    pub(super) fn rename_no_replace(_from: &Path, _to: &Path) -> Result<(), (Stage, io::Error)> {
        Err((Stage::Rename, io::Error::from_raw_os_error(libc::ENOTSUP)))
    }
}

#[cfg(windows)]
mod platform {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS,
        ERROR_INVALID_FUNCTION, ERROR_NOT_SAME_DEVICE, ERROR_NOT_SUPPORTED,
        ERROR_SHARING_VIOLATION, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO,
        FILE_READ_ATTRIBUTES, FILE_RENAME_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FileIdInfo, FileRenameInfo, GetFileInformationByHandleEx,
        OPEN_EXISTING, SYNCHRONIZE, SetFileInformationByHandle,
    };

    use super::{FileIdentity, RenameFailure, Stage};

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Opens `path` itself (a directory, or a link without following it),
    /// sharing everything so the open never blocks another process.
    fn open(path: &Path, access: u32) -> io::Result<OwnedHandle> {
        let path = wide(path);
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE { Err(io::Error::last_os_error()) } else { Ok(OwnedHandle(handle)) }
    }

    pub(super) fn rename_no_replace(from: &Path, to: &Path) -> Result<(), (Stage, io::Error)> {
        // `FileRenameInfo` needs a full destination path when no root
        // directory handle is given.
        let to = std::path::absolute(to).map_err(|error| (Stage::Rename, error))?;
        let source = open(from, DELETE | SYNCHRONIZE).map_err(|error| (Stage::Open, error))?;

        let name = wide(&to);
        let name_bytes = (name.len() - 1) * size_of::<u16>();
        let total = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + name_bytes + size_of::<u16>();
        // A zeroed, 8-byte-aligned buffer: `Flags` and `RootDirectory` stay
        // zero, and `ReplaceIfExists` is FALSE.
        let mut buffer = vec![0u64; total.div_ceil(size_of::<u64>())];
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        unsafe {
            (*info).FileNameLength = name_bytes as u32;
            std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
        }
        let renamed = unsafe { SetFileInformationByHandle(source.0, FileRenameInfo, info.cast(), total as u32) };
        if renamed == 0 { Err((Stage::Rename, io::Error::last_os_error())) } else { Ok(()) }
    }

    pub(super) fn identity(path: &Path) -> io::Result<FileIdentity> {
        let handle = open(path, FILE_READ_ATTRIBUTES)?;
        handle_identity(handle.0)
    }

    pub(super) fn handle_identity(handle: HANDLE) -> io::Result<FileIdentity> {
        let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
        let read = unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                size_of::<FILE_ID_INFO>() as u32,
            )
        };
        if read == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(FileIdentity::Windows {
            volume_serial: info.VolumeSerialNumber,
            file_id: u128::from_le_bytes(info.FileId.Identifier),
        })
    }

    pub(super) fn classify(stage: Stage, code: i32) -> RenameFailure {
        let code = code as u32;
        match (stage, code) {
            (Stage::Open, ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION) => RenameFailure::Permission,
            // The `DELETE` open succeeded, so the denial comes from something
            // inside the tree being in use.
            (Stage::Rename, ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION) => RenameFailure::Sharing,
            (_, ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS) => RenameFailure::Collision,
            (_, ERROR_NOT_SAME_DEVICE) => RenameFailure::CrossDevice,
            (_, ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION) => RenameFailure::Unsupported,
            _ => RenameFailure::Other,
        }
    }
}

#[cfg(test)]
mod tests;
