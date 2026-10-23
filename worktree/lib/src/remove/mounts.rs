//! The filesystems mounted inside a checkout, which removal never deletes
//! across. A mount is found by its mount point in the platform's mount
//! table, never by comparing device IDs: a Linux bind mount of a directory
//! on the same filesystem has the same device as its parent.
//!
//! [`NativeMounts`] reads the host's table; tests inject a fixed one.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "linux", test))]
mod mountinfo;
#[cfg(windows)]
mod windows;

/// One entry of the mount table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub mount_point: PathBuf,
    /// What is mounted there, as the platform names it (a device, a volume,
    /// or the source directory of a bind mount); shown in the warning.
    pub source: String,
}

/// Where the mount table comes from. Tests inject one, including
/// same-device bind mounts, so no test needs privileges to mount.
pub trait MountInfo: Sync {
    /// The mount table. A native table spells mount points as the OS's real
    /// paths (on macOS `/var/folders/...` is `/private/var/folders/...`), so
    /// the caller resolves the checkout path the same way, for example with
    /// `biscuit_file::canonicalize_simplified`, before comparing.
    ///
    /// ## Errors
    ///
    /// The table could not be read.
    fn mounts(&self) -> io::Result<Vec<Mount>>;
}

/// The host's mount table, read without mounting, unmounting, or elevating:
/// `/proc/self/mountinfo` on Linux (which lists same-device bind mounts),
/// `getfsstat` on macOS, and every volume's mount points on Windows, with
/// the volume GUID name as the source.
///
/// ## Errors
///
/// The table could not be read, a Linux mountinfo line is malformed (failing
/// rather than skipping it, which could hide a mount), or the platform has
/// no reader ([`io::ErrorKind::Unsupported`] on Unix other than Linux and
/// macOS).
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeMounts;

impl MountInfo for NativeMounts {
    fn mounts(&self) -> io::Result<Vec<Mount>> {
        #[cfg(target_os = "linux")]
        return linux::mounts();
        #[cfg(target_os = "macos")]
        return macos::mounts();
        #[cfg(windows)]
        return windows::mounts();
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        return Err(io::Error::new(io::ErrorKind::Unsupported, "no mount table reader for this platform"));
    }
}

/// The mounts strictly inside `root`, outermost first. A mount at `root`
/// itself is the checkout's own filesystem, not a nested one.
pub fn mounts_within<'m>(mounts: &'m [Mount], root: &Path) -> Vec<&'m Mount> {
    let mut nested: Vec<&Mount> = mounts.iter().filter(|mount| mount.mount_point.starts_with(root) && mount.mount_point != root).collect();
    nested.sort_by_key(|mount| mount.mount_point.components().count());
    nested
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::FixedMounts;

    fn mount(point: &Path, source: &str) -> Mount {
        Mount { mount_point: point.to_path_buf(), source: source.to_string() }
    }

    #[test]
    fn only_mounts_strictly_inside_the_checkout_are_nested() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wts").join("feat");
        let table = FixedMounts(vec![
            mount(dir.path(), "/dev/disk1"),
            mount(&root, "/dev/disk2"),
            mount(&root.join("target").join("cache"), "/dev/disk3"),
            mount(&root.join("target"), "/dev/disk4"),
            // A sibling whose name extends the checkout's is not inside it.
            mount(&dir.path().join("wts").join("feat-other"), "/dev/disk5"),
        ]);

        let table = table.mounts().unwrap();
        let nested: Vec<&str> = mounts_within(&table, &root).iter().map(|mount| mount.source.as_str()).collect();

        assert_eq!(nested, ["/dev/disk4", "/dev/disk3"], "outermost first");
    }

    /// A bind mount on the same device is found by its mount point alone.
    #[test]
    fn a_same_device_bind_mount_is_nested() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("feat");
        let bound = root.join("vendor");
        let table = FixedMounts(vec![mount(dir.path(), "/dev/sda1"), mount(&bound, "/dev/sda1")]);

        let table = table.mounts().unwrap();

        assert_eq!(mounts_within(&table, &root), [&mount(&bound, "/dev/sda1")]);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_native_table_contains_the_root_filesystem() {
        let table = NativeMounts.mounts().unwrap();

        assert!(table.iter().any(|mount| mount.mount_point == Path::new("/")), "{table:?}");
    }

    #[cfg(windows)]
    #[test]
    fn the_native_table_contains_the_system_drive_root() {
        let drive_root = format!("{}\\", std::env::var("SystemDrive").unwrap());

        let table = NativeMounts.mounts().unwrap();

        assert!(table.iter().any(|mount| mount.mount_point.as_os_str().to_string_lossy().eq_ignore_ascii_case(&drive_root)), "{table:?}");
        assert!(table.iter().all(|mount| mount.source.starts_with("\\\\?\\Volume{")), "{table:?}");
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    #[test]
    fn the_native_table_is_unsupported_on_other_unix() {
        assert_eq!(NativeMounts.mounts().unwrap_err().kind(), io::ErrorKind::Unsupported);
    }
}
