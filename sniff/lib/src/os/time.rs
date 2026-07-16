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
///
/// ## Platform Notes
///
/// - **Linux / macOS**: `timezone` is an IANA name (e.g. `America/Los_Angeles`).
/// - **Windows**: `timezone` is a mapped IANA name when the Windows zone is
///   recognised, otherwise the raw Windows timezone ID from `tzutil`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeInfo {
    /// Best-effort timezone identifier.
    ///
    /// On Linux/macOS this is an IANA name.  On Windows this is a mapped IANA
    /// name when possible, falling back to the raw Windows timezone ID.
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

/// Default timeout for NTP detection commands (3 seconds).
const NTP_TIMEOUT_SECS: u64 = 3;

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

/// Maps an IANA timezone name to its common abbreviation.
///
/// Covers US, European, and other major timezones. Returns `None` for
/// unmapped zones (the caller falls back to the numeric offset).
fn iana_to_abbreviation(iana: &str, is_dst: bool) -> Option<String> {
    let abbr = match iana {
        // US
        "America/New_York" | "America/Detroit" | "US/Eastern" => {
            if is_dst {
                "EDT"
            } else {
                "EST"
            }
        }
        "America/Chicago" | "America/Indiana/Knox" | "US/Central" => {
            if is_dst {
                "CDT"
            } else {
                "CST"
            }
        }
        "America/Denver" | "America/Boise" | "US/Mountain" => {
            if is_dst {
                "MDT"
            } else {
                "MST"
            }
        }
        "America/Los_Angeles" | "America/Tijuana" | "US/Pacific" => {
            if is_dst {
                "PDT"
            } else {
                "PST"
            }
        }
        "America/Anchorage" | "US/Alaska" => {
            if is_dst {
                "AKDT"
            } else {
                "AKST"
            }
        }
        "Pacific/Honolulu" | "US/Hawaii" => "HST",
        // Europe
        "Europe/London" | "Europe/Dublin" | "Europe/Lisbon" => {
            if is_dst {
                "BST"
            } else {
                "GMT"
            }
        }
        "Europe/Paris" | "Europe/Berlin" | "Europe/Rome" | "Europe/Madrid" | "Europe/Amsterdam"
        | "Europe/Brussels" | "Europe/Vienna" | "Europe/Zurich" | "Europe/Stockholm"
        | "Europe/Oslo" | "Europe/Copenhagen" | "Europe/Warsaw" | "Europe/Prague"
        | "Europe/Budapest" => {
            if is_dst {
                "CEST"
            } else {
                "CET"
            }
        }
        "Europe/Helsinki" | "Europe/Bucharest" | "Europe/Athens" | "Europe/Sofia"
        | "Europe/Tallinn" | "Europe/Riga" | "Europe/Vilnius" => {
            if is_dst {
                "EEST"
            } else {
                "EET"
            }
        }
        "Europe/Moscow" | "Europe/Minsk" => "MSK",
        // Asia / Oceania
        "Asia/Tokyo" | "Japan" => "JST",
        "Asia/Shanghai" | "Asia/Taipei" | "Asia/Hong_Kong" => "CST",
        "Asia/Kolkata" | "Asia/Calcutta" => "IST",
        "Asia/Singapore" | "Asia/Kuala_Lumpur" => "SGT",
        "Asia/Seoul" => "KST",
        "Australia/Sydney" | "Australia/Melbourne" => {
            if is_dst {
                "AEDT"
            } else {
                "AEST"
            }
        }
        "Australia/Perth" => "AWST",
        "Pacific/Auckland" | "NZ" => {
            if is_dst {
                "NZDT"
            } else {
                "NZST"
            }
        }
        _ => return None,
    };
    Some(abbr.to_string())
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
#[allow(dead_code)]
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
/// - **Windows**: Probes `tzutil /g` and maps the result to an IANA name
///   (falls back to the raw Windows timezone ID when unmapped)
///
/// ## Returns
///
/// The timezone identifier if detected, `None` otherwise.
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
    let raw_output = Command::new("tzutil")
        .arg("/g")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !raw_output.status.success() {
        return None;
    }

    let windows_id = parse_windows_timezone_id_output(&raw_output.stdout)?;
    Some(
        crate::os::windows_timezone_map::map_windows_timezone_to_iana(&windows_id)
            .map(|s| s.to_string())
            .unwrap_or(windows_id),
    )
}

/// Parse raw `tzutil /g` stdout into a clean Windows timezone ID.
///
/// Strips trailing CR/LF and whitespace, returning `None` for empty output.
/// This is a pure function so it can be unit-tested on any platform.
#[cfg(any(target_os = "windows", test))]
fn parse_windows_timezone_id_output(stdout: &[u8]) -> Option<String> {
    let id = std::str::from_utf8(stdout).ok()?;
    let trimmed = id.trim_end().trim_end_matches('\r').trim_end_matches('\n');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
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
/// - **macOS**: Queries `sntp` with the server from `/etc/ntp.conf` (5-second timeout)
/// - **Windows**: Queries `w32tm /query /status` with a 5-second timeout
///
/// ## Returns
///
/// The NTP synchronization status. Returns `Unknown` for permission errors,
/// unsupported platforms, or when the status cannot be determined.
#[cfg(target_os = "linux")]
pub fn detect_ntp_status() -> NtpStatus {
    // Use a single timedatectl call to check both NTP synchronized and NTP active status.
    // This halves the command overhead compared to two separate invocations.
    let output = run_command_with_timeout(
        "timedatectl",
        &["show", "--property=NTPSynchronized,NTP", "--value"],
        NTP_TIMEOUT_SECS,
    );

    let lines: Vec<&str> = output
        .as_deref()
        .map(|s| s.lines().collect())
        .unwrap_or_default();

    // timedatectl outputs values in the order the properties were requested:
    // line 0 -> NTPSynchronized, line 1 -> NTP
    let ntp_synchronized = lines.first().copied().map(str::trim);
    let ntp_active = lines.get(1).copied().map(str::trim);

    match ntp_synchronized {
        Some("yes") => NtpStatus::Synchronized,
        Some("no") => match ntp_active {
            Some("yes") => NtpStatus::Unsynchronized,
            Some("no") => NtpStatus::Inactive,
            _ => NtpStatus::Unknown,
        },
        _ => NtpStatus::Unknown,
    }
}

#[cfg(target_os = "macos")]
pub fn detect_ntp_status() -> NtpStatus {
    // Read the configured NTP server from /etc/ntp.conf, fall back to time.apple.com
    let server = std::fs::read_to_string("/etc/ntp.conf")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with("server "))
                .and_then(|line| line.split_whitespace().nth(1))
                .map(String::from)
        })
        .unwrap_or_else(|| "time.apple.com".to_string());

    // sntp ships with macOS and works without admin privileges
    let output = run_command_with_timeout("sntp", &[&server], NTP_TIMEOUT_SECS);
    match output {
        // A successful response contains the offset, e.g. "+0.001527 +/- 0.004895 time.apple.com"
        Some(text) if text.contains("+/-") => NtpStatus::Synchronized,
        _ => NtpStatus::Unknown,
    }
}

#[cfg(target_os = "windows")]
pub fn detect_ntp_status() -> NtpStatus {
    let output = run_command_with_timeout("w32tm", &["/query", "/status"], NTP_TIMEOUT_SECS);
    match output {
        Some(text) if text.contains("Leap Indicator: 0") => NtpStatus::Synchronized,
        Some(text) if text.contains("Leap Indicator:") => NtpStatus::Unsynchronized,
        _ => NtpStatus::Unknown,
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn detect_ntp_status() -> NtpStatus {
    NtpStatus::Unknown
}

// Test-only stand-in for the live NTP probe. `None` (the default) selects
// production behavior.
#[cfg(test)]
thread_local! {
    static NTP_PROBE_STUB: std::cell::Cell<Option<NtpStatus>> =
        const { std::cell::Cell::new(None) };
}

/// Resolves the NTP synchronization status for [`detect_timezone_with_options`].
///
/// This is the crate-internal decision seam sitting below both public entry
/// points. Production always runs the live [`detect_ntp_status`] probe; in test
/// builds a thread-local stub can stand in for it, letting Sniff prove that the
/// bare API selects a probe and the configurable API honors both flag values
/// without issuing a live NTP request.
fn run_ntp_probe() -> NtpStatus {
    #[cfg(test)]
    if let Some(status) = NTP_PROBE_STUB.with(std::cell::Cell::get) {
        return status;
    }
    detect_ntp_status()
}

/// Detects timezone and time-related system information, with optional NTP probing.
///
/// When `probe_ntp` is `false`, the `ntp_status` field is set to
/// [`NtpStatus::Unknown`] without invoking any external commands, keeping
/// this function fast and purely local.  When `probe_ntp` is `true`,
/// [`detect_ntp_status`] is called, which can take up to 10 seconds on Linux.
///
/// ## Examples
///
/// ```
/// use sniff::os::detect_timezone_with_options;
///
/// // Cheap: timezone data only
/// let time_info = detect_timezone_with_options(false);
/// println!("Timezone: {:?}", time_info.timezone);
/// println!("UTC offset: {} seconds", time_info.utc_offset_seconds);
///
/// // Full: includes NTP probe
/// let time_info = detect_timezone_with_options(true);
/// println!("NTP status: {:?}", time_info.ntp_status);
/// ```
///
/// ## Returns
///
/// A [`TimeInfo`] struct containing all detected time information.
/// Fields that cannot be detected will have sensible defaults.
pub fn detect_timezone_with_options(probe_ntp: bool) -> TimeInfo {
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

    // Get timezone name from OS (IANA, e.g., "America/Los_Angeles")
    let timezone = detect_timezone_name();

    // Get timezone abbreviation: chrono's %Z returns the offset string
    // on some platforms (notably macOS) instead of the abbreviation.
    // Fall back to deriving it from the IANA name + DST status.
    let chrono_tz = now.format("%Z").to_string();
    let timezone_abbr = if chrono_tz.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(chrono_tz)
    } else {
        timezone
            .as_deref()
            .and_then(|iana| iana_to_abbreviation(iana, is_dst))
            .or(Some(chrono_tz))
    };

    let ntp_status = if probe_ntp {
        run_ntp_probe()
    } else {
        NtpStatus::Unknown
    };

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

/// Detects timezone and time-related system information.
///
/// This zero-argument convenience API reports the **full** status: it is
/// equivalent to calling [`detect_timezone_with_options`] with `true`, so it
/// runs the NTP probe and populates `ntp_status`. The probe can take up to
/// several seconds; callers that only need local timezone data and want to
/// avoid the network round-trip should call `detect_timezone_with_options(false)`.
///
/// ## Examples
///
/// ```
/// use sniff::os::detect_timezone;
///
/// let time_info = detect_timezone();
/// println!("Timezone: {:?}", time_info.timezone);
/// println!("UTC offset: {} seconds", time_info.utc_offset_seconds);
/// println!("DST active: {}", time_info.is_dst);
/// ```
///
/// ## Returns
///
/// A [`TimeInfo`] struct containing all detected time information.
/// Fields that cannot be detected will have sensible defaults.
pub fn detect_timezone() -> TimeInfo {
    detect_timezone_with_options(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `f` with the NTP probe stubbed to `status`, resetting the stub
    /// afterward so the seam does not leak across tests (nextest isolates each
    /// test in its own process, but a plain `cargo test` thread pool reuses
    /// worker threads, so the thread-local is always cleared here).
    fn with_ntp_stub<R>(status: NtpStatus, f: impl FnOnce() -> R) -> R {
        NTP_PROBE_STUB.with(|c| c.set(Some(status)));
        let result = f();
        NTP_PROBE_STUB.with(|c| c.set(None));
        result
    }

    // ========================================
    // Timezone tests
    // ========================================

    /// Finding 1 restore: the bare convenience API requests the NTP probe
    /// (selects `probe_ntp = true`). Proven through the crate-internal stub so
    /// no live network request is made.
    #[test]
    fn bare_detect_timezone_probes_ntp() {
        let info = with_ntp_stub(NtpStatus::Synchronized, detect_timezone);
        assert_eq!(
            info.ntp_status,
            NtpStatus::Synchronized,
            "bare detect_timezone() must invoke the NTP probe",
        );
    }

    /// The configurable API honors both flag values: `true` invokes the probe,
    /// `false` skips it and leaves `ntp_status` at [`NtpStatus::Unknown`].
    #[test]
    fn detect_timezone_with_options_respects_probe_flag() {
        let probed = with_ntp_stub(NtpStatus::Synchronized, || {
            detect_timezone_with_options(true)
        });
        assert_eq!(
            probed.ntp_status,
            NtpStatus::Synchronized,
            "probe_ntp = true must invoke the probe",
        );

        let skipped = with_ntp_stub(NtpStatus::Synchronized, || {
            detect_timezone_with_options(false)
        });
        assert_eq!(
            skipped.ntp_status,
            NtpStatus::Unknown,
            "probe_ntp = false must skip the probe",
        );
    }

    #[test]
    fn test_detect_timezone_returns_valid_offset() {
        // Stub the probe: this test asserts only timezone data, which is
        // probe-independent, and must not hit the network.
        let info = with_ntp_stub(NtpStatus::Unknown, detect_timezone);
        // UTC offset should be within reasonable bounds (-12h to +14h)
        assert!(info.utc_offset_seconds >= -12 * 3600);
        assert!(info.utc_offset_seconds <= 14 * 3600);
    }

    #[test]
    fn test_detect_timezone_monotonic_available() {
        let info = with_ntp_stub(NtpStatus::Unknown, detect_timezone);
        assert!(info.monotonic_available);
    }

    #[test]
    fn test_detect_timezone_has_abbreviation() {
        let info = with_ntp_stub(NtpStatus::Unknown, detect_timezone);
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

    #[test]
    fn test_parse_windows_timezone_id_trims_output() {
        let raw = b"Pacific Standard Time\r\n";
        let result = parse_windows_timezone_id_output(raw);
        assert_eq!(result, Some("Pacific Standard Time".to_string()));

        let trailing_only = b"  Eastern Standard Time  \r\n";
        let result2 = parse_windows_timezone_id_output(trailing_only);
        assert_eq!(result2, Some("  Eastern Standard Time".to_string()));
    }

    #[test]
    fn test_parse_windows_timezone_id_rejects_empty() {
        assert_eq!(parse_windows_timezone_id_output(b""), None);
        assert_eq!(parse_windows_timezone_id_output(b"\r\n"), None);
        assert_eq!(parse_windows_timezone_id_output(b"   \r\n"), None);
    }

    #[test]
    fn test_windows_timezone_map_common_ids() {
        use crate::os::windows_timezone_map::map_windows_timezone_to_iana;

        assert_eq!(
            map_windows_timezone_to_iana("Pacific Standard Time"),
            Some("America/Los_Angeles")
        );
        assert_eq!(map_windows_timezone_to_iana("UTC"), Some("Etc/UTC"));
        assert_eq!(
            map_windows_timezone_to_iana("W. Europe Standard Time"),
            Some("Europe/Berlin")
        );
    }

    #[test]
    fn test_windows_timezone_map_unknown_returns_none() {
        use crate::os::windows_timezone_map::map_windows_timezone_to_iana;

        assert_eq!(map_windows_timezone_to_iana("Nonexistent/Zone"), None);
        assert_eq!(map_windows_timezone_to_iana(""), None);
    }

    // ============================================================================
    // NTP timeout tests (issue #19)
    // ============================================================================

    #[test]
    fn test_ntp_timeout_is_three_seconds() {
        // The timeout was reduced from 5s to 3s as part of issue #19.
        assert_eq!(NTP_TIMEOUT_SECS, 3);
    }

    #[test]
    fn test_run_command_with_timeout_respects_timeout() {
        // Use a command that sleeps longer than the timeout
        let start = std::time::Instant::now();
        let result = run_command_with_timeout("sleep", &["10"], 1);
        let elapsed = start.elapsed();

        // Should return None (timeout) and take less than 5 seconds
        assert!(result.is_none());
        assert!(elapsed < std::time::Duration::from_secs(5));
    }
}
