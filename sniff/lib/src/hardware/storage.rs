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
///
/// ## Notes
///
/// Network mounts (SMB, NFS, etc.) are currently not detected by the
/// underlying sysinfo library on most platforms. This is a known limitation.
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
