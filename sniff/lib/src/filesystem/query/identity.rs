//! Native file identity: what two paths must share to name the same object.

use super::native::decimal;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// The OS identity of a filesystem object.
///
/// Unix compares `(st_dev, st_ino)`. Windows compares the volume serial number
/// with the full 128-bit `FILE_ID_INFO` identifier; the 64-bit file index is
/// not unique on ReFS. Values serialize as decimal strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FileIdentity {
    Unix {
        #[serde(with = "decimal")]
        device: u64,
        #[serde(with = "decimal")]
        inode: u64,
    },
    Windows {
        #[serde(with = "decimal")]
        volume_serial: u64,
        #[serde(with = "decimal")]
        file_id: u128,
    },
}

#[cfg(unix)]
pub(crate) fn of_metadata(metadata: &std::fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    FileIdentity::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

/// Identity of the object at `path`. With `follow_links` false a symlink,
/// junction, or other reparse point reports its own identity.
pub(crate) fn of_path(path: &Path, follow_links: bool) -> io::Result<FileIdentity> {
    platform::of_path(path, follow_links)
}

/// Identity of a tree-walk entry without following it.
pub(crate) fn of_entry(entry: &walkdir::DirEntry) -> io::Result<FileIdentity> {
    #[cfg(unix)]
    {
        entry.metadata().map(|m| of_metadata(&m)).map_err(io::Error::from)
    }
    #[cfg(windows)]
    {
        of_path(entry.path(), false)
    }
}

#[cfg(unix)]
mod platform {
    use super::*;

    pub(super) fn of_path(path: &Path, follow_links: bool) -> io::Result<FileIdentity> {
        let metadata = if follow_links {
            std::fs::metadata(path)?
        } else {
            std::fs::symlink_metadata(path)?
        };
        Ok(of_metadata(&metadata))
    }
}

#[cfg(windows)]
pub(crate) use platform::io_error;

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo,
        GetFileInformationByHandleEx, OPEN_EXISTING,
    };
    use windows::core::PCWSTR;

    /// `io::Error::from(windows::core::Error)` keeps the HRESULT as the raw OS
    /// error, so `kind()` never reports `NotFound`; unwrap `FACILITY_WIN32`
    /// HRESULTs back to their Win32 code first.
    pub(crate) fn io_error(error: windows::core::Error) -> io::Error {
        let code = error.code().0 as u32;
        if code & 0xFFFF_0000 == 0x8007_0000 {
            io::Error::from_raw_os_error((code & 0xFFFF) as i32)
        } else {
            io::Error::from(error)
        }
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    /// Opens a fresh attribute-only handle owned by this query. The stall seen
    /// when querying a handle duplicated from another process (whose owner can
    /// hold the file object in synchronous I/O) does not apply to it.
    pub(super) fn of_path(path: &Path, follow_links: bool) -> io::Result<FileIdentity> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
        if !follow_links {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                flags,
                None,
            )
        }
        .map_err(io_error)?;
        let handle = OwnedHandle(handle);

        let mut info = FILE_ID_INFO::default();
        unsafe {
            GetFileInformationByHandleEx(
                handle.0,
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        }
        .map_err(io_error)?;
        Ok(FileIdentity::Windows {
            volume_serial: info.VolumeSerialNumber,
            file_id: u128::from_le_bytes(info.FileId.Identifier),
        })
    }
}
