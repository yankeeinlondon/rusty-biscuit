use sniff_lib::SniffResult;
use std::path::Path;

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
    if let Some(root) = repo_root {
        if let Ok(rel) = path.strip_prefix(root) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

#[allow(unused_variables)]
pub fn print_text(result: &SniffResult, verbose: u8, include_only_mode: bool) {
    // Get repo root for relative paths
    let repo_root = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.repo_root.as_path());

    // Print hardware section if present
    if let Some(ref hardware) = result.hardware {
        print_hardware_section(hardware, verbose, repo_root);
    }

    // Print network section if present
    if let Some(ref network) = result.network {
        print_network_section(network);
    }

    // Print filesystem section if present
    if let Some(ref filesystem) = result.filesystem {
        print_filesystem_section(filesystem, verbose, repo_root);
    }
}

fn print_hardware_section(
    hardware: &sniff_lib::HardwareInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Hardware ===");

    // Prefer long_version if available, otherwise fall back to name + version
    if let Some(ref long_ver) = hardware.os.long_version {
        println!("OS: {}", long_ver);
    } else {
        println!("OS: {} {}", hardware.os.name, hardware.os.version);
    }
    if let Some(ref distro) = hardware.os.distribution {
        println!("Distribution: {}", distro);
    }
    println!("Kernel: {}", hardware.os.kernel);
    println!("Architecture: {}", hardware.os.arch);
    println!("Hostname: {}", hardware.os.hostname);
    println!();

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
    if !hardware.gpus.is_empty() {
        println!("GPUs:");
        for gpu in &hardware.gpus {
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

fn print_filesystem_section(
    fs: &sniff_lib::FilesystemInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Filesystem ===");

    // Print EditorConfig formatting info at verbose level 2+
    if verbose > 1 {
        if let Some(ref formatting) = fs.formatting {
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
                    println!("    ... and {} more files", lang.files.len() - file_show_count);
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
        if let Some(ref commit) = git.head_commit {
            println!("  HEAD: {} ({})", &commit.sha[..8], commit.author);
            println!("  Message: {}", commit.message.lines().next().unwrap_or(""));
        }
        let dirty = if git.status.is_dirty { "dirty" } else { "clean" };
        println!(
            "  Status: {} ({} staged, {} unstaged, {} untracked)",
            dirty,
            git.status.staged_count,
            git.status.unstaged_count,
            git.status.untracked_count
        );

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

        for remote in &git.remotes {
            println!("  Remote {}: {:?}", remote.name, remote.provider);
        }
    }
    println!();

    if let Some(ref mono) = fs.monorepo {
        println!("Monorepo: {:?}", mono.tool);
        println!("  Packages: {}", mono.packages.len());
        let show_count = if verbose > 0 { mono.packages.len() } else { 5 };
        for pkg in mono.packages.iter().take(show_count) {
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
        if mono.packages.len() > show_count {
            println!("    ... and {} more", mono.packages.len() - show_count);
        }
    }

    if let Some(ref deps) = fs.dependencies {
        if !deps.detected_managers.is_empty() {
            println!("Package Managers:");
            for pm in &deps.detected_managers {
                println!("  {:?} ({})", pm, pm.primary_language());
            }
            if verbose > 0 && !deps.manifests.is_empty() {
                println!("Manifests ({}):", deps.manifests.len());
                for manifest in &deps.manifests {
                    println!("  {:?}: {}", manifest.manager, manifest.path.display());
                }
            }
        }
    }
}

pub fn print_json(result: &SniffResult) -> serde_json::Result<()> {
    println!("{}", serde_json::to_string_pretty(result)?);
    Ok(())
}
