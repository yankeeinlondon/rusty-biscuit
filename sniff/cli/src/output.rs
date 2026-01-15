use sniff_lib::SniffResult;
use sniff_lib::filesystem::git::BehindStatus;
use sniff_lib::hardware::NtpStatus;
use std::path::Path;

/// Filter mode for output - determines which subsection to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFilter {
    /// Show all sections (no filtering)
    #[default]
    All,
    /// Show only OS section
    Os,
    /// Show only hardware section (full) - used via include-only mode
    #[allow(dead_code)]
    Hardware,
    /// Show only filesystem section (full) - used via include-only mode
    #[allow(dead_code)]
    Filesystem,
    /// Show only CPU info (hardware subsection)
    Cpu,
    /// Show only GPU info (hardware subsection)
    Gpu,
    /// Show only memory info (hardware subsection)
    Memory,
    /// Show only storage info (hardware subsection)
    Storage,
    /// Show only git info (filesystem subsection)
    Git,
    /// Show only repo/monorepo info (filesystem subsection)
    Repo,
    /// Show only language detection (filesystem subsection)
    Language,
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
        OutputFilter::Os => {
            if let Some(ref os) = result.os {
                print_os_section(os, verbose);
            }
        }
        // Note: Hardware and Filesystem are not used as filter modes;
        // they're handled via include-only mode in OutputFilter::All
        OutputFilter::Hardware | OutputFilter::Filesystem => {
            // This branch shouldn't normally be reached since output_filter()
            // returns All for --hardware/--filesystem flags (they use include-only mode).
            // But we handle it for completeness.
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
    println!("Architecture: {}", os.arch);
    println!("Hostname: {}", os.hostname);
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
    if let Some(physical) = hardware.cpu.physical_cores {
        println!("Physical cores: {}", physical);
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0 {
        let simd = &hardware.cpu.simd;
        let mut caps = Vec::new();
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
        if !caps.is_empty() {
            println!("SIMD: {}", caps.join(", "));
        }
    }
    println!();

    println!("Memory:");
    println!("  Total: {}", format_bytes(hardware.memory.total_bytes));
    println!(
        "  Available: {}",
        format_bytes(hardware.memory.available_bytes)
    );
    println!("  Used: {}", format_bytes(hardware.memory.used_bytes));
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
                let caps = &gpu.capabilities;
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
                if !cap_list.is_empty() {
                    println!("    Capabilities: {}", cap_list.join(", "));
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
    }
    println!();
}

// ============================================================================
// Subsection print functions (for --cpu, --gpu, --memory, --storage filters)
// ============================================================================

fn print_cpu_section(cpu: &sniff_lib::hardware::CpuInfo, verbose: u8) {
    println!("=== CPU ===");
    println!("Brand: {}", cpu.brand);
    println!("Logical cores: {}", cpu.logical_cores);
    if let Some(physical) = cpu.physical_cores {
        println!("Physical cores: {}", physical);
    }

    // Print SIMD capabilities at verbose level 1+
    if verbose > 0 {
        let simd = &cpu.simd;
        let mut caps = Vec::new();
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
        if !caps.is_empty() {
            println!("SIMD: {}", caps.join(", "));
        }
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
                let caps = &gpu.capabilities;
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
                if !cap_list.is_empty() {
                    println!("  Capabilities: {}", cap_list.join(", "));
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
    let usage_percent =
        (memory.used_bytes as f64 / memory.total_bytes as f64) * 100.0;
    println!("Usage: {:.1}%", usage_percent);
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
        println!(
            "Message: {}",
            commit.message.lines().next().unwrap_or("")
        );
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
        dirty,
        git.status.staged_count,
        git.status.unstaged_count,
        git.status.untracked_count
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
    use serde_json::{json, Value};

    match filter {
        OutputFilter::All => {
            // No filtering - serialize everything
            serde_json::to_value(result).unwrap_or(Value::Null)
        }
        OutputFilter::Os => {
            json!({
                "os": result.os
            })
        }
        OutputFilter::Hardware => {
            json!({
                "hardware": result.hardware
            })
        }
        OutputFilter::Filesystem => {
            json!({
                "filesystem": result.filesystem
            })
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
    }
}

pub fn print_json(result: &SniffResult, filter: OutputFilter) -> serde_json::Result<()> {
    let filtered_json = apply_filter_to_json(result, filter);
    println!("{}", serde_json::to_string_pretty(&filtered_json)?);
    Ok(())
}
