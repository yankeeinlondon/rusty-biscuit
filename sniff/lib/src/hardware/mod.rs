use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::Result;

mod cpu;
mod memory;
mod os;
mod storage;

pub use cpu::CpuInfo;
pub use memory::MemoryInfo;
pub use os::OsInfo;
pub use storage::StorageInfo;

/// Complete hardware information.
///
/// Aggregates operating system, CPU, memory, and storage information
/// detected from the current system.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// Operating system information
    pub os: OsInfo,
    /// CPU information
    pub cpu: CpuInfo,
    /// Memory information
    pub memory: MemoryInfo,
    /// Storage devices (disks)
    pub storage: Vec<StorageInfo>,
}

/// Detects hardware information from the current system.
///
/// This function gathers operating system details, CPU specifications,
/// memory statistics, and storage information using the sysinfo crate.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::hardware::detect_hardware;
///
/// let hw = detect_hardware().unwrap();
/// println!("OS: {} {}", hw.os.name, hw.os.version);
/// println!("CPU: {} ({} cores)", hw.cpu.brand, hw.cpu.logical_cores);
/// println!("Memory: {} GB total", hw.memory.total_bytes / (1024 * 1024 * 1024));
/// ```
///
/// ## Errors
///
/// Currently returns `Ok` in all cases, but future versions may return
/// errors for system information gathering failures.
pub fn detect_hardware() -> Result<HardwareInfo> {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    let os = OsInfo {
        name: System::name().unwrap_or_default(),
        version: System::os_version().unwrap_or_default(),
        kernel: System::kernel_version().unwrap_or_default(),
        arch: std::env::consts::ARCH.to_string(),
        hostname: System::host_name().unwrap_or_default(),
    };

    let cpu = CpuInfo {
        brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        logical_cores: sys.cpus().len(),
        physical_cores: System::physical_core_count(),
    };

    let memory = MemoryInfo {
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
        used_bytes: sys.used_memory(),
    };

    let disks = Disks::new_with_refreshed_list();
    let storage = disks
        .iter()
        .map(|d| StorageInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_path_buf(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            file_system: d.file_system().to_string_lossy().to_string(),
        })
        .collect();

    Ok(HardwareInfo {
        os,
        cpu,
        memory,
        storage,
    })
}

/// Detects hardware information with optional CPU usage sampling.
///
/// This function is identical to [`detect_hardware`] but is designed
/// to support future CPU usage sampling (which requires ~200ms of
/// measurement time for accurate readings).
///
/// Currently, this function simply calls [`detect_hardware`] and returns
/// the same result. Future versions may add CPU usage statistics.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::hardware::detect_hardware_with_usage;
///
/// let hw = detect_hardware_with_usage().unwrap();
/// println!("CPU: {}", hw.cpu.brand);
/// ```
pub fn detect_hardware_with_usage() -> Result<HardwareInfo> {
    // For now, just call detect_hardware - can add CPU sampling later
    detect_hardware()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hardware_returns_valid_info() {
        let info = detect_hardware().unwrap();
        assert!(!info.os.name.is_empty());
        assert!(!info.os.arch.is_empty());
    }

    #[test]
    fn test_memory_values_are_reasonable() {
        let info = detect_hardware().unwrap();
        assert!(info.memory.total_bytes > 0);
        assert!(info.memory.total_bytes < u64::MAX);
        assert!(info.memory.available_bytes <= info.memory.total_bytes);
    }

    #[test]
    fn test_cpu_count_is_positive() {
        let info = detect_hardware().unwrap();
        assert!(info.cpu.logical_cores > 0);
    }

    #[test]
    fn test_storage_info_collected() {
        let info = detect_hardware().unwrap();
        // At least one disk should be present
        assert!(!info.storage.is_empty());
    }

    #[test]
    fn test_os_info_fields_populated() {
        let info = detect_hardware().unwrap();
        // OS name should be non-empty (macOS, Linux, etc.)
        assert!(!info.os.name.is_empty());
        // Architecture should be known
        assert!(!info.os.arch.is_empty());
    }
}
