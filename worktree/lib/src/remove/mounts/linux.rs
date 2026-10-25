use std::io;

use super::{Mount, mountinfo};

/// `/proc/self/mountinfo` rather than `/proc/mounts`: both list bind mounts,
/// but only mountinfo's escaping is specified for every field.
pub(super) fn mounts() -> io::Result<Vec<Mount>> {
    let text = std::fs::read("/proc/self/mountinfo")?;
    Ok(mountinfo::parse(&text)?.into_iter().map(mountinfo::Entry::into_mount).collect())
}

/// Whether the directory at `path` is the root of a mount, from `statx`'s
/// `STATX_ATTR_MOUNT_ROOT`; `None` when the kernel cannot say (before 5.8)
/// or the call fails. Made through `syscall` so no glibc 2.28 symbol is
/// needed.
#[cfg(target_env = "gnu")]
pub(super) fn is_mount_root(path: &std::path::Path) -> Option<bool> {
    use std::os::unix::ffi::OsStrExt;

    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut found = std::mem::MaybeUninit::<libc::statx>::zeroed();
    let flags = libc::AT_SYMLINK_NOFOLLOW | libc::AT_NO_AUTOMOUNT;
    // SAFETY: `path` is NUL-terminated and `found` is a writable `statx`;
    // a zero mask asks for no fields, and attributes are always reported.
    let status = unsafe { libc::syscall(libc::SYS_statx, libc::AT_FDCWD, path.as_ptr(), flags, 0u32, found.as_mut_ptr()) };
    if status != 0 {
        return None;
    }
    // SAFETY: the kernel filled `found`, and it was valid all-zero before.
    let found = unsafe { found.assume_init() };
    let mount_root = libc::STATX_ATTR_MOUNT_ROOT as u64;
    (found.stx_attributes_mask & mount_root != 0).then_some(found.stx_attributes & mount_root != 0)
}
