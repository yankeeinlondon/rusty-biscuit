use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::Result;

mod cpu;
mod gpu;
mod memory;
mod os;
mod storage;

pub use cpu::{detect_simd, CpuInfo, SimdCapabilities};
pub use gpu::{detect_gpus, GpuCapabilities, GpuDeviceType, GpuInfo};
pub use memory::MemoryInfo;
pub use os::{
    command_exists_in_path, detect_bsd_package_managers, detect_linux_distro,
    detect_linux_package_managers, detect_locale, detect_macos_package_managers,
    detect_ntp_status, detect_os_type, detect_timezone, detect_windows_package_managers,
    extract_encoding, extract_language_code, get_commands_for_manager, get_path_dirs,
    infer_linux_family, parse_lsb_release_content, parse_os_release_content,
    parse_system_release_content, DetectedPackageManager, LinuxDistro, LinuxFamily, LocaleInfo,
    NtpStatus, OsInfo, OsType, PackageManagerCommands, SystemPackageManager, SystemPackageManagers,
    TimeInfo,
};
pub use storage::{StorageInfo, StorageKind};

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
    pub gpus: Vec<GpuInfo>,
}

/// Detects operating system information from the current system.
///
/// This function gathers OS type, distribution details, package managers,
/// locale, and timezone information.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::hardware::detect_os;
///
/// let os = detect_os().unwrap();
/// println!("OS: {} {}", os.name, os.version);
/// println!("Arch: {}", os.arch);
/// println!("Hostname: {}", os.hostname);
/// ```
///
/// ## Errors
///
/// Currently returns `Ok` in all cases, but future versions may return
/// errors for system information gathering failures.
pub fn detect_os() -> Result<OsInfo> {
    // Helper to convert empty strings to None
    let non_empty = |s: String| if s.is_empty() { None } else { Some(s) };

    let os_type = detect_os_type();
    let linux_distro = detect_linux_distro();
    // Extract linux family before moving linux_distro into OsInfo
    let linux_family = linux_distro.as_ref().map(|d| d.family);

    // Detect system package managers based on OS type
    let system_package_managers = match os_type {
        OsType::Linux => Some(detect_linux_package_managers(linux_family)),
        OsType::MacOS => Some(detect_macos_package_managers()),
        OsType::Windows => Some(detect_windows_package_managers()),
        OsType::FreeBSD | OsType::OpenBSD | OsType::NetBSD => {
            Some(detect_bsd_package_managers(os_type))
        }
        OsType::IOS | OsType::Android | OsType::Other => None,
    };

    // Detect locale settings
    let locale = Some(detect_locale());

    // Detect timezone and time information
    let time = Some(detect_timezone());

    Ok(OsInfo {
        os_type,
        name: System::name().unwrap_or_default(),
        version: System::os_version().unwrap_or_default(),
        long_version: System::long_os_version(),
        distribution: non_empty(System::distribution_id()),
        linux_distro,
        kernel: System::kernel_version().unwrap_or_default(),
        arch: {
            let arch = System::cpu_arch();
            if arch.is_empty() {
                std::env::consts::ARCH.to_string()
            } else {
                arch
            }
        },
        hostname: System::host_name().unwrap_or_default(),
        system_package_managers,
        locale,
        time,
    })
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
/// println!("GPUs: {}", hw.gpus.len());
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
        logical_cores: sys.cpus().len(),
        physical_cores: System::physical_core_count(),
        simd: detect_simd(),
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
            kind: match d.kind() {
                DiskKind::SSD => StorageKind::Ssd,
                DiskKind::HDD => StorageKind::Hdd,
                DiskKind::Unknown(_) => StorageKind::Unknown,
            },
            is_removable: d.is_removable(),
        })
        .collect();

    let gpus = detect_gpus();

    Ok(HardwareInfo {
        cpu,
        memory,
        storage,
        gpus,
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
        // Architecture should be known
        assert!(!os.arch.is_empty());
    }
}
