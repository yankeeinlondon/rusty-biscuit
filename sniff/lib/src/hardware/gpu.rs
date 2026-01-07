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
    /// External GPU (eGPU via Thunderbolt)
    External,
    /// Unknown or undetected type
    #[default]
    Unknown,
}

impl std::fmt::Display for GpuDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuDeviceType::Discrete => write!(f, "Discrete"),
            GpuDeviceType::Integrated => write!(f, "Integrated"),
            GpuDeviceType::External => write!(f, "External"),
            GpuDeviceType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// GPU compute and graphics capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuCapabilities {
    /// Supports hardware ray tracing
    pub raytracing: bool,
    /// Supports 32-bit float texture filtering
    pub float32_filtering: bool,
    /// Supports dynamic libraries (shader compilation)
    pub dynamic_libraries: bool,
    /// Supports function pointers in shaders
    pub function_pointers: bool,
    /// Supports mesh shaders
    pub mesh_shaders: bool,
    /// Supports barycentric coordinates
    pub barycentric_coords: bool,
    /// Has unified memory architecture (CPU/GPU shared memory)
    pub unified_memory: bool,
}

/// GPU information.
///
/// Contains details about a detected GPU including name, vendor,
/// device type, graphics backend, memory, and capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name (e.g., "Apple M4 Max")
    pub name: String,
    /// GPU vendor (e.g., "Apple", "NVIDIA", "AMD", "Intel")
    pub vendor: Option<String>,
    /// Device type classification
    pub device_type: GpuDeviceType,
    /// Graphics backend (e.g., "Metal", "Vulkan", "D3D12")
    pub backend: String,
    /// GPU memory in bytes (recommended working set on Metal)
    pub memory_bytes: Option<u64>,
    /// Maximum buffer length in bytes
    pub max_buffer_bytes: Option<u64>,
    /// Whether GPU is headless (no display attached)
    pub is_headless: bool,
    /// Whether GPU is removable (eGPU)
    pub is_removable: bool,
    /// Unique registry/device ID
    pub registry_id: Option<u64>,
    /// Metal GPU family (e.g., "apple9", "mac2") - macOS only
    pub metal_family: Option<String>,
    /// GPU capabilities
    pub capabilities: GpuCapabilities,
}

/// Detects all available GPUs on the system.
///
/// Uses platform-specific APIs to enumerate GPU devices:
/// - macOS: Uses Metal API with full capability detection
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
///     if let Some(mem) = gpu.memory_bytes {
///         println!("  Memory: {} GB", mem / (1024 * 1024 * 1024));
///     }
///     println!("  Raytracing: {}", gpu.capabilities.raytracing);
///     println!("  Unified Memory: {}", gpu.capabilities.unified_memory);
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn detect_gpus() -> Vec<GpuInfo> {
    use metal::{Device, MTLGPUFamily};

    Device::all()
        .into_iter()
        .map(|device| {
            let name = device.name().to_string();
            let vendor = infer_vendor(&name);

            // Determine device type
            let device_type = if device.is_removable() {
                GpuDeviceType::External
            } else if device.is_low_power() {
                GpuDeviceType::Integrated
            } else {
                GpuDeviceType::Discrete
            };

            // Detect Metal GPU family
            let metal_family = detect_metal_family(&device);

            // Detect capabilities
            let capabilities = GpuCapabilities {
                raytracing: device.supports_raytracing(),
                float32_filtering: device.supports_32bit_float_filtering(),
                dynamic_libraries: device.supports_dynamic_libraries(),
                function_pointers: device.supports_function_pointers(),
                mesh_shaders: device.supports_family(MTLGPUFamily::Metal3),
                barycentric_coords: device.supports_shader_barycentric_coordinates(),
                unified_memory: device.has_unified_memory(),
            };

            GpuInfo {
                name,
                vendor,
                device_type,
                backend: "Metal".to_string(),
                memory_bytes: Some(device.recommended_max_working_set_size()),
                max_buffer_bytes: Some(device.max_buffer_length()),
                is_headless: device.is_headless(),
                is_removable: device.is_removable(),
                registry_id: Some(device.registry_id()),
                metal_family,
                capabilities,
            }
        })
        .collect()
}

/// Detects the highest supported Metal GPU family.
#[cfg(target_os = "macos")]
fn detect_metal_family(device: &metal::Device) -> Option<String> {
    use metal::MTLGPUFamily;

    // Check Apple Silicon families (newest first)
    let families = [
        (MTLGPUFamily::Apple9, "apple9"),
        (MTLGPUFamily::Apple8, "apple8"),
        (MTLGPUFamily::Apple7, "apple7"),
        (MTLGPUFamily::Apple6, "apple6"),
        (MTLGPUFamily::Apple5, "apple5"),
        (MTLGPUFamily::Apple4, "apple4"),
        (MTLGPUFamily::Apple3, "apple3"),
        (MTLGPUFamily::Apple2, "apple2"),
        (MTLGPUFamily::Apple1, "apple1"),
        // Mac families
        (MTLGPUFamily::Mac2, "mac2"),
        (MTLGPUFamily::Mac1, "mac1"),
        // Metal feature sets
        (MTLGPUFamily::Metal3, "metal3"),
    ];

    for (family, name) in families {
        if device.supports_family(family) {
            return Some(name.to_string());
        }
    }

    None
}

/// Infers the GPU vendor from the device name.
#[cfg(target_os = "macos")]
fn infer_vendor(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    if name_lower.contains("apple")
        || name_lower.starts_with("m1")
        || name_lower.starts_with("m2")
        || name_lower.starts_with("m3")
        || name_lower.starts_with("m4")
    {
        Some("Apple".to_string())
    } else if name_lower.contains("nvidia")
        || name_lower.contains("geforce")
        || name_lower.contains("quadro")
        || name_lower.contains("tesla")
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

/// Detects all available GPUs on the system.
///
/// Stub implementation for non-macOS platforms.
/// Returns an empty vector.
#[cfg(not(target_os = "macos"))]
pub fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
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
            memory_bytes: Some(8 * 1024 * 1024 * 1024),
            max_buffer_bytes: Some(4 * 1024 * 1024 * 1024),
            is_headless: false,
            is_removable: false,
            registry_id: Some(12345),
            metal_family: Some("apple9".to_string()),
            capabilities: GpuCapabilities {
                raytracing: true,
                float32_filtering: true,
                dynamic_libraries: true,
                function_pointers: true,
                mesh_shaders: true,
                barycentric_coords: true,
                unified_memory: true,
            },
        };

        let json = serde_json::to_string(&gpu).unwrap();
        let deserialized: GpuInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Test GPU");
        assert_eq!(deserialized.vendor, Some("TestVendor".to_string()));
        assert_eq!(deserialized.device_type, GpuDeviceType::Discrete);
        assert_eq!(deserialized.backend, "Vulkan");
        assert_eq!(deserialized.memory_bytes, Some(8 * 1024 * 1024 * 1024));
        assert!(deserialized.capabilities.raytracing);
        assert!(deserialized.capabilities.unified_memory);
    }

    #[test]
    fn test_gpu_device_type_display() {
        assert_eq!(GpuDeviceType::Discrete.to_string(), "Discrete");
        assert_eq!(GpuDeviceType::Integrated.to_string(), "Integrated");
        assert_eq!(GpuDeviceType::External.to_string(), "External");
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
        assert!(!default.capabilities.raytracing);
    }

    #[test]
    fn test_gpu_capabilities_default() {
        let caps: GpuCapabilities = Default::default();
        assert!(!caps.raytracing);
        assert!(!caps.unified_memory);
        assert!(!caps.mesh_shaders);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_infer_vendor() {
        assert_eq!(infer_vendor("Apple M1 Pro"), Some("Apple".to_string()));
        assert_eq!(infer_vendor("M2 Max"), Some("Apple".to_string()));
        assert_eq!(infer_vendor("M4 Max"), Some("Apple".to_string()));
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
        // On macOS, we should detect at least one GPU
        assert!(!gpus.is_empty(), "Expected at least one GPU on macOS");

        for gpu in &gpus {
            // Every GPU should have a name
            assert!(!gpu.name.is_empty());
            // Backend should be Metal on macOS
            assert_eq!(gpu.backend, "Metal");
            // Memory should be reported
            assert!(gpu.memory_bytes.is_some());
            // Max buffer length should be reported
            assert!(gpu.max_buffer_bytes.is_some());
            // Registry ID should be available
            assert!(gpu.registry_id.is_some());
            // Metal family should be detected
            assert!(gpu.metal_family.is_some());
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_apple_silicon_capabilities() {
        let gpus = detect_gpus();
        // Find Apple Silicon GPU if present
        if let Some(apple_gpu) = gpus.iter().find(|g| {
            g.vendor.as_deref() == Some("Apple")
        }) {
            // Apple Silicon has unified memory
            assert!(
                apple_gpu.capabilities.unified_memory,
                "Apple Silicon should have unified memory"
            );
        }
    }
}
