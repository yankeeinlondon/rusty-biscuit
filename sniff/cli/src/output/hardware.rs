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

/// For each device, compute the `<dim>…</dim>` suffix (if any) that should be
/// appended to its name so that name collisions are visually disambiguated.
///
/// Non-colliding devices get `""`.
///
/// Colliding devices (two or more with the same `name`) get a suffix derived
/// from the longest common prefix of their `uid`s. If every device in a
/// collision group would produce the same suffix (e.g. identical uids, or
/// any group member ends up with an empty tail), we fall back to 1-based
/// `:1`, `:2`, … ordered by lexicographic uid.
fn build_name_suffixes(devices: &[sniff::hardware::AudioDeviceInfo]) -> Vec<String> {
    use std::collections::HashMap;

    let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, dev) in devices.iter().enumerate() {
        groups.entry(dev.name.as_str()).or_default().push(idx);
    }

    let mut suffixes: Vec<String> = vec![String::new(); devices.len()];

    for (_name, indices) in groups {
        if indices.len() < 2 {
            continue;
        }

        let uids: Vec<&str> = indices.iter().map(|&i| devices[i].uid.as_str()).collect();
        let prefix_len = longest_common_prefix_len(&uids);
        let tails: Vec<&str> = uids.iter().map(|u| &u[prefix_len..]).collect();

        let mut unique_tails = tails.clone();
        unique_tails.sort();
        unique_tails.dedup();

        if unique_tails.len() == indices.len() && tails.iter().all(|t| !t.is_empty()) {
            for (i, tail) in indices.iter().zip(tails.iter()) {
                suffixes[*i] = format!("<dim>{}</dim>", tail);
            }
        } else {
            let mut ordered: Vec<usize> = indices.clone();
            ordered.sort_by(|a, b| devices[*a].uid.cmp(&devices[*b].uid));
            for (rank, idx) in ordered.iter().enumerate() {
                suffixes[*idx] = format!("<dim>:{}</dim>", rank + 1);
            }
        }
    }

    suffixes
}

/// Length in bytes of the longest common prefix of every string in `values`,
/// rounded down to the nearest char boundary in the first string.
fn longest_common_prefix_len(values: &[&str]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let first = values[0].as_bytes();
    let mut len = first.len();
    for v in &values[1..] {
        let b = v.as_bytes();
        len = len.min(b.len());
        len = (0..len).take_while(|&i| b[i] == first[i]).count();
        if len == 0 {
            return 0;
        }
    }
    while len > 0 && !values[0].is_char_boundary(len) {
        len -= 1;
    }
    len
}

/// Which group ("Input" or "Output") a device line is being rendered under.
/// Used to decide whether the default marker should be appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupSide {
    Input,
    Output,
}

/// Build the inline markup for one device line (no bullet, no newline).
///
/// Format: `{name}{name_suffix?} (<kind>[, {rates}]){ default-marker?}`.
fn build_device_line(
    dev: &sniff::hardware::AudioDeviceInfo,
    name_suffix: &str,
    side: GroupSide,
) -> String {
    let kind_markup = style_audio_kind(dev.kind);

    let mut rates: Vec<f64> = dev.available_sample_rates.clone();
    if dev.sample_rate > 0.0 {
        rates.push(dev.sample_rate);
    }
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rates.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    let rates_markup: String = if rates.is_empty() {
        String::new()
    } else {
        let current = dev.sample_rate;
        rates
            .iter()
            .map(|r| {
                let label = format_sample_rate_khz(*r);
                if current > 0.0 && (*r - current).abs() < 0.01 {
                    format!("<b>{}</b>", label)
                } else {
                    format!("<dim>{}</dim>", label)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let parens = if rates_markup.is_empty() {
        format!("({})", kind_markup)
    } else {
        format!("({}, {})", kind_markup, rates_markup)
    };

    let is_default_here = match side {
        GroupSide::Input => dev.is_default_input,
        GroupSide::Output => dev.is_default_output,
    };
    let marker = if is_default_here {
        " <b><yellow>*</yellow></b>"
    } else {
        ""
    };

    format!("{}{} {}{}", dev.name, name_suffix, parens, marker)
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

#[cfg(test)]
mod audio_suffix_tests {
    use super::build_name_suffixes;
    use sniff::hardware::AudioDeviceInfo;

    fn dev(name: &str, uid: &str) -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: name.to_string(),
            uid: uid.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn no_collision_no_suffix() {
        let devices = vec![dev("Speakers", "uid-a"), dev("Microphone", "uid-b")];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(suffixes, vec!["".to_string(), "".to_string()]);
    }

    #[test]
    fn collision_with_clean_trailing_suffix() {
        let devices = vec![
            dev("LG UltraFine Display Audio", "LGDisplayAudio:1"),
            dev("LG UltraFine Display Audio", "LGDisplayAudio:2"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec!["<dim>1</dim>".to_string(), "<dim>2</dim>".to_string()]
        );
    }

    #[test]
    fn collision_with_identical_uids_falls_back_to_indexed() {
        let devices = vec![dev("Clone", "same-uid"), dev("Clone", "same-uid")];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec!["<dim>:1</dim>".to_string(), "<dim>:2</dim>".to_string()]
        );
    }

    #[test]
    fn collision_three_way_with_shared_prefix() {
        let devices = vec![
            dev("Clone", "prefix_alpha"),
            dev("Clone", "prefix_beta"),
            dev("Clone", "prefix_gamma"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>alpha</dim>".to_string(),
                "<dim>beta</dim>".to_string(),
                "<dim>gamma</dim>".to_string(),
            ]
        );
    }

    #[test]
    fn collision_leaves_non_colliding_devices_empty() {
        let devices = vec![
            dev("Clone", "prefix_a"),
            dev("Solo", "unrelated"),
            dev("Clone", "prefix_b"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>a</dim>".to_string(),
                "".to_string(),
                "<dim>b</dim>".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod audio_line_tests {
    use super::{build_device_line, GroupSide};
    use sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection};

    fn macbook_speakers() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "MacBook Pro Speakers".to_string(),
            uid: "BuiltInSpeakerDevice".to_string(),
            kind: AudioDeviceKind::BuiltIn,
            direction: AudioDirection::Output,
            is_default_input: false,
            is_default_output: true,
            sample_rate: 48000.0,
            available_sample_rates: vec![44100.0, 48000.0, 96000.0],
            input_channels: 0,
            output_channels: 2,
        }
    }

    #[test]
    fn output_side_shows_default_marker() {
        let line = build_device_line(&macbook_speakers(), "", GroupSide::Output);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>, <dim>44.1k</dim> <b>48k</b> <dim>96k</dim>) <b><yellow>*</yellow></b>"
        );
    }

    #[test]
    fn input_side_omits_default_output_marker() {
        let line = build_device_line(&macbook_speakers(), "", GroupSide::Input);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>, <dim>44.1k</dim> <b>48k</b> <dim>96k</dim>)"
        );
    }

    #[test]
    fn name_suffix_is_appended_before_parens() {
        let line = build_device_line(&macbook_speakers(), "<dim>:1</dim>", GroupSide::Output);
        assert!(line.starts_with("MacBook Pro Speakers<dim>:1</dim> ("));
    }

    #[test]
    fn missing_rates_drops_the_rate_segment() {
        let mut dev = macbook_speakers();
        dev.sample_rate = 0.0;
        dev.available_sample_rates.clear();
        let line = build_device_line(&dev, "", GroupSide::Output);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>) <b><yellow>*</yellow></b>"
        );
    }

    #[test]
    fn current_rate_not_in_available_list_is_still_rendered_bold() {
        let mut dev = macbook_speakers();
        dev.sample_rate = 192000.0;
        dev.available_sample_rates = vec![48000.0, 96000.0];
        let line = build_device_line(&dev, "", GroupSide::Output);
        assert!(line.contains("<dim>48k</dim> <dim>96k</dim> <b>192k</b>"));
    }
}
