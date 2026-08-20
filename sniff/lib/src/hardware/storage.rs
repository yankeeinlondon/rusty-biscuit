use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Storage device kind.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageKind {
    /// Solid-state drive
    Ssd,
    /// Traditional hard disk drive
    Hdd,
    /// Unknown or undetected type
    #[default]
    Unknown,
}

/// Storage device information.
///
/// Contains details about a single storage device including name,
/// mount point, capacity, and file system type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Device name
    pub name: String,
    /// Mount point path
    pub mount_point: PathBuf,
    /// Total storage capacity in bytes
    pub total_bytes: u64,
    /// Available storage in bytes
    pub available_bytes: u64,
    /// File system type (e.g., "apfs", "ext4", "ntfs")
    pub file_system: String,
    /// Storage device kind (SSD, HDD, Unknown)
    pub kind: StorageKind,
    /// Whether the storage is removable
    pub is_removable: bool,
}

/// Detect storage devices with platform-specific filtering.
///
/// On macOS, uses `getmntinfo` to enumerate mounts and filters out
/// virtual/irrelevant filesystems before probing, avoiding expensive
/// `statfs` calls on autofs and CoreSimulator disk images.
///
/// On Linux, reads `/proc/mounts` and filters out virtual filesystems
/// before calling `statvfs`.
///
/// On other platforms, falls back to `sysinfo::Disks`.
pub fn detect_storage() -> Vec<StorageInfo> {
    detect_storage_impl()
}

#[cfg(target_os = "macos")]
fn detect_storage_impl() -> Vec<StorageInfo> {
    use std::ffi::CStr;

    unsafe {
        let mut mntbuf: *mut libc::statfs = std::ptr::null_mut();
        let count = libc::getmntinfo(&mut mntbuf, libc::MNT_NOWAIT);
        if count <= 0 || mntbuf.is_null() {
            return Vec::new();
        }

        let entries = std::slice::from_raw_parts(mntbuf, count as usize);
        let mut storage = Vec::new();

        // Collect unique device names for batched diskutil query
        let mut device_names: Vec<String> = Vec::new();
        let mut device_to_kind: std::collections::HashMap<String, StorageKind> =
            std::collections::HashMap::new();

        for entry in entries {
            let fstype = CStr::from_ptr(entry.f_fstypename.as_ptr()).to_string_lossy();
            let mount_point = CStr::from_ptr(entry.f_mntonname.as_ptr()).to_string_lossy();
            let device_name = CStr::from_ptr(entry.f_mntfromname.as_ptr()).to_string_lossy();

            // Skip virtual and problematic filesystem types
            if matches!(fstype.as_ref(), "devfs" | "autofs" | "nullfs") {
                continue;
            }

            // Skip system snapshot volumes (except /System/Volumes/Data which is the real data volume)
            if mount_point.starts_with("/System/Volumes/")
                && mount_point.as_ref() != "/System/Volumes/Data"
            {
                continue;
            }

            // Skip Xcode CoreSimulator disk images
            if mount_point.contains("/Library/Developer/CoreSimulator/") {
                continue;
            }

            // Skip temporary sandbox volumes
            if mount_point.starts_with("/private/var/folders/") {
                continue;
            }

            let block_size = entry.f_bsize as u64;
            let total_bytes = entry.f_blocks * block_size;
            let available_bytes = entry.f_bavail * block_size;

            // Collect unique device names for batched diskutil query
            let dev_name = std::path::Path::new(device_name.as_ref())
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !dev_name.is_empty() && !device_to_kind.contains_key(dev_name) {
                device_names.push(dev_name.to_string());
            }

            storage.push(StorageInfo {
                name: device_name.into_owned(),
                mount_point: PathBuf::from(mount_point.as_ref()),
                total_bytes,
                available_bytes,
                file_system: fstype.into_owned(),
                kind: StorageKind::Unknown, // Will be filled in after batch query
                is_removable: false,
            });
        }

        // Batch query all unique devices with a single diskutil call
        if !device_names.is_empty() {
            let mut args = vec!["info"];
            for name in &device_names {
                args.push(name.as_str());
            }
            if let Ok(output) = crate::process::run_with_timeout(
                "diskutil",
                &args,
                crate::process::timeouts::DISKUTIL,
            ) {
                let stdout = output.stdout_lossy();
                let mut current_device: Option<&str> = None;
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    // diskutil info output separates devices with blank lines and headers
                    if trimmed.starts_with("Device Identifier:") {
                        // Extract device identifier like "disk0s1"
                        if let Some(id) = trimmed.split(':').nth(1) {
                            let id = id.trim();
                            current_device =
                                device_names.iter().find(|n| *n == id).map(|s| s.as_str());
                        }
                    }
                    if trimmed.starts_with("Solid State:") {
                        let kind = if trimmed.contains("Yes") {
                            StorageKind::Ssd
                        } else if trimmed.contains("No") {
                            StorageKind::Hdd
                        } else {
                            StorageKind::Unknown
                        };
                        if let Some(dev) = current_device {
                            device_to_kind.insert(dev.to_string(), kind);
                        }
                    }
                }
            }
        }

        // Fill in the kinds from the batch query
        for info in &mut storage {
            let dev_name = std::path::Path::new(&info.name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if let Some(kind) = device_to_kind.get(dev_name) {
                info.kind = *kind;
            }
        }

        storage
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_storage_kind_macos(device: &str) -> StorageKind {
    let dev_name = std::path::Path::new(device)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if dev_name.is_empty() {
        return StorageKind::Unknown;
    }
    let output = match crate::process::run_with_timeout(
        "diskutil",
        &["info", dev_name],
        crate::process::timeouts::DISKUTIL,
    ) {
        Ok(o) => o,
        Err(_) => return StorageKind::Unknown,
    };
    parse_diskutil_info_output(&output.stdout)
}

// Available under test on every platform so the pure-parser coverage below
// compiles on Linux and Windows; only macOS uses it at runtime.
#[cfg(any(target_os = "macos", test))]
#[allow(dead_code)]
fn parse_diskutil_info_output(stdout: &[u8]) -> StorageKind {
    let stdout = String::from_utf8_lossy(stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Solid State:") {
            if trimmed.contains("Yes") {
                return StorageKind::Ssd;
            } else if trimmed.contains("No") {
                return StorageKind::Hdd;
            }
        }
    }
    StorageKind::Unknown
}

#[cfg(target_os = "linux")]
fn detect_storage_impl() -> Vec<StorageInfo> {
    use std::ffi::CString;
    use std::io::BufRead;

    const VIRTUAL_FS: &[&str] = &[
        "proc",
        "sysfs",
        "tmpfs",
        "devpts",
        "cgroup",
        "cgroup2",
        "pstore",
        "debugfs",
        "securityfs",
        "configfs",
        "fusectl",
        "mqueue",
        "hugetlbfs",
        "devtmpfs",
        "binfmt_misc",
        "autofs",
        "tracefs",
        "bpf",
        "efivarfs",
        "nsfs",
        "ramfs",
        "overlay",
    ];

    let file = match std::fs::File::open("/proc/mounts") {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = std::io::BufReader::new(file);
    let mut storage = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let device = parts[0];
        let mount_point = parts[1];
        let fstype = parts[2];

        if VIRTUAL_FS.contains(&fstype) {
            continue;
        }

        let c_path = match CString::new(mount_point) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
        if ret != 0 {
            continue;
        }

        let block_size = stat.f_frsize as u64;
        let total_bytes = stat.f_blocks * block_size;
        let available_bytes = stat.f_bavail * block_size;
        let kind = detect_storage_kind_linux(device);

        storage.push(StorageInfo {
            name: device.to_string(),
            mount_point: PathBuf::from(mount_point),
            total_bytes,
            available_bytes,
            file_system: fstype.to_string(),
            kind,
            is_removable: false,
        });
    }

    storage
}

#[cfg(target_os = "linux")]
fn detect_storage_kind_linux(device: &str) -> StorageKind {
    let dev_name = std::path::Path::new(device)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if dev_name.is_empty() {
        return StorageKind::Unknown;
    }
    let rotational_path = format!("/sys/block/{dev_name}/queue/rotational");
    match std::fs::read_to_string(&rotational_path) {
        Ok(s) if s.trim() == "0" => StorageKind::Ssd,
        Ok(s) if s.trim() == "1" => StorageKind::Hdd,
        _ => StorageKind::Unknown,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn detect_storage_impl() -> Vec<StorageInfo> {
    use sysinfo::{DiskKind, Disks};

    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|d| StorageInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_path_buf(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            file_system: d.file_system().to_string_lossy().to_string(),
            kind: match d.kind() {
                DiskKind::SSD => StorageKind::Ssd,
                DiskKind::HDD => StorageKind::Hdd,
                DiskKind::Unknown(_) => StorageKind::Unknown,
            },
            is_removable: d.is_removable(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    // These exercise the macOS `diskutil info` text parser only. They run on
    // every platform purely to keep the parser coverage portable — they assert
    // nothing about Linux (`/sys/block/.../rotational`) or Windows storage
    // detection, which use entirely separate code paths.
    use super::*;

    #[test]
    fn parse_diskutil_info_output_detects_ssd() {
        let output = b"Device Identifier: disk0s1\nSolid State: Yes\n";
        assert_eq!(parse_diskutil_info_output(output), StorageKind::Ssd);
    }

    #[test]
    fn parse_diskutil_info_output_detects_hdd() {
        let output = b"Device Identifier: disk1s1\nSolid State: No\n";
        assert_eq!(parse_diskutil_info_output(output), StorageKind::Hdd);
    }

    #[test]
    fn parse_diskutil_info_output_returns_unknown_when_missing() {
        let output = b"Device Identifier: disk0s1\n";
        assert_eq!(parse_diskutil_info_output(output), StorageKind::Unknown);
    }
}
