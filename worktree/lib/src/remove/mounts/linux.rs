use std::io;

use super::{Mount, mountinfo};

/// `/proc/self/mountinfo` rather than `/proc/mounts`: both list bind mounts,
/// but only mountinfo's escaping is specified for every field.
pub(super) fn mounts() -> io::Result<Vec<Mount>> {
    let text = std::fs::read("/proc/self/mountinfo")?;
    Ok(mountinfo::parse(&text)?.into_iter().map(mountinfo::Entry::into_mount).collect())
}
