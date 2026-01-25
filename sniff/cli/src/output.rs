use sniff_lib::SniffResult;
use sniff_lib::filesystem::git::BehindStatus;
use sniff_lib::hardware::NtpStatus;
use sniff_lib::programs::ProgramsInfo;
use std::path::Path;

/// Filter mode for output - determines which subsection to display.
///
/// When a single top-level section is requested (Os, Hardware, Network, Filesystem),
/// the JSON output is flattened - the section's fields appear at the top level without
/// a wrapper object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFilter {
    /// Show all sections (no filtering)
    #[default]
    All,
    /// Show only OS section (flattened in JSON)
    Os,
    /// Show only hardware section (flattened in JSON)
    Hardware,
    /// Show only network section (flattened in JSON)
    Network,
    /// Show only filesystem section (flattened in JSON)
    Filesystem,
    /// Show only CPU info (hardware subsection, flattened in JSON)
    Cpu,
    /// Show only GPU info (hardware subsection, flattened in JSON)
    Gpu,
    /// Show only memory info (hardware subsection, flattened in JSON)
    Memory,
    /// Show only storage info (hardware subsection, flattened in JSON)
    Storage,
    /// Show only git info (filesystem subsection, flattened in JSON)
    Git,
    /// Show only repo/monorepo info (filesystem subsection, flattened in JSON)
    Repo,
    /// Show only language detection (filesystem subsection, flattened in JSON)
    Language,
    /// Show only programs info (installed programs detection)
    Programs,
    /// Show only editors (programs subsection)
    Editors,
    /// Show only utilities (programs subsection)
    Utilities,
    /// Show only language package managers (programs subsection)
    LanguagePackageManagers,
    /// Show only OS package managers (programs subsection)
    OsPackageManagers,
    /// Show only TTS clients (programs subsection)
    TtsClients,
    /// Show only terminal apps (programs subsection)
    TerminalApps,
    /// Show only headless audio players (programs subsection)
    HeadlessAudio,
}

/// Format bytes into human-readable units (KB, MB, GB, TB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format large numbers with comma separators
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Convert absolute path to relative path from repo root
fn relative_path(path: &Path, repo_root: Option<&Path>) -> String {
    if let Some(root) = repo_root
        && let Ok(rel) = path.strip_prefix(root)
    {
        return rel.display().to_string();
    }
    path.display().to_string()
}

/// Format SIMD capabilities into a comma-separated string.
///
/// Returns `None` if no capabilities are detected.
/// Uses hierarchical display: shows highest AVX level (512 > AVX2 > AVX).
fn format_simd_caps(simd: &sniff_lib::hardware::SimdCapabilities) -> Option<String> {
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
fn format_gpu_caps(caps: &sniff_lib::hardware::GpuCapabilities) -> Option<String> {
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

/// Format uptime in seconds to a human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }
    if minutes > 0 || (days == 0 && hours == 0 && secs == 0) {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if secs > 0 && days == 0 && hours == 0 {
        parts.push(format!(
            "{} second{}",
            secs,
            if secs == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn print_text(result: &SniffResult, verbose: u8, filter: OutputFilter) {
    // Get repo root for relative paths
    let repo_root = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.repo_root.as_path());

    match filter {
        OutputFilter::All => {
            // Print all sections that are present
            if let Some(ref os) = result.os {
                print_os_section(os, verbose);
            }
            if let Some(ref hardware) = result.hardware {
                print_hardware_section(hardware, verbose, repo_root);
            }
            if let Some(ref network) = result.network {
                print_network_section(network);
            }
            if let Some(ref filesystem) = result.filesystem {
                print_filesystem_section(filesystem, verbose, repo_root);
            }
        }
        // Top-level section filters (used for single-section requests)
        OutputFilter::Os => {
            if let Some(ref os) = result.os {
                print_os_section(os, verbose);
            }
        }
        OutputFilter::Hardware => {
            if let Some(ref hardware) = result.hardware {
                print_hardware_section(hardware, verbose, repo_root);
            }
        }
        OutputFilter::Network => {
            if let Some(ref network) = result.network {
                print_network_section(network);
            }
        }
        OutputFilter::Filesystem => {
            if let Some(ref filesystem) = result.filesystem {
                print_filesystem_section(filesystem, verbose, repo_root);
            }
        }
        OutputFilter::Cpu => {
            if let Some(ref hardware) = result.hardware {
                print_cpu_section(&hardware.cpu, verbose);
            }
        }
        OutputFilter::Gpu => {
            if let Some(ref hardware) = result.hardware {
                print_gpu_section(&hardware.gpu, verbose);
            }
        }
        OutputFilter::Memory => {
            if let Some(ref hardware) = result.hardware {
                print_memory_section(&hardware.memory);
            }
        }
        OutputFilter::Storage => {
            if let Some(ref hardware) = result.hardware {
                print_storage_section(&hardware.storage, verbose, repo_root);
            }
        }
        OutputFilter::Git => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref git) = filesystem.git
            {
                print_git_section(git, verbose, repo_root);
            }
        }
        OutputFilter::Repo => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref repo) = filesystem.repo
            {
                print_repo_section(repo, verbose, repo_root);
            }
        }
        OutputFilter::Language => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref langs) = filesystem.languages
            {
                print_language_section(langs, verbose);
            }
        }
        // Programs filters are handled separately in main.rs
        OutputFilter::Programs
        | OutputFilter::Editors
        | OutputFilter::Utilities
        | OutputFilter::LanguagePackageManagers
        | OutputFilter::OsPackageManagers
        | OutputFilter::TtsClients
        | OutputFilter::TerminalApps
        | OutputFilter::HeadlessAudio => {
            // These are handled by print_programs_text, should not reach here
            unreachable!("Programs filters should be handled separately")
        }
    }
}

fn print_os_section(os: &sniff_lib::OsInfo, verbose: u8) {
    println!("=== OS ===");

    // Prefer long_version if available, otherwise fall back to name + version
    if let Some(ref long_ver) = os.long_version {
        println!("Name: {}", long_ver);
    } else {
        println!("Name: {} {}", os.name, os.version);
    }
    if let Some(ref distro) = os.distribution {
        println!("Distribution: {}", distro);
    }
    println!("Kernel: {}", os.kernel);
    println!("Hostname: {}", os.hostname);
    println!("Uptime: {}", format_uptime(os.uptime_seconds));
    println!();

    // Print package managers section if detected
    if let Some(ref pkg_managers) = os.system_package_managers
        && !pkg_managers.managers.is_empty()
    {
        if verbose == 0 {
            // Compact output at verbose level 0
            let primary_str = pkg_managers
                .primary
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_else(|| "none".to_string());
            println!(
                "Package Managers: Primary: {} ({} detected)",
                primary_str,
                pkg_managers.managers.len()
            );
        } else {
            // Detailed output at verbose level 1+
            println!("Package Managers:");
            if let Some(ref primary) = pkg_managers.primary {
                println!("  Primary: {}", primary);
            }
            println!("  Detected:");
            for pm in &pkg_managers.managers {
                println!("    - {} ({})", pm.manager, pm.path);
                // Show commands at verbose level 2+
                if verbose > 1 {
                    if let Some(ref list_cmd) = pm.commands.list {
                        println!("      list: {}", list_cmd);
                    }
                    if let Some(ref update_cmd) = pm.commands.update {
                        println!("      update: {}", update_cmd);
                    }
                    if let Some(ref upgrade_cmd) = pm.commands.upgrade {
                        println!("      upgrade: {}", upgrade_cmd);
                    }
                    if let Some(ref search_cmd) = pm.commands.search {
                        println!("      search: {}", search_cmd);
                    }
                }
            }
        }
        println!();
    }

    // Print locale section if detected
    if let Some(ref locale) = os.locale {
        // Get the effective locale (LC_ALL overrides LANG)
        let effective_locale = locale.lc_all.as_ref().or(locale.lang.as_ref());
        if let Some(loc) = effective_locale {
            let encoding_str = locale
                .encoding
                .as_ref()
                .map(|e| format!(" ({})", e))
                .unwrap_or_default();
            println!("Locale: {}{}", loc, encoding_str);
            println!();
        }
    }

    // Print timezone section if detected
    if let Some(ref time) = os.time {
        // Format UTC offset as hours:minutes
        let offset_hours = time.utc_offset_seconds / 3600;
        let offset_minutes = (time.utc_offset_seconds.abs() % 3600) / 60;
        let offset_sign = if time.utc_offset_seconds >= 0 {
            "+"
        } else {
            "-"
        };
        let offset_str = format!(
            "UTC{}{:02}:{:02}",
            offset_sign,
            offset_hours.abs(),
            offset_minutes
        );

        // Build timezone display string
        let tz_name = time.timezone.as_deref().unwrap_or("Unknown");
        let abbr_str = time
            .timezone_abbr
            .as_deref()
            .map(|a| format!(" ({}, {})", a, offset_str))
            .unwrap_or_else(|| format!(" ({})", offset_str));

        if verbose == 0 {
            println!("Timezone: {}{}", tz_name, abbr_str);
        } else {
            println!("Timezone: {}{}", tz_name, abbr_str);

            // Show NTP status
            let ntp_str = match time.ntp_status {
                NtpStatus::Synchronized => "synchronized",
                NtpStatus::Unsynchronized => "not synchronized",
                NtpStatus::Inactive => "inactive",
                NtpStatus::Unknown => "unknown",
            };
            println!("  NTP: {}", ntp_str);

            // Show DST status
            let dst_str = if time.is_dst { "active" } else { "inactive" };
            println!("  DST: {}", dst_str);
        }
        println!();
    }
}

fn print_hardware_section(
    hardware: &sniff_lib::HardwareInfo,
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

    println!("Storage:");
    for disk in &hardware.storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff_lib::hardware::StorageKind::Ssd => "SSD",
            sniff_lib::hardware::StorageKind::Hdd => "HDD",
            sniff_lib::hardware::StorageKind::Unknown => "",
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

fn print_network_section(network: &sniff_lib::NetworkInfo) {
    println!("=== Network ===");
    if network.permission_denied {
        println!("Permission denied - unable to enumerate interfaces");
    } else {
        if let Some(ref primary) = network.primary_interface {
            println!("Primary interface: {}", primary);
        }
        println!("Interfaces: {}", network.interfaces.len());
        for iface in &network.interfaces {
            let status = if iface.flags.is_up { "UP" } else { "DOWN" };
            let loopback = if iface.flags.is_loopback {
                " (loopback)"
            } else {
                ""
            };
            println!("  {} [{}]{}", iface.name, status, loopback);
            for ip in &iface.ipv4_addresses {
                println!("    IPv4: {}", ip);
            }
            for ip in &iface.ipv6_addresses {
                println!("    IPv6: {}", ip);
            }
        }

        // Print aggregated IP addresses (only if there are any)
        if !network.ip_addresses.v4.is_empty() {
            println!();
            println!("All IPv4 Addresses ({}):", network.ip_addresses.v4.len());
            for addr in &network.ip_addresses.v4 {
                println!("  {} ({})", addr.address, addr.interface);
            }
        }

        if !network.ip_addresses.v6.is_empty() {
            println!();
            println!("All IPv6 Addresses ({}):", network.ip_addresses.v6.len());
            for addr in &network.ip_addresses.v6 {
                println!("  {} ({})", addr.address, addr.interface);
            }
        }
    }
    println!();
}

// ============================================================================
// Subsection print functions (for --cpu, --gpu, --memory, --storage filters)
// ============================================================================

fn print_cpu_section(cpu: &sniff_lib::hardware::CpuInfo, verbose: u8) {
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

fn print_gpu_section(gpus: &[sniff_lib::hardware::GpuInfo], verbose: u8) {
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

fn print_memory_section(memory: &sniff_lib::hardware::MemoryInfo) {
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

fn print_storage_section(
    storage: &[sniff_lib::hardware::StorageInfo],
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Storage ===");
    for disk in storage {
        let mount_str = relative_path(&disk.mount_point, repo_root);
        let kind_str = match disk.kind {
            sniff_lib::hardware::StorageKind::Ssd => "SSD",
            sniff_lib::hardware::StorageKind::Hdd => "HDD",
            sniff_lib::hardware::StorageKind::Unknown => "",
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

fn print_git_section(
    git: &sniff_lib::filesystem::git::GitInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Git ===");
    let root_str = relative_path(&git.repo_root, repo_root);
    println!(
        "Root: {}",
        if root_str.is_empty() {
            ".".to_string()
        } else {
            root_str
        }
    );
    if let Some(ref branch) = git.current_branch {
        println!("Branch: {}", branch);
    }

    if git.in_worktree {
        println!("In Worktree: yes");
    }

    if let Some(commit) = git.recent.first() {
        println!("HEAD: {} ({})", &commit.sha[..8], commit.author);
        println!("Message: {}", commit.message.lines().next().unwrap_or(""));
        if let Some(ref remotes) = commit.remotes {
            println!("Synced to: {}", remotes.join(", "));
        }
    }

    let dirty = if git.status.is_dirty {
        "dirty"
    } else {
        "clean"
    };
    println!(
        "Status: {} ({} staged, {} unstaged, {} untracked)",
        dirty, git.status.staged_count, git.status.unstaged_count, git.status.untracked_count
    );

    if let Some(ref behind) = git.status.is_behind {
        match behind {
            BehindStatus::NotBehind => println!("Behind: no"),
            BehindStatus::Behind(remotes) => {
                println!("Behind: {}", remotes.join(", "));
            }
        }
    }

    if verbose > 0 && git.recent.len() > 1 {
        println!("Recent commits:");
        for commit in git.recent.iter().skip(1).take(5) {
            let short_msg = commit.message.lines().next().unwrap_or("");
            let truncated = if short_msg.len() > 50 {
                format!("{}...", &short_msg[..47])
            } else {
                short_msg.to_string()
            };
            print!("  {} - {}", &commit.sha[..8], truncated);
            if verbose > 1
                && let Some(ref remotes) = commit.remotes
            {
                print!(" [{}]", remotes.join(", "));
            }
            println!();
        }
        if git.recent.len() > 6 {
            println!("  ... and {} more", git.recent.len() - 6);
        }
    }

    for remote in &git.remotes {
        print!("Remote {}: {:?}", remote.name, remote.provider);
        if let Some(ref branches) = remote.branches {
            print!(" ({} branches)", branches.len());
        }
        println!();
    }
    println!();
}

fn print_repo_section(
    repo: &sniff_lib::filesystem::repo::RepoInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Repository ===");

    if repo.is_monorepo {
        if let Some(ref tool) = repo.monorepo_tool {
            println!("Monorepo: {:?}", tool);
        }
        if let Some(ref packages) = repo.packages {
            println!("Packages: {}", packages.len());
            let show_count = if verbose > 0 { packages.len() } else { 5 };
            for pkg in packages.iter().take(show_count) {
                let path_str = relative_path(&pkg.path, repo_root);
                let lang_info = pkg
                    .primary_language
                    .as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                println!("  {} ({}){}", pkg.name, path_str, lang_info);

                if verbose > 0 && !pkg.detected_managers.is_empty() {
                    println!("    Managers: {}", pkg.detected_managers.join(", "));
                }
                if verbose > 1 && !pkg.languages.is_empty() {
                    println!("    Languages: {}", pkg.languages.join(", "));
                }
            }
            if packages.len() > show_count {
                println!("  ... and {} more", packages.len() - show_count);
            }
        }
    } else {
        println!("Single-package repository");
        println!("Root: {}", repo.root.display());
    }
    println!();
}

fn print_language_section(
    langs: &sniff_lib::filesystem::languages::LanguageBreakdown,
    verbose: u8,
) {
    println!("=== Languages ===");
    println!("Files analyzed: {}", format_number(langs.total_files));
    if let Some(ref primary) = langs.primary {
        println!("Primary: {}", primary);
    }
    let show_count = if verbose > 0 { 10 } else { 5 };
    for lang in langs.languages.iter().take(show_count) {
        println!(
            "{}: {} files ({:.1}%)",
            lang.language,
            format_number(lang.file_count),
            lang.percentage
        );
        if verbose > 1 && !lang.files.is_empty() {
            let file_show_count = 3.min(lang.files.len());
            for file in lang.files.iter().take(file_show_count) {
                println!("  - {}", file.display());
            }
            if lang.files.len() > file_show_count {
                println!(
                    "  ... and {} more files",
                    lang.files.len() - file_show_count
                );
            }
        }
    }
    if langs.languages.len() > show_count {
        println!("... and {} more", langs.languages.len() - show_count);
    }
    println!();
}

fn print_filesystem_section(fs: &sniff_lib::FilesystemInfo, verbose: u8, repo_root: Option<&Path>) {
    println!("=== Filesystem ===");

    // Print EditorConfig formatting info at verbose level 2+
    if verbose > 1
        && let Some(ref formatting) = fs.formatting
    {
        println!("EditorConfig: {}", formatting.config_path.display());
        for section in &formatting.sections {
            println!("  [{}]", section.pattern);
            if let Some(style) = &section.indent_style {
                println!("    indent_style: {}", style);
            }
            if let Some(size) = section.indent_size {
                println!("    indent_size: {}", size);
            }
        }
        println!();
    }

    if let Some(ref langs) = fs.languages {
        println!(
            "Languages ({} files analyzed):",
            format_number(langs.total_files)
        );
        if let Some(ref primary) = langs.primary {
            println!("  Primary: {}", primary);
        }
        let show_count = if verbose > 0 { 10 } else { 5 };
        for lang in langs.languages.iter().take(show_count) {
            println!(
                "  {}: {} files ({:.1}%)",
                lang.language,
                format_number(lang.file_count),
                lang.percentage
            );
            // Show file list at verbose level 2+
            if verbose > 1 && !lang.files.is_empty() {
                let file_show_count = 3.min(lang.files.len());
                for file in lang.files.iter().take(file_show_count) {
                    println!("    - {}", file.display());
                }
                if lang.files.len() > file_show_count {
                    println!(
                        "    ... and {} more files",
                        lang.files.len() - file_show_count
                    );
                }
            }
        }
        if langs.languages.len() > show_count {
            println!("  ... and {} more", langs.languages.len() - show_count);
        }
    }
    println!();

    if let Some(ref git) = fs.git {
        println!("Git Repository:");
        let root_str = relative_path(&git.repo_root, repo_root);
        println!(
            "  Root: {}",
            if root_str.is_empty() {
                ".".to_string()
            } else {
                root_str
            }
        );
        if let Some(ref branch) = git.current_branch {
            println!("  Branch: {}", branch);
        }

        // Show in_worktree indicator when true
        if git.in_worktree {
            println!("  In Worktree: yes");
        }

        // Show HEAD commit (first recent commit)
        if let Some(commit) = git.recent.first() {
            println!("  HEAD: {} ({})", &commit.sha[..8], commit.author);
            println!("  Message: {}", commit.message.lines().next().unwrap_or(""));
            // Show which remotes have this commit (deep mode)
            if let Some(ref remotes) = commit.remotes {
                println!("    Synced to: {}", remotes.join(", "));
            }
        }

        let dirty = if git.status.is_dirty {
            "dirty"
        } else {
            "clean"
        };
        println!(
            "  Status: {} ({} staged, {} unstaged, {} untracked)",
            dirty, git.status.staged_count, git.status.unstaged_count, git.status.untracked_count
        );

        // Show is_behind status (deep mode only)
        if let Some(ref behind) = git.status.is_behind {
            match behind {
                BehindStatus::NotBehind => println!("  Behind: no"),
                BehindStatus::Behind(remotes) => {
                    println!("  Behind: {}", remotes.join(", "));
                }
            }
        }

        // Show more recent commits at verbose level 1+
        if verbose > 0 && git.recent.len() > 1 {
            println!("  Recent commits:");
            for commit in git.recent.iter().skip(1).take(5) {
                let short_msg = commit.message.lines().next().unwrap_or("");
                let truncated = if short_msg.len() > 50 {
                    format!("{}...", &short_msg[..47])
                } else {
                    short_msg.to_string()
                };
                print!("    {} - {}", &commit.sha[..8], truncated);
                // Show commit remotes at verbose level 2+ with deep
                if verbose > 1
                    && let Some(ref remotes) = commit.remotes
                {
                    print!(" [{}]", remotes.join(", "));
                }
                println!();
            }
            if git.recent.len() > 6 {
                println!("    ... and {} more", git.recent.len() - 6);
            }
        }

        // Show dirty file details at verbose level 1+
        if verbose > 0 && !git.status.dirty.is_empty() {
            println!("  Dirty files:");
            for dirty_file in &git.status.dirty {
                println!("    - {}", dirty_file.filepath.display());
                // Show diff at verbose level 2+
                if verbose > 1 && !dirty_file.diff.is_empty() {
                    for line in dirty_file.diff.lines().take(5) {
                        println!("      {}", line);
                    }
                    let line_count = dirty_file.diff.lines().count();
                    if line_count > 5 {
                        println!("      ... ({} more lines)", line_count - 5);
                    }
                }
            }
        }

        // Show untracked files at verbose level 1+
        if verbose > 0 && !git.status.untracked.is_empty() {
            println!("  Untracked files:");
            let show_count = 5.min(git.status.untracked.len());
            for untracked in git.status.untracked.iter().take(show_count) {
                println!("    - {}", untracked.filepath.display());
            }
            if git.status.untracked.len() > show_count {
                println!(
                    "    ... and {} more",
                    git.status.untracked.len() - show_count
                );
            }
        }

        // Show worktrees at verbose level 1+
        if verbose > 0 && !git.worktrees.is_empty() {
            println!("  Worktrees:");
            for (branch, info) in &git.worktrees {
                let dirty_indicator = if info.dirty { " (dirty)" } else { "" };
                println!("    {} @ {}{}", branch, &info.sha[..8], dirty_indicator);
                if verbose > 1 {
                    println!("      Path: {}", info.filepath.display());
                }
            }
        }

        // Show remotes with enhanced branch info
        for remote in &git.remotes {
            print!("  Remote {}: {:?}", remote.name, remote.provider);
            // Show branch count in deep mode
            if let Some(ref branches) = remote.branches {
                print!(" ({} branches)", branches.len());
            }
            println!();
            // Show branches at verbose level 2+ with deep
            if verbose > 1
                && let Some(ref branches) = remote.branches
            {
                let show_count = 5.min(branches.len());
                for branch in branches.iter().take(show_count) {
                    println!("    - {}", branch);
                }
                if branches.len() > show_count {
                    println!("    ... and {} more", branches.len() - show_count);
                }
            }
        }
    }
    println!();

    if let Some(ref repo) = fs.repo
        && repo.is_monorepo
    {
        if let Some(ref tool) = repo.monorepo_tool {
            println!("Monorepo: {:?}", tool);
        }
        if let Some(ref packages) = repo.packages {
            println!("  Packages: {}", packages.len());
            let show_count = if verbose > 0 { packages.len() } else { 5 };
            for pkg in packages.iter().take(show_count) {
                let path_str = relative_path(&pkg.path, repo_root);
                let lang_info = pkg
                    .primary_language
                    .as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                println!("    {} ({}){}", pkg.name, path_str, lang_info);

                // Show package details at verbose level 1+
                if verbose > 0 {
                    if !pkg.detected_managers.is_empty() {
                        println!("      Managers: {}", pkg.detected_managers.join(", "));
                    }
                    if verbose > 1 && !pkg.languages.is_empty() {
                        println!("      Languages: {}", pkg.languages.join(", "));
                    }
                }
            }
            if packages.len() > show_count {
                println!("    ... and {} more", packages.len() - show_count);
            }
        }
    }
}

/// Apply output filter to create a custom JSON value with only the requested fields.
///
/// This ensures that filter flags like --cpu, --gpu work consistently for both
/// text and JSON output modes, and that filtered JSON only contains the relevant fields.
///
/// For subsection filters (--cpu, --gpu, --memory, --storage, --git, --repo, --language),
/// the output is flattened to the top level without the parent container.
fn apply_filter_to_json(result: &SniffResult, filter: OutputFilter) -> serde_json::Value {
    use serde_json::{Value, json};

    match filter {
        OutputFilter::All => {
            // No filtering - serialize everything
            serde_json::to_value(result).unwrap_or(Value::Null)
        }
        OutputFilter::Os => {
            // Flatten: return OS fields at top level (name, version, kernel, etc.)
            if let Some(ref os) = result.os {
                serde_json::to_value(os).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Network => {
            // Flatten: return network fields at top level (interfaces, primary_interface, etc.)
            if let Some(ref network) = result.network {
                serde_json::to_value(network).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Hardware => {
            // Flatten: return hardware fields at top level (cpu, gpu, memory, storage)
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(hw).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Filesystem => {
            // Flatten: return filesystem fields at top level (git, languages, repo, formatting)
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(fs).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Cpu => {
            // Flatten: return CPU data at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.cpu).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Gpu => {
            // Flatten: return GPU array at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.gpu).unwrap_or(Value::Null)
            } else {
                json!([])
            }
        }
        OutputFilter::Memory => {
            // Flatten: return memory data at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.memory).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Storage => {
            // Flatten: return storage array at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.storage).unwrap_or(Value::Null)
            } else {
                json!([])
            }
        }
        OutputFilter::Git => {
            // Flatten: return git data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.git).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Repo => {
            // Flatten: return repo data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.repo).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Language => {
            // Flatten: return languages data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.languages).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        // Programs filters are handled by print_programs_json
        OutputFilter::Programs
        | OutputFilter::Editors
        | OutputFilter::Utilities
        | OutputFilter::LanguagePackageManagers
        | OutputFilter::OsPackageManagers
        | OutputFilter::TtsClients
        | OutputFilter::TerminalApps
        | OutputFilter::HeadlessAudio => {
            unreachable!("Programs filters should be handled by print_programs_json")
        }
    }
}

pub fn print_json(result: &SniffResult, filter: OutputFilter) -> serde_json::Result<()> {
    let filtered_json = apply_filter_to_json(result, filter);
    println!("{}", serde_json::to_string_pretty(&filtered_json)?);
    Ok(())
}

// ============================================================================
// Programs output functions
// ============================================================================

/// Print programs information as text.
pub fn print_programs_text(programs: &ProgramsInfo, verbose: u8, filter: OutputFilter) {
    match filter {
        OutputFilter::Programs => {
            print_all_programs(programs, verbose);
        }
        OutputFilter::Editors => {
            print_editors_section(&programs.editors, verbose);
        }
        OutputFilter::Utilities => {
            print_utilities_section(&programs.utilities, verbose);
        }
        OutputFilter::LanguagePackageManagers => {
            print_lang_pkg_mgrs_section(&programs.language_package_managers, verbose);
        }
        OutputFilter::OsPackageManagers => {
            print_os_pkg_mgrs_section(&programs.os_package_managers, verbose);
        }
        OutputFilter::TtsClients => {
            print_tts_clients_section(&programs.tts_clients, verbose);
        }
        OutputFilter::TerminalApps => {
            print_terminal_apps_section(&programs.terminal_apps, verbose);
        }
        OutputFilter::HeadlessAudio => {
            print_headless_audio_section(&programs.headless_audio, verbose);
        }
        _ => {
            // Should not reach here, but print all as fallback
            print_all_programs(programs, verbose);
        }
    }
}

fn print_all_programs(programs: &ProgramsInfo, verbose: u8) {
    print_editors_section(&programs.editors, verbose);
    print_utilities_section(&programs.utilities, verbose);
    print_lang_pkg_mgrs_section(&programs.language_package_managers, verbose);
    print_os_pkg_mgrs_section(&programs.os_package_managers, verbose);
    print_tts_clients_section(&programs.tts_clients, verbose);
    print_terminal_apps_section(&programs.terminal_apps, verbose);
    print_headless_audio_section(&programs.headless_audio, verbose);
}

fn print_editors_section(editors: &sniff_lib::programs::InstalledEditors, verbose: u8) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== Editors ===");
    let installed = editors.installed();
    if installed.is_empty() {
        println!("No editors detected");
    } else {
        println!("Installed ({}):", installed.len());
        for editor in &installed {
            let name = editor.display_name();
            if verbose > 0 {
                let desc = editor.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", editor.website());
                    if let Some(path) = editors.path(*editor) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_utilities_section(utilities: &sniff_lib::programs::InstalledUtilities, verbose: u8) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== Utilities ===");
    let installed = utilities.installed();
    if installed.is_empty() {
        println!("No utilities detected");
    } else {
        println!("Installed ({}):", installed.len());
        for util in &installed {
            let name = util.display_name();
            if verbose > 0 {
                let desc = util.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", util.website());
                    if let Some(path) = utilities.path(*util) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_lang_pkg_mgrs_section(
    pkg_mgrs: &sniff_lib::programs::InstalledLanguagePackageManagers,
    verbose: u8,
) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== Language Package Managers ===");
    let installed = pkg_mgrs.installed();
    if installed.is_empty() {
        println!("No language package managers detected");
    } else {
        println!("Installed ({}):", installed.len());
        for pm in &installed {
            let name = pm.display_name();
            if verbose > 0 {
                let desc = pm.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", pm.website());
                    if let Some(path) = pkg_mgrs.path(*pm) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_os_pkg_mgrs_section(
    pkg_mgrs: &sniff_lib::programs::InstalledOsPackageManagers,
    verbose: u8,
) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== OS Package Managers ===");
    let installed = pkg_mgrs.installed();
    if installed.is_empty() {
        println!("No OS package managers detected");
    } else {
        println!("Installed ({}):", installed.len());
        for pm in &installed {
            let name = pm.display_name();
            if verbose > 0 {
                let desc = pm.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", pm.website());
                    if let Some(path) = pkg_mgrs.path(*pm) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_tts_clients_section(clients: &sniff_lib::programs::InstalledTtsClients, verbose: u8) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== TTS Clients ===");
    let installed = clients.installed();
    if installed.is_empty() {
        println!("No TTS clients detected");
    } else {
        println!("Installed ({}):", installed.len());
        for client in &installed {
            let name = client.display_name();
            if verbose > 0 {
                let desc = client.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", client.website());
                    if let Some(path) = clients.path(*client) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_terminal_apps_section(apps: &sniff_lib::programs::InstalledTerminalApps, verbose: u8) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== Terminal Apps ===");
    let installed = apps.installed();
    if installed.is_empty() {
        println!("No terminal apps detected");
    } else {
        println!("Installed ({}):", installed.len());
        for app in &installed {
            let name = app.display_name();
            if verbose > 0 {
                let desc = app.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", app.website());
                    if let Some(path) = apps.path(*app) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

fn print_headless_audio_section(
    players: &sniff_lib::programs::InstalledHeadlessAudio,
    verbose: u8,
) {
    use sniff_lib::programs::ProgramMetadata;

    println!("=== Headless Audio Players ===");
    let installed = players.installed();
    if installed.is_empty() {
        println!("No headless audio players detected");
    } else {
        println!("Installed ({}):", installed.len());
        for player in &installed {
            let name = player.display_name();
            if verbose > 0 {
                let desc = player.description();
                println!("  {} - {}", name, desc);
                if verbose > 1 {
                    println!("    Website: {}", player.website());
                    if let Some(path) = players.path(*player) {
                        println!("    Path: {}", path.display());
                    }
                }
            } else {
                print!("  {}", name);
                println!();
            }
        }
    }
    println!();
}

/// Print programs information as JSON.
pub fn print_programs_json(
    programs: &ProgramsInfo,
    filter: OutputFilter,
) -> serde_json::Result<()> {
    use serde_json::json;

    let json_value = match filter {
        OutputFilter::Programs => serde_json::to_value(programs)?,
        OutputFilter::Editors => serde_json::to_value(&programs.editors)?,
        OutputFilter::Utilities => serde_json::to_value(&programs.utilities)?,
        OutputFilter::LanguagePackageManagers => {
            serde_json::to_value(&programs.language_package_managers)?
        }
        OutputFilter::OsPackageManagers => serde_json::to_value(&programs.os_package_managers)?,
        OutputFilter::TtsClients => serde_json::to_value(&programs.tts_clients)?,
        OutputFilter::TerminalApps => serde_json::to_value(&programs.terminal_apps)?,
        OutputFilter::HeadlessAudio => serde_json::to_value(&programs.headless_audio)?,
        _ => json!({}),
    };

    println!("{}", serde_json::to_string_pretty(&json_value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_zero() {
        assert_eq!(format_uptime(0), "0 minutes");
    }

    #[test]
    fn test_format_uptime_seconds() {
        assert_eq!(format_uptime(30), "30 seconds");
        assert_eq!(format_uptime(1), "1 second");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(60), "1 minute");
        assert_eq!(format_uptime(120), "2 minutes");
        assert_eq!(format_uptime(90), "1 minute, 30 seconds");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1 hour");
        assert_eq!(format_uptime(3660), "1 hour, 1 minute");
        assert_eq!(format_uptime(7200), "2 hours");
        assert_eq!(format_uptime(7320), "2 hours, 2 minutes");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(86400), "1 day");
        assert_eq!(format_uptime(86400 + 3600), "1 day, 1 hour");
        assert_eq!(format_uptime(86400 + 3660), "1 day, 1 hour, 1 minute");
        assert_eq!(
            format_uptime(2 * 86400 + 5 * 3600 + 30 * 60),
            "2 days, 5 hours, 30 minutes"
        );
    }

    #[test]
    fn test_format_uptime_long() {
        // 16 days, 13 hours, 26 minutes
        assert_eq!(
            format_uptime(16 * 86400 + 13 * 3600 + 26 * 60),
            "16 days, 13 hours, 26 minutes"
        );
    }
}
