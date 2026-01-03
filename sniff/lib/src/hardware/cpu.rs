use serde::{Deserialize, Serialize};

/// CPU information.
///
/// Contains details about the processor including brand, logical cores,
/// and physical cores when available.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU brand string (e.g., "Intel(R) Core(TM) i7-9750H")
    pub brand: String,
    /// Number of logical CPU cores (includes hyperthreading)
    pub logical_cores: usize,
    /// Number of physical CPU cores (None if unavailable)
    pub physical_cores: Option<usize>,
}
