use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}
