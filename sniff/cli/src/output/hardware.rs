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

    if !hardware.audio_devices.is_empty() {
        writeln!(out).unwrap();
        out.push_str(&render_audio_device_list(&hardware.audio_devices, verbose));
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
/// Non-colliding devices get `""`. Colliding devices get `:1`, `:2`, …
/// ordered by lexicographic uid.
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

        let mut ordered: Vec<usize> = indices.clone();
        ordered.sort_by(|a, b| devices[*a].uid.cmp(&devices[*b].uid));
        for (rank, idx) in ordered.iter().enumerate() {
            suffixes[*idx] = format!("<dim>:{}</dim>", rank + 1);
        }
    }

    suffixes
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
    let name = if is_default_here {
        format!("<b><yellow>{}</yellow></b>", dev.name)
    } else {
        dev.name.clone()
    };

    format!("{}{} {}{}", name, name_suffix, parens, marker)
}

/// Build a `Prose` item for the "none" placeholder child of an empty group.
fn empty_group_placeholder() -> biscuit_terminal::components::prose::Prose {
    biscuit_terminal::components::prose::Prose::new("<dim><i>none</i></dim>")
}

/// Build the Input or Output group as a nested `UnorderedList`.
///
/// `rendered` is the subset of devices whose `direction` places them on
/// this side, already sorted alphabetically (case-insensitive) by name.
/// `all_name_suffixes` is indexed by the device's index in the *original*
/// device slice.
fn build_group_list(
    heading: &str,
    rendered: &[(usize, &sniff::hardware::AudioDeviceInfo)],
    all_name_suffixes: &[String],
    side: GroupSide,
    verbose: u8,
) -> biscuit_terminal::components::list::UnorderedList {
    use biscuit_terminal::components::{list::UnorderedList, prose::Prose};

    let mut group = UnorderedList::empty();
    group.add(Prose::new(format!("<b>{}</b>", heading)));

    let mut children = UnorderedList::empty();
    if rendered.is_empty() {
        children.add(empty_group_placeholder());
    } else {
        for (original_idx, dev) in rendered {
            let suffix = all_name_suffixes[*original_idx].as_str();
            children.add(Prose::new(build_device_line(dev, suffix, side)));

            if verbose > 0 {
                if dev.output_channels > 0 {
                    children.add(Prose::new(format!(
                        "  <dim>Output channels:</dim> {}",
                        dev.output_channels
                    )));
                }
                if dev.input_channels > 0 {
                    children.add(Prose::new(format!(
                        "  <dim>Input channels:</dim> {}",
                        dev.input_channels
                    )));
                }
            }
            if verbose > 1 && !dev.uid.is_empty() {
                children.add(Prose::new(format!("  <dim>UID:</dim> {}", dev.uid)));
            }
        }
    }

    group.add(children);
    group
}

/// Render a list of audio devices as the grouped Input/Output block.
///
/// This is the shared builder used by both `sniff audio-devices` and the
/// embedded "Audio Devices" subsection inside `sniff hardware`. The output
/// does NOT start with a leading newline — callers decide on preceding
/// spacing. It does end with a trailing newline after the footer.
fn render_audio_device_list(devices: &[sniff::hardware::AudioDeviceInfo], verbose: u8) -> String {
    use biscuit_terminal::{
        components::{
            compose::Compose,
            list::UnorderedList,
            prose::Prose,
            renderable::Renderable,
            status::{Status, StatusState, StatusTheme},
        },
        terminal::Terminal,
    };
    use sniff::hardware::AudioDirection;

    // Short-circuit when no devices were detected: there is nothing worth
    // styling, and skipping Terminal::new() avoids an unnecessary terminal
    // capability probe on platforms (e.g. WSL 1) where probes are costly.
    if devices.is_empty() {
        return "Audio Devices: none detected\n".to_string();
    }

    let terminal = Terminal::new();

    let suffixes = build_name_suffixes(devices);

    let mut input_devs: Vec<(usize, &sniff::hardware::AudioDeviceInfo)> = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            matches!(
                d.direction,
                AudioDirection::Input | AudioDirection::InputOutput
            )
        })
        .collect();
    let mut output_devs: Vec<(usize, &sniff::hardware::AudioDeviceInfo)> = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            matches!(
                d.direction,
                AudioDirection::Output | AudioDirection::InputOutput
            )
        })
        .collect();

    input_devs.sort_by_key(|(_, a)| a.name.to_lowercase());
    output_devs.sort_by_key(|(_, a)| a.name.to_lowercase());

    let input_group = build_group_list("Input", &input_devs, &suffixes, GroupSide::Input, verbose);
    let output_group = build_group_list(
        "Output",
        &output_devs,
        &suffixes,
        GroupSide::Output,
        verbose,
    );

    let mut outer = UnorderedList::empty();
    outer.add(input_group);
    outer.add(output_group);

    let mut doc = Compose::default();
    doc.add_prose(Prose::new("<b><uu>Audio Devices</uu></b>"));
    doc.add_text("\n\n");
    doc.add_unordered_list(outer);

    let mut out = doc.display(&terminal).to_string();

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    let footer = Status::from_prose(
        "<i><dim>items with <b><yellow>*</yellow></b> are the <b>default</b> for the input/output</dim></i>",
    )
    .state(StatusState::Info)
    .theme(StatusTheme::Circular);
    out.push_str(&footer.display(&terminal).to_string());
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

/// Render standalone audio devices section (for `sniff audio-devices`).
///
/// Same block as [`render_audio_device_list`], but with a leading blank line
/// so the title has breathing room when printed at the top of the standalone
/// command's output.
pub fn render_audio_devices_section(
    devices: &[sniff::hardware::AudioDeviceInfo],
    verbose: u8,
) -> String {
    let mut out = String::from("\n");
    out.push_str(&render_audio_device_list(devices, verbose));
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
        assert_eq!(
            style_audio_kind(AudioDeviceKind::BuiltIn),
            "<dim>Built-in</dim>"
        );
    }

    #[test]
    fn usb() {
        assert_eq!(
            style_audio_kind(AudioDeviceKind::Usb),
            "<indigo-500>USB</indigo-500>"
        );
    }

    #[test]
    fn bluetooth() {
        assert_eq!(
            style_audio_kind(AudioDeviceKind::Bluetooth),
            "<blue>Bluetooth</blue>"
        );
    }

    #[test]
    fn thunderbolt() {
        assert_eq!(
            style_audio_kind(AudioDeviceKind::Thunderbolt),
            "<yellow>Thunderbolt</yellow>"
        );
    }

    #[test]
    fn hdmi() {
        assert_eq!(
            style_audio_kind(AudioDeviceKind::Hdmi),
            "<yellow>HDMI</yellow>"
        );
    }

    #[test]
    fn virtual_kind() {
        assert_eq!(
            style_audio_kind(AudioDeviceKind::Virtual),
            "<dim><i>Virtual</i></dim>"
        );
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
    fn collision_uses_numeric_suffix_ordered_by_uid() {
        let devices = vec![
            dev("LG UltraFine Display Audio", "LGDisplayAudio:b"),
            dev("LG UltraFine Display Audio", "LGDisplayAudio:a"),
        ];
        let suffixes = build_name_suffixes(&devices);
        // Sorted by UID: "LGDisplayAudio:a" → :1, "LGDisplayAudio:b" → :2
        assert_eq!(
            suffixes,
            vec!["<dim>:2</dim>".to_string(), "<dim>:1</dim>".to_string()]
        );
    }

    #[test]
    fn collision_three_way() {
        let devices = vec![
            dev("Clone", "uid-c"),
            dev("Clone", "uid-a"),
            dev("Clone", "uid-b"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>:3</dim>".to_string(),
                "<dim>:1</dim>".to_string(),
                "<dim>:2</dim>".to_string(),
            ]
        );
    }

    #[test]
    fn collision_leaves_non_colliding_devices_empty() {
        let devices = vec![
            dev("Clone", "uid-a"),
            dev("Solo", "unrelated"),
            dev("Clone", "uid-b"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>:1</dim>".to_string(),
                "".to_string(),
                "<dim>:2</dim>".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod audio_line_tests {
    use super::{GroupSide, build_device_line};
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
            "<b><yellow>MacBook Pro Speakers</yellow></b> (<dim>Built-in</dim>, <dim>44.1k</dim> <b>48k</b> <dim>96k</dim>) <b><yellow>*</yellow></b>"
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
        assert!(line.starts_with("<b><yellow>MacBook Pro Speakers</yellow></b><dim>:1</dim> ("));
    }

    #[test]
    fn missing_rates_drops_the_rate_segment() {
        let mut dev = macbook_speakers();
        dev.sample_rate = 0.0;
        dev.available_sample_rates.clear();
        let line = build_device_line(&dev, "", GroupSide::Output);
        assert_eq!(
            line,
            "<b><yellow>MacBook Pro Speakers</yellow></b> (<dim>Built-in</dim>) <b><yellow>*</yellow></b>"
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

#[cfg(test)]
mod audio_section_tests {
    use super::render_audio_devices_section;
    use sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection};

    fn mic() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "USB Microphone".to_string(),
            uid: "usb-mic".to_string(),
            kind: AudioDeviceKind::Usb,
            direction: AudioDirection::Input,
            is_default_input: true,
            is_default_output: false,
            sample_rate: 48000.0,
            available_sample_rates: vec![48000.0, 96000.0],
            input_channels: 1,
            output_channels: 0,
        }
    }

    fn speakers() -> AudioDeviceInfo {
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

    fn interface_io_default_out_only() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "USB Audio Interface".to_string(),
            uid: "usb-iface".to_string(),
            kind: AudioDeviceKind::Usb,
            direction: AudioDirection::InputOutput,
            is_default_input: false,
            is_default_output: false,
            sample_rate: 44100.0,
            available_sample_rates: vec![44100.0],
            input_channels: 2,
            output_channels: 2,
        }
    }

    #[test]
    fn title_and_footer_appear() {
        let devices = vec![mic(), speakers()];
        let out = render_audio_devices_section(&devices, 0);
        assert!(out.contains("Audio Devices"), "title missing:\n{}", out);
        assert!(
            out.contains("default") && out.contains("input/output"),
            "footer missing:\n{}",
            out
        );
    }

    #[test]
    fn default_markers_are_side_specific() {
        let devices = vec![speakers(), interface_io_default_out_only()];
        let out = render_audio_devices_section(&devices, 0);
        assert!(
            out.contains("*"),
            "expected at least one * marker:\n{}",
            out
        );
    }

    #[test]
    fn empty_input_group_shows_none_placeholder() {
        let devices = vec![speakers()];
        let out = render_audio_devices_section(&devices, 0);
        assert!(out.contains("Input"), "Input header missing:\n{}", out);
        assert!(out.contains("none"), "'none' placeholder missing:\n{}", out);
    }

    #[test]
    fn empty_device_list_emits_concise_message() {
        // When the platform reports no audio devices at all (e.g. WSL 1, headless
        // Linux without ALSA), skip the Input/Output grouping and return a single
        // line. This avoids invoking `Terminal::new()`, whose terminal-capability
        // probes hang on WSL 1.
        let out = render_audio_devices_section(&[], 0);
        assert!(out.contains("Audio Devices"), "title missing:\n{}", out);
        assert!(
            out.contains("none detected"),
            "'none detected' message missing:\n{}",
            out
        );
        assert!(!out.contains("Input"));
        assert!(!out.contains("Output"));
    }

    #[test]
    fn verbose_one_adds_channel_counts() {
        let out = render_audio_devices_section(&[speakers()], 1);
        assert!(
            out.contains("Output channels:") && out.contains(" 2"),
            "missing -v extras:\n{}",
            out
        );
    }

    #[test]
    fn verbose_two_adds_uid() {
        let out = render_audio_devices_section(&[speakers()], 2);
        assert!(
            out.contains("UID:") && out.contains("BuiltInSpeakerDevice"),
            "missing -vv extras:\n{}",
            out
        );
    }
}
