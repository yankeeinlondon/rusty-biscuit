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
    /// Number of GPU cores (if detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_count: Option<u32>,
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
/// use sniff::hardware::detect_gpus;
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
/// Detects GPUs on macOS using IOKit (via `ioreg`).
///
/// Metal framework initialization (`MTLCreateSystemDefaultDevice` /
/// `MTLCopyAllDevices`) can block for 14+ seconds in non-GUI contexts
/// (test runners, SSH sessions, headless CI). IOKit returns the same
/// model name and core count in ~25 ms.
#[cfg(target_os = "macos")]
pub fn detect_gpus() -> Vec<GpuInfo> {
    detect_gpus_iokit().unwrap_or_default()
}

/// Queries IOKit directly for GPU info, avoiding a subprocess spawn.
///
/// Reads the `IOAccelerator` IOService entries for `model` and
/// `gpu-core-count` properties — the same data `ioreg -rd1 -c IOAccelerator`
/// returns, but via direct FFI (~200µs vs ~22ms for process spawn).
#[cfg(target_os = "macos")]
fn detect_gpus_iokit() -> Option<Vec<GpuInfo>> {
    use core_foundation::base::{CFType, TCFType, kCFAllocatorDefault};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    type CFMutableDictionaryRef = *mut std::ffi::c_void;
    type CFAllocatorRef = *const std::ffi::c_void;

    // IOKit FFI — these are stable C APIs, not Metal framework
    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const std::ffi::c_char) -> CFMutableDictionaryRef;
        fn IOServiceGetMatchingServices(
            main_port: u32,
            matching: CFMutableDictionaryRef,
            existing: *mut u32,
        ) -> i32;
        fn IOIteratorNext(iterator: u32) -> u32;
        fn IORegistryEntryCreateCFProperties(
            entry: u32,
            properties: *mut CFMutableDictionaryRef,
            allocator: CFAllocatorRef,
            options: u32,
        ) -> i32;
        fn IOObjectRelease(object: u32) -> i32;
    }

    unsafe {
        let class_name = std::ffi::CString::new("IOAccelerator").ok()?;
        let matching = IOServiceMatching(class_name.as_ptr());
        if matching.is_null() {
            return None;
        }

        let mut iterator: u32 = 0;
        let kr = IOServiceGetMatchingServices(0, matching, &mut iterator);
        if kr != 0 || iterator == 0 {
            return None;
        }

        let mut gpus = Vec::new();

        loop {
            let entry = IOIteratorNext(iterator);
            if entry == 0 {
                break;
            }

            let mut props_ref: CFMutableDictionaryRef = std::ptr::null_mut();
            let kr = IORegistryEntryCreateCFProperties(
                entry,
                &mut props_ref,
                kCFAllocatorDefault as CFAllocatorRef,
                0,
            );

            if kr == 0 && !props_ref.is_null() {
                let props: CFDictionary<CFString, CFType> =
                    CFDictionary::wrap_under_create_rule(props_ref as _);

                let name = props
                    .find(CFString::new("model"))
                    .and_then(|v| v.downcast::<CFString>())
                    .map(|s| s.to_string());

                let core_count = props
                    .find(CFString::new("gpu-core-count"))
                    .and_then(|v| v.downcast::<CFNumber>())
                    .and_then(|n| n.to_i64())
                    .map(|n| n as u32);

                if let Some(name) = name {
                    let vendor = infer_vendor(&name);
                    let is_apple_silicon = vendor.as_deref() == Some("Apple");

                    gpus.push(GpuInfo {
                        name,
                        vendor,
                        device_type: if is_apple_silicon {
                            GpuDeviceType::Integrated
                        } else {
                            GpuDeviceType::Unknown
                        },
                        backend: "Metal".to_string(),
                        memory_bytes: None,
                        max_buffer_bytes: None,
                        is_headless: false,
                        is_removable: false,
                        registry_id: None,
                        metal_family: None,
                        capabilities: GpuCapabilities {
                            unified_memory: is_apple_silicon,
                            ..Default::default()
                        },
                        core_count,
                    });
                }
            }

            IOObjectRelease(entry);
        }

        IOObjectRelease(iterator);
        Some(gpus)
    }
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
            core_count: Some(40),
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

    /// Whether this process runs inside a hypervisor guest.
    ///
    /// `kern.hv_vmm_present` is the canonical macOS "am I virtualized" probe;
    /// it reads 1 on Apple Virtualization guests (GitHub macOS CI runners).
    #[cfg(target_os = "macos")]
    fn running_in_vm() -> bool {
        let name = std::ffi::CString::new("kern.hv_vmm_present").expect("static sysctl name");
        let mut value: i32 = 0;
        let mut size = std::mem::size_of::<i32>();
        let rc = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                (&raw mut value).cast(),
                &raw mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        rc == 0 && value != 0
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_detect_gpus_on_macos() {
        let gpus = detect_gpus();
        // Virtualized macOS guests expose a paravirtual display with no
        // `IOAccelerator` service, so zero GPUs is a truthful answer there.
        // Real hardware keeps the non-empty requirement.
        if !running_in_vm() {
            assert!(!gpus.is_empty(), "Expected at least one GPU on macOS");
        }

        for gpu in &gpus {
            // Every GPU should have a name
            assert!(!gpu.name.is_empty());
            // Backend should be Metal on macOS
            assert_eq!(gpu.backend, "Metal");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_apple_silicon_capabilities() {
        let gpus = detect_gpus();
        // Find Apple Silicon GPU if present
        if let Some(apple_gpu) = gpus.iter().find(|g| g.vendor.as_deref() == Some("Apple")) {
            // Apple Silicon has unified memory
            assert!(
                apple_gpu.capabilities.unified_memory,
                "Apple Silicon should have unified memory"
            );
        }
    }
}
