use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use sysinfo::System;

use crate::Result;

// ============================================================================
// OS Type Detection
// ============================================================================

/// Operating system type classification.
///
/// Represents the high-level operating system category detected from
/// the runtime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OsType {
    /// Microsoft Windows
    Windows,
    /// Linux (any distribution)
    Linux,
    /// Apple macOS
    MacOS,
    /// Apple iOS
    IOS,
    /// Google Android
    Android,
    /// FreeBSD
    FreeBSD,
    /// OpenBSD
    OpenBSD,
    /// NetBSD
    NetBSD,
    /// Unknown or unsupported operating system
    #[default]
    Other,
}

impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsType::Windows => write!(f, "Windows"),
            OsType::Linux => write!(f, "Linux"),
            OsType::MacOS => write!(f, "macOS"),
            OsType::IOS => write!(f, "iOS"),
            OsType::Android => write!(f, "Android"),
            OsType::FreeBSD => write!(f, "FreeBSD"),
            OsType::OpenBSD => write!(f, "OpenBSD"),
            OsType::NetBSD => write!(f, "NetBSD"),
            OsType::Other => write!(f, "Other"),
        }
    }
}

/// Detects the operating system type from the runtime environment.
///
/// Uses `std::env::consts::OS` to determine the current platform
/// and maps it to an [`OsType`] variant.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::detect_os_type;
///
/// let os_type = detect_os_type();
/// println!("Running on: {}", os_type);
/// ```
///
/// ## Returns
///
/// The detected [`OsType`], or [`OsType::Other`] if the platform
/// is not recognized.
pub fn detect_os_type() -> OsType {
    match std::env::consts::OS {
        "windows" => OsType::Windows,
        "linux" => OsType::Linux,
        "macos" => OsType::MacOS,
        "ios" => OsType::IOS,
        "android" => OsType::Android,
        "freebsd" => OsType::FreeBSD,
        "openbsd" => OsType::OpenBSD,
        "netbsd" => OsType::NetBSD,
        _ => OsType::Other,
    }
}

// ============================================================================
// Linux Distribution Detection
// ============================================================================

/// Linux distribution family classification.
///
/// Groups Linux distributions by their upstream lineage or package
/// management system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LinuxFamily {
    /// Debian-based distributions (apt/dpkg)
    Debian,
    /// Red Hat-based distributions (dnf/yum/rpm)
    RedHat,
    /// Arch-based distributions (pacman)
    Arch,
    /// SUSE-based distributions (zypper/rpm)
    SUSE,
    /// Gentoo-based distributions (portage)
    Gentoo,
    /// Alpine Linux (apk)
    Alpine,
    /// Void Linux (xbps)
    Void,
    /// Slackware-based distributions
    Slackware,
    /// NixOS (nix)
    NixOS,
    /// Unknown or unclassified distribution
    #[default]
    Other,
}

impl std::fmt::Display for LinuxFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinuxFamily::Debian => write!(f, "Debian"),
            LinuxFamily::RedHat => write!(f, "Red Hat"),
            LinuxFamily::Arch => write!(f, "Arch"),
            LinuxFamily::SUSE => write!(f, "SUSE"),
            LinuxFamily::Gentoo => write!(f, "Gentoo"),
            LinuxFamily::Alpine => write!(f, "Alpine"),
            LinuxFamily::Void => write!(f, "Void"),
            LinuxFamily::Slackware => write!(f, "Slackware"),
            LinuxFamily::NixOS => write!(f, "NixOS"),
            LinuxFamily::Other => write!(f, "Other"),
        }
    }
}

/// Linux distribution information.
///
/// Contains detailed information about a specific Linux distribution,
/// parsed from system release files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinuxDistro {
    /// Distribution identifier (e.g., "ubuntu", "fedora", "arch")
    pub id: String,
    /// Human-readable distribution name (e.g., "Ubuntu", "Fedora Linux")
    pub name: String,
    /// Distribution version (e.g., "22.04", "39")
    pub version: Option<String>,
    /// Version codename (e.g., "jammy", "noble")
    pub codename: Option<String>,
    /// Distribution family classification
    pub family: LinuxFamily,
}

/// Infers the Linux distribution family from a distribution ID.
///
/// Maps common distribution identifiers to their parent family
/// based on package management lineage.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::{infer_linux_family, LinuxFamily};
///
/// assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
/// assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
/// assert_eq!(infer_linux_family("manjaro"), LinuxFamily::Arch);
/// ```
///
/// ## Arguments
///
/// * `distro_id` - The lowercase distribution identifier (e.g., from `ID=` in os-release)
///
/// ## Returns
///
/// The inferred [`LinuxFamily`], or [`LinuxFamily::Other`] if the
/// distribution is not recognized.
pub fn infer_linux_family(distro_id: &str) -> LinuxFamily {
    let id = distro_id.to_lowercase();

    // Debian family (apt/dpkg)
    const DEBIAN_DISTROS: &[&str] = &[
        "debian",
        "ubuntu",
        "mint",
        "linuxmint",
        "pop",
        "pop_os",
        "elementary",
        "zorin",
        "kali",
        "raspbian",
        "mx",
        "mxlinux",
        "lmde",
        "devuan",
        "parrot",
        "pureos",
        "deepin",
        "bunsenlabs",
        "antiX",
        "antix",
        "steamos",
    ];

    // Red Hat family (dnf/yum/rpm)
    const REDHAT_DISTROS: &[&str] = &[
        "fedora",
        "rhel",
        "centos",
        "rocky",
        "rockylinux",
        "alma",
        "almalinux",
        "oracle",
        "oraclelinux",
        "amazon",
        "amzn",
        "scientific",
        "clearos",
        "eurolinux",
        "navy",
    ];

    // Arch family (pacman)
    const ARCH_DISTROS: &[&str] = &[
        "arch",
        "archlinux",
        "manjaro",
        "endeavouros",
        "garuda",
        "artix",
        "arcolinux",
        "cachyos",
        "crystal",
        "rebornos",
        "archcraft",
        "bluestar",
    ];

    // SUSE family (zypper/rpm)
    const SUSE_DISTROS: &[&str] = &[
        "opensuse",
        "suse",
        "sles",
        "opensuse-leap",
        "opensuse-tumbleweed",
    ];

    // Check each family
    if DEBIAN_DISTROS
        .iter()
        .any(|d| id == *d || id.starts_with(&format!("{d}-")))
    {
        return LinuxFamily::Debian;
    }

    if REDHAT_DISTROS
        .iter()
        .any(|d| id == *d || id.starts_with(&format!("{d}-")))
    {
        return LinuxFamily::RedHat;
    }

    if ARCH_DISTROS
        .iter()
        .any(|d| id == *d || id.starts_with(&format!("{d}-")))
    {
        return LinuxFamily::Arch;
    }

    if SUSE_DISTROS
        .iter()
        .any(|d| id == *d || id.starts_with(&format!("{d}-")))
    {
        return LinuxFamily::SUSE;
    }

    // Single-distro families
    match id.as_str() {
        "alpine" => LinuxFamily::Alpine,
        "void" => LinuxFamily::Void,
        "slackware" => LinuxFamily::Slackware,
        "nixos" => LinuxFamily::NixOS,
        "gentoo" | "funtoo" | "calculate" => LinuxFamily::Gentoo,
        _ => LinuxFamily::Other,
    }
}

/// Detects Linux distribution information from system files.
///
/// Attempts to read distribution information using a fallback chain:
/// 1. `/etc/os-release` (freedesktop.org standard)
/// 2. `/etc/lsb-release` (LSB standard)
/// 3. `/etc/system-release` (Red Hat legacy)
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::detect_linux_distro;
///
/// if let Some(distro) = detect_linux_distro() {
///     println!("Distribution: {} ({})", distro.name, distro.id);
///     println!("Family: {}", distro.family);
///     if let Some(ver) = &distro.version {
///         println!("Version: {}", ver);
///     }
/// }
/// ```
///
/// ## Returns
///
/// - `Some(LinuxDistro)` with detected distribution information
/// - `None` if not running on Linux or if detection fails
pub fn detect_linux_distro() -> Option<LinuxDistro> {
    // Only attempt detection on Linux
    if detect_os_type() != OsType::Linux {
        return None;
    }

    detect_linux_distro_from_paths(
        Path::new("/etc/os-release"),
        Path::new("/etc/lsb-release"),
        Path::new("/etc/system-release"),
    )
}

/// Internal function that allows testing with custom file paths.
fn detect_linux_distro_from_paths(
    os_release_path: &Path,
    lsb_release_path: &Path,
    system_release_path: &Path,
) -> Option<LinuxDistro> {
    // Try /etc/os-release first (freedesktop.org standard)
    if let Some(distro) = parse_os_release(os_release_path) {
        return Some(distro);
    }

    // Fallback to /etc/lsb-release
    if let Some(distro) = parse_lsb_release(lsb_release_path) {
        return Some(distro);
    }

    // Fallback to /etc/system-release (Red Hat legacy)
    if let Some(distro) = parse_system_release(system_release_path) {
        return Some(distro);
    }

    None
}

/// Parses `/etc/os-release` format.
///
/// The os-release file uses a shell-compatible variable assignment format:
/// ```text
/// ID=ubuntu
/// NAME="Ubuntu"
/// VERSION_ID="22.04"
/// VERSION_CODENAME=jammy
/// ```
fn parse_os_release(path: &Path) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_os_release_content(&content)
}

/// Parses os-release content from a string.
///
/// Exposed for testing purposes.
pub fn parse_os_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = String::new();
    let mut name = String::new();
    let mut version = None;
    let mut codename = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "ID" => id = value.to_lowercase(),
                "NAME" => name = value.to_string(),
                "VERSION_ID" => version = Some(value.to_string()),
                "VERSION_CODENAME" => {
                    if !value.is_empty() {
                        codename = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    if id.is_empty() && name.is_empty() {
        return None;
    }

    // Use ID for name if NAME is empty
    if name.is_empty() {
        name = id.clone();
    }

    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id,
        name,
        version,
        codename,
        family,
    })
}

/// Parses `/etc/lsb-release` format.
///
/// The lsb-release file uses a similar format to os-release:
/// ```text
/// DISTRIB_ID=Ubuntu
/// DISTRIB_RELEASE=22.04
/// DISTRIB_CODENAME=jammy
/// DISTRIB_DESCRIPTION="Ubuntu 22.04.3 LTS"
/// ```
fn parse_lsb_release(path: &Path) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_lsb_release_content(&content)
}

/// Parses lsb-release content from a string.
///
/// Exposed for testing purposes.
pub fn parse_lsb_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = String::new();
    let mut name = String::new();
    let mut version = None;
    let mut codename = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "DISTRIB_ID" => {
                    id = value.to_lowercase();
                    name = value.to_string();
                }
                "DISTRIB_RELEASE" => version = Some(value.to_string()),
                "DISTRIB_CODENAME" => {
                    if !value.is_empty() {
                        codename = Some(value.to_string());
                    }
                }
                "DISTRIB_DESCRIPTION" => {
                    // Use description as name if we have it
                    if !value.is_empty() {
                        name = value.to_string();
                    }
                }
                _ => {}
            }
        }
    }

    if id.is_empty() {
        return None;
    }

    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id,
        name,
        version,
        codename,
        family,
    })
}

/// Parses `/etc/system-release` format.
///
/// The system-release file contains a single line with the distribution
/// name and version:
/// ```text
/// Fedora release 39 (Thirty Nine)
/// CentOS Linux release 7.9.2009 (Core)
/// ```
fn parse_system_release(path: &Path) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_system_release_content(&content)
}

/// Parses system-release content from a string.
///
/// Exposed for testing purposes.
pub fn parse_system_release_content(content: &str) -> Option<LinuxDistro> {
    let line = content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }

    // Try to extract: "Name release Version (Codename)"
    // or: "Name Linux release Version (Codename)"
    let name = line.to_string();

    // Extract ID from the first word (lowercase)
    let id = line
        .split_whitespace()
        .next()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Try to extract version (number after "release")
    let version = line.to_lowercase().find("release").and_then(|pos| {
        let after_release = &line[pos + "release".len()..];
        after_release
            .split_whitespace()
            .next()
            .map(|v| v.to_string())
    });

    // Try to extract codename (text in parentheses)
    let codename = if let (Some(start), Some(end)) = (line.rfind('('), line.rfind(')')) {
        if start < end {
            Some(line[start + 1..end].to_string())
        } else {
            None
        }
    } else {
        None
    };

    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id,
        name,
        version,
        codename,
        family,
    })
}

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

/// Operating system information.
///
/// Contains details about the operating system, kernel version,
/// architecture, hostname, package managers, locale, and timezone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsInfo {
    /// Operating system type classification
    pub os_type: OsType,
    /// Operating system name (e.g., "macOS", "Linux", "Windows")
    pub name: String,
    /// Operating system version (short form)
    pub version: String,
    /// Long OS version with additional details (e.g., "macOS 14.5 Sonoma")
    pub long_version: Option<String>,
    /// Linux distribution ID (e.g., "ubuntu", "fedora", "arch")
    /// None on non-Linux systems
    pub distribution: Option<String>,
    /// Detailed Linux distribution information
    /// None on non-Linux systems or if detection fails
    pub linux_distro: Option<LinuxDistro>,
    /// Kernel version
    pub kernel: String,
    /// System architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// System hostname
    pub hostname: String,
    /// System package managers detected on the system
    pub system_package_managers: Option<SystemPackageManagers>,
    /// Locale and language settings
    pub locale: Option<LocaleInfo>,
    /// Timezone and time synchronization information
    pub time: Option<TimeInfo>,
}

/// Detects operating system information from the current system.
///
/// This function gathers OS type, distribution details, package managers,
/// locale, and timezone information.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::detect_os;
///
/// let os = detect_os().unwrap();
/// println!("OS: {} {}", os.name, os.version);
/// println!("Arch: {}", os.arch);
/// println!("Hostname: {}", os.hostname);
/// ```
///
/// ## Errors
///
/// Currently returns `Ok` in all cases, but future versions may return
/// errors for system information gathering failures.
pub fn detect_os() -> Result<OsInfo> {
    // Helper to convert empty strings to None
    let non_empty = |s: String| if s.is_empty() { None } else { Some(s) };

    let os_type = detect_os_type();
    let linux_distro = detect_linux_distro();
    // Extract linux family before moving linux_distro into OsInfo
    let linux_family = linux_distro.as_ref().map(|d| d.family);

    // Detect system package managers based on OS type
    let system_package_managers = match os_type {
        OsType::Linux => Some(detect_linux_package_managers(linux_family)),
        OsType::MacOS => Some(detect_macos_package_managers()),
        OsType::Windows => Some(detect_windows_package_managers()),
        OsType::FreeBSD | OsType::OpenBSD | OsType::NetBSD => {
            Some(detect_bsd_package_managers(os_type))
        }
        OsType::IOS | OsType::Android | OsType::Other => None,
    };

    // Detect locale settings
    let locale = Some(detect_locale());

    // Detect timezone and time information
    let time = Some(detect_timezone());

    Ok(OsInfo {
        os_type,
        name: System::name().unwrap_or_default(),
        version: System::os_version().unwrap_or_default(),
        long_version: System::long_os_version(),
        distribution: non_empty(System::distribution_id()),
        linux_distro,
        kernel: System::kernel_version().unwrap_or_default(),
        arch: {
            let arch = System::cpu_arch();
            if arch.is_empty() {
                std::env::consts::ARCH.to_string()
            } else {
                arch
            }
        },
        hostname: System::host_name().unwrap_or_default(),
        system_package_managers,
        locale,
        time,
    })
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
fn run_command_with_timeout(cmd: &str, args: &[&str], timeout_secs: u64) -> Option<String> {
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
fn extract_timezone_from_path(path: &str) -> Option<String> {
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

/// Locale information from environment variables.
///
/// Contains the various LC_* and LANG environment variables used to
/// configure system locale settings. Also provides extracted language
/// code and encoding from the highest-priority locale setting.
///
/// ## Locale String Format
///
/// Locale strings follow the format: `language[_territory][.codeset][@modifier]`
///
/// Examples:
/// - `en_US.UTF-8` - English, United States, UTF-8 encoding
/// - `de_DE.ISO-8859-1` - German, Germany, ISO-8859-1 encoding
/// - `zh_CN.GB18030` - Chinese, China, GB18030 encoding
/// - `C` or `POSIX` - Minimal/portable locale
///
/// ## Priority Order
///
/// The preferred language is determined by checking (in order):
/// 1. `LC_ALL` - Overrides all other LC_* variables
/// 2. `LC_MESSAGES` - Language for messages
/// 3. `LANG` - Default locale
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocaleInfo {
    /// LANG environment variable (default locale)
    pub lang: Option<String>,
    /// LC_ALL environment variable (highest priority, overrides all others)
    pub lc_all: Option<String>,
    /// LC_CTYPE environment variable (character classification)
    pub lc_ctype: Option<String>,
    /// LC_MESSAGES environment variable (message language)
    pub lc_messages: Option<String>,
    /// LC_TIME environment variable (date/time formatting)
    pub lc_time: Option<String>,
    /// Extracted language code (e.g., "en" from "en_US.UTF-8")
    pub preferred_language: Option<String>,
    /// Extracted encoding (e.g., "UTF-8" from "en_US.UTF-8")
    pub encoding: Option<String>,
}

/// Detects locale information from environment variables.
///
/// Reads the standard locale environment variables (LANG, LC_ALL, LC_CTYPE,
/// LC_MESSAGES, LC_TIME) and extracts the preferred language and encoding
/// based on priority rules.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::detect_locale;
///
/// let locale = detect_locale();
/// if let Some(lang) = &locale.preferred_language {
///     println!("Preferred language: {}", lang);
/// }
/// if let Some(enc) = &locale.encoding {
///     println!("Encoding: {}", enc);
/// }
/// ```
///
/// ## Priority Rules
///
/// The preferred language and encoding are extracted from the first
/// non-empty, non-"C", non-"POSIX" value in this order:
/// 1. `LC_ALL`
/// 2. `LC_MESSAGES`
/// 3. `LANG`
pub fn detect_locale() -> LocaleInfo {
    let lang = std::env::var("LANG").ok();
    let lc_all = std::env::var("LC_ALL").ok();
    let lc_ctype = std::env::var("LC_CTYPE").ok();
    let lc_messages = std::env::var("LC_MESSAGES").ok();
    let lc_time = std::env::var("LC_TIME").ok();

    // Determine the effective locale for language/encoding extraction
    // Priority: LC_ALL > LC_MESSAGES > LANG
    let effective_locale = lc_all
        .as_deref()
        .filter(|s| is_extractable_locale(s))
        .or_else(|| lc_messages.as_deref().filter(|s| is_extractable_locale(s)))
        .or_else(|| lang.as_deref().filter(|s| is_extractable_locale(s)));

    let preferred_language = effective_locale.and_then(extract_language_code);
    let encoding = effective_locale.and_then(extract_encoding);

    LocaleInfo {
        lang,
        lc_all,
        lc_ctype,
        lc_messages,
        lc_time,
        preferred_language,
        encoding,
    }
}

/// Checks if a locale string can yield meaningful language/encoding info.
///
/// Returns `false` for empty strings, "C", and "POSIX" locales which
/// don't contain extractable language or encoding information.
fn is_extractable_locale(locale: &str) -> bool {
    !locale.is_empty() && locale != "C" && locale != "POSIX"
}

/// Extracts the language code from a locale string.
///
/// Parses locale strings in the format `language[_territory][.codeset][@modifier]`
/// and returns just the language portion.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::extract_language_code;
///
/// assert_eq!(extract_language_code("en_US.UTF-8"), Some("en".to_string()));
/// assert_eq!(extract_language_code("de_DE"), Some("de".to_string()));
/// assert_eq!(extract_language_code("zh"), Some("zh".to_string()));
/// assert_eq!(extract_language_code("C"), None);
/// assert_eq!(extract_language_code("POSIX"), None);
/// ```
///
/// ## Returns
///
/// - `Some(language)` - The extracted language code
/// - `None` - If the locale is empty, "C", or "POSIX"
pub fn extract_language_code(locale: &str) -> Option<String> {
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return None;
    }

    // Extract language before any separator (_, ., @)
    let language = locale.split(['_', '.', '@']).next()?;

    if language.is_empty() {
        None
    } else {
        Some(language.to_string())
    }
}

/// Extracts the encoding from a locale string.
///
/// Parses locale strings in the format `language[_territory][.codeset][@modifier]`
/// and returns just the codeset/encoding portion.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::extract_encoding;
///
/// assert_eq!(extract_encoding("en_US.UTF-8"), Some("UTF-8".to_string()));
/// assert_eq!(extract_encoding("de_DE.ISO-8859-1"), Some("ISO-8859-1".to_string()));
/// assert_eq!(extract_encoding("zh_CN.GB18030@stroke"), Some("GB18030".to_string()));
/// assert_eq!(extract_encoding("en_US"), None);
/// assert_eq!(extract_encoding("C"), None);
/// ```
///
/// ## Returns
///
/// - `Some(encoding)` - The extracted encoding/codeset
/// - `None` - If no encoding is present in the locale string
pub fn extract_encoding(locale: &str) -> Option<String> {
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return None;
    }

    // Find the position after the dot
    let dot_pos = locale.find('.')?;
    let after_dot = &locale[dot_pos + 1..];

    if after_dot.is_empty() {
        return None;
    }

    // Extract encoding up to any modifier (@)
    let encoding = after_dot.split('@').next()?;

    if encoding.is_empty() {
        None
    } else {
        Some(encoding.to_string())
    }
}

// ============================================================================
// Package Manager Detection Infrastructure
// ============================================================================

use std::hash::Hash;
use std::path::PathBuf;

/// System package manager identification.
///
/// Represents all known system-level package managers across different
/// operating systems and distributions. Each variant corresponds to a
/// specific package management tool.
///
/// ## Categories
///
/// - **Debian family**: Apt, Aptitude, Dpkg, Nala, AptFast
/// - **RedHat family**: Dnf, Yum, Microdnf, Rpm
/// - **Arch family**: Pacman, Makepkg, Yay, Paru, Pamac
/// - **SUSE**: Zypper
/// - **Gentoo**: Portage (emerge)
/// - **Alpine**: Apk
/// - **Void**: Xbps
/// - **Slackware**: Pkgtool
/// - **Cross-distro**: Snap, Flatpak, Guix, Nix, NixEnv
/// - **macOS**: Homebrew, MacPorts, Fink, Softwareupdate
/// - **Windows**: Winget, Dism, Chocolatey, Scoop, Msys2Pacman
/// - **BSD**: Pkg, Ports, PkgAdd, Pkgin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SystemPackageManager {
    // Debian family
    /// APT (Advanced Package Tool) - Debian/Ubuntu primary package manager
    Apt,
    /// Aptitude - High-level Debian package manager with ncurses interface
    Aptitude,
    /// dpkg - Low-level Debian package manager
    Dpkg,
    /// Nala - Modern apt frontend with parallel downloads
    Nala,
    /// apt-fast - apt accelerator using aria2c
    AptFast,

    // RedHat family
    /// DNF (Dandified YUM) - Fedora/RHEL 8+ package manager
    Dnf,
    /// YUM (Yellowdog Updater Modified) - Legacy RHEL/CentOS package manager
    Yum,
    /// microdnf - Minimal DNF for containers
    Microdnf,
    /// RPM - Low-level RedHat package manager
    Rpm,

    // Arch family
    /// pacman - Arch Linux package manager
    Pacman,
    /// makepkg - Arch build tool for AUR packages
    Makepkg,
    /// yay - Yet Another Yogurt, AUR helper
    Yay,
    /// paru - Feature-rich AUR helper written in Rust
    Paru,
    /// pamac - Manjaro's graphical package manager
    Pamac,

    // SUSE
    /// zypper - SUSE/openSUSE package manager
    Zypper,

    // Gentoo
    /// Portage/emerge - Gentoo source-based package manager
    Portage,

    // Alpine
    /// apk - Alpine Linux package manager
    Apk,

    // Void
    /// xbps - X Binary Package System for Void Linux
    Xbps,

    // Slackware
    /// pkgtool - Slackware package manager
    Pkgtool,

    // Cross-distro
    /// Snap - Canonical's universal package format
    Snap,
    /// Flatpak - Cross-distro application sandboxing
    Flatpak,
    /// GNU Guix - Functional package manager
    Guix,
    /// Nix - Nix package manager (nix-env, nix profile)
    Nix,
    /// nix-env - Legacy Nix profile management
    NixEnv,

    // macOS
    /// Homebrew - macOS/Linux community package manager
    Homebrew,
    /// MacPorts - macOS package manager (formerly DarwinPorts)
    MacPorts,
    /// Fink - Debian-based macOS package manager
    Fink,
    /// softwareupdate - macOS system update tool
    Softwareupdate,

    // Windows
    /// winget - Windows Package Manager
    Winget,
    /// DISM - Windows Deployment Image Servicing and Management
    Dism,
    /// Chocolatey - Windows community package manager
    Chocolatey,
    /// Scoop - Windows command-line installer
    Scoop,
    /// MSYS2's pacman - Windows Unix-like environment
    Msys2Pacman,

    // BSD
    /// pkg - FreeBSD package manager
    Pkg,
    /// ports - BSD ports collection
    Ports,
    /// pkg_add - OpenBSD package manager
    PkgAdd,
    /// pkgin - NetBSD binary package manager
    Pkgin,
}

impl std::fmt::Display for SystemPackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            // Debian family
            SystemPackageManager::Apt => "apt",
            SystemPackageManager::Aptitude => "aptitude",
            SystemPackageManager::Dpkg => "dpkg",
            SystemPackageManager::Nala => "nala",
            SystemPackageManager::AptFast => "apt-fast",
            // RedHat family
            SystemPackageManager::Dnf => "dnf",
            SystemPackageManager::Yum => "yum",
            SystemPackageManager::Microdnf => "microdnf",
            SystemPackageManager::Rpm => "rpm",
            // Arch family
            SystemPackageManager::Pacman => "pacman",
            SystemPackageManager::Makepkg => "makepkg",
            SystemPackageManager::Yay => "yay",
            SystemPackageManager::Paru => "paru",
            SystemPackageManager::Pamac => "pamac",
            // SUSE
            SystemPackageManager::Zypper => "zypper",
            // Gentoo
            SystemPackageManager::Portage => "emerge",
            // Alpine
            SystemPackageManager::Apk => "apk",
            // Void
            SystemPackageManager::Xbps => "xbps-install",
            // Slackware
            SystemPackageManager::Pkgtool => "pkgtool",
            // Cross-distro
            SystemPackageManager::Snap => "snap",
            SystemPackageManager::Flatpak => "flatpak",
            SystemPackageManager::Guix => "guix",
            SystemPackageManager::Nix => "nix",
            SystemPackageManager::NixEnv => "nix-env",
            // macOS
            SystemPackageManager::Homebrew => "brew",
            SystemPackageManager::MacPorts => "port",
            SystemPackageManager::Fink => "fink",
            SystemPackageManager::Softwareupdate => "softwareupdate",
            // Windows
            SystemPackageManager::Winget => "winget",
            SystemPackageManager::Dism => "dism",
            SystemPackageManager::Chocolatey => "choco",
            SystemPackageManager::Scoop => "scoop",
            SystemPackageManager::Msys2Pacman => "pacman (MSYS2)",
            // BSD
            SystemPackageManager::Pkg => "pkg",
            SystemPackageManager::Ports => "ports",
            SystemPackageManager::PkgAdd => "pkg_add",
            SystemPackageManager::Pkgin => "pkgin",
        };
        write!(f, "{}", name)
    }
}

impl SystemPackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::os::SystemPackageManager;
    ///
    /// assert_eq!(SystemPackageManager::Apt.executable_name(), "apt");
    /// assert_eq!(SystemPackageManager::Homebrew.executable_name(), "brew");
    /// assert_eq!(SystemPackageManager::Portage.executable_name(), "emerge");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            // Debian family
            SystemPackageManager::Apt => "apt",
            SystemPackageManager::Aptitude => "aptitude",
            SystemPackageManager::Dpkg => "dpkg",
            SystemPackageManager::Nala => "nala",
            SystemPackageManager::AptFast => "apt-fast",
            // RedHat family
            SystemPackageManager::Dnf => "dnf",
            SystemPackageManager::Yum => "yum",
            SystemPackageManager::Microdnf => "microdnf",
            SystemPackageManager::Rpm => "rpm",
            // Arch family
            SystemPackageManager::Pacman | SystemPackageManager::Msys2Pacman => "pacman",
            SystemPackageManager::Makepkg => "makepkg",
            SystemPackageManager::Yay => "yay",
            SystemPackageManager::Paru => "paru",
            SystemPackageManager::Pamac => "pamac",
            // SUSE
            SystemPackageManager::Zypper => "zypper",
            // Gentoo
            SystemPackageManager::Portage => "emerge",
            // Alpine
            SystemPackageManager::Apk => "apk",
            // Void
            SystemPackageManager::Xbps => "xbps-install",
            // Slackware
            SystemPackageManager::Pkgtool => "pkgtool",
            // Cross-distro
            SystemPackageManager::Snap => "snap",
            SystemPackageManager::Flatpak => "flatpak",
            SystemPackageManager::Guix => "guix",
            SystemPackageManager::Nix => "nix",
            SystemPackageManager::NixEnv => "nix-env",
            // macOS
            SystemPackageManager::Homebrew => "brew",
            SystemPackageManager::MacPorts => "port",
            SystemPackageManager::Fink => "fink",
            SystemPackageManager::Softwareupdate => "softwareupdate",
            // Windows
            SystemPackageManager::Winget => "winget",
            SystemPackageManager::Dism => "dism",
            SystemPackageManager::Chocolatey => "choco",
            SystemPackageManager::Scoop => "scoop",
            // BSD
            SystemPackageManager::Pkg => "pkg",
            SystemPackageManager::Ports => "make", // ports uses make
            SystemPackageManager::PkgAdd => "pkg_add",
            SystemPackageManager::Pkgin => "pkgin",
        }
    }
}

/// Available commands for a package manager.
///
/// Contains the command-line syntax for common package management operations.
/// Each field is `Option<String>` because not all package managers support
/// all operations.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::PackageManagerCommands;
///
/// let apt_commands = PackageManagerCommands {
///     list: Some("apt list --installed".to_string()),
///     update: Some("apt update".to_string()),
///     upgrade: Some("apt upgrade".to_string()),
///     search: Some("apt search".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageManagerCommands {
    /// Command to list installed packages
    pub list: Option<String>,
    /// Command to update package index/metadata
    pub update: Option<String>,
    /// Command to upgrade installed packages
    pub upgrade: Option<String>,
    /// Command to search for packages
    pub search: Option<String>,
}

/// A detected package manager with its location and capabilities.
///
/// Represents a package manager found on the system, including its
/// full path, whether it's the primary manager, and available commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPackageManager {
    /// The package manager type
    pub manager: SystemPackageManager,
    /// Full path to the executable
    pub path: String,
    /// Whether this is the primary package manager for the OS/distro
    pub is_primary: bool,
    /// Available commands for this manager
    pub commands: PackageManagerCommands,
}

/// Collection of detected system package managers.
///
/// Contains all package managers found on the system along with
/// identification of the primary (default) manager.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemPackageManagers {
    /// All detected package managers
    pub managers: Vec<DetectedPackageManager>,
    /// The primary/default package manager for this system
    pub primary: Option<SystemPackageManager>,
}

/// Parses the PATH environment variable into a list of directories.
///
/// Splits the PATH on the platform-specific separator (`:` on Unix, `;` on Windows)
/// and returns only directories that exist.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::get_path_dirs;
///
/// let dirs = get_path_dirs();
/// // dirs contains PathBuf entries for each valid directory in PATH
/// ```
///
/// ## Returns
///
/// A vector of `PathBuf` for each directory in PATH that exists.
/// Returns an empty vector if PATH is not set or contains no valid directories.
#[must_use]
pub fn get_path_dirs() -> Vec<PathBuf> {
    let path_var = match std::env::var("PATH") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    #[cfg(target_os = "windows")]
    let separator = ';';
    #[cfg(not(target_os = "windows"))]
    let separator = ':';

    path_var
        .split(separator)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect()
}

/// Checks if a command exists in the given PATH directories.
///
/// Searches through the provided directories for an executable with the
/// given name. On Unix, checks the executable permission bit. On Windows,
/// checks for common executable extensions (.exe, .cmd, .bat, .com).
///
/// **Important:** This function does NOT spawn any processes. It performs
/// a direct filesystem check which is more efficient than calling `which`
/// or `where` for each command.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::{get_path_dirs, command_exists_in_path};
///
/// let path_dirs = get_path_dirs();
/// if let Some(path) = command_exists_in_path("git", &path_dirs) {
///     println!("git found at: {}", path.display());
/// }
/// ```
///
/// ## Arguments
///
/// * `cmd` - The command name to search for (without extension on Windows)
/// * `path_dirs` - Directories to search (typically from `get_path_dirs()`)
///
/// ## Returns
///
/// `Some(PathBuf)` with the full path if found, `None` otherwise.
#[must_use]
pub fn command_exists_in_path(cmd: &str, path_dirs: &[PathBuf]) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        command_exists_in_path_windows(cmd, path_dirs)
    }
    #[cfg(not(target_os = "windows"))]
    {
        command_exists_in_path_unix(cmd, path_dirs)
    }
}

/// Unix implementation of command existence check.
///
/// Checks if the file exists and has the executable bit set.
#[cfg(not(target_os = "windows"))]
fn command_exists_in_path_unix(cmd: &str, path_dirs: &[PathBuf]) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    for dir in path_dirs {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            // Check executable permission
            if let Ok(metadata) = candidate.metadata() {
                let mode = metadata.permissions().mode();
                // Check if any execute bit is set (owner, group, or other)
                if mode & 0o111 != 0 {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Windows implementation of command existence check.
///
/// Checks for the command with common executable extensions.
#[cfg(target_os = "windows")]
fn command_exists_in_path_windows(cmd: &str, path_dirs: &[PathBuf]) -> Option<PathBuf> {
    // Windows executable extensions in priority order
    const EXTENSIONS: &[&str] = &[".exe", ".cmd", ".bat", ".com", ""];

    for dir in path_dirs {
        for ext in EXTENSIONS {
            let filename = if ext.is_empty() {
                cmd.to_string()
            } else {
                format!("{}{}", cmd, ext)
            };
            let candidate = dir.join(&filename);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Returns the command database entry for a package manager.
///
/// Provides the standard CLI syntax for list, update, upgrade, and search
/// operations for each supported package manager.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::{get_commands_for_manager, SystemPackageManager};
///
/// let apt_cmds = get_commands_for_manager(SystemPackageManager::Apt);
/// assert_eq!(apt_cmds.list, Some("apt list --installed".to_string()));
/// assert_eq!(apt_cmds.update, Some("apt update".to_string()));
/// ```
///
/// ## Returns
///
/// A `PackageManagerCommands` struct with the available commands.
/// Commands that don't exist for a manager will be `None`.
#[must_use]
pub fn get_commands_for_manager(manager: SystemPackageManager) -> PackageManagerCommands {
    match manager {
        // ===== Debian family =====
        SystemPackageManager::Apt => PackageManagerCommands {
            list: Some("apt list --installed".to_string()),
            update: Some("apt update".to_string()),
            upgrade: Some("apt upgrade".to_string()),
            search: Some("apt search".to_string()),
        },
        SystemPackageManager::Aptitude => PackageManagerCommands {
            list: Some("aptitude search '~i'".to_string()),
            update: Some("aptitude update".to_string()),
            upgrade: Some("aptitude upgrade".to_string()),
            search: Some("aptitude search".to_string()),
        },
        SystemPackageManager::Dpkg => PackageManagerCommands {
            list: Some("dpkg -l".to_string()),
            update: None, // dpkg is low-level, no update
            upgrade: None,
            search: Some("dpkg -S".to_string()),
        },
        SystemPackageManager::Nala => PackageManagerCommands {
            list: Some("nala list --installed".to_string()),
            update: Some("nala update".to_string()),
            upgrade: Some("nala upgrade".to_string()),
            search: Some("nala search".to_string()),
        },
        SystemPackageManager::AptFast => PackageManagerCommands {
            list: Some("apt-fast list --installed".to_string()),
            update: Some("apt-fast update".to_string()),
            upgrade: Some("apt-fast upgrade".to_string()),
            search: Some("apt-fast search".to_string()),
        },

        // ===== RedHat family =====
        SystemPackageManager::Dnf => PackageManagerCommands {
            list: Some("dnf list installed".to_string()),
            update: Some("dnf check-update".to_string()),
            upgrade: Some("dnf upgrade".to_string()),
            search: Some("dnf search".to_string()),
        },
        SystemPackageManager::Yum => PackageManagerCommands {
            list: Some("yum list installed".to_string()),
            update: Some("yum check-update".to_string()),
            upgrade: Some("yum update".to_string()),
            search: Some("yum search".to_string()),
        },
        SystemPackageManager::Microdnf => PackageManagerCommands {
            list: Some("microdnf list installed".to_string()),
            update: Some("microdnf check-update".to_string()),
            upgrade: Some("microdnf upgrade".to_string()),
            search: Some("microdnf search".to_string()),
        },
        SystemPackageManager::Rpm => PackageManagerCommands {
            list: Some("rpm -qa".to_string()),
            update: None, // rpm is low-level
            upgrade: None,
            search: Some("rpm -q".to_string()),
        },

        // ===== Arch family =====
        SystemPackageManager::Pacman | SystemPackageManager::Msys2Pacman => {
            PackageManagerCommands {
                list: Some("pacman -Q".to_string()),
                update: Some("pacman -Sy".to_string()),
                upgrade: Some("pacman -Syu".to_string()),
                search: Some("pacman -Ss".to_string()),
            }
        }
        SystemPackageManager::Makepkg => PackageManagerCommands {
            list: None, // makepkg is a build tool
            update: None,
            upgrade: None,
            search: None,
        },
        SystemPackageManager::Yay => PackageManagerCommands {
            list: Some("yay -Q".to_string()),
            update: Some("yay -Sy".to_string()),
            upgrade: Some("yay -Syu".to_string()),
            search: Some("yay -Ss".to_string()),
        },
        SystemPackageManager::Paru => PackageManagerCommands {
            list: Some("paru -Q".to_string()),
            update: Some("paru -Sy".to_string()),
            upgrade: Some("paru -Syu".to_string()),
            search: Some("paru -Ss".to_string()),
        },
        SystemPackageManager::Pamac => PackageManagerCommands {
            list: Some("pamac list --installed".to_string()),
            update: Some("pamac checkupdates".to_string()),
            upgrade: Some("pamac upgrade".to_string()),
            search: Some("pamac search".to_string()),
        },

        // ===== SUSE =====
        SystemPackageManager::Zypper => PackageManagerCommands {
            list: Some("zypper packages --installed-only".to_string()),
            update: Some("zypper refresh".to_string()),
            upgrade: Some("zypper update".to_string()),
            search: Some("zypper search".to_string()),
        },

        // ===== Gentoo =====
        SystemPackageManager::Portage => PackageManagerCommands {
            list: Some("qlist -I".to_string()),
            update: Some("emerge --sync".to_string()),
            upgrade: Some("emerge --update --deep @world".to_string()),
            search: Some("emerge --search".to_string()),
        },

        // ===== Alpine =====
        SystemPackageManager::Apk => PackageManagerCommands {
            list: Some("apk list --installed".to_string()),
            update: Some("apk update".to_string()),
            upgrade: Some("apk upgrade".to_string()),
            search: Some("apk search".to_string()),
        },

        // ===== Void =====
        SystemPackageManager::Xbps => PackageManagerCommands {
            list: Some("xbps-query -l".to_string()),
            update: Some("xbps-install -S".to_string()),
            upgrade: Some("xbps-install -Su".to_string()),
            search: Some("xbps-query -Rs".to_string()),
        },

        // ===== Slackware =====
        SystemPackageManager::Pkgtool => PackageManagerCommands {
            list: Some("ls /var/log/packages".to_string()),
            update: None, // slackpkg handles updates
            upgrade: None,
            search: None,
        },

        // ===== Cross-distro =====
        SystemPackageManager::Snap => PackageManagerCommands {
            list: Some("snap list".to_string()),
            update: Some("snap refresh --list".to_string()),
            upgrade: Some("snap refresh".to_string()),
            search: Some("snap find".to_string()),
        },
        SystemPackageManager::Flatpak => PackageManagerCommands {
            list: Some("flatpak list".to_string()),
            update: None, // flatpak combines update/upgrade
            upgrade: Some("flatpak update".to_string()),
            search: Some("flatpak search".to_string()),
        },
        SystemPackageManager::Guix => PackageManagerCommands {
            list: Some("guix package --list-installed".to_string()),
            update: Some("guix pull".to_string()),
            upgrade: Some("guix upgrade".to_string()),
            search: Some("guix search".to_string()),
        },
        SystemPackageManager::Nix => PackageManagerCommands {
            list: Some("nix profile list".to_string()),
            update: None, // nix flake update for flakes
            upgrade: Some("nix profile upgrade '.*'".to_string()),
            search: Some("nix search nixpkgs".to_string()),
        },
        SystemPackageManager::NixEnv => PackageManagerCommands {
            list: Some("nix-env -q".to_string()),
            update: Some("nix-channel --update".to_string()),
            upgrade: Some("nix-env -u".to_string()),
            search: Some("nix-env -qaP".to_string()),
        },

        // ===== macOS =====
        SystemPackageManager::Homebrew => PackageManagerCommands {
            list: Some("brew list".to_string()),
            update: Some("brew update".to_string()),
            upgrade: Some("brew upgrade".to_string()),
            search: Some("brew search".to_string()),
        },
        SystemPackageManager::MacPorts => PackageManagerCommands {
            list: Some("port installed".to_string()),
            update: Some("port sync".to_string()),
            upgrade: Some("port upgrade outdated".to_string()),
            search: Some("port search".to_string()),
        },
        SystemPackageManager::Fink => PackageManagerCommands {
            list: Some("fink list --installed".to_string()),
            update: Some("fink selfupdate".to_string()),
            upgrade: Some("fink update-all".to_string()),
            search: Some("fink list".to_string()),
        },
        SystemPackageManager::Softwareupdate => PackageManagerCommands {
            list: Some("softwareupdate --list".to_string()),
            update: None, // list is also check for updates
            upgrade: Some("softwareupdate --install --all".to_string()),
            search: None, // softwareupdate doesn't search
        },

        // ===== Windows =====
        SystemPackageManager::Winget => PackageManagerCommands {
            list: Some("winget list".to_string()),
            update: None, // winget list shows updates
            upgrade: Some("winget upgrade --all".to_string()),
            search: Some("winget search".to_string()),
        },
        SystemPackageManager::Dism => PackageManagerCommands {
            list: Some("dism /online /get-packages".to_string()),
            update: None, // DISM is for servicing
            upgrade: None,
            search: None,
        },
        SystemPackageManager::Chocolatey => PackageManagerCommands {
            list: Some("choco list".to_string()),
            update: None, // choco outdated shows updates
            upgrade: Some("choco upgrade all".to_string()),
            search: Some("choco search".to_string()),
        },
        SystemPackageManager::Scoop => PackageManagerCommands {
            list: Some("scoop list".to_string()),
            update: Some("scoop update".to_string()),
            upgrade: Some("scoop update *".to_string()),
            search: Some("scoop search".to_string()),
        },

        // ===== BSD =====
        SystemPackageManager::Pkg => PackageManagerCommands {
            list: Some("pkg info".to_string()),
            update: Some("pkg update".to_string()),
            upgrade: Some("pkg upgrade".to_string()),
            search: Some("pkg search".to_string()),
        },
        SystemPackageManager::Ports => PackageManagerCommands {
            list: Some("pkg info".to_string()), // ports uses pkg for installed
            update: Some("portsnap fetch update".to_string()),
            upgrade: Some("portmaster -a".to_string()),
            search: Some("make search name=".to_string()),
        },
        SystemPackageManager::PkgAdd => PackageManagerCommands {
            list: Some("pkg_info".to_string()),
            update: None, // OpenBSD doesn't have update
            upgrade: Some("pkg_add -u".to_string()),
            search: Some("pkg_info -Q".to_string()),
        },
        SystemPackageManager::Pkgin => PackageManagerCommands {
            list: Some("pkgin list".to_string()),
            update: Some("pkgin update".to_string()),
            upgrade: Some("pkgin upgrade".to_string()),
            search: Some("pkgin search".to_string()),
        },
    }
}

// ============================================================================
// Linux Package Manager Detection
// ============================================================================

/// Linux package manager definitions for detection.
///
/// Each entry contains the package manager variant and its executable name.
const LINUX_PACKAGE_MANAGERS: &[(SystemPackageManager, &str)] = &[
    // Debian family
    (SystemPackageManager::Apt, "apt"),
    (SystemPackageManager::Aptitude, "aptitude"),
    (SystemPackageManager::Dpkg, "dpkg"),
    (SystemPackageManager::Nala, "nala"),
    (SystemPackageManager::AptFast, "apt-fast"),
    // RedHat family
    (SystemPackageManager::Dnf, "dnf"),
    (SystemPackageManager::Yum, "yum"),
    (SystemPackageManager::Microdnf, "microdnf"),
    (SystemPackageManager::Rpm, "rpm"),
    // Arch family
    (SystemPackageManager::Pacman, "pacman"),
    (SystemPackageManager::Makepkg, "makepkg"),
    (SystemPackageManager::Yay, "yay"),
    (SystemPackageManager::Paru, "paru"),
    (SystemPackageManager::Pamac, "pamac"),
    // SUSE
    (SystemPackageManager::Zypper, "zypper"),
    // Gentoo
    (SystemPackageManager::Portage, "emerge"),
    // Alpine
    (SystemPackageManager::Apk, "apk"),
    // Void
    (SystemPackageManager::Xbps, "xbps-install"),
    // Slackware
    (SystemPackageManager::Pkgtool, "pkgtool"),
    // Cross-distro
    (SystemPackageManager::Snap, "snap"),
    (SystemPackageManager::Flatpak, "flatpak"),
    (SystemPackageManager::Guix, "guix"),
    (SystemPackageManager::Nix, "nix"),
    (SystemPackageManager::NixEnv, "nix-env"),
];

/// AUR helpers that should never be marked as primary package managers.
///
/// These are helper tools for the Arch User Repository, not the core
/// package manager for the distribution.
const AUR_HELPERS: &[SystemPackageManager] = &[
    SystemPackageManager::Yay,
    SystemPackageManager::Paru,
    SystemPackageManager::Pamac,
];

/// Detects Linux package managers available on the system.
///
/// Scans the PATH for known Linux package manager executables and returns
/// information about each one found, including which is the primary manager
/// for the distribution.
///
/// ## Arguments
///
/// * `linux_family` - Optional Linux distribution family hint. When provided,
///   helps determine which package manager should be marked as primary.
///   If `None`, the primary is inferred from which managers are found.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::{detect_linux_package_managers, LinuxFamily};
///
/// // With family hint (recommended on Linux)
/// let managers = detect_linux_package_managers(Some(LinuxFamily::Debian));
/// if let Some(primary) = managers.primary {
///     println!("Primary package manager: {}", primary);
/// }
/// for mgr in &managers.managers {
///     println!("Found: {} at {}", mgr.manager, mgr.path);
/// }
///
/// // Without family hint
/// let managers = detect_linux_package_managers(None);
/// ```
///
/// ## Primary Package Manager Selection
///
/// The primary package manager is selected based on the Linux family:
///
/// | Family     | Primary (if found)      |
/// |------------|-------------------------|
/// | Debian     | apt, else dpkg          |
/// | RedHat     | dnf, else yum           |
/// | Arch       | pacman                  |
/// | SUSE       | zypper                  |
/// | Alpine     | apk                     |
/// | Void       | xbps                    |
/// | Slackware  | pkgtool                 |
/// | NixOS      | nix                     |
/// | Gentoo     | portage (emerge)        |
/// | Other      | First distro-native found |
///
/// ## Notes
///
/// - AUR helpers (yay, paru, pamac) are never marked as primary
/// - Cross-distro managers (snap, flatpak, guix, nix) are only primary on NixOS
/// - This function does not spawn any processes; it only checks file existence
#[must_use]
pub fn detect_linux_package_managers(linux_family: Option<LinuxFamily>) -> SystemPackageManagers {
    let path_dirs = get_path_dirs();
    let mut detected: Vec<DetectedPackageManager> = Vec::new();

    // Scan for all known package managers
    for &(manager, executable) in LINUX_PACKAGE_MANAGERS {
        if let Some(path) = command_exists_in_path(executable, &path_dirs) {
            detected.push(DetectedPackageManager {
                manager,
                path: path.to_string_lossy().to_string(),
                is_primary: false, // Will be set later
                commands: get_commands_for_manager(manager),
            });
        }
    }

    // Determine the primary package manager
    let primary = determine_linux_primary(&detected, linux_family);

    // Mark the primary manager
    if let Some(primary_mgr) = primary {
        for entry in &mut detected {
            if entry.manager == primary_mgr {
                entry.is_primary = true;
                break;
            }
        }
    }

    SystemPackageManagers {
        managers: detected,
        primary,
    }
}

/// Determines the primary package manager based on Linux family and detected managers.
fn determine_linux_primary(
    detected: &[DetectedPackageManager],
    linux_family: Option<LinuxFamily>,
) -> Option<SystemPackageManager> {
    // Helper to check if a manager was detected
    let has = |mgr: SystemPackageManager| detected.iter().any(|d| d.manager == mgr);

    // Try to select primary based on family
    if let Some(family) = linux_family {
        let candidate = match family {
            LinuxFamily::Debian => {
                if has(SystemPackageManager::Apt) {
                    Some(SystemPackageManager::Apt)
                } else if has(SystemPackageManager::Dpkg) {
                    Some(SystemPackageManager::Dpkg)
                } else {
                    None
                }
            }
            LinuxFamily::RedHat => {
                if has(SystemPackageManager::Dnf) {
                    Some(SystemPackageManager::Dnf)
                } else if has(SystemPackageManager::Yum) {
                    Some(SystemPackageManager::Yum)
                } else {
                    None
                }
            }
            LinuxFamily::Arch => {
                if has(SystemPackageManager::Pacman) {
                    Some(SystemPackageManager::Pacman)
                } else {
                    None
                }
            }
            LinuxFamily::SUSE => {
                if has(SystemPackageManager::Zypper) {
                    Some(SystemPackageManager::Zypper)
                } else {
                    None
                }
            }
            LinuxFamily::Alpine => {
                if has(SystemPackageManager::Apk) {
                    Some(SystemPackageManager::Apk)
                } else {
                    None
                }
            }
            LinuxFamily::Void => {
                if has(SystemPackageManager::Xbps) {
                    Some(SystemPackageManager::Xbps)
                } else {
                    None
                }
            }
            LinuxFamily::Slackware => {
                if has(SystemPackageManager::Pkgtool) {
                    Some(SystemPackageManager::Pkgtool)
                } else {
                    None
                }
            }
            LinuxFamily::NixOS => {
                if has(SystemPackageManager::Nix) {
                    Some(SystemPackageManager::Nix)
                } else if has(SystemPackageManager::NixEnv) {
                    Some(SystemPackageManager::NixEnv)
                } else {
                    None
                }
            }
            LinuxFamily::Gentoo => {
                if has(SystemPackageManager::Portage) {
                    Some(SystemPackageManager::Portage)
                } else {
                    None
                }
            }
            LinuxFamily::Other => None,
        };

        if candidate.is_some() {
            return candidate;
        }
    }

    // Fallback: find the first non-AUR-helper, non-cross-distro manager
    // Priority order based on common distros
    const PRIORITY_ORDER: &[SystemPackageManager] = &[
        // Debian family
        SystemPackageManager::Apt,
        SystemPackageManager::Dpkg,
        // RedHat family
        SystemPackageManager::Dnf,
        SystemPackageManager::Yum,
        // Arch
        SystemPackageManager::Pacman,
        // SUSE
        SystemPackageManager::Zypper,
        // Alpine
        SystemPackageManager::Apk,
        // Void
        SystemPackageManager::Xbps,
        // Gentoo
        SystemPackageManager::Portage,
        // Slackware
        SystemPackageManager::Pkgtool,
        // Nix (last among distro-native)
        SystemPackageManager::Nix,
        SystemPackageManager::NixEnv,
    ];

    PRIORITY_ORDER
        .iter()
        .copied()
        .find(|&mgr| has(mgr) && !AUR_HELPERS.contains(&mgr))
}

// ============================================================================
// macOS Package Manager Detection
// ============================================================================

/// Known locations for Homebrew on macOS.
///
/// Homebrew installs to different locations depending on the CPU architecture:
/// - Apple Silicon (arm64): `/opt/homebrew/bin/brew`
/// - Intel (x86_64): `/usr/local/bin/brew`
const HOMEBREW_APPLE_SILICON_PATH: &str = "/opt/homebrew/bin/brew";
const HOMEBREW_INTEL_PATH: &str = "/usr/local/bin/brew";

/// System location for softwareupdate on macOS.
const SOFTWAREUPDATE_PATH: &str = "/usr/sbin/softwareupdate";

/// Detects package managers available on macOS.
///
/// Searches for the following package managers:
/// - **Homebrew** (`brew`): Checked at both Apple Silicon (`/opt/homebrew/bin/brew`)
///   and Intel (`/usr/local/bin/brew`) locations
/// - **MacPorts** (`port`): Searched in PATH directories
/// - **Fink** (`fink`): Searched in PATH directories
/// - **softwareupdate**: Always included as it's a built-in macOS system utility
///
/// ## Primary Selection
///
/// The primary package manager is selected in this order:
/// 1. Homebrew (if found) - most commonly used third-party package manager
/// 2. softwareupdate - always available as fallback
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::detect_macos_package_managers;
///
/// let managers = detect_macos_package_managers();
/// println!("Primary: {:?}", managers.primary);
/// for m in &managers.managers {
///     println!("  {} at {}", m.manager, m.path);
/// }
/// ```
///
/// ## Notes
///
/// This function should only be called on macOS systems. It does not use
/// `#[cfg(target_os)]` attributes to allow the caller to control platform
/// selection logic.
#[must_use]
pub fn detect_macos_package_managers() -> SystemPackageManagers {
    let path_dirs = get_path_dirs();
    let mut managers = Vec::new();
    let mut primary: Option<SystemPackageManager> = None;

    // Check for Homebrew at known locations
    // Apple Silicon location takes precedence
    let homebrew_path = if Path::new(HOMEBREW_APPLE_SILICON_PATH).is_file() {
        Some(PathBuf::from(HOMEBREW_APPLE_SILICON_PATH))
    } else if Path::new(HOMEBREW_INTEL_PATH).is_file() {
        Some(PathBuf::from(HOMEBREW_INTEL_PATH))
    } else {
        None
    };

    if let Some(path) = homebrew_path {
        managers.push(DetectedPackageManager {
            manager: SystemPackageManager::Homebrew,
            path: path.to_string_lossy().to_string(),
            is_primary: true, // Will be updated if not primary
            commands: get_commands_for_manager(SystemPackageManager::Homebrew),
        });
        primary = Some(SystemPackageManager::Homebrew);
    }

    // Check for MacPorts
    if let Some(path) = command_exists_in_path("port", &path_dirs) {
        managers.push(DetectedPackageManager {
            manager: SystemPackageManager::MacPorts,
            path: path.to_string_lossy().to_string(),
            is_primary: false,
            commands: get_commands_for_manager(SystemPackageManager::MacPorts),
        });
    }

    // Check for Fink
    if let Some(path) = command_exists_in_path("fink", &path_dirs) {
        managers.push(DetectedPackageManager {
            manager: SystemPackageManager::Fink,
            path: path.to_string_lossy().to_string(),
            is_primary: false,
            commands: get_commands_for_manager(SystemPackageManager::Fink),
        });
    }

    // softwareupdate is always available on macOS
    let softwareupdate_is_primary = primary.is_none();
    managers.push(DetectedPackageManager {
        manager: SystemPackageManager::Softwareupdate,
        path: SOFTWAREUPDATE_PATH.to_string(),
        is_primary: softwareupdate_is_primary,
        commands: get_commands_for_manager(SystemPackageManager::Softwareupdate),
    });

    if primary.is_none() {
        primary = Some(SystemPackageManager::Softwareupdate);
    }

    // Update is_primary flags based on final primary selection
    if let Some(primary_manager) = primary {
        for m in &mut managers {
            m.is_primary = m.manager == primary_manager;
        }
    }

    SystemPackageManagers { managers, primary }
}

// ============================================================================
// Windows Package Manager Detection
// ============================================================================

/// Detects package managers available on Windows.
///
/// Scans the system for Windows-specific package managers including winget,
/// DISM, Chocolatey, Scoop, and MSYS2's pacman.
///
/// ## Package Managers Detected
///
/// - **winget** - Windows Package Manager (primary on Windows 10+)
/// - **dism** - Deployment Image Servicing and Management (system utility)
/// - **chocolatey** (choco) - Community package manager
/// - **scoop** - Command-line installer focused on developer tools
/// - **msys2 pacman** - Unix-like environment package manager
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::detect_windows_package_managers;
///
/// let managers = detect_windows_package_managers();
/// if let Some(primary) = managers.primary {
///     println!("Primary package manager: {}", primary);
/// }
/// for manager in &managers.managers {
///     println!("  {} at {}", manager.manager, manager.path);
/// }
/// ```
///
/// ## Notes
///
/// - This function should only be called on Windows systems
/// - The caller is responsible for platform checking via `#[cfg(target_os = "windows")]`
/// - DISM is always detected if present at the standard Windows system path
/// - MSYS2 pacman is distinguished from native pacman by checking the known
///   MSYS2 installation path (`C:\msys64\usr\bin\pacman.exe`)
#[must_use]
pub fn detect_windows_package_managers() -> SystemPackageManagers {
    let path_dirs = get_path_dirs();
    let mut managers: Vec<DetectedPackageManager> = Vec::new();
    let mut primary: Option<SystemPackageManager> = None;

    // Check for winget (primary on modern Windows)
    if let Some(path) = command_exists_in_path("winget", &path_dirs) {
        let manager = SystemPackageManager::Winget;
        managers.push(DetectedPackageManager {
            manager,
            path: path.to_string_lossy().to_string(),
            is_primary: true,
            commands: get_commands_for_manager(manager),
        });
        primary = Some(manager);
    }

    // Check for DISM (system utility, always at %windir%\system32)
    // DISM is a built-in Windows tool for servicing images
    let windir = std::env::var("windir")
        .or_else(|_| std::env::var("WINDIR"))
        .unwrap_or_else(|_| r"C:\Windows".to_string());
    let dism_path = PathBuf::from(&windir).join("system32").join("dism.exe");
    if dism_path.is_file() {
        let manager = SystemPackageManager::Dism;
        managers.push(DetectedPackageManager {
            manager,
            path: dism_path.to_string_lossy().to_string(),
            is_primary: false,
            commands: get_commands_for_manager(manager),
        });
    }

    // Check for Chocolatey
    if let Some(path) = command_exists_in_path("choco", &path_dirs) {
        let manager = SystemPackageManager::Chocolatey;
        let is_primary = primary.is_none();
        managers.push(DetectedPackageManager {
            manager,
            path: path.to_string_lossy().to_string(),
            is_primary,
            commands: get_commands_for_manager(manager),
        });
        if is_primary {
            primary = Some(manager);
        }
    }

    // Check for Scoop
    if let Some(path) = command_exists_in_path("scoop", &path_dirs) {
        let manager = SystemPackageManager::Scoop;
        let is_primary = primary.is_none();
        managers.push(DetectedPackageManager {
            manager,
            path: path.to_string_lossy().to_string(),
            is_primary,
            commands: get_commands_for_manager(manager),
        });
        if is_primary {
            primary = Some(manager);
        }
    }

    // Check for MSYS2 pacman at known location
    // MSYS2 typically installs to C:\msys64 and provides a Unix-like environment
    let msys2_pacman_path = PathBuf::from(r"C:\msys64\usr\bin\pacman.exe");
    if msys2_pacman_path.is_file() {
        let manager = SystemPackageManager::Msys2Pacman;
        managers.push(DetectedPackageManager {
            manager,
            path: msys2_pacman_path.to_string_lossy().to_string(),
            is_primary: false,
            commands: get_commands_for_manager(manager),
        });
    }

    // Update is_primary flags based on final primary selection
    if let Some(primary_manager) = primary {
        for m in &mut managers {
            m.is_primary = m.manager == primary_manager;
        }
    }

    SystemPackageManagers { managers, primary }
}

// ============================================================================
// BSD Package Manager Detection
// ============================================================================

/// Detects package managers available on BSD operating systems.
///
/// Detects package managers specific to each BSD variant:
///
/// ## FreeBSD
///
/// - **pkg** - Primary binary package manager
/// - **ports** - Ports collection (source-based)
///
/// ## OpenBSD
///
/// - **pkg_add** - Primary package manager
///
/// ## NetBSD
///
/// - **pkgin** - Primary binary package manager
/// - **pkg_add** - Secondary package tools
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::os::{detect_bsd_package_managers, OsType};
///
/// let managers = detect_bsd_package_managers(OsType::FreeBSD);
/// assert_eq!(managers.primary, Some(sniff_lib::os::SystemPackageManager::Pkg));
/// ```
///
/// ## Arguments
///
/// * `os_type` - The BSD variant to detect package managers for
///
/// ## Returns
///
/// A `SystemPackageManagers` struct containing detected managers.
/// Returns an empty result if `os_type` is not a BSD variant.
///
/// ## Notes
///
/// - This function should only be called on BSD systems
/// - The caller is responsible for platform checking
/// - For non-BSD `OsType` values, returns an empty `SystemPackageManagers`
#[must_use]
pub fn detect_bsd_package_managers(os_type: OsType) -> SystemPackageManagers {
    let path_dirs = get_path_dirs();
    let mut managers: Vec<DetectedPackageManager> = Vec::new();
    let mut primary: Option<SystemPackageManager> = None;

    match os_type {
        OsType::FreeBSD => {
            // FreeBSD: pkg is primary, ports is secondary
            if let Some(path) = command_exists_in_path("pkg", &path_dirs) {
                let manager = SystemPackageManager::Pkg;
                managers.push(DetectedPackageManager {
                    manager,
                    path: path.to_string_lossy().to_string(),
                    is_primary: true,
                    commands: get_commands_for_manager(manager),
                });
                primary = Some(manager);
            }

            // Check for ports collection (uses make in /usr/ports)
            let ports_dir = Path::new("/usr/ports");
            if ports_dir.is_dir() {
                // Ports uses make, verify make exists
                if let Some(make_path) = command_exists_in_path("make", &path_dirs) {
                    let manager = SystemPackageManager::Ports;
                    managers.push(DetectedPackageManager {
                        manager,
                        path: make_path.to_string_lossy().to_string(),
                        is_primary: false,
                        commands: get_commands_for_manager(manager),
                    });
                }
            }
        }

        OsType::OpenBSD => {
            // OpenBSD: pkg_add is the primary and only package manager
            if let Some(path) = command_exists_in_path("pkg_add", &path_dirs) {
                let manager = SystemPackageManager::PkgAdd;
                managers.push(DetectedPackageManager {
                    manager,
                    path: path.to_string_lossy().to_string(),
                    is_primary: true,
                    commands: get_commands_for_manager(manager),
                });
                primary = Some(manager);
            }
        }

        OsType::NetBSD => {
            // NetBSD: pkgin is primary, pkg_add is secondary
            if let Some(path) = command_exists_in_path("pkgin", &path_dirs) {
                let manager = SystemPackageManager::Pkgin;
                managers.push(DetectedPackageManager {
                    manager,
                    path: path.to_string_lossy().to_string(),
                    is_primary: true,
                    commands: get_commands_for_manager(manager),
                });
                primary = Some(manager);
            }

            // Also check for pkg_add as secondary
            if let Some(path) = command_exists_in_path("pkg_add", &path_dirs) {
                let manager = SystemPackageManager::PkgAdd;
                let is_primary = primary.is_none();
                managers.push(DetectedPackageManager {
                    manager,
                    path: path.to_string_lossy().to_string(),
                    is_primary,
                    commands: get_commands_for_manager(manager),
                });
                if is_primary {
                    primary = Some(manager);
                }
            }
        }

        // Non-BSD variants return empty result
        _ => {}
    }

    SystemPackageManagers { managers, primary }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // OsType tests
    // ========================================

    #[test]
    fn test_os_type_default() {
        let default: OsType = Default::default();
        assert_eq!(default, OsType::Other);
    }

    #[test]
    fn test_os_type_display() {
        assert_eq!(OsType::Windows.to_string(), "Windows");
        assert_eq!(OsType::Linux.to_string(), "Linux");
        assert_eq!(OsType::MacOS.to_string(), "macOS");
        assert_eq!(OsType::IOS.to_string(), "iOS");
        assert_eq!(OsType::Android.to_string(), "Android");
        assert_eq!(OsType::FreeBSD.to_string(), "FreeBSD");
        assert_eq!(OsType::OpenBSD.to_string(), "OpenBSD");
        assert_eq!(OsType::NetBSD.to_string(), "NetBSD");
        assert_eq!(OsType::Other.to_string(), "Other");
    }

    #[test]
    fn test_os_type_serialization() {
        let os_type = OsType::Linux;
        let json = serde_json::to_string(&os_type).expect("serialization should succeed");
        let deserialized: OsType =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized, OsType::Linux);
    }

    #[test]
    fn test_detect_os_type_returns_valid_type() {
        let os_type = detect_os_type();
        // Should return a known type on any supported platform
        #[cfg(target_os = "macos")]
        assert_eq!(os_type, OsType::MacOS);

        #[cfg(target_os = "linux")]
        assert_eq!(os_type, OsType::Linux);

        #[cfg(target_os = "windows")]
        assert_eq!(os_type, OsType::Windows);

        // On any platform, should not panic
        let _ = os_type.to_string();
    }

    // ========================================
    // LinuxFamily tests
    // ========================================

    #[test]
    fn test_linux_family_default() {
        let default: LinuxFamily = Default::default();
        assert_eq!(default, LinuxFamily::Other);
    }

    #[test]
    fn test_linux_family_display() {
        assert_eq!(LinuxFamily::Debian.to_string(), "Debian");
        assert_eq!(LinuxFamily::RedHat.to_string(), "Red Hat");
        assert_eq!(LinuxFamily::Arch.to_string(), "Arch");
        assert_eq!(LinuxFamily::SUSE.to_string(), "SUSE");
        assert_eq!(LinuxFamily::Gentoo.to_string(), "Gentoo");
        assert_eq!(LinuxFamily::Alpine.to_string(), "Alpine");
        assert_eq!(LinuxFamily::Void.to_string(), "Void");
        assert_eq!(LinuxFamily::Slackware.to_string(), "Slackware");
        assert_eq!(LinuxFamily::NixOS.to_string(), "NixOS");
        assert_eq!(LinuxFamily::Other.to_string(), "Other");
    }

    #[test]
    fn test_linux_family_serialization() {
        let family = LinuxFamily::Debian;
        let json = serde_json::to_string(&family).expect("serialization should succeed");
        let deserialized: LinuxFamily =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized, LinuxFamily::Debian);
    }

    // ========================================
    // infer_linux_family tests
    // ========================================

    #[test]
    fn test_infer_linux_family_debian() {
        // Core Debian family
        assert_eq!(infer_linux_family("debian"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("mint"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("linuxmint"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("pop"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("pop_os"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("elementary"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("zorin"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("kali"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("raspbian"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("mx"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("deepin"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("steamos"), LinuxFamily::Debian);
    }

    #[test]
    fn test_infer_linux_family_redhat() {
        assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rhel"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("centos"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rocky"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rockylinux"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("alma"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("almalinux"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("oracle"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("amazon"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("amzn"), LinuxFamily::RedHat);
    }

    #[test]
    fn test_infer_linux_family_arch() {
        assert_eq!(infer_linux_family("arch"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("archlinux"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("manjaro"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("endeavouros"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("garuda"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("artix"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("arcolinux"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("cachyos"), LinuxFamily::Arch);
    }

    #[test]
    fn test_infer_linux_family_suse() {
        assert_eq!(infer_linux_family("opensuse"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("suse"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("sles"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-leap"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-tumbleweed"), LinuxFamily::SUSE);
    }

    #[test]
    fn test_infer_linux_family_other_families() {
        assert_eq!(infer_linux_family("alpine"), LinuxFamily::Alpine);
        assert_eq!(infer_linux_family("void"), LinuxFamily::Void);
        assert_eq!(infer_linux_family("slackware"), LinuxFamily::Slackware);
        assert_eq!(infer_linux_family("nixos"), LinuxFamily::NixOS);
        assert_eq!(infer_linux_family("gentoo"), LinuxFamily::Gentoo);
        assert_eq!(infer_linux_family("funtoo"), LinuxFamily::Gentoo);
        assert_eq!(infer_linux_family("calculate"), LinuxFamily::Gentoo);
    }

    #[test]
    fn test_infer_linux_family_unknown() {
        assert_eq!(infer_linux_family("unknown"), LinuxFamily::Other);
        assert_eq!(infer_linux_family("custom"), LinuxFamily::Other);
        assert_eq!(infer_linux_family(""), LinuxFamily::Other);
    }

    #[test]
    fn test_infer_linux_family_case_insensitive() {
        assert_eq!(infer_linux_family("Ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("FEDORA"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("Arch"), LinuxFamily::Arch);
    }

    // ========================================
    // parse_os_release_content tests
    // ========================================

    #[test]
    fn test_parse_os_release_content_ubuntu() {
        let content = r#"
NAME="Ubuntu"
VERSION="22.04.3 LTS (Jammy Jellyfish)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 22.04.3 LTS"
VERSION_ID="22.04"
HOME_URL="https://www.ubuntu.com/"
VERSION_CODENAME=jammy
"#;

        let distro = parse_os_release_content(content).expect("should parse ubuntu");
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu");
        assert_eq!(distro.version, Some("22.04".to_string()));
        assert_eq!(distro.codename, Some("jammy".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_os_release_content_fedora() {
        let content = r#"
NAME="Fedora Linux"
VERSION="39 (Workstation Edition)"
ID=fedora
VERSION_ID=39
VERSION_CODENAME=""
PLATFORM_ID="platform:f39"
PRETTY_NAME="Fedora Linux 39 (Workstation Edition)"
"#;

        let distro = parse_os_release_content(content).expect("should parse fedora");
        assert_eq!(distro.id, "fedora");
        assert_eq!(distro.name, "Fedora Linux");
        assert_eq!(distro.version, Some("39".to_string()));
        assert_eq!(distro.codename, None); // Empty codename should be None
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_os_release_content_arch() {
        let content = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
"#;

        let distro = parse_os_release_content(content).expect("should parse arch");
        assert_eq!(distro.id, "arch");
        assert_eq!(distro.name, "Arch Linux");
        assert_eq!(distro.version, None); // Arch is rolling release
        assert_eq!(distro.codename, None);
        assert_eq!(distro.family, LinuxFamily::Arch);
    }

    #[test]
    fn test_parse_os_release_content_alpine() {
        let content = r#"
NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.19.0
PRETTY_NAME="Alpine Linux v3.19"
"#;

        let distro = parse_os_release_content(content).expect("should parse alpine");
        assert_eq!(distro.id, "alpine");
        assert_eq!(distro.name, "Alpine Linux");
        assert_eq!(distro.version, Some("3.19.0".to_string()));
        assert_eq!(distro.family, LinuxFamily::Alpine);
    }

    #[test]
    fn test_parse_os_release_content_empty() {
        let content = "";
        assert!(parse_os_release_content(content).is_none());
    }

    #[test]
    fn test_parse_os_release_content_comments_only() {
        let content = r#"
# This is a comment
# Another comment
"#;
        assert!(parse_os_release_content(content).is_none());
    }

    #[test]
    fn test_parse_os_release_content_single_quotes() {
        let content = "ID='ubuntu'\nNAME='Ubuntu'";
        let distro = parse_os_release_content(content).expect("should parse single quotes");
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu");
    }

    // ========================================
    // parse_lsb_release_content tests
    // ========================================

    #[test]
    fn test_parse_lsb_release_content_ubuntu() {
        let content = r#"
DISTRIB_ID=Ubuntu
DISTRIB_RELEASE=22.04
DISTRIB_CODENAME=jammy
DISTRIB_DESCRIPTION="Ubuntu 22.04.3 LTS"
"#;

        let distro = parse_lsb_release_content(content).expect("should parse lsb ubuntu");
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 22.04.3 LTS");
        assert_eq!(distro.version, Some("22.04".to_string()));
        assert_eq!(distro.codename, Some("jammy".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_lsb_release_content_minimal() {
        let content = "DISTRIB_ID=Debian";
        let distro = parse_lsb_release_content(content).expect("should parse minimal lsb");
        assert_eq!(distro.id, "debian");
        assert_eq!(distro.name, "Debian");
        assert_eq!(distro.version, None);
        assert_eq!(distro.codename, None);
    }

    #[test]
    fn test_parse_lsb_release_content_empty() {
        assert!(parse_lsb_release_content("").is_none());
    }

    // ========================================
    // parse_system_release_content tests
    // ========================================

    #[test]
    fn test_parse_system_release_content_fedora() {
        let content = "Fedora release 39 (Thirty Nine)";

        let distro = parse_system_release_content(content).expect("should parse fedora release");
        assert_eq!(distro.id, "fedora");
        assert_eq!(distro.name, "Fedora release 39 (Thirty Nine)");
        assert_eq!(distro.version, Some("39".to_string()));
        assert_eq!(distro.codename, Some("Thirty Nine".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_system_release_content_centos() {
        let content = "CentOS Linux release 7.9.2009 (Core)";

        let distro = parse_system_release_content(content).expect("should parse centos release");
        assert_eq!(distro.id, "centos");
        assert_eq!(distro.name, "CentOS Linux release 7.9.2009 (Core)");
        assert_eq!(distro.version, Some("7.9.2009".to_string()));
        assert_eq!(distro.codename, Some("Core".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_system_release_content_rocky() {
        let content = "Rocky Linux release 9.3 (Blue Onyx)";

        let distro = parse_system_release_content(content).expect("should parse rocky release");
        assert_eq!(distro.id, "rocky");
        assert_eq!(distro.version, Some("9.3".to_string()));
        assert_eq!(distro.codename, Some("Blue Onyx".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_system_release_content_empty() {
        assert!(parse_system_release_content("").is_none());
        assert!(parse_system_release_content("   ").is_none());
    }

    // ========================================
    // LinuxDistro tests
    // ========================================

    #[test]
    fn test_linux_distro_default() {
        let default: LinuxDistro = Default::default();
        assert!(default.id.is_empty());
        assert!(default.name.is_empty());
        assert!(default.version.is_none());
        assert!(default.codename.is_none());
        assert_eq!(default.family, LinuxFamily::Other);
    }

    #[test]
    fn test_linux_distro_serialization() {
        let distro = LinuxDistro {
            id: "ubuntu".to_string(),
            name: "Ubuntu".to_string(),
            version: Some("22.04".to_string()),
            codename: Some("jammy".to_string()),
            family: LinuxFamily::Debian,
        };

        let json = serde_json::to_string(&distro).expect("serialization should succeed");
        let deserialized: LinuxDistro =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.id, "ubuntu");
        assert_eq!(deserialized.name, "Ubuntu");
        assert_eq!(deserialized.version, Some("22.04".to_string()));
        assert_eq!(deserialized.codename, Some("jammy".to_string()));
        assert_eq!(deserialized.family, LinuxFamily::Debian);
    }

    // ========================================
    // OsInfo tests
    // ========================================

    #[test]
    fn test_os_info_default() {
        let default: OsInfo = Default::default();
        assert_eq!(default.os_type, OsType::Other);
        assert!(default.name.is_empty());
        assert!(default.version.is_empty());
        assert!(default.linux_distro.is_none());
    }

    #[test]
    fn test_os_info_serialization() {
        let os_info = OsInfo {
            os_type: OsType::Linux,
            name: "Linux".to_string(),
            version: "6.5.0".to_string(),
            long_version: Some("Linux 6.5.0-generic".to_string()),
            distribution: Some("ubuntu".to_string()),
            linux_distro: Some(LinuxDistro {
                id: "ubuntu".to_string(),
                name: "Ubuntu".to_string(),
                version: Some("22.04".to_string()),
                codename: Some("jammy".to_string()),
                family: LinuxFamily::Debian,
            }),
            kernel: "6.5.0-generic".to_string(),
            arch: "x86_64".to_string(),
            hostname: "myhost".to_string(),
            system_package_managers: None,
            locale: None,
            time: None,
        };

        let json = serde_json::to_string(&os_info).expect("serialization should succeed");
        let deserialized: OsInfo =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.os_type, OsType::Linux);
        assert_eq!(deserialized.name, "Linux");
        assert!(deserialized.linux_distro.is_some());
        let distro = deserialized.linux_distro.as_ref().unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    // ========================================
    // detect_linux_distro tests
    // ========================================

    #[test]
    fn test_detect_linux_distro_on_non_linux() {
        // On non-Linux systems, should return None
        #[cfg(not(target_os = "linux"))]
        {
            let result = detect_linux_distro();
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_detect_linux_distro_from_paths_with_os_release() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("should create temp dir");
        let os_release_path = temp_dir.path().join("os-release");
        let lsb_release_path = temp_dir.path().join("lsb-release");
        let system_release_path = temp_dir.path().join("system-release");

        // Write os-release file
        {
            let mut file = fs::File::create(&os_release_path).expect("should create file");
            writeln!(file, "ID=fedora").expect("should write");
            writeln!(file, "NAME=\"Fedora Linux\"").expect("should write");
            writeln!(file, "VERSION_ID=39").expect("should write");
        }

        let result = detect_linux_distro_from_paths(
            &os_release_path,
            &lsb_release_path,
            &system_release_path,
        );

        assert!(result.is_some());
        let distro = result.unwrap();
        assert_eq!(distro.id, "fedora");
        assert_eq!(distro.name, "Fedora Linux");
        assert_eq!(distro.version, Some("39".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_detect_linux_distro_from_paths_fallback_to_lsb() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("should create temp dir");
        let os_release_path = temp_dir.path().join("os-release-nonexistent");
        let lsb_release_path = temp_dir.path().join("lsb-release");
        let system_release_path = temp_dir.path().join("system-release");

        // Only write lsb-release file
        {
            let mut file = fs::File::create(&lsb_release_path).expect("should create file");
            writeln!(file, "DISTRIB_ID=Ubuntu").expect("should write");
            writeln!(file, "DISTRIB_RELEASE=22.04").expect("should write");
        }

        let result = detect_linux_distro_from_paths(
            &os_release_path,
            &lsb_release_path,
            &system_release_path,
        );

        assert!(result.is_some());
        let distro = result.unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.version, Some("22.04".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_detect_linux_distro_from_paths_fallback_to_system_release() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("should create temp dir");
        let os_release_path = temp_dir.path().join("os-release-nonexistent");
        let lsb_release_path = temp_dir.path().join("lsb-release-nonexistent");
        let system_release_path = temp_dir.path().join("system-release");

        // Only write system-release file
        {
            let mut file = fs::File::create(&system_release_path).expect("should create file");
            writeln!(file, "CentOS Linux release 7.9.2009 (Core)").expect("should write");
        }

        let result = detect_linux_distro_from_paths(
            &os_release_path,
            &lsb_release_path,
            &system_release_path,
        );

        assert!(result.is_some());
        let distro = result.unwrap();
        assert_eq!(distro.id, "centos");
        assert_eq!(distro.version, Some("7.9.2009".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_detect_linux_distro_from_paths_no_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("should create temp dir");
        let os_release_path = temp_dir.path().join("os-release-nonexistent");
        let lsb_release_path = temp_dir.path().join("lsb-release-nonexistent");
        let system_release_path = temp_dir.path().join("system-release-nonexistent");

        let result = detect_linux_distro_from_paths(
            &os_release_path,
            &lsb_release_path,
            &system_release_path,
        );

        assert!(result.is_none());
    }

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

    // ========== Locale Tests ==========

    mod extract_language_code_tests {
        use super::*;

        #[test]
        fn test_full_locale_with_encoding() {
            assert_eq!(extract_language_code("en_US.UTF-8"), Some("en".to_string()));
        }

        #[test]
        fn test_locale_with_territory_only() {
            assert_eq!(extract_language_code("de_DE"), Some("de".to_string()));
        }

        #[test]
        fn test_language_only() {
            assert_eq!(extract_language_code("zh"), Some("zh".to_string()));
        }

        #[test]
        fn test_locale_with_modifier() {
            assert_eq!(extract_language_code("sr_RS@latin"), Some("sr".to_string()));
        }

        #[test]
        fn test_full_locale_with_modifier() {
            assert_eq!(
                extract_language_code("zh_CN.GB18030@stroke"),
                Some("zh".to_string())
            );
        }

        #[test]
        fn test_c_locale_returns_none() {
            assert_eq!(extract_language_code("C"), None);
        }

        #[test]
        fn test_posix_locale_returns_none() {
            assert_eq!(extract_language_code("POSIX"), None);
        }

        #[test]
        fn test_empty_string_returns_none() {
            assert_eq!(extract_language_code(""), None);
        }

        #[test]
        fn test_iso_encoding() {
            assert_eq!(
                extract_language_code("de_DE.ISO-8859-1"),
                Some("de".to_string())
            );
        }
    }

    mod extract_encoding_tests {
        use super::*;

        #[test]
        fn test_utf8_encoding() {
            assert_eq!(extract_encoding("en_US.UTF-8"), Some("UTF-8".to_string()));
        }

        #[test]
        fn test_iso_encoding() {
            assert_eq!(
                extract_encoding("de_DE.ISO-8859-1"),
                Some("ISO-8859-1".to_string())
            );
        }

        #[test]
        fn test_gb18030_encoding() {
            assert_eq!(
                extract_encoding("zh_CN.GB18030"),
                Some("GB18030".to_string())
            );
        }

        #[test]
        fn test_encoding_with_modifier() {
            assert_eq!(
                extract_encoding("zh_CN.GB18030@stroke"),
                Some("GB18030".to_string())
            );
        }

        #[test]
        fn test_no_encoding_returns_none() {
            assert_eq!(extract_encoding("en_US"), None);
        }

        #[test]
        fn test_language_only_returns_none() {
            assert_eq!(extract_encoding("en"), None);
        }

        #[test]
        fn test_c_locale_returns_none() {
            assert_eq!(extract_encoding("C"), None);
        }

        #[test]
        fn test_posix_locale_returns_none() {
            assert_eq!(extract_encoding("POSIX"), None);
        }

        #[test]
        fn test_empty_string_returns_none() {
            assert_eq!(extract_encoding(""), None);
        }

        #[test]
        fn test_trailing_dot_returns_none() {
            assert_eq!(extract_encoding("en_US."), None);
        }
    }

    mod detect_locale_tests {
        use super::*;
        use std::sync::Mutex;

        // Mutex to ensure env var tests don't interfere with each other
        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        /// RAII guard for temporarily setting environment variables in tests.
        struct ScopedEnv {
            vars: Vec<(String, Option<String>)>,
        }

        impl ScopedEnv {
            fn new() -> Self {
                Self { vars: Vec::new() }
            }

            fn set(&mut self, key: &str, value: &str) -> &mut Self {
                // Store original value for restoration
                let original = std::env::var(key).ok();
                self.vars.push((key.to_string(), original));
                // SAFETY: Tests are run single-threaded with ENV_MUTEX protection,
                // and we restore the original values in Drop.
                unsafe { std::env::set_var(key, value) };
                self
            }

            fn remove(&mut self, key: &str) -> &mut Self {
                let original = std::env::var(key).ok();
                self.vars.push((key.to_string(), original));
                // SAFETY: Tests are run single-threaded with ENV_MUTEX protection,
                // and we restore the original values in Drop.
                unsafe { std::env::remove_var(key) };
                self
            }
        }

        impl Drop for ScopedEnv {
            fn drop(&mut self) {
                // Restore original values in reverse order
                for (key, original) in self.vars.iter().rev() {
                    // SAFETY: Restoring original values; tests are single-threaded.
                    match original {
                        Some(value) => unsafe { std::env::set_var(key, value) },
                        None => unsafe { std::env::remove_var(key) },
                    }
                }
            }
        }

        #[test]
        fn test_detect_locale_reads_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .remove("LC_ALL")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.lang, Some("en_US.UTF-8".to_string()));
            assert_eq!(locale.preferred_language, Some("en".to_string()));
            assert_eq!(locale.encoding, Some("UTF-8".to_string()));
        }

        #[test]
        fn test_lc_all_takes_priority_over_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "de_DE.ISO-8859-1")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("de".to_string()));
            assert_eq!(locale.encoding, Some("ISO-8859-1".to_string()));
        }

        #[test]
        fn test_lc_messages_takes_priority_over_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_MESSAGES", "fr_FR.UTF-8")
                .remove("LC_ALL");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("fr".to_string()));
        }

        #[test]
        fn test_lc_all_takes_priority_over_lc_messages() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_MESSAGES", "fr_FR.UTF-8")
                .set("LC_ALL", "ja_JP.UTF-8");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("ja".to_string()));
        }

        #[test]
        fn test_c_locale_is_skipped_in_priority() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "C")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            // LC_ALL is "C" so should fall back to LANG
            assert_eq!(locale.preferred_language, Some("en".to_string()));
            assert_eq!(locale.lc_all, Some("C".to_string())); // But LC_ALL is still captured
        }

        #[test]
        fn test_posix_locale_is_skipped_in_priority() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "de_DE.UTF-8")
                .set("LC_ALL", "POSIX")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("de".to_string()));
        }

        #[test]
        fn test_all_lc_vars_captured() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "de_DE.UTF-8")
                .set("LC_CTYPE", "fr_FR.UTF-8")
                .set("LC_MESSAGES", "ja_JP.UTF-8")
                .set("LC_TIME", "zh_CN.UTF-8");

            let locale = detect_locale();

            assert_eq!(locale.lang, Some("en_US.UTF-8".to_string()));
            assert_eq!(locale.lc_all, Some("de_DE.UTF-8".to_string()));
            assert_eq!(locale.lc_ctype, Some("fr_FR.UTF-8".to_string()));
            assert_eq!(locale.lc_messages, Some("ja_JP.UTF-8".to_string()));
            assert_eq!(locale.lc_time, Some("zh_CN.UTF-8".to_string()));
        }

        #[test]
        fn test_missing_vars_are_none() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.remove("LANG")
                .remove("LC_ALL")
                .remove("LC_CTYPE")
                .remove("LC_MESSAGES")
                .remove("LC_TIME");

            let locale = detect_locale();

            assert!(locale.lang.is_none());
            assert!(locale.lc_all.is_none());
            assert!(locale.lc_ctype.is_none());
            assert!(locale.lc_messages.is_none());
            assert!(locale.lc_time.is_none());
            assert!(locale.preferred_language.is_none());
            assert!(locale.encoding.is_none());
        }

        #[test]
        fn test_locale_info_default() {
            let info = LocaleInfo::default();
            assert!(info.lang.is_none());
            assert!(info.lc_all.is_none());
            assert!(info.lc_ctype.is_none());
            assert!(info.lc_messages.is_none());
            assert!(info.lc_time.is_none());
            assert!(info.preferred_language.is_none());
            assert!(info.encoding.is_none());
        }

        #[test]
        fn test_locale_info_serialization() {
            let locale = LocaleInfo {
                lang: Some("en_US.UTF-8".to_string()),
                lc_all: None,
                lc_ctype: Some("en_US.UTF-8".to_string()),
                lc_messages: None,
                lc_time: None,
                preferred_language: Some("en".to_string()),
                encoding: Some("UTF-8".to_string()),
            };

            let json = serde_json::to_string(&locale).unwrap();
            let deserialized: LocaleInfo = serde_json::from_str(&json).unwrap();

            assert_eq!(locale.lang, deserialized.lang);
            assert_eq!(locale.preferred_language, deserialized.preferred_language);
            assert_eq!(locale.encoding, deserialized.encoding);
        }
    }

    // ========================================
    // Package Manager tests
    // ========================================

    mod package_manager_tests {
        use super::*;

        #[test]
        fn test_system_package_manager_display() {
            assert_eq!(SystemPackageManager::Apt.to_string(), "apt");
            assert_eq!(SystemPackageManager::Homebrew.to_string(), "brew");
            assert_eq!(SystemPackageManager::Pacman.to_string(), "pacman");
            assert_eq!(SystemPackageManager::Portage.to_string(), "emerge");
            assert_eq!(SystemPackageManager::Winget.to_string(), "winget");
            assert_eq!(
                SystemPackageManager::Msys2Pacman.to_string(),
                "pacman (MSYS2)"
            );
        }

        #[test]
        fn test_system_package_manager_executable_name() {
            assert_eq!(SystemPackageManager::Apt.executable_name(), "apt");
            assert_eq!(SystemPackageManager::Homebrew.executable_name(), "brew");
            assert_eq!(SystemPackageManager::Portage.executable_name(), "emerge");
            assert_eq!(SystemPackageManager::Pacman.executable_name(), "pacman");
            assert_eq!(
                SystemPackageManager::Msys2Pacman.executable_name(),
                "pacman"
            );
            assert_eq!(SystemPackageManager::Ports.executable_name(), "make");
        }

        #[test]
        fn test_system_package_manager_serialization() {
            let managers = [
                SystemPackageManager::Apt,
                SystemPackageManager::Dnf,
                SystemPackageManager::Pacman,
                SystemPackageManager::Homebrew,
                SystemPackageManager::Winget,
                SystemPackageManager::Pkg,
            ];

            for manager in managers {
                let json = serde_json::to_string(&manager).expect("serialization should succeed");
                let deserialized: SystemPackageManager =
                    serde_json::from_str(&json).expect("deserialization should succeed");
                assert_eq!(manager, deserialized);
            }
        }

        #[test]
        fn test_system_package_manager_hash() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(SystemPackageManager::Apt);
            set.insert(SystemPackageManager::Dnf);
            set.insert(SystemPackageManager::Apt); // Duplicate

            assert_eq!(set.len(), 2);
            assert!(set.contains(&SystemPackageManager::Apt));
            assert!(set.contains(&SystemPackageManager::Dnf));
        }

        #[test]
        fn test_package_manager_commands_default() {
            let cmds = PackageManagerCommands::default();
            assert!(cmds.list.is_none());
            assert!(cmds.update.is_none());
            assert!(cmds.upgrade.is_none());
            assert!(cmds.search.is_none());
        }

        #[test]
        fn test_system_package_managers_default() {
            let managers = SystemPackageManagers::default();
            assert!(managers.managers.is_empty());
            assert!(managers.primary.is_none());
        }

        #[test]
        fn test_get_commands_for_manager_apt() {
            let cmds = get_commands_for_manager(SystemPackageManager::Apt);
            assert_eq!(cmds.list, Some("apt list --installed".to_string()));
            assert_eq!(cmds.update, Some("apt update".to_string()));
            assert_eq!(cmds.upgrade, Some("apt upgrade".to_string()));
            assert_eq!(cmds.search, Some("apt search".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_dnf() {
            let cmds = get_commands_for_manager(SystemPackageManager::Dnf);
            assert_eq!(cmds.list, Some("dnf list installed".to_string()));
            assert_eq!(cmds.update, Some("dnf check-update".to_string()));
            assert_eq!(cmds.upgrade, Some("dnf upgrade".to_string()));
            assert_eq!(cmds.search, Some("dnf search".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_pacman() {
            let cmds = get_commands_for_manager(SystemPackageManager::Pacman);
            assert_eq!(cmds.list, Some("pacman -Q".to_string()));
            assert_eq!(cmds.update, Some("pacman -Sy".to_string()));
            assert_eq!(cmds.upgrade, Some("pacman -Syu".to_string()));
            assert_eq!(cmds.search, Some("pacman -Ss".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_homebrew() {
            let cmds = get_commands_for_manager(SystemPackageManager::Homebrew);
            assert_eq!(cmds.list, Some("brew list".to_string()));
            assert_eq!(cmds.update, Some("brew update".to_string()));
            assert_eq!(cmds.upgrade, Some("brew upgrade".to_string()));
            assert_eq!(cmds.search, Some("brew search".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_winget() {
            let cmds = get_commands_for_manager(SystemPackageManager::Winget);
            assert_eq!(cmds.list, Some("winget list".to_string()));
            assert_eq!(cmds.update, None);
            assert_eq!(cmds.upgrade, Some("winget upgrade --all".to_string()));
            assert_eq!(cmds.search, Some("winget search".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_dpkg_is_low_level() {
            let cmds = get_commands_for_manager(SystemPackageManager::Dpkg);
            assert_eq!(cmds.list, Some("dpkg -l".to_string()));
            assert!(cmds.update.is_none()); // Low-level, no update
            assert!(cmds.upgrade.is_none());
            assert_eq!(cmds.search, Some("dpkg -S".to_string()));
        }

        #[test]
        fn test_get_commands_for_manager_makepkg_is_build_tool() {
            let cmds = get_commands_for_manager(SystemPackageManager::Makepkg);
            assert!(cmds.list.is_none()); // Build tool only
            assert!(cmds.update.is_none());
            assert!(cmds.upgrade.is_none());
            assert!(cmds.search.is_none());
        }

        #[test]
        fn test_get_commands_for_all_managers() {
            // Verify that all managers have at least one command defined
            // (except build tools like makepkg)
            let managers = [
                SystemPackageManager::Apt,
                SystemPackageManager::Aptitude,
                SystemPackageManager::Dpkg,
                SystemPackageManager::Nala,
                SystemPackageManager::AptFast,
                SystemPackageManager::Dnf,
                SystemPackageManager::Yum,
                SystemPackageManager::Microdnf,
                SystemPackageManager::Rpm,
                SystemPackageManager::Pacman,
                SystemPackageManager::Yay,
                SystemPackageManager::Paru,
                SystemPackageManager::Pamac,
                SystemPackageManager::Zypper,
                SystemPackageManager::Portage,
                SystemPackageManager::Apk,
                SystemPackageManager::Xbps,
                SystemPackageManager::Snap,
                SystemPackageManager::Flatpak,
                SystemPackageManager::Guix,
                SystemPackageManager::Nix,
                SystemPackageManager::NixEnv,
                SystemPackageManager::Homebrew,
                SystemPackageManager::MacPorts,
                SystemPackageManager::Fink,
                SystemPackageManager::Softwareupdate,
                SystemPackageManager::Winget,
                SystemPackageManager::Chocolatey,
                SystemPackageManager::Scoop,
                SystemPackageManager::Pkg,
                SystemPackageManager::Ports,
                SystemPackageManager::PkgAdd,
                SystemPackageManager::Pkgin,
            ];

            for manager in managers {
                let cmds = get_commands_for_manager(manager);
                let has_at_least_one = cmds.list.is_some()
                    || cmds.update.is_some()
                    || cmds.upgrade.is_some()
                    || cmds.search.is_some();
                assert!(
                    has_at_least_one,
                    "{:?} should have at least one command",
                    manager
                );
            }
        }
    }

    mod path_parsing_tests {
        use super::*;
        use std::sync::Mutex;
        use tempfile::TempDir;

        // Mutex to ensure env var tests don't interfere with each other
        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        /// RAII guard for temporarily setting environment variables in tests.
        struct ScopedEnv {
            vars: Vec<(String, Option<String>)>,
        }

        impl ScopedEnv {
            fn new() -> Self {
                Self { vars: Vec::new() }
            }

            fn set(&mut self, key: &str, value: &str) -> &mut Self {
                let original = std::env::var(key).ok();
                self.vars.push((key.to_string(), original));
                // SAFETY: Tests are run single-threaded with ENV_MUTEX protection
                unsafe { std::env::set_var(key, value) };
                self
            }
        }

        impl Drop for ScopedEnv {
            fn drop(&mut self) {
                for (key, original) in self.vars.iter().rev() {
                    // SAFETY: Restoring original values; tests are single-threaded
                    match original {
                        Some(value) => unsafe { std::env::set_var(key, value) },
                        None => unsafe { std::env::remove_var(key) },
                    }
                }
            }
        }

        #[test]
        fn test_get_path_dirs_parses_valid_dirs() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let temp = TempDir::new().expect("should create temp dir");
            let dir1 = temp.path().join("bin1");
            let dir2 = temp.path().join("bin2");

            fs::create_dir(&dir1).expect("should create dir1");
            fs::create_dir(&dir2).expect("should create dir2");

            #[cfg(not(target_os = "windows"))]
            let path_value = format!("{}:{}", dir1.display(), dir2.display());
            #[cfg(target_os = "windows")]
            let path_value = format!("{};{}", dir1.display(), dir2.display());

            let mut env = ScopedEnv::new();
            env.set("PATH", &path_value);

            let dirs = get_path_dirs();
            assert!(dirs.contains(&dir1));
            assert!(dirs.contains(&dir2));
        }

        #[test]
        fn test_get_path_dirs_filters_nonexistent() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let temp = TempDir::new().expect("should create temp dir");
            let existing = temp.path().join("exists");
            let nonexistent = temp.path().join("does_not_exist");

            fs::create_dir(&existing).expect("should create dir");

            #[cfg(not(target_os = "windows"))]
            let path_value = format!("{}:{}", existing.display(), nonexistent.display());
            #[cfg(target_os = "windows")]
            let path_value = format!("{};{}", existing.display(), nonexistent.display());

            let mut env = ScopedEnv::new();
            env.set("PATH", &path_value);

            let dirs = get_path_dirs();
            assert!(dirs.contains(&existing));
            assert!(!dirs.contains(&nonexistent));
        }

        #[test]
        fn test_get_path_dirs_handles_empty_entries() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let temp = TempDir::new().expect("should create temp dir");
            let dir = temp.path().join("bin");
            fs::create_dir(&dir).expect("should create dir");

            #[cfg(not(target_os = "windows"))]
            let path_value = format!("::{}::", dir.display());
            #[cfg(target_os = "windows")]
            let path_value = format!(";;{};;", dir.display());

            let mut env = ScopedEnv::new();
            env.set("PATH", &path_value);

            let dirs = get_path_dirs();
            assert!(dirs.contains(&dir));
            // Empty entries should be filtered out
            assert!(!dirs.iter().any(|p| p.as_os_str().is_empty()));
        }

        #[test]
        #[cfg(not(target_os = "windows"))]
        fn test_command_exists_in_path_finds_executable() {
            use std::os::unix::fs::PermissionsExt;

            let temp = TempDir::new().expect("should create temp dir");
            let bin_dir = temp.path().join("bin");
            fs::create_dir(&bin_dir).expect("should create bin dir");

            // Create an executable file
            let exe_path = bin_dir.join("test_cmd");
            fs::write(&exe_path, "#!/bin/sh\necho hello").expect("should write file");
            fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))
                .expect("should set permissions");

            let path_dirs = vec![bin_dir.clone()];
            let result = command_exists_in_path("test_cmd", &path_dirs);

            assert!(result.is_some());
            assert_eq!(result.unwrap(), exe_path);
        }

        #[test]
        #[cfg(not(target_os = "windows"))]
        fn test_command_exists_in_path_rejects_non_executable() {
            let temp = TempDir::new().expect("should create temp dir");
            let bin_dir = temp.path().join("bin");
            fs::create_dir(&bin_dir).expect("should create bin dir");

            // Create a non-executable file
            let file_path = bin_dir.join("not_exe");
            fs::write(&file_path, "just data").expect("should write file");
            // Don't set executable bit

            let path_dirs = vec![bin_dir];
            let result = command_exists_in_path("not_exe", &path_dirs);

            assert!(result.is_none());
        }

        #[test]
        fn test_command_exists_in_path_returns_none_for_missing() {
            let temp = TempDir::new().expect("should create temp dir");
            let bin_dir = temp.path().join("bin");
            fs::create_dir(&bin_dir).expect("should create bin dir");

            let path_dirs = vec![bin_dir];
            let result = command_exists_in_path("nonexistent_command_xyz", &path_dirs);

            assert!(result.is_none());
        }

        #[test]
        fn test_command_exists_in_path_searches_in_order() {
            use std::os::unix::fs::PermissionsExt;

            let temp = TempDir::new().expect("should create temp dir");
            let dir1 = temp.path().join("dir1");
            let dir2 = temp.path().join("dir2");
            fs::create_dir(&dir1).expect("should create dir1");
            fs::create_dir(&dir2).expect("should create dir2");

            // Create executable in both directories
            let exe1 = dir1.join("mycmd");
            let exe2 = dir2.join("mycmd");
            fs::write(&exe1, "first").expect("should write exe1");
            fs::write(&exe2, "second").expect("should write exe2");

            #[cfg(not(target_os = "windows"))]
            {
                fs::set_permissions(&exe1, fs::Permissions::from_mode(0o755))
                    .expect("should set permissions");
                fs::set_permissions(&exe2, fs::Permissions::from_mode(0o755))
                    .expect("should set permissions");
            }

            // dir1 comes first, so exe1 should be found
            let path_dirs = vec![dir1.clone(), dir2.clone()];
            let result = command_exists_in_path("mycmd", &path_dirs);

            assert!(result.is_some());
            assert_eq!(result.unwrap(), exe1);
        }
    }

    mod detected_package_manager_tests {
        use super::*;

        #[test]
        fn test_detected_package_manager_serialization() {
            let detected = DetectedPackageManager {
                manager: SystemPackageManager::Apt,
                path: "/usr/bin/apt".to_string(),
                is_primary: true,
                commands: PackageManagerCommands {
                    list: Some("apt list --installed".to_string()),
                    update: Some("apt update".to_string()),
                    upgrade: Some("apt upgrade".to_string()),
                    search: Some("apt search".to_string()),
                },
            };

            let json = serde_json::to_string(&detected).expect("serialization should succeed");
            let deserialized: DetectedPackageManager =
                serde_json::from_str(&json).expect("deserialization should succeed");

            assert_eq!(deserialized.manager, SystemPackageManager::Apt);
            assert_eq!(deserialized.path, "/usr/bin/apt");
            assert!(deserialized.is_primary);
            assert_eq!(
                deserialized.commands.list,
                Some("apt list --installed".to_string())
            );
        }

        #[test]
        fn test_system_package_managers_serialization() {
            let managers = SystemPackageManagers {
                managers: vec![
                    DetectedPackageManager {
                        manager: SystemPackageManager::Apt,
                        path: "/usr/bin/apt".to_string(),
                        is_primary: true,
                        commands: get_commands_for_manager(SystemPackageManager::Apt),
                    },
                    DetectedPackageManager {
                        manager: SystemPackageManager::Snap,
                        path: "/usr/bin/snap".to_string(),
                        is_primary: false,
                        commands: get_commands_for_manager(SystemPackageManager::Snap),
                    },
                ],
                primary: Some(SystemPackageManager::Apt),
            };

            let json = serde_json::to_string(&managers).expect("serialization should succeed");
            let deserialized: SystemPackageManagers =
                serde_json::from_str(&json).expect("deserialization should succeed");

            assert_eq!(deserialized.managers.len(), 2);
            assert_eq!(deserialized.primary, Some(SystemPackageManager::Apt));
        }
    }

    mod detect_linux_package_managers_tests {
        use super::*;

        #[test]
        fn test_detect_linux_package_managers_does_not_panic() {
            // Should not panic regardless of what's in PATH
            let _ = detect_linux_package_managers(None);
            let _ = detect_linux_package_managers(Some(LinuxFamily::Debian));
            let _ = detect_linux_package_managers(Some(LinuxFamily::RedHat));
            let _ = detect_linux_package_managers(Some(LinuxFamily::Arch));
            let _ = detect_linux_package_managers(Some(LinuxFamily::Other));
        }

        #[test]
        fn test_detect_linux_package_managers_returns_valid_structure() {
            let result = detect_linux_package_managers(None);

            // Verify structure is valid
            for mgr in &result.managers {
                // Path should not be empty for detected managers
                assert!(!mgr.path.is_empty());
                // Commands should be populated
                let cmds = &mgr.commands;
                let has_cmd = cmds.list.is_some()
                    || cmds.update.is_some()
                    || cmds.upgrade.is_some()
                    || cmds.search.is_some();
                // All managers except makepkg should have at least one command
                if mgr.manager != SystemPackageManager::Makepkg {
                    assert!(
                        has_cmd,
                        "{:?} should have at least one command",
                        mgr.manager
                    );
                }
            }
        }

        #[test]
        fn test_detect_linux_package_managers_primary_consistency() {
            let result = detect_linux_package_managers(None);

            // If primary is set, it should be in the managers list
            if let Some(primary) = result.primary {
                let found = result.managers.iter().any(|m| m.manager == primary);
                assert!(found, "Primary {:?} should be in managers list", primary);

                // The primary manager should have is_primary = true
                let primary_entry = result.managers.iter().find(|m| m.manager == primary);
                assert!(
                    primary_entry.map_or(false, |e| e.is_primary),
                    "Primary manager should have is_primary=true"
                );
            }

            // Only one manager should be marked as primary
            let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
            assert!(
                primary_count <= 1,
                "At most one manager should be marked primary"
            );
        }

        #[test]
        fn test_determine_linux_primary_debian_family() {
            // Simulate detected managers for Debian
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Apt,
                    path: "/usr/bin/apt".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Dpkg,
                    path: "/usr/bin/dpkg".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Snap,
                    path: "/usr/bin/snap".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::Debian));
            assert_eq!(primary, Some(SystemPackageManager::Apt));
        }

        #[test]
        fn test_determine_linux_primary_debian_fallback_to_dpkg() {
            // Only dpkg available
            let detected = vec![DetectedPackageManager {
                manager: SystemPackageManager::Dpkg,
                path: "/usr/bin/dpkg".to_string(),
                is_primary: false,
                commands: PackageManagerCommands::default(),
            }];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::Debian));
            assert_eq!(primary, Some(SystemPackageManager::Dpkg));
        }

        #[test]
        fn test_determine_linux_primary_redhat_prefers_dnf() {
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Dnf,
                    path: "/usr/bin/dnf".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Yum,
                    path: "/usr/bin/yum".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::RedHat));
            assert_eq!(primary, Some(SystemPackageManager::Dnf));
        }

        #[test]
        fn test_determine_linux_primary_redhat_fallback_to_yum() {
            let detected = vec![DetectedPackageManager {
                manager: SystemPackageManager::Yum,
                path: "/usr/bin/yum".to_string(),
                is_primary: false,
                commands: PackageManagerCommands::default(),
            }];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::RedHat));
            assert_eq!(primary, Some(SystemPackageManager::Yum));
        }

        #[test]
        fn test_determine_linux_primary_arch() {
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Pacman,
                    path: "/usr/bin/pacman".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Yay,
                    path: "/usr/bin/yay".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::Arch));
            // Should be pacman, not yay (AUR helper)
            assert_eq!(primary, Some(SystemPackageManager::Pacman));
        }

        #[test]
        fn test_determine_linux_primary_nixos() {
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Nix,
                    path: "/run/current-system/sw/bin/nix".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::NixEnv,
                    path: "/run/current-system/sw/bin/nix-env".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::NixOS));
            // Should prefer nix over nix-env
            assert_eq!(primary, Some(SystemPackageManager::Nix));
        }

        #[test]
        fn test_determine_linux_primary_fallback_without_family() {
            // Test fallback when no family hint is provided
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Apt,
                    path: "/usr/bin/apt".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Flatpak,
                    path: "/usr/bin/flatpak".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, None);
            // Should pick apt from priority order
            assert_eq!(primary, Some(SystemPackageManager::Apt));
        }

        #[test]
        fn test_determine_linux_primary_aur_helpers_not_primary() {
            // Only AUR helpers detected (edge case)
            let detected = vec![
                DetectedPackageManager {
                    manager: SystemPackageManager::Yay,
                    path: "/usr/bin/yay".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
                DetectedPackageManager {
                    manager: SystemPackageManager::Paru,
                    path: "/usr/bin/paru".to_string(),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                },
            ];

            let primary = determine_linux_primary(&detected, None);
            // AUR helpers should not be selected as primary
            assert_eq!(primary, None);
        }

        #[test]
        fn test_determine_linux_primary_empty_detected() {
            let detected: Vec<DetectedPackageManager> = vec![];

            let primary = determine_linux_primary(&detected, Some(LinuxFamily::Debian));
            assert_eq!(primary, None);
        }

        #[test]
        fn test_determine_linux_primary_all_families() {
            // Test that each family selects the correct primary
            let test_cases = [
                (LinuxFamily::Debian, SystemPackageManager::Apt),
                (LinuxFamily::RedHat, SystemPackageManager::Dnf),
                (LinuxFamily::Arch, SystemPackageManager::Pacman),
                (LinuxFamily::SUSE, SystemPackageManager::Zypper),
                (LinuxFamily::Alpine, SystemPackageManager::Apk),
                (LinuxFamily::Void, SystemPackageManager::Xbps),
                (LinuxFamily::Slackware, SystemPackageManager::Pkgtool),
                (LinuxFamily::NixOS, SystemPackageManager::Nix),
                (LinuxFamily::Gentoo, SystemPackageManager::Portage),
            ];

            for (family, expected_manager) in test_cases {
                let detected = vec![DetectedPackageManager {
                    manager: expected_manager,
                    path: format!("/usr/bin/{}", expected_manager),
                    is_primary: false,
                    commands: PackageManagerCommands::default(),
                }];

                let primary = determine_linux_primary(&detected, Some(family));
                assert_eq!(
                    primary,
                    Some(expected_manager),
                    "Family {:?} should select {:?}",
                    family,
                    expected_manager
                );
            }
        }

        #[test]
        fn test_linux_package_managers_constant_coverage() {
            // Ensure all expected managers are in the constant
            let expected_managers = [
                SystemPackageManager::Apt,
                SystemPackageManager::Aptitude,
                SystemPackageManager::Dpkg,
                SystemPackageManager::Nala,
                SystemPackageManager::AptFast,
                SystemPackageManager::Dnf,
                SystemPackageManager::Yum,
                SystemPackageManager::Microdnf,
                SystemPackageManager::Rpm,
                SystemPackageManager::Pacman,
                SystemPackageManager::Makepkg,
                SystemPackageManager::Yay,
                SystemPackageManager::Paru,
                SystemPackageManager::Pamac,
                SystemPackageManager::Zypper,
                SystemPackageManager::Portage,
                SystemPackageManager::Apk,
                SystemPackageManager::Xbps,
                SystemPackageManager::Pkgtool,
                SystemPackageManager::Snap,
                SystemPackageManager::Flatpak,
                SystemPackageManager::Guix,
                SystemPackageManager::Nix,
                SystemPackageManager::NixEnv,
            ];

            for expected in expected_managers {
                let found = LINUX_PACKAGE_MANAGERS
                    .iter()
                    .any(|(mgr, _)| *mgr == expected);
                assert!(found, "{:?} should be in LINUX_PACKAGE_MANAGERS", expected);
            }
        }

        #[test]
        fn test_aur_helpers_constant() {
            // Verify AUR helpers are correctly defined
            assert!(AUR_HELPERS.contains(&SystemPackageManager::Yay));
            assert!(AUR_HELPERS.contains(&SystemPackageManager::Paru));
            assert!(AUR_HELPERS.contains(&SystemPackageManager::Pamac));

            // Pacman should NOT be an AUR helper
            assert!(!AUR_HELPERS.contains(&SystemPackageManager::Pacman));
        }
    }

    mod detect_macos_package_managers_tests {
        use super::*;

        #[test]
        fn test_detect_macos_package_managers_does_not_panic() {
            // Should not panic regardless of what's installed
            let _ = detect_macos_package_managers();
        }

        #[test]
        fn test_detect_macos_package_managers_always_includes_softwareupdate() {
            let result = detect_macos_package_managers();

            // softwareupdate should always be present
            let has_softwareupdate = result
                .managers
                .iter()
                .any(|m| m.manager == SystemPackageManager::Softwareupdate);
            assert!(
                has_softwareupdate,
                "softwareupdate should always be detected on macOS"
            );

            // softwareupdate path should be /usr/sbin/softwareupdate
            let softwareupdate = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Softwareupdate);
            assert!(softwareupdate.is_some());
            assert_eq!(softwareupdate.unwrap().path, "/usr/sbin/softwareupdate");
        }

        #[test]
        fn test_detect_macos_package_managers_always_has_primary() {
            let result = detect_macos_package_managers();

            // There should always be a primary (at minimum softwareupdate)
            assert!(
                result.primary.is_some(),
                "macOS detection should always have a primary manager"
            );
        }

        #[test]
        fn test_detect_macos_package_managers_primary_consistency() {
            let result = detect_macos_package_managers();

            // If primary is set, it should be in the managers list
            if let Some(primary) = result.primary {
                let found = result.managers.iter().any(|m| m.manager == primary);
                assert!(found, "Primary {:?} should be in managers list", primary);

                // The primary manager should have is_primary = true
                let primary_entry = result.managers.iter().find(|m| m.manager == primary);
                assert!(
                    primary_entry.map_or(false, |e| e.is_primary),
                    "Primary manager should have is_primary=true"
                );
            }

            // Only one manager should be marked as primary
            let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
            assert_eq!(
                primary_count, 1,
                "Exactly one manager should be marked primary"
            );
        }

        #[test]
        fn test_detect_macos_package_managers_valid_structure() {
            let result = detect_macos_package_managers();

            for mgr in &result.managers {
                // Path should not be empty
                assert!(!mgr.path.is_empty(), "{:?} should have a path", mgr.manager);

                // Commands should be populated for macOS managers
                let cmds = &mgr.commands;
                let has_cmd = cmds.list.is_some()
                    || cmds.update.is_some()
                    || cmds.upgrade.is_some()
                    || cmds.search.is_some();
                assert!(
                    has_cmd,
                    "{:?} should have at least one command",
                    mgr.manager
                );
            }
        }

        #[test]
        fn test_homebrew_locations_constants() {
            // Verify the constants are set correctly
            assert_eq!(HOMEBREW_APPLE_SILICON_PATH, "/opt/homebrew/bin/brew");
            assert_eq!(HOMEBREW_INTEL_PATH, "/usr/local/bin/brew");
            assert_eq!(SOFTWAREUPDATE_PATH, "/usr/sbin/softwareupdate");
        }

        #[test]
        #[cfg(target_os = "macos")]
        fn test_detect_macos_package_managers_on_macos() {
            let result = detect_macos_package_managers();

            // On actual macOS, we expect softwareupdate to exist at its path
            let softwareupdate = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Softwareupdate);
            assert!(softwareupdate.is_some());

            // The path should point to an actual file on macOS
            let path = std::path::Path::new("/usr/sbin/softwareupdate");
            assert!(path.exists(), "softwareupdate should exist on macOS");

            // If Homebrew is detected, verify its path exists
            if let Some(brew) = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Homebrew)
            {
                let brew_path = std::path::Path::new(&brew.path);
                assert!(brew_path.exists(), "Detected Homebrew path should exist");
            }
        }
    }

    // ========================================
    // Windows Package Manager Detection tests
    // ========================================

    mod detect_windows_package_managers_tests {
        use super::*;

        #[test]
        fn test_detect_windows_package_managers_does_not_panic() {
            // Should not panic even on non-Windows systems
            let _ = detect_windows_package_managers();
        }

        #[test]
        fn test_detect_windows_package_managers_returns_valid_structure() {
            let result = detect_windows_package_managers();

            // Verify that all detected managers have valid commands
            for manager in &result.managers {
                let cmds = &manager.commands;
                // At least one command should be defined for most managers
                let has_any_command = cmds.list.is_some()
                    || cmds.update.is_some()
                    || cmds.upgrade.is_some()
                    || cmds.search.is_some();

                // All Windows package managers should have at least list or search
                assert!(
                    has_any_command,
                    "{:?} should have at least one command defined",
                    manager.manager
                );
            }
        }

        #[test]
        fn test_detect_windows_package_managers_primary_consistency() {
            let result = detect_windows_package_managers();

            // If a primary is set, exactly one manager should have is_primary = true
            if let Some(primary) = result.primary {
                let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
                assert_eq!(
                    primary_count, 1,
                    "Exactly one manager should be marked as primary"
                );

                // The primary flag should match the primary field
                let primary_manager = result
                    .managers
                    .iter()
                    .find(|m| m.is_primary)
                    .expect("Should have a primary manager");
                assert_eq!(
                    primary_manager.manager, primary,
                    "Primary manager should match the primary field"
                );
            } else {
                // If no primary, no manager should have is_primary = true
                let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
                assert_eq!(
                    primary_count, 0,
                    "No manager should be primary when primary is None"
                );
            }
        }

        #[test]
        fn test_detect_windows_package_managers_correct_managers_detected() {
            let result = detect_windows_package_managers();

            // All detected managers should be Windows-specific managers
            let valid_windows_managers = [
                SystemPackageManager::Winget,
                SystemPackageManager::Dism,
                SystemPackageManager::Chocolatey,
                SystemPackageManager::Scoop,
                SystemPackageManager::Msys2Pacman,
            ];

            for detected in &result.managers {
                assert!(
                    valid_windows_managers.contains(&detected.manager),
                    "Detected manager {:?} should be a Windows-specific manager",
                    detected.manager
                );
            }
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn test_detect_windows_package_managers_on_windows() {
            let result = detect_windows_package_managers();

            // On actual Windows, we expect to find at least DISM (system utility)
            // Note: DISM might not be found if the test runs in a non-standard environment
            // but if it's found, it should have the correct path format
            if let Some(dism) = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Dism)
            {
                // DISM path should end with dism.exe
                assert!(
                    dism.path.to_lowercase().ends_with("dism.exe"),
                    "DISM path should end with dism.exe"
                );
            }
        }
    }

    // ========================================
    // BSD Package Manager Detection tests
    // ========================================

    mod detect_bsd_package_managers_tests {
        use super::*;

        #[test]
        fn test_detect_bsd_package_managers_freebsd() {
            let result = detect_bsd_package_managers(OsType::FreeBSD);

            // Verify that if any managers are detected, they're FreeBSD-appropriate
            let valid_freebsd_managers = [SystemPackageManager::Pkg, SystemPackageManager::Ports];

            for detected in &result.managers {
                assert!(
                    valid_freebsd_managers.contains(&detected.manager),
                    "Detected manager {:?} should be a FreeBSD-specific manager",
                    detected.manager
                );
            }

            // If pkg is detected, it should be primary
            if let Some(pkg) = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Pkg)
            {
                assert!(pkg.is_primary, "pkg should be primary on FreeBSD");
                assert_eq!(result.primary, Some(SystemPackageManager::Pkg));
            }
        }

        #[test]
        fn test_detect_bsd_package_managers_openbsd() {
            let result = detect_bsd_package_managers(OsType::OpenBSD);

            // Verify that if any managers are detected, they're OpenBSD-appropriate
            let valid_openbsd_managers = [SystemPackageManager::PkgAdd];

            for detected in &result.managers {
                assert!(
                    valid_openbsd_managers.contains(&detected.manager),
                    "Detected manager {:?} should be an OpenBSD-specific manager",
                    detected.manager
                );
            }

            // If pkg_add is detected, it should be primary
            if let Some(pkg_add) = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::PkgAdd)
            {
                assert!(pkg_add.is_primary, "pkg_add should be primary on OpenBSD");
                assert_eq!(result.primary, Some(SystemPackageManager::PkgAdd));
            }
        }

        #[test]
        fn test_detect_bsd_package_managers_netbsd() {
            let result = detect_bsd_package_managers(OsType::NetBSD);

            // Verify that if any managers are detected, they're NetBSD-appropriate
            let valid_netbsd_managers = [SystemPackageManager::Pkgin, SystemPackageManager::PkgAdd];

            for detected in &result.managers {
                assert!(
                    valid_netbsd_managers.contains(&detected.manager),
                    "Detected manager {:?} should be a NetBSD-specific manager",
                    detected.manager
                );
            }

            // If pkgin is detected, it should be primary
            if let Some(pkgin) = result
                .managers
                .iter()
                .find(|m| m.manager == SystemPackageManager::Pkgin)
            {
                assert!(pkgin.is_primary, "pkgin should be primary on NetBSD");
                assert_eq!(result.primary, Some(SystemPackageManager::Pkgin));
            }
        }

        #[test]
        fn test_detect_bsd_package_managers_non_bsd_returns_empty() {
            // Non-BSD OS types should return empty results
            let non_bsd_types = [OsType::Linux, OsType::MacOS, OsType::Windows, OsType::Other];

            for os_type in non_bsd_types {
                let result = detect_bsd_package_managers(os_type);
                assert!(
                    result.managers.is_empty(),
                    "Non-BSD OS type {:?} should return empty managers",
                    os_type
                );
                assert!(
                    result.primary.is_none(),
                    "Non-BSD OS type {:?} should have no primary",
                    os_type
                );
            }
        }

        #[test]
        fn test_detect_bsd_package_managers_primary_consistency() {
            for os_type in [OsType::FreeBSD, OsType::OpenBSD, OsType::NetBSD] {
                let result = detect_bsd_package_managers(os_type);

                // If a primary is set, exactly one manager should have is_primary = true
                if let Some(primary) = result.primary {
                    let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
                    assert_eq!(
                        primary_count, 1,
                        "{:?}: Exactly one manager should be marked as primary",
                        os_type
                    );

                    // The primary flag should match the primary field
                    let primary_manager = result
                        .managers
                        .iter()
                        .find(|m| m.is_primary)
                        .expect("Should have a primary manager");
                    assert_eq!(
                        primary_manager.manager, primary,
                        "{:?}: Primary manager should match the primary field",
                        os_type
                    );
                } else if !result.managers.is_empty() {
                    // If we have managers but no primary, check consistency
                    let primary_count = result.managers.iter().filter(|m| m.is_primary).count();
                    assert_eq!(
                        primary_count, 0,
                        "{:?}: No manager should be primary when primary is None",
                        os_type
                    );
                }
            }
        }

        #[test]
        fn test_detect_bsd_package_managers_commands_populated() {
            for os_type in [OsType::FreeBSD, OsType::OpenBSD, OsType::NetBSD] {
                let result = detect_bsd_package_managers(os_type);

                for manager in &result.managers {
                    let cmds = &manager.commands;
                    // Each detected manager should have commands populated
                    let has_any_command = cmds.list.is_some()
                        || cmds.update.is_some()
                        || cmds.upgrade.is_some()
                        || cmds.search.is_some();

                    assert!(
                        has_any_command,
                        "{:?} on {:?} should have at least one command",
                        manager.manager, os_type
                    );
                }
            }
        }
    }
}
