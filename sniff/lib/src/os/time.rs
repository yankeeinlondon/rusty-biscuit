//! Timezone and NTP status detection.
//!
//! This module provides functionality for detecting system timezone,
//! UTC offset, daylight saving time status, and NTP synchronization state.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

// ============================================================================
// NTP and Timezone Detection
// ============================================================================

/// NTP synchronization status.
///
/// Indicates whether the system's time is synchronized via Network Time Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NtpStatus {
    /// NTP is active and time is synchronized
    Synchronized,
    /// NTP service is active but not yet synchronized
    Unsynchronized,
    /// NTP service is not running
    Inactive,
    /// Cannot determine NTP status (permission denied, unsupported platform, etc.)
    #[default]
    Unknown,
}

/// Time and timezone information.
///
/// Contains details about the system's timezone configuration, UTC offset,
/// daylight saving time status, and NTP synchronization state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeInfo {
    /// IANA timezone name (e.g., "America/Los_Angeles", "Europe/London")
    pub timezone: Option<String>,
    /// Offset from UTC in seconds (negative = west of UTC, positive = east)
    pub utc_offset_seconds: i32,
    /// Whether daylight saving time is currently active
    pub is_dst: bool,
    /// Abbreviated timezone name (e.g., "PST", "PDT", "GMT")
    pub timezone_abbr: Option<String>,
    /// NTP synchronization status
    pub ntp_status: NtpStatus,
    /// Whether a monotonic clock is available (always true on modern systems)
    pub monotonic_available: bool,
}

/// Runs a command with a timeout, returning stdout as a string if successful.
///
/// ## Arguments
///
/// * `cmd` - The command to execute
/// * `args` - Arguments to pass to the command
/// * `timeout_secs` - Maximum time to wait for the command (in seconds)
///
/// ## Returns
///
/// `Some(String)` containing stdout if the command succeeds, `None` otherwise.
/// Returns `None` for permission errors, timeouts, or any execution failure.
#[allow(dead_code)] // Used only on Linux for NTP detection
pub(crate) fn run_command_with_timeout(
    cmd: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Option<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    // Wait with timeout using a simple polling approach
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    let output = child.wait_with_output().ok()?;
                    return String::from_utf8(output.stdout).ok();
                }
                return None;
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    // Timeout exceeded, kill the process
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

/// Extracts the IANA timezone name from a symlink path.
///
/// Parses paths like `/var/db/timezone/zoneinfo/America/Los_Angeles` or
/// `/usr/share/zoneinfo/Europe/London` to extract the timezone portion.
///
/// ## Arguments
///
/// * `path` - The symlink target path to parse
///
/// ## Returns
///
/// The timezone name (e.g., "America/Los_Angeles") if found, `None` otherwise.
pub(crate) fn extract_timezone_from_path(path: &str) -> Option<String> {
    // Common patterns for timezone paths
    let markers = ["zoneinfo/", "timezone/zoneinfo/"];

    for marker in markers {
        if let Some(pos) = path.find(marker) {
            let tz = &path[pos + marker.len()..];
            // Validate it looks like a timezone (contains at least one component)
            if !tz.is_empty() && !tz.starts_with('/') {
                return Some(tz.to_string());
            }
        }
    }
    None
}

/// Detects the system timezone name from OS-specific sources.
///
/// ## Platform Behavior
///
/// - **Linux**: Reads `/etc/timezone` or parses `/etc/localtime` symlink target
/// - **macOS**: Parses `/etc/localtime` symlink target
/// - **Windows**: Returns `None` (registry query not implemented)
///
/// ## Returns
///
/// The IANA timezone name (e.g., "America/Los_Angeles") if detected, `None` otherwise.
#[cfg(target_os = "linux")]
fn detect_timezone_name() -> Option<String> {
    // Try /etc/timezone first (Debian/Ubuntu style)
    if let Ok(contents) = std::fs::read_to_string("/etc/timezone") {
        let tz = contents.trim();
        if !tz.is_empty() {
            return Some(tz.to_string());
        }
    }

    // Fall back to parsing /etc/localtime symlink
    if let Ok(target) = std::fs::read_link("/etc/localtime")
        && let Some(path_str) = target.to_str()
    {
        return extract_timezone_from_path(path_str);
    }

    None
}

#[cfg(target_os = "macos")]
fn detect_timezone_name() -> Option<String> {
    // macOS uses /etc/localtime as a symlink to the timezone file
    // Target is typically /var/db/timezone/zoneinfo/America/Los_Angeles
    if let Ok(target) = std::fs::read_link("/etc/localtime")
        && let Some(path_str) = target.to_str()
    {
        return extract_timezone_from_path(path_str);
    }
    None
}

#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> {
    // Windows timezone detection requires registry queries
    // which is complex - return None for initial implementation
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn detect_timezone_name() -> Option<String> {
    None
}

/// Detects NTP synchronization status.
///
/// ## Platform Behavior
///
/// - **Linux**: Queries `timedatectl` with a 5-second timeout
/// - **macOS**: Returns `Unknown` (sntp status check is complex)
/// - **Windows**: Returns `Unknown` (w32tm query not implemented)
///
/// ## Returns
///
/// The NTP synchronization status. Returns `Unknown` for permission errors,
/// unsupported platforms, or when the status cannot be determined.
#[cfg(target_os = "linux")]
pub fn detect_ntp_status() -> NtpStatus {
    // Use timedatectl to check NTP status
    let output = run_command_with_timeout(
        "timedatectl",
        &["show", "--property=NTPSynchronized", "--value"],
        5,
    );

    match output.as_deref().map(str::trim) {
        Some("yes") => NtpStatus::Synchronized,
        Some("no") => {
            // Check if NTP is active but not synced vs inactive
            let ntp_active =
                run_command_with_timeout("timedatectl", &["show", "--property=NTP", "--value"], 5);
            match ntp_active.as_deref().map(str::trim) {
                Some("yes") => NtpStatus::Unsynchronized,
                Some("no") => NtpStatus::Inactive,
                _ => NtpStatus::Unknown,
            }
        }
        _ => NtpStatus::Unknown,
    }
}

#[cfg(target_os = "macos")]
pub fn detect_ntp_status() -> NtpStatus {
    // macOS NTP status detection is complex (requires sntp or systemsetup)
    // Return Unknown for initial implementation
    NtpStatus::Unknown
}

#[cfg(target_os = "windows")]
pub fn detect_ntp_status() -> NtpStatus {
    // Windows NTP status requires w32tm query
    // Return Unknown for initial implementation
    NtpStatus::Unknown
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn detect_ntp_status() -> NtpStatus {
    NtpStatus::Unknown
}

/// Detects timezone and time-related system information.
///
/// Gathers timezone name, UTC offset, DST status, and NTP synchronization state.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::detect_timezone;
///
/// let time_info = detect_timezone();
/// println!("Timezone: {:?}", time_info.timezone);
/// println!("UTC offset: {} seconds", time_info.utc_offset_seconds);
/// println!("DST active: {}", time_info.is_dst);
/// ```
///
/// ## Returns
///
/// A `TimeInfo` struct containing all detected time information.
/// Fields that cannot be detected will have sensible defaults.
pub fn detect_timezone() -> TimeInfo {
    use chrono::{Datelike, Local, Offset, TimeZone};

    let now = Local::now();
    let offset = now.offset();

    // Get UTC offset in seconds
    let utc_offset_seconds = offset.fix().local_minus_utc();

    // Detect DST by comparing current offset to standard time offset
    // This is a heuristic: if the offset differs from what we'd expect
    // at the start of the year, DST is likely active
    let jan_1 = Local.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0);
    let is_dst = if let chrono::LocalResult::Single(jan) = jan_1 {
        let jan_offset = jan.offset().fix().local_minus_utc();
        utc_offset_seconds != jan_offset
    } else {
        false
    };

    // Get timezone abbreviation from chrono's format
    let timezone_abbr = Some(now.format("%Z").to_string());

    // Get timezone name from OS
    let timezone = detect_timezone_name();

    // Detect NTP status
    let ntp_status = detect_ntp_status();

    TimeInfo {
        timezone,
        utc_offset_seconds,
        is_dst,
        timezone_abbr,
        ntp_status,
        // Monotonic clock is always available on modern systems
        // (Rust's std::time::Instant uses it internally)
        monotonic_available: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Timezone tests
    // ========================================

    #[test]
    fn test_detect_timezone_returns_valid_offset() {
        let info = detect_timezone();
        // UTC offset should be within reasonable bounds (-12h to +14h)
        assert!(info.utc_offset_seconds >= -12 * 3600);
        assert!(info.utc_offset_seconds <= 14 * 3600);
    }

    #[test]
    fn test_detect_timezone_monotonic_available() {
        let info = detect_timezone();
        assert!(info.monotonic_available);
    }

    #[test]
    fn test_detect_timezone_has_abbreviation() {
        let info = detect_timezone();
        assert!(info.timezone_abbr.is_some());
        let abbr = info.timezone_abbr.unwrap();
        // Abbreviations are typically 2-5 characters
        assert!(!abbr.is_empty());
        assert!(abbr.len() <= 10);
    }

    #[test]
    fn test_ntp_status_serialization() {
        use serde_json;

        let statuses = [
            NtpStatus::Synchronized,
            NtpStatus::Unsynchronized,
            NtpStatus::Inactive,
            NtpStatus::Unknown,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: NtpStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_ntp_status_default() {
        assert_eq!(NtpStatus::default(), NtpStatus::Unknown);
    }

    #[test]
    fn test_time_info_default() {
        let info = TimeInfo::default();
        assert!(info.timezone.is_none());
        assert_eq!(info.utc_offset_seconds, 0);
        assert!(!info.is_dst);
        assert!(info.timezone_abbr.is_none());
        assert_eq!(info.ntp_status, NtpStatus::Unknown);
        assert!(!info.monotonic_available);
    }

    #[test]
    fn test_extract_timezone_from_path_macos_style() {
        let path = "/var/db/timezone/zoneinfo/America/Los_Angeles";
        assert_eq!(
            extract_timezone_from_path(path),
            Some("America/Los_Angeles".to_string())
        );
    }

    #[test]
    fn test_extract_timezone_from_path_linux_style() {
        let path = "/usr/share/zoneinfo/Europe/London";
        assert_eq!(
            extract_timezone_from_path(path),
            Some("Europe/London".to_string())
        );
    }

    #[test]
    fn test_extract_timezone_from_path_posix() {
        // Some systems use paths like this
        let path = "/usr/share/zoneinfo/Etc/UTC";
        assert_eq!(
            extract_timezone_from_path(path),
            Some("Etc/UTC".to_string())
        );
    }

    #[test]
    fn test_extract_timezone_from_path_invalid() {
        assert_eq!(extract_timezone_from_path("/etc/localtime"), None);
        assert_eq!(extract_timezone_from_path(""), None);
        assert_eq!(extract_timezone_from_path("/some/random/path"), None);
    }

    #[test]
    fn test_detect_ntp_status_returns_valid_variant() {
        let status = detect_ntp_status();
        // Just verify it returns one of the valid variants (doesn't panic)
        matches!(
            status,
            NtpStatus::Synchronized
                | NtpStatus::Unsynchronized
                | NtpStatus::Inactive
                | NtpStatus::Unknown
        );
    }
}
