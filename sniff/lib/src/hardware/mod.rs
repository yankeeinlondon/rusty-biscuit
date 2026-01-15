use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::Result;

mod cpu;
mod gpu;
mod memory;
mod storage;

pub use cpu::{CpuInfo, SimdCapabilities, detect_simd};
pub use gpu::{GpuCapabilities, GpuDeviceType, GpuInfo, detect_gpus};
pub use memory::MemoryInfo;
pub use storage::{StorageInfo, StorageKind};

// Re-export OS types from the dedicated os module for backward compatibility.
// The canonical path is now `sniff_lib::os::*`.
#[doc(inline)]
pub use crate::os::{
    DetectedPackageManager, LinuxDistro, LinuxFamily, LocaleInfo, NtpStatus, OsInfo, OsType,
    PackageManagerCommands, SystemPackageManager, SystemPackageManagers, TimeInfo,
    command_exists_in_path, detect_bsd_package_managers, detect_linux_distro,
    detect_linux_package_managers, detect_locale, detect_macos_package_managers, detect_ntp_status,
    detect_os, detect_os_type, detect_timezone, detect_windows_package_managers, extract_encoding,
    extract_language_code, get_commands_for_manager, get_path_dirs, infer_linux_family,
    parse_lsb_release_content, parse_os_release_content, parse_system_release_content,
};

/// Complete hardware information.
///
/// Aggregates CPU, memory, storage, and GPU information detected
/// from the current system. OS information is available separately
/// via the top-level `os` field in `SniffResult`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU information
    pub cpu: CpuInfo,
    /// Memory information
    pub memory: MemoryInfo,
    /// Storage devices (disks)
    pub storage: Vec<StorageInfo>,
    /// GPU devices
    pub gpu: Vec<GpuInfo>,
}

/// Detects hardware information from the current system.
///
/// This function gathers CPU specifications, memory statistics,
/// storage information, and GPU devices. For OS information,
/// use `detect_os()` separately.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::hardware::detect_hardware;
///
/// let hw = detect_hardware().unwrap();
/// println!("CPU: {} ({} cores)", hw.cpu.brand, hw.cpu.logical_cores);
/// println!("Memory: {} GB total", hw.memory.total_bytes / (1024 * 1024 * 1024));
/// println!("GPUs: {}", hw.gpu.len());
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

    let cpu = CpuInfo {
        brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        arch: {
            let arch = System::cpu_arch();
            if arch.is_empty() {
                std::env::consts::ARCH.to_string()
            } else {
                arch
            }
        },
        logical_cores: sys.cpus().len(),
        physical_cores: System::physical_core_count(),
        simd: detect_simd(),
    };

    // On macOS (and possibly other platforms), available_memory() may return 0.
    // In this case, fall back to free_memory() which provides usable memory info.
    let available = sys.available_memory();
    let available_bytes = if available == 0 {
        sys.free_memory()
    } else {
        available
    };

    let memory = MemoryInfo {
        total_bytes: sys.total_memory(),
        available_bytes,
        used_bytes: sys.used_memory(),
        total_swap: sys.total_swap(),
        free_swap: sys.free_swap(),
        used_swap: sys.used_swap(),
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
            kind: match d.kind() {
                DiskKind::SSD => StorageKind::Ssd,
                DiskKind::HDD => StorageKind::Hdd,
                DiskKind::Unknown(_) => StorageKind::Unknown,
            },
            is_removable: d.is_removable(),
        })
        .collect();

    let gpu = detect_gpus();

    Ok(HardwareInfo {
        cpu,
        memory,
        storage,
        gpu,
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
        assert!(info.cpu.logical_cores > 0);
        assert!(info.memory.total_bytes > 0);
    }

    #[test]
    fn test_memory_values_are_reasonable() {
        let info = detect_hardware().unwrap();
        assert!(info.memory.total_bytes > 0);
        assert!(info.memory.total_bytes < u64::MAX);
        assert!(info.memory.available_bytes <= info.memory.total_bytes);
    }

    #[test]
    fn test_available_bytes_is_not_zero() {
        // Regression test: available_bytes should never be 0 on a running system
        // This catches the macOS bug where sysinfo's available_memory() returns 0
        let info = detect_hardware().unwrap();
        assert_ne!(
            info.memory.available_bytes, 0,
            "available_bytes should not be 0 on a running system (total: {}, used: {})",
            info.memory.total_bytes, info.memory.used_bytes
        );
    }

    #[test]
    fn test_memory_accounting_is_valid() {
        // Regression test: verify memory values have sensible relationships
        let info = detect_hardware().unwrap();

        // Available memory should be non-zero
        assert!(
            info.memory.available_bytes > 0,
            "available_bytes should be > 0 (got {})",
            info.memory.available_bytes
        );

        // Available memory should be less than or equal to total
        assert!(
            info.memory.available_bytes <= info.memory.total_bytes,
            "available_bytes ({}) should be <= total_bytes ({})",
            info.memory.available_bytes,
            info.memory.total_bytes
        );

        // Used memory should be less than or equal to total
        assert!(
            info.memory.used_bytes <= info.memory.total_bytes,
            "used_bytes ({}) should be <= total_bytes ({})",
            info.memory.used_bytes,
            info.memory.total_bytes
        );

        // The sum of available and used can exceed total due to caching/buffers,
        // but both should be reasonable values
        assert!(
            info.memory.available_bytes < u64::MAX / 2,
            "available_bytes suspiciously large: {}",
            info.memory.available_bytes
        );
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
        let os = detect_os().unwrap();
        // OS name should be non-empty (macOS, Linux, etc.)
        assert!(!os.name.is_empty());
    }

    #[test]
    fn test_cpu_arch_populated() {
        let hw = detect_hardware().unwrap();
        // CPU architecture should be known
        assert!(!hw.cpu.arch.is_empty());
    }
}
