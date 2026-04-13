//! Hardware section output formatting.

use std::fmt::Write;
use std::path::Path;

use super::{format_bytes, relative_path};

/// Format SIMD capabilities into a comma-separated string.
///
/// Returns `None` if no capabilities are detected.
/// Uses hierarchical display: shows highest AVX level (512 > AVX2 > AVX).
fn format_simd_caps(simd: &sniff::hardware::SimdCapabilities) -> Option<String> {
    let mut caps = Vec::new();
    // AVX hierarchy: show highest level only
    if simd.avx512f {
        caps.push("AVX-512");
    } else if simd.avx2 {
        caps.push("AVX2");
    } else if simd.avx {
        caps.push("AVX");
    }
    if simd.sse4_2 {
        caps.push("SSE4.2");
    }
    if simd.fma {
        caps.push("FMA");
    }
    if simd.neon {
        caps.push("NEON");
    }
    if caps.is_empty() {
        None
    } else {
        Some(caps.join(", "))
    }
}

/// Format GPU capabilities into a comma-separated string.
///
/// Returns `None` if no capabilities are detected.
fn format_gpu_caps(caps: &sniff::hardware::GpuCapabilities) -> Option<String> {
    let mut cap_list = Vec::new();
    if caps.raytracing {
        cap_list.push("Raytracing");
    }
    if caps.mesh_shaders {
        cap_list.push("Mesh Shaders");
    }
    if caps.unified_memory {
        cap_list.push("Unified Memory");
    }
    if caps.dynamic_libraries {
        cap_list.push("Dynamic Libraries");
    }
    if cap_list.is_empty() {
        None
    } else {
        Some(cap_list.join(", "))
    }
}

pub fn render_hardware_section(
    hardware: &sniff::HardwareInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) -> String {
    let mut out = String::new();

    writeln!(out, "=== Hardware ===").unwrap();

    writeln!(
        out,
        "CPU: {} ({} logical cores)",
        hardware.cpu.brand, hardware.cpu.logical_cores
    )
    .unwrap();
    writeln!(out, "Architecture: {}", hardware.cpu.arch).unwrap();
    if let Some(physical) = hardware.cpu.physical_cores {
        writeln!(out, "Physical cores: {}", physical).unwrap();
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0
        && let Some(simd_str) = format_simd_caps(&hardware.cpu.simd)
    {
        writeln!(out, "SIMD: {}", simd_str).unwrap();
    }
    writeln!(out).unwrap();

    writeln!(out, "Memory:").unwrap();
    writeln!(
        out,
        "  Total: {}",
        format_bytes(hardware.memory.total_bytes)
    )
    .unwrap();
    writeln!(
        out,
        "  Available: {}",
        format_bytes(hardware.memory.available_bytes)
    )
    .unwrap();
    writeln!(out, "  Used: {}", format_bytes(hardware.memory.used_bytes)).unwrap();
    if hardware.memory.total_swap > 0 {
        writeln!(
            out,
            "  Swap: {} total, {} used",
            format_bytes(hardware.memory.total_swap),
            format_bytes(hardware.memory.used_swap)
        )
        .unwrap();
    }
    writeln!(out).unwrap();

    // Print GPU info if available
    if !hardware.gpu.is_empty() {
        writeln!(out, "GPUs:").unwrap();
        for gpu in &hardware.gpu {
            let vendor_str = gpu.vendor.as_deref().unwrap_or("Unknown");
            writeln!(out, "  {} ({}, {})", gpu.name, vendor_str, gpu.backend).unwrap();
            if verbose > 0 {
                if let Some(mem) = gpu.memory_bytes {
                    writeln!(out, "    Memory: {}", format_bytes(mem)).unwrap();
                }
                writeln!(out, "    Type: {:?}", gpu.device_type).unwrap();
                if let Some(ref family) = gpu.metal_family {
                    writeln!(out, "    Metal Family: {}", family).unwrap();
                }
                if gpu.is_headless {
                    writeln!(out, "    Headless: yes").unwrap();
                }
                if gpu.is_removable {
                    writeln!(out, "    Removable: yes (eGPU)").unwrap();
                }
            }
            if verbose > 1 {
                // Show capabilities at -vv
                if let Some(caps_str) = format_gpu_caps(&gpu.capabilities) {
                    writeln!(out, "    Capabilities: {}", caps_str).unwrap();
                }
                if let Some(max_buf) = gpu.max_buffer_bytes {
                    writeln!(out, "    Max Buffer: {}", format_bytes(max_buf)).unwrap();
                }
            }
        }
        writeln!(out).unwrap();
    }

    // Print audio devices if available
    if !hardware.audio_devices.is_empty() {
        writeln!(out, "Audio Devices:").unwrap();
        out.push_str(&render_audio_device_list(&hardware.audio_devices, verbose));
        writeln!(out).unwrap();
    }

    writeln!(out, "Storage:").unwrap();
    for disk in &hardware.storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff::hardware::StorageKind::Ssd => "SSD",
            sniff::hardware::StorageKind::Hdd => "HDD",
            sniff::hardware::StorageKind::Unknown => "",
        };
        if kind_str.is_empty() {
            writeln!(out, "  {} ({})", mount_str, disk.file_system).unwrap();
        } else {
            writeln!(out, "  {} ({}, {})", mount_str, disk.file_system, kind_str).unwrap();
        }
        if verbose > 0 {
            writeln!(out, "    Total: {}", format_bytes(disk.total_bytes)).unwrap();
            writeln!(out, "    Available: {}", format_bytes(disk.available_bytes)).unwrap();
            if disk.is_removable {
                writeln!(out, "    Removable: yes").unwrap();
            }
        }
    }
    writeln!(out).unwrap();

    out
}

// ============================================================================
// Subsection print functions (for --cpu, --gpu, --memory, --storage filters)
// ============================================================================

pub fn render_cpu_section(cpu: &sniff::hardware::CpuInfo, verbose: u8) -> String {
    let mut out = String::new();

    writeln!(out, "=== CPU ===").unwrap();
    writeln!(out, "Brand: {}", cpu.brand).unwrap();
    writeln!(out, "Architecture: {}", cpu.arch).unwrap();
    writeln!(out, "Logical cores: {}", cpu.logical_cores).unwrap();
    if let Some(physical) = cpu.physical_cores {
        writeln!(out, "Physical cores: {}", physical).unwrap();
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0
        && let Some(simd_str) = format_simd_caps(&cpu.simd)
    {
        writeln!(out, "SIMD: {}", simd_str).unwrap();
    }

    writeln!(out).unwrap();

    out
}

pub fn render_gpu_section(gpus: &[sniff::hardware::GpuInfo], verbose: u8) -> String {
    let mut out = String::new();

    writeln!(out, "=== GPU ===").unwrap();
    if gpus.is_empty() {
        writeln!(out, "No GPUs detected").unwrap();
    } else {
        for gpu in gpus {
            let vendor_str = gpu.vendor.as_deref().unwrap_or("Unknown");
            writeln!(out, "{} ({}, {})", gpu.name, vendor_str, gpu.backend).unwrap();
            if verbose > 0 {
                if let Some(mem) = gpu.memory_bytes {
                    writeln!(out, "  Memory: {}", format_bytes(mem)).unwrap();
                }
                writeln!(out, "  Type: {:?}", gpu.device_type).unwrap();
                if let Some(ref family) = gpu.metal_family {
                    writeln!(out, "  Metal Family: {}", family).unwrap();
                }
                if gpu.is_headless {
                    writeln!(out, "  Headless: yes").unwrap();
                }
                if gpu.is_removable {
                    writeln!(out, "  Removable: yes (eGPU)").unwrap();
                }
            }
            if verbose > 1 {
                if let Some(caps_str) = format_gpu_caps(&gpu.capabilities) {
                    writeln!(out, "  Capabilities: {}", caps_str).unwrap();
                }
                if let Some(max_buf) = gpu.max_buffer_bytes {
                    writeln!(out, "  Max Buffer: {}", format_bytes(max_buf)).unwrap();
                }
            }
        }
    }
    writeln!(out).unwrap();

    out
}

pub fn render_memory_section(memory: &sniff::hardware::MemoryInfo) -> String {
    let mut out = String::new();

    writeln!(out, "=== Memory ===").unwrap();
    writeln!(out, "Total: {}", format_bytes(memory.total_bytes)).unwrap();
    writeln!(out, "Available: {}", format_bytes(memory.available_bytes)).unwrap();
    writeln!(out, "Used: {}", format_bytes(memory.used_bytes)).unwrap();
    let usage_percent = (memory.used_bytes as f64 / memory.total_bytes as f64) * 100.0;
    writeln!(out, "Usage: {:.1}%", usage_percent).unwrap();

    // Show swap information if swap is available
    if memory.total_swap > 0 {
        writeln!(out).unwrap();
        writeln!(out, "Swap:").unwrap();
        writeln!(out, "  Total: {}", format_bytes(memory.total_swap)).unwrap();
        writeln!(out, "  Free: {}", format_bytes(memory.free_swap)).unwrap();
        writeln!(out, "  Used: {}", format_bytes(memory.used_swap)).unwrap();
        let swap_usage_percent = (memory.used_swap as f64 / memory.total_swap as f64) * 100.0;
        writeln!(out, "  Usage: {:.1}%", swap_usage_percent).unwrap();
    }
    writeln!(out).unwrap();

    out
}

pub fn render_storage_section(
    storage: &[sniff::hardware::StorageInfo],
    verbose: u8,
    repo_root: Option<&Path>,
) -> String {
    let mut out = String::new();

    writeln!(out, "=== Storage ===").unwrap();
    for disk in storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff::hardware::StorageKind::Ssd => "SSD",
            sniff::hardware::StorageKind::Hdd => "HDD",
            sniff::hardware::StorageKind::Unknown => "",
        };
        if kind_str.is_empty() {
            writeln!(out, "{} ({})", mount_str, disk.file_system).unwrap();
        } else {
            writeln!(out, "{} ({}, {})", mount_str, disk.file_system, kind_str).unwrap();
        }
        if verbose > 0 {
            writeln!(out, "  Total: {}", format_bytes(disk.total_bytes)).unwrap();
            writeln!(out, "  Available: {}", format_bytes(disk.available_bytes)).unwrap();
            if disk.is_removable {
                writeln!(out, "  Removable: yes").unwrap();
            }
        }
    }
    writeln!(out).unwrap();

    out
}

// ============================================================================
// Audio devices
// ============================================================================

/// Format a sample rate for display.
///
/// Integer rates (e.g., 48000.0) display without decimals.
/// Fractional rates display with 1 decimal place.
fn format_sample_rate(rate: f64) -> String {
    if (rate - rate.round()).abs() < 0.01 {
        format!("{}", rate as u64)
    } else {
        format!("{:.1}", rate)
    }
}

/// Format a sample rate (Hz) as a compact kHz string.
///
/// Integer kHz values render without a decimal (`48000.0` → `"48k"`).
/// Non-integer kHz values render with a single decimal place
/// (`44100.0` → `"44.1k"`).
fn format_sample_rate_khz(rate_hz: f64) -> String {
    let khz = rate_hz / 1000.0;
    if (khz - khz.round()).abs() < 0.01 {
        format!("{}k", khz.round() as i64)
    } else {
        format!("{:.1}k", khz)
    }
}

/// Return the styled markup for an [`AudioDeviceKind`] as used in the
/// parenthesized device descriptor.
fn style_audio_kind(kind: sniff::hardware::AudioDeviceKind) -> String {
    use sniff::hardware::AudioDeviceKind as K;
    match kind {
        K::BuiltIn => "<dim>Built-in</dim>".to_string(),
        K::Usb => "<indigo-500>USB</indigo-500>".to_string(),
        K::Bluetooth => "<blue>Bluetooth</blue>".to_string(),
        K::Thunderbolt => "<yellow>Thunderbolt</yellow>".to_string(),
        K::Hdmi => "<yellow>HDMI</yellow>".to_string(),
        K::Virtual => "<dim><i>Virtual</i></dim>".to_string(),
        K::Unknown => "Unknown".to_string(),
    }
}

/// Render a list of audio devices with verbosity levels.
///
/// - Default: name, kind, direction, default markers
/// - `-v`: adds sample rate + channel counts
/// - `-vv`: adds available sample rates + UID
fn render_audio_device_list(devices: &[sniff::hardware::AudioDeviceInfo], verbose: u8) -> String {
    let mut out = String::new();

    for dev in devices {
        let mut markers = Vec::new();
        if dev.is_default_output {
            markers.push("default output");
        }
        if dev.is_default_input {
            markers.push("default input");
        }

        let marker_str = if markers.is_empty() {
            String::new()
        } else {
            format!(" [{}]", markers.join(", "))
        };

        writeln!(
            out,
            "  {} ({}, {}){}",
            dev.name, dev.kind, dev.direction, marker_str
        )
        .unwrap();

        if verbose > 0 {
            if dev.sample_rate > 0.0 {
                writeln!(
                    out,
                    "    Sample rate: {} Hz",
                    format_sample_rate(dev.sample_rate)
                )
                .unwrap();
            }
            if dev.output_channels > 0 {
                writeln!(out, "    Output channels: {}", dev.output_channels).unwrap();
            }
            if dev.input_channels > 0 {
                writeln!(out, "    Input channels: {}", dev.input_channels).unwrap();
            }
        }

        if verbose > 1 {
            if !dev.available_sample_rates.is_empty() {
                let rates: Vec<String> = dev
                    .available_sample_rates
                    .iter()
                    .map(|r| format_sample_rate(*r))
                    .collect();
                writeln!(out, "    Available rates: {} Hz", rates.join(", ")).unwrap();
            }
            if !dev.uid.is_empty() {
                writeln!(out, "    UID: {}", dev.uid).unwrap();
            }
        }
    }

    out
}

/// Render standalone audio devices section (for `sniff audio-devices`).
pub fn render_audio_devices_section(
    devices: &[sniff::hardware::AudioDeviceInfo],
    verbose: u8,
) -> String {
    let mut out = String::new();

    writeln!(out, "=== Audio Devices ===").unwrap();
    if devices.is_empty() {
        writeln!(out, "No audio devices detected").unwrap();
    } else {
        out.push_str(&render_audio_device_list(devices, verbose));
    }
    writeln!(out).unwrap();

    out
}

#[cfg(test)]
mod audio_format_tests {
    use super::format_sample_rate_khz;

    #[test]
    fn khz_integer_khz() {
        assert_eq!(format_sample_rate_khz(48000.0), "48k");
        assert_eq!(format_sample_rate_khz(96000.0), "96k");
        assert_eq!(format_sample_rate_khz(192000.0), "192k");
    }

    #[test]
    fn khz_fractional_khz() {
        assert_eq!(format_sample_rate_khz(44100.0), "44.1k");
        assert_eq!(format_sample_rate_khz(88200.0), "88.2k");
    }

    #[test]
    fn khz_sub_khz_or_weird() {
        assert_eq!(format_sample_rate_khz(500.0), "0.5k");
    }

    #[test]
    fn khz_zero_returns_empty() {
        assert_eq!(format_sample_rate_khz(0.0), "0k");
    }
}

#[cfg(test)]
mod audio_kind_tests {
    use super::style_audio_kind;
    use sniff::hardware::AudioDeviceKind;

    #[test]
    fn built_in() {
        assert_eq!(style_audio_kind(AudioDeviceKind::BuiltIn), "<dim>Built-in</dim>");
    }

    #[test]
    fn usb() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Usb), "<indigo-500>USB</indigo-500>");
    }

    #[test]
    fn bluetooth() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Bluetooth), "<blue>Bluetooth</blue>");
    }

    #[test]
    fn thunderbolt() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Thunderbolt), "<yellow>Thunderbolt</yellow>");
    }

    #[test]
    fn hdmi() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Hdmi), "<yellow>HDMI</yellow>");
    }

    #[test]
    fn virtual_kind() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Virtual), "<dim><i>Virtual</i></dim>");
    }

    #[test]
    fn unknown_is_plain() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Unknown), "Unknown");
    }
}
