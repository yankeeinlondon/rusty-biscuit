//! OS section output formatting.
use biscuit_terminal::components::{prose::Prose, renderable::Renderable};

use sniff_lib::hardware::NtpStatus;

use super::format_uptime;

pub fn print_os_section(os: &sniff_lib::OsInfo, verbose: u8) {
    let title = Prose::new("<b><u>Operating System:</u></b>").render(None);
    println!("\n{}\n", title);
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
