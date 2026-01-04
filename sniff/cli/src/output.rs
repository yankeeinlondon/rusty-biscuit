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

pub fn print_text(result: &SniffResult, verbose: u8) {
    // Get repo root for relative paths
    let repo_root = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.repo_root.as_path());
    println!("=== Hardware ===");
    // Prefer long_version if available, otherwise fall back to name + version
    if let Some(ref long_ver) = result.hardware.os.long_version {
        println!("OS: {}", long_ver);
    } else {
        println!("OS: {} {}", result.hardware.os.name, result.hardware.os.version);
    }
    if let Some(ref distro) = result.hardware.os.distribution {
        println!("Distribution: {}", distro);
    }
    println!("Kernel: {}", result.hardware.os.kernel);
    println!("Architecture: {}", result.hardware.os.arch);
    println!("Hostname: {}", result.hardware.os.hostname);
    println!();

    println!("CPU: {} ({} logical cores)",
        result.hardware.cpu.brand,
        result.hardware.cpu.logical_cores
    );
    if let Some(physical) = result.hardware.cpu.physical_cores {
        println!("Physical cores: {}", physical);
    }
    println!();

    println!("Memory:");
    println!("  Total: {}", format_bytes(result.hardware.memory.total_bytes));
    println!("  Available: {}", format_bytes(result.hardware.memory.available_bytes));
    println!("  Used: {}", format_bytes(result.hardware.memory.used_bytes));
    println!();

    println!("Storage:");
    for disk in &result.hardware.storage {
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

    println!("=== Network ===");
    if result.network.permission_denied {
        println!("Permission denied - unable to enumerate interfaces");
    } else {
        if let Some(ref primary) = result.network.primary_interface {
            println!("Primary interface: {}", primary);
        }
        println!("Interfaces: {}", result.network.interfaces.len());
        for iface in &result.network.interfaces {
            let status = if iface.flags.is_up { "UP" } else { "DOWN" };
            let loopback = if iface.flags.is_loopback { " (loopback)" } else { "" };
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

    if let Some(ref fs) = result.filesystem {
        println!("=== Filesystem ===");

        if let Some(ref langs) = fs.languages {
            println!("Languages ({} files analyzed):", format_number(langs.total_files));
            if let Some(ref primary) = langs.primary {
                println!("  Primary: {}", primary);
            }
            let show_count = if verbose > 0 { 10 } else { 5 };
            for lang in langs.languages.iter().take(show_count) {
                println!("  {}: {} files ({:.1}%)", lang.language, format_number(lang.file_count), lang.percentage);
            }
            if langs.languages.len() > show_count {
                println!("  ... and {} more", langs.languages.len() - show_count);
            }
        }
        println!();

        if let Some(ref git) = fs.git {
            println!("Git Repository:");
            let root_str = relative_path(&git.repo_root, repo_root);
            println!("  Root: {}", if root_str.is_empty() { ".".to_string() } else { root_str });
            if let Some(ref branch) = git.current_branch {
                println!("  Branch: {}", branch);
            }
            if let Some(ref commit) = git.head_commit {
                println!("  HEAD: {} ({})", &commit.sha[..8], commit.author);
                println!("  Message: {}", commit.message.lines().next().unwrap_or(""));
            }
            let dirty = if git.status.is_dirty { "dirty" } else { "clean" };
            println!("  Status: {} ({} staged, {} unstaged, {} untracked)",
                dirty,
                git.status.staged_count,
                git.status.unstaged_count,
                git.status.untracked_count
            );
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
                println!("    {} ({})", pkg.name, path_str);
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
}

pub fn print_json(result: &SniffResult) -> serde_json::Result<()> {
    println!("{}", serde_json::to_string_pretty(result)?);
    Ok(())
}
