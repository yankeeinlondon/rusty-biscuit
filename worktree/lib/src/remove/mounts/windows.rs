use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStringExt;

use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH};
use windows_sys::Win32::Storage::FileSystem::{
    FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumeInformationW, GetVolumePathNamesForVolumeNameW,
};

use super::Mount;

/// Every path of every volume (drive roots and folder mount points), each
/// with the volume GUID name (`\\?\Volume{...}\`) as its source and the
/// volume's filesystem name (`NTFS`, `ReFS`), empty when it can't be read.
pub(super) fn mounts() -> io::Result<Vec<Mount>> {
    let mut volume = [0u16; MAX_PATH as usize];
    // SAFETY: `volume` is writable for its length.
    let search = unsafe { FindFirstVolumeW(volume.as_mut_ptr(), volume.len() as u32) };
    if search == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let search = VolumeSearch(search);
    let mut mounts = Vec::new();
    loop {
        let name_len = volume.iter().position(|&unit| unit == 0).unwrap_or(volume.len());
        let source = String::from_utf16_lossy(&volume[..name_len]);
        let paths = path_names(&volume)?;
        let fs_type = fs_name(&volume);
        mounts.extend(
            paths
                .split(|&unit| unit == 0)
                .take_while(|path| !path.is_empty())
                .map(|path| Mount {
                    mount_point: OsString::from_wide(path).into(),
                    source: source.clone(),
                    fs_type: fs_type.clone(),
                    super_options: String::new(),
                }),
        );
        // SAFETY: `search` is open and `volume` is writable for its length.
        if unsafe { FindNextVolumeW(search.0, volume.as_mut_ptr(), volume.len() as u32) } == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                return Ok(mounts);
            }
            return Err(error);
        }
    }
}

/// The filesystem name of the volume named by NUL-terminated `volume` (a
/// GUID path with its trailing backslash), or empty when Windows can't say,
/// as for an empty card reader.
fn fs_name(volume: &[u16]) -> String {
    let mut name = [0u16; MAX_PATH as usize + 1];
    // SAFETY: `volume` is NUL-terminated, `name` is writable for its length,
    // and every other out-pointer is null, which the call accepts.
    let read = unsafe {
        GetVolumeInformationW(
            volume.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            name.as_mut_ptr(),
            name.len() as u32,
        )
    };
    if read == 0 {
        return String::new();
    }
    let len = name.iter().position(|&unit| unit == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..len])
}

/// The volume's paths as a double-NUL-terminated list.
fn path_names(volume: &[u16]) -> io::Result<Vec<u16>> {
    let mut paths = vec![0u16; MAX_PATH as usize];
    loop {
        let mut needed = 0u32;
        // SAFETY: `volume` is NUL-terminated, `paths` is writable for its
        // length, and `needed` is a valid out-pointer.
        let listed = unsafe { GetVolumePathNamesForVolumeNameW(volume.as_ptr(), paths.as_mut_ptr(), paths.len() as u32, &mut needed) };
        if listed != 0 {
            return Ok(paths);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_MORE_DATA as i32) {
            return Err(error);
        }
        let grown = (needed as usize).max(paths.len() * 2);
        paths.resize(grown, 0);
    }
}

struct VolumeSearch(HANDLE);

impl Drop for VolumeSearch {
    fn drop(&mut self) {
        // SAFETY: the handle came from `FindFirstVolumeW` and is closed once.
        unsafe { FindVolumeClose(self.0) };
    }
}
