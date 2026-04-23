use serde::{Deserialize, Serialize};
use std::time::Instant;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing::Level;
use tracing::instrument;

use crate::Result;
use crate::performance;
use crate::request::HardwareRequest;

mod audio;
mod cpu;
mod gpu;
mod memory;
mod storage;

pub use audio::{AudioDeviceInfo, AudioDeviceKind, AudioDirection, detect_audio_devices};
pub use cpu::{CpuInfo, SimdCapabilities, detect_simd};
pub use gpu::{GpuCapabilities, GpuDeviceType, GpuInfo, detect_gpus};
pub use memory::MemoryInfo;
pub use storage::{StorageInfo, StorageKind, detect_storage};

// Re-export OS types from the dedicated os module for backward compatibility.
// The canonical path is now `sniff::os::*`.
#[doc(inline)]
pub use crate::os::{
    DetectedPackageManager, LinuxDistro, LinuxFamily, LocaleInfo, NtpStatus, OsInfo, OsType,
    PackageManagerCommands, SystemPackageManager, SystemPackageManagers, TimeInfo,
    command_exists_in_path, detect_bsd_package_managers, detect_linux_distro,
    detect_linux_package_managers, detect_locale, detect_macos_package_managers, detect_ntp_status,
    detect_os, detect_os_type, detect_os_with_request, detect_timezone,
    detect_timezone_with_options, detect_windows_package_managers, extract_encoding,
    extract_language_code, get_commands_for_manager, get_path_dirs, infer_linux_family,
    parse_lsb_release_content, parse_os_release_content, parse_system_release_content,
};

/// Complete hardware information.
///
/// Aggregates CPU, memory, storage, GPU, and audio device information
/// detected from the current system. OS information is available separately
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
    /// Audio devices (inputs and outputs)
    pub audio_devices: Vec<AudioDeviceInfo>,
}

/// Detect hardware information according to the given request.
///
/// Use `HardwareRequest::summary()` for CPU and memory only (fast),
/// or `HardwareRequest::full()` for everything including storage,
/// GPU, and audio devices.
#[instrument(skip(request), fields(
    storage = request.include_storage,
    gpu = request.include_gpu,
    audio = request.include_audio,
))]
pub fn detect_hardware_with_request(request: &HardwareRequest) -> Result<HardwareInfo> {
    let core_started = Instant::now();
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
    performance::record_logged_stage("hardware.core", core_started.elapsed(), Level::DEBUG);

    let collector = performance::current_collector();
    let (audio_devices, storage, gpu) = std::thread::scope(|scope| {
        let audio_handle = request.include_audio.then(|| {
            let collector = collector.clone();
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let audio_started = Instant::now();
                    let devices = detect_audio_devices();
                    performance::record_logged_stage(
                        "hardware.audio",
                        audio_started.elapsed(),
                        Level::DEBUG,
                    );
                    devices
                })
            })
        });

        let storage_handle = request.include_storage.then(|| {
            let collector = collector.clone();
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let storage_started = Instant::now();
                    let devices = storage::detect_storage();
                    performance::record_logged_stage(
                        "hardware.storage",
                        storage_started.elapsed(),
                        Level::DEBUG,
                    );
                    devices
                })
            })
        });

        // Run audio, storage, and GPU in parallel on all platforms.
        //
        // Historically on macOS, audio initialization (CoreAudio) was sequenced
        // before GPU detection because Metal framework initialization (~14s in
        // non-GUI contexts) interacted badly with CoreAudio. However, GPU
        // detection now uses IOKit FFI (~200µs), which does not conflict with
        // CoreAudio. The IOKit path was introduced to avoid the Metal delay and
        // is safe to run concurrently with audio enumeration.
        //
        // If issues arise, set the environment variable `SNIFF_SERIALIZE_MACOS_GPU=1`
        // to restore the old audio-first ordering.
        #[cfg(target_os = "macos")]
        let serialize_gpu = std::env::var("SNIFF_SERIALIZE_MACOS_GPU").is_ok();
        #[cfg(not(target_os = "macos"))]
        let serialize_gpu = false;

        let gpu_handle = if serialize_gpu {
            // Legacy path: block on audio before starting GPU.
            // We must join audio here because ScopedJoinHandle::join takes ownership.
            let audio_devices = match audio_handle {
                Some(handle) => handle.join().unwrap(),
                None => Vec::new(),
            };
            let gpu = request.include_gpu.then(|| {
                let collector = collector.clone();
                scope.spawn(move || {
                    performance::with_current_collector(collector, || {
                        let gpu_started = Instant::now();
                        let devices = detect_gpus();
                        performance::record_logged_stage(
                            "hardware.gpu",
                            gpu_started.elapsed(),
                            Level::DEBUG,
                        );
                        devices
                    })
                })
            });
            let storage = match storage_handle {
                Some(handle) => handle.join().unwrap(),
                None => Vec::new(),
            };
            let gpu = match gpu {
                Some(handle) => handle.join().unwrap(),
                None => Vec::new(),
            };
            return (audio_devices, storage, gpu);
        } else {
            // Parallel path: GPU runs concurrently with audio and storage
            request.include_gpu.then(|| {
                let collector = collector.clone();
                scope.spawn(move || {
                    performance::with_current_collector(collector, || {
                        let gpu_started = Instant::now();
                        let devices = detect_gpus();
                        performance::record_logged_stage(
                            "hardware.gpu",
                            gpu_started.elapsed(),
                            Level::DEBUG,
                        );
                        devices
                    })
                })
            })
        };

        let audio_devices = match audio_handle {
            Some(handle) => handle.join().unwrap(),
            None => Vec::new(),
        };
        let storage = match storage_handle {
            Some(handle) => handle.join().unwrap(),
            None => Vec::new(),
        };
        let gpu = match gpu_handle {
            Some(handle) => handle.join().unwrap(),
            None => Vec::new(),
        };

        (audio_devices, storage, gpu)
    });

    Ok(HardwareInfo {
        cpu,
        memory,
        storage,
        gpu,
        audio_devices,
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
/// use sniff::hardware::detect_hardware;
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
    detect_hardware_with_request(&HardwareRequest::full())
}

/// Lightweight hardware summary for compose context.
///
/// Returns only CPU (cores, arch) and memory information.
/// Skips audio device enumeration (~1.5s on macOS), storage inventory,
/// and GPU detection.
pub fn detect_hardware_summary() -> Result<HardwareInfo> {
    detect_hardware_with_request(&HardwareRequest::summary())
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

    #[test]
    fn test_detect_hardware_summary_skips_expensive() {
        let info = detect_hardware_with_request(&HardwareRequest::summary()).unwrap();
        assert!(info.cpu.logical_cores > 0);
        assert!(info.memory.total_bytes > 0);
        assert!(info.storage.is_empty());
        assert!(info.gpu.is_empty());
        assert!(info.audio_devices.is_empty());
    }

    #[test]
    fn test_detect_hardware_full_includes_gpu_on_macos() {
        // On macOS, GPU detection should run in parallel with audio/storage
        // and return results when full detection is requested.
        let info = detect_hardware_with_request(&HardwareRequest::full()).unwrap();

        // CPU and memory should always be present
        assert!(info.cpu.logical_cores > 0);
        assert!(info.memory.total_bytes > 0);

        // On macOS, GPU should be detected (even if empty, it should not hang)
        #[cfg(target_os = "macos")]
        {
            // The parallel path should complete without deadlock.
            // We can't assert gpu.len() > 0 because VMs may not expose GPUs,
            // but we can verify the function returned (no deadlock).
        }
    }

    #[test]
    fn test_detect_hardware_with_request_respects_flags() {
        let with_all = HardwareRequest::full();
        let with_none = HardwareRequest::summary();

        let all = detect_hardware_with_request(&with_all).unwrap();
        let none = detect_hardware_with_request(&with_none).unwrap();

        // Summary should have CPU and memory but no storage/GPU/audio
        assert!(none.cpu.logical_cores > 0);
        assert!(none.memory.total_bytes > 0);
        assert!(none.storage.is_empty());
        assert!(none.gpu.is_empty());
        assert!(none.audio_devices.is_empty());

        // Full may have storage, GPU, audio (platform-dependent)
        assert!(all.cpu.logical_cores > 0);
        assert!(all.memory.total_bytes > 0);
    }
}
