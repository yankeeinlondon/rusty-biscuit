//! The filesystems mounted inside a checkout, which removal never deletes
//! across. A mount is found by its mount point in the platform's mount
//! table, never by comparing device IDs: a Linux bind mount of a directory
//! on the same filesystem has the same device as its parent.

use std::io;
use std::path::{Path, PathBuf};

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
    /// The mount table, with mount points spelled as the caller spells the
    /// checkout path it compares them with.
    ///
    /// ## Errors
    ///
    /// The table could not be read.
    fn mounts(&self) -> io::Result<Vec<Mount>>;
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
}
