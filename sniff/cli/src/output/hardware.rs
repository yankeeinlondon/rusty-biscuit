//! Hardware section output formatting.

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

pub fn print_hardware_section(
    hardware: &sniff::HardwareInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Hardware ===");

    println!(
        "CPU: {} ({} logical cores)",
        hardware.cpu.brand, hardware.cpu.logical_cores
    );
    println!("Architecture: {}", hardware.cpu.arch);
    if let Some(physical) = hardware.cpu.physical_cores {
        println!("Physical cores: {}", physical);
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0
        && let Some(simd_str) = format_simd_caps(&hardware.cpu.simd)
    {
        println!("SIMD: {}", simd_str);
    }
    println!();

    println!("Memory:");
    println!("  Total: {}", format_bytes(hardware.memory.total_bytes));
    println!(
        "  Available: {}",
        format_bytes(hardware.memory.available_bytes)
    );
    println!("  Used: {}", format_bytes(hardware.memory.used_bytes));
    if hardware.memory.total_swap > 0 {
        println!(
            "  Swap: {} total, {} used",
            format_bytes(hardware.memory.total_swap),
            format_bytes(hardware.memory.used_swap)
        );
    }
    println!();

    // Print GPU info if available
    if !hardware.gpu.is_empty() {
        println!("GPUs:");
        for gpu in &hardware.gpu {
            let vendor_str = gpu.vendor.as_deref().unwrap_or("Unknown");
            println!("  {} ({}, {})", gpu.name, vendor_str, gpu.backend);
            if verbose > 0 {
                if let Some(mem) = gpu.memory_bytes {
                    println!("    Memory: {}", format_bytes(mem));
                }
                println!("    Type: {:?}", gpu.device_type);
                if let Some(ref family) = gpu.metal_family {
                    println!("    Metal Family: {}", family);
                }
                if gpu.is_headless {
                    println!("    Headless: yes");
                }
                if gpu.is_removable {
                    println!("    Removable: yes (eGPU)");
                }
            }
            if verbose > 1 {
                // Show capabilities at -vv
                if let Some(caps_str) = format_gpu_caps(&gpu.capabilities) {
                    println!("    Capabilities: {}", caps_str);
                }
                if let Some(max_buf) = gpu.max_buffer_bytes {
                    println!("    Max Buffer: {}", format_bytes(max_buf));
                }
            }
        }
        println!();
    }

    // Print audio devices if available
    if !hardware.audio_devices.is_empty() {
        println!("Audio Devices:");
        print_audio_device_list(&hardware.audio_devices, verbose);
        println!();
    }

    println!("Storage:");
    for disk in &hardware.storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff::hardware::StorageKind::Ssd => "SSD",
            sniff::hardware::StorageKind::Hdd => "HDD",
            sniff::hardware::StorageKind::Unknown => "",
        };
        if kind_str.is_empty() {
            println!("  {} ({})", mount_str, disk.file_system);
        } else {
            println!("  {} ({}, {})", mount_str, disk.file_system, kind_str);
        }
        if verbose > 0 {
            println!("    Total: {}", format_bytes(disk.total_bytes));
            println!("    Available: {}", format_bytes(disk.available_bytes));
            if disk.is_removable {
                println!("    Removable: yes");
            }
        }
    }
    println!();
}

// ============================================================================
// Subsection print functions (for --cpu, --gpu, --memory, --storage filters)
// ============================================================================

pub fn print_cpu_section(cpu: &sniff::hardware::CpuInfo, verbose: u8) {
    println!("=== CPU ===");
    println!("Brand: {}", cpu.brand);
    println!("Architecture: {}", cpu.arch);
    println!("Logical cores: {}", cpu.logical_cores);
    if let Some(physical) = cpu.physical_cores {
        println!("Physical cores: {}", physical);
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0
        && let Some(simd_str) = format_simd_caps(&cpu.simd)
    {
        println!("SIMD: {}", simd_str);
    }

    println!();
}

pub fn print_gpu_section(gpus: &[sniff::hardware::GpuInfo], verbose: u8) {
    println!("=== GPU ===");
    if gpus.is_empty() {
        println!("No GPUs detected");
    } else {
        for gpu in gpus {
            let vendor_str = gpu.vendor.as_deref().unwrap_or("Unknown");
            println!("{} ({}, {})", gpu.name, vendor_str, gpu.backend);
            if verbose > 0 {
                if let Some(mem) = gpu.memory_bytes {
                    println!("  Memory: {}", format_bytes(mem));
                }
                println!("  Type: {:?}", gpu.device_type);
                if let Some(ref family) = gpu.metal_family {
                    println!("  Metal Family: {}", family);
                }
                if gpu.is_headless {
                    println!("  Headless: yes");
                }
                if gpu.is_removable {
                    println!("  Removable: yes (eGPU)");
                }
            }
            if verbose > 1 {
                if let Some(caps_str) = format_gpu_caps(&gpu.capabilities) {
                    println!("  Capabilities: {}", caps_str);
                }
                if let Some(max_buf) = gpu.max_buffer_bytes {
                    println!("  Max Buffer: {}", format_bytes(max_buf));
                }
            }
        }
    }
    println!();
}

pub fn print_memory_section(memory: &sniff::hardware::MemoryInfo) {
    println!("=== Memory ===");
    println!("Total: {}", format_bytes(memory.total_bytes));
    println!("Available: {}", format_bytes(memory.available_bytes));
    println!("Used: {}", format_bytes(memory.used_bytes));
    let usage_percent = (memory.used_bytes as f64 / memory.total_bytes as f64) * 100.0;
    println!("Usage: {:.1}%", usage_percent);

    // Show swap information if swap is available
    if memory.total_swap > 0 {
        println!();
        println!("Swap:");
        println!("  Total: {}", format_bytes(memory.total_swap));
        println!("  Free: {}", format_bytes(memory.free_swap));
        println!("  Used: {}", format_bytes(memory.used_swap));
        let swap_usage_percent = (memory.used_swap as f64 / memory.total_swap as f64) * 100.0;
        println!("  Usage: {:.1}%", swap_usage_percent);
    }
    println!();
}

pub fn print_storage_section(
    storage: &[sniff::hardware::StorageInfo],
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Storage ===");
    for disk in storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff::hardware::StorageKind::Ssd => "SSD",
            sniff::hardware::StorageKind::Hdd => "HDD",
            sniff::hardware::StorageKind::Unknown => "",
        };
        if kind_str.is_empty() {
            println!("{} ({})", mount_str, disk.file_system);
        } else {
            println!("{} ({}, {})", mount_str, disk.file_system, kind_str);
        }
        if verbose > 0 {
            println!("  Total: {}", format_bytes(disk.total_bytes));
            println!("  Available: {}", format_bytes(disk.available_bytes));
            if disk.is_removable {
                println!("  Removable: yes");
            }
        }
    }
    println!();
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

/// Print a list of audio devices with verbosity levels.
///
/// - Default: name, kind, direction, default markers
/// - `-v`: adds sample rate + channel counts
/// - `-vv`: adds available sample rates + UID
fn print_audio_device_list(devices: &[sniff::hardware::AudioDeviceInfo], verbose: u8) {
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

        println!(
            "  {} ({}, {}){}",
            dev.name, dev.kind, dev.direction, marker_str
        );

        if verbose > 0 {
            if dev.sample_rate > 0.0 {
                println!(
                    "    Sample rate: {} Hz",
                    format_sample_rate(dev.sample_rate)
                );
            }
            if dev.output_channels > 0 {
                println!("    Output channels: {}", dev.output_channels);
            }
            if dev.input_channels > 0 {
                println!("    Input channels: {}", dev.input_channels);
            }
        }

        if verbose > 1 {
            if !dev.available_sample_rates.is_empty() {
                let rates: Vec<String> = dev
                    .available_sample_rates
                    .iter()
                    .map(|r| format_sample_rate(*r))
                    .collect();
                println!("    Available rates: {} Hz", rates.join(", "));
            }
            if !dev.uid.is_empty() {
                println!("    UID: {}", dev.uid);
            }
        }
    }
}

/// Print standalone audio devices section (for `sniff audio-devices`).
pub fn print_audio_devices_section(devices: &[sniff::hardware::AudioDeviceInfo], verbose: u8) {
    println!("=== Audio Devices ===");
    if devices.is_empty() {
        println!("No audio devices detected");
    } else {
        print_audio_device_list(devices, verbose);
    }
    println!();
}
