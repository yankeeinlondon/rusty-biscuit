use sniff_lib::SniffResult;

pub fn print_text(result: &SniffResult) {
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
    println!("  Total: {} GB", result.hardware.memory.total_bytes / (1024 * 1024 * 1024));
    println!("  Available: {} GB", result.hardware.memory.available_bytes / (1024 * 1024 * 1024));
    println!("  Used: {} GB", result.hardware.memory.used_bytes / (1024 * 1024 * 1024));
    println!();

    println!("Storage:");
    for disk in &result.hardware.storage {
        println!("  {} ({})", disk.mount_point.display(), disk.file_system);
        println!("    Total: {} GB", disk.total_bytes / (1024 * 1024 * 1024));
        println!("    Available: {} GB", disk.available_bytes / (1024 * 1024 * 1024));
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
            println!("Languages ({} files analyzed):", langs.total_files);
            if let Some(ref primary) = langs.primary {
                println!("  Primary: {}", primary);
            }
            for lang in langs.languages.iter().take(5) {
                println!("  {}: {} files ({:.1}%)", lang.language, lang.file_count, lang.percentage);
            }
            if langs.languages.len() > 5 {
                println!("  ... and {} more", langs.languages.len() - 5);
            }
        }
        println!();

        if let Some(ref git) = fs.git {
            println!("Git Repository:");
            println!("  Root: {}", git.repo_root.display());
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
            for pkg in mono.packages.iter().take(5) {
                println!("    {}", pkg.name);
            }
            if mono.packages.len() > 5 {
                println!("    ... and {} more", mono.packages.len() - 5);
            }
        }

        if let Some(ref deps) = fs.dependencies {
            if !deps.detected_managers.is_empty() {
                println!("Package Managers:");
                for pm in &deps.detected_managers {
                    println!("  {:?} ({})", pm, pm.primary_language());
                }
            }
        }
    }
}

pub fn print_json(result: &SniffResult) -> serde_json::Result<()> {
    println!("{}", serde_json::to_string_pretty(result)?);
    Ok(())
}
