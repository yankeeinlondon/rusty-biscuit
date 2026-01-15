use serde::{Deserialize, Serialize};

/// System memory information.
///
/// Contains details about total, available, and used memory in bytes,
/// as well as swap memory statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total system memory in bytes
    pub total_bytes: u64,
    /// Available memory in bytes
    pub available_bytes: u64,
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Total swap memory in bytes
    pub total_swap: u64,
    /// Free swap memory in bytes
    pub free_swap: u64,
    /// Used swap memory in bytes
    pub used_swap: u64,
}
