use std::ffi::OsStr;
use std::io;
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;

use super::Mount;

/// Room for mounts that appear between counting and listing them.
const SLACK: usize = 8;

pub(super) fn mounts() -> io::Result<Vec<Mount>> {
    // SAFETY: a null buffer asks only for the number of mounts.
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    let mut capacity = usize::try_from(count).map_err(|_| io::Error::last_os_error())? + SLACK;
    loop {
        // SAFETY: `statfs` is plain data, valid when all-zero.
        let mut table = vec![unsafe { std::mem::zeroed::<libc::statfs>() }; capacity];
        let bytes = c_int::try_from(capacity * size_of::<libc::statfs>()).map_err(|_| io::Error::other("mount table too large"))?;
        // SAFETY: `table` is `bytes` long and writable.
        let returned = unsafe { libc::getfsstat(table.as_mut_ptr(), bytes, libc::MNT_NOWAIT) };
        let returned = usize::try_from(returned).map_err(|_| io::Error::last_os_error())?;
        // A full buffer may have cut the table short.
        if returned < capacity {
            table.truncate(returned);
            return Ok(table
                .iter()
                .map(|entry| Mount {
                    mount_point: OsStr::from_bytes(&c_bytes(&entry.f_mntonname)).into(),
                    source: String::from_utf8_lossy(&c_bytes(&entry.f_mntfromname)).into_owned(),
                    fs_type: String::from_utf8_lossy(&c_bytes(&entry.f_fstypename)).into_owned(),
                    super_options: String::new(),
                })
                .collect());
        }
        capacity *= 2;
    }
}

/// The bytes of a NUL-terminated fixed-size name (all of it if unterminated).
fn c_bytes(name: &[c_char]) -> Vec<u8> {
    name.iter().map(|&byte| byte as u8).take_while(|&byte| byte != 0).collect()
}
