//! GPU detection module.
//!
//! This module provides GPU detection capabilities using platform-specific
//! APIs. On macOS, it uses the Metal API for GPU enumeration. On other
//! platforms, it returns an empty result.

use serde::{Deserialize, Serialize};

/// GPU device type classification.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuDeviceType {
    /// Discrete GPU (dedicated graphics card)
    Discrete,
    /// Integrated GPU (built into CPU)
    Integrated,
    /// Workstation GPU (Quadro, RadeonPro, etc.)
    Workstation,
    /// Datacenter GPU (Tesla, Instinct, etc.)
    Datacenter,
    /// Virtual GPU
    Virtual,
    /// Unknown or undetected type
    #[default]
    Unknown,
}

impl std::fmt::Display for GpuDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuDeviceType::Discrete => write!(f, "Discrete"),
            GpuDeviceType::Integrated => write!(f, "Integrated"),
            GpuDeviceType::Workstation => write!(f, "Workstation"),
            GpuDeviceType::Datacenter => write!(f, "Datacenter"),
            GpuDeviceType::Virtual => write!(f, "Virtual"),
            GpuDeviceType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// GPU information.
///
/// Contains details about a detected GPU including name, vendor,
/// device type, graphics backend, and available memory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name (e.g., "Apple M1 Pro")
    pub name: String,
    /// GPU vendor (e.g., "Apple", "NVIDIA", "AMD", "Intel")
    pub vendor: Option<String>,
    /// Device type classification
    pub device_type: GpuDeviceType,
    /// Graphics backend (e.g., "Metal", "Vulkan", "D3D12")
    pub backend: String,
    /// GPU memory in bytes (None if unavailable)
    pub memory_bytes: Option<u64>,
}

/// Detects all available GPUs on the system.
///
/// Uses platform-specific APIs to enumerate GPU devices:
/// - macOS: Uses Metal API
/// - Other platforms: Returns empty vector (future: add Vulkan/D3D12 support)
///
/// Returns an empty vector if no GPUs are detected or if detection fails.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::hardware::detect_gpus;
///
/// let gpus = detect_gpus();
/// for gpu in &gpus {
///     println!("GPU: {} ({:?})", gpu.name, gpu.device_type);
/// }
/// ```
///
/// ## Notes
///
/// GPU detection may be limited by:
/// - Platform-specific driver availability
/// - Permission restrictions
/// - Running in virtualized environments
#[cfg(target_os = "macos")]
pub fn detect_gpus() -> Vec<GpuInfo> {
    use metal::Device;

    Device::all()
        .into_iter()
        .map(|device| {
            let name = device.name().to_string();
            let vendor = infer_vendor(&name);
            let device_type = if device.is_low_power() {
                GpuDeviceType::Integrated
            } else {
                GpuDeviceType::Discrete
            };
            let memory_bytes = Some(device.recommended_max_working_set_size());

            GpuInfo {
                name,
                vendor,
                device_type,
                backend: "Metal".to_string(),
                memory_bytes,
            }
        })
        .collect()
}

/// Detects all available GPUs on the system.
///
/// Stub implementation for non-macOS platforms.
/// Returns an empty vector.
#[cfg(not(target_os = "macos"))]
pub fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

/// Infers the GPU vendor from the device name.
#[cfg(target_os = "macos")]
fn infer_vendor(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    if name_lower.contains("apple") || name_lower.starts_with("m1") || name_lower.starts_with("m2")
        || name_lower.starts_with("m3") || name_lower.starts_with("m4")
    {
        Some("Apple".to_string())
    } else if name_lower.contains("nvidia") || name_lower.contains("geforce")
        || name_lower.contains("quadro") || name_lower.contains("tesla")
    {
        Some("NVIDIA".to_string())
    } else if name_lower.contains("amd") || name_lower.contains("radeon") {
        Some("AMD".to_string())
    } else if name_lower.contains("intel") {
        Some("Intel".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gpus_returns_vec() {
        // Should return a Vec (may be empty on some systems)
        let gpus = detect_gpus();
        // Just verify it doesn't panic and returns a valid Vec
        let _ = gpus.len();
    }

    #[test]
    fn test_gpu_info_serialization() {
        let gpu = GpuInfo {
            name: "Test GPU".to_string(),
            vendor: Some("TestVendor".to_string()),
            device_type: GpuDeviceType::Discrete,
            backend: "Vulkan".to_string(),
            memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8 GB
        };

        let json = serde_json::to_string(&gpu).unwrap();
        let deserialized: GpuInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Test GPU");
        assert_eq!(deserialized.vendor, Some("TestVendor".to_string()));
        assert_eq!(deserialized.device_type, GpuDeviceType::Discrete);
        assert_eq!(deserialized.backend, "Vulkan");
        assert_eq!(deserialized.memory_bytes, Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_gpu_device_type_display() {
        assert_eq!(GpuDeviceType::Discrete.to_string(), "Discrete");
        assert_eq!(GpuDeviceType::Integrated.to_string(), "Integrated");
        assert_eq!(GpuDeviceType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_gpu_device_type_default() {
        let default: GpuDeviceType = Default::default();
        assert_eq!(default, GpuDeviceType::Unknown);
    }

    #[test]
    fn test_gpu_info_default() {
        let default: GpuInfo = Default::default();
        assert!(default.name.is_empty());
        assert!(default.vendor.is_none());
        assert_eq!(default.device_type, GpuDeviceType::Unknown);
        assert!(default.backend.is_empty());
        assert!(default.memory_bytes.is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_infer_vendor() {
        assert_eq!(infer_vendor("Apple M1 Pro"), Some("Apple".to_string()));
        assert_eq!(infer_vendor("M2 Max"), Some("Apple".to_string()));
        assert_eq!(
            infer_vendor("NVIDIA GeForce RTX 3080"),
            Some("NVIDIA".to_string())
        );
        assert_eq!(
            infer_vendor("AMD Radeon Pro 5500M"),
            Some("AMD".to_string())
        );
        assert_eq!(
            infer_vendor("Intel UHD Graphics 630"),
            Some("Intel".to_string())
        );
        assert_eq!(infer_vendor("Unknown GPU"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_detect_gpus_on_macos() {
        let gpus = detect_gpus();
        // On macOS, we should detect at least one GPU (the integrated one)
        assert!(!gpus.is_empty(), "Expected at least one GPU on macOS");

        for gpu in &gpus {
            // Every GPU should have a name
            assert!(!gpu.name.is_empty());
            // Backend should be Metal on macOS
            assert_eq!(gpu.backend, "Metal");
            // Memory should be reported
            assert!(gpu.memory_bytes.is_some());
        }
    }
}
