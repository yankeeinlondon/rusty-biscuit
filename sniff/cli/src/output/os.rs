//! OS section output formatting.
use std::fmt::Write;

use biscuit_terminal::components::{prose::Prose, renderable::TerminalRenderable};

use sniff::hardware::NtpStatus;

use super::format_uptime;

pub fn render_runtime_environment(runtime: sniff::os::RuntimeEnvironment) -> String {
    Prose::new(format!("<b>Runtime:</b> {runtime}")).render_optimistic(None)
}

pub fn render_os_section(os: &sniff::OsInfo, verbose: u8) -> String {
    let mut out = String::new();
    let title = Prose::new("<b><u>Operating System:</u></b>").render_optimistic(None);
    writeln!(out, "\n{}\n", title).unwrap();
    // Prefer long_version if available, otherwise fall back to name + version
    if let Some(ref long_ver) = os.long_version {
        writeln!(out, "Name: {}", long_ver).unwrap();
    } else {
        writeln!(out, "Name: {} {}", os.name, os.version).unwrap();
    }
    if let Some(ref distro) = os.distribution {
        writeln!(out, "Distribution: {}", distro).unwrap();
    }
    writeln!(out, "Kernel: {}", os.kernel).unwrap();
    writeln!(out, "Hostname: {}", os.hostname).unwrap();
    writeln!(out, "Uptime: {}", format_uptime(os.uptime_seconds)).unwrap();
    writeln!(out).unwrap();

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
            writeln!(
                out,
                "Package Managers: Primary: {} ({} detected)",
                primary_str,
                pkg_managers.managers.len()
            )
            .unwrap();
        } else {
            // Detailed output at verbose level 1+
            writeln!(out, "Package Managers:").unwrap();
            if let Some(ref primary) = pkg_managers.primary {
                writeln!(out, "  Primary: {}", primary).unwrap();
            }
            writeln!(out, "  Detected:").unwrap();
            for pm in &pkg_managers.managers {
                writeln!(out, "    - {} ({})", pm.manager, pm.path).unwrap();
                // Show commands at verbose level 2+
                if verbose > 1 {
                    if let Some(ref list_cmd) = pm.commands.list {
                        writeln!(out, "      list: {}", list_cmd).unwrap();
                    }
                    if let Some(ref update_cmd) = pm.commands.update {
                        writeln!(out, "      update: {}", update_cmd).unwrap();
                    }
                    if let Some(ref upgrade_cmd) = pm.commands.upgrade {
                        writeln!(out, "      upgrade: {}", upgrade_cmd).unwrap();
                    }
                    if let Some(ref search_cmd) = pm.commands.search {
                        writeln!(out, "      search: {}", search_cmd).unwrap();
                    }
                }
            }
        }
        writeln!(out).unwrap();
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
            writeln!(out, "Locale: {}{}", loc, encoding_str).unwrap();
            writeln!(out).unwrap();
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
            writeln!(out, "Timezone: {}{}", tz_name, abbr_str).unwrap();
        } else {
            writeln!(out, "Timezone: {}{}", tz_name, abbr_str).unwrap();

            // Show NTP status
            let ntp_str = match time.ntp_status {
                NtpStatus::Synchronized => "synchronized",
                NtpStatus::Unsynchronized => "not synchronized",
                NtpStatus::Inactive => "inactive",
                NtpStatus::Unknown => "unknown",
            };
            writeln!(out, "  NTP: {}", ntp_str).unwrap();

            // Show DST status
            let dst_str = if time.is_dst { "active" } else { "inactive" };
            writeln!(out, "  DST: {}", dst_str).unwrap();
        }
        writeln!(out).unwrap();
    }
    out
}
