//! Operating system detection and information gathering.
//!
//! This module provides functionality for detecting the operating system type,
//! Linux distribution details, locale settings, timezone information, and
//! available package managers.
//!
//! ## Modules
//!
//! - [`distro`] - Linux distribution detection and classification
//! - [`locale`] - Locale detection from environment variables
//! - [`time`] - Timezone and NTP status detection
//! - [`package_manager`] - System package manager detection

use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::Result;

// Submodules
mod distro;
mod locale;
mod package_manager;
mod time;

// Re-export all public types for API stability
pub use distro::{
    LinuxDistro, LinuxFamily, detect_linux_distro, infer_linux_family, parse_lsb_release_content,
    parse_os_release_content, parse_system_release_content,
};
pub use locale::{LocaleInfo, detect_locale, extract_encoding, extract_language_code};
pub use package_manager::{
    DetectedPackageManager, PackageManagerCommands, SystemPackageManager, SystemPackageManagers,
    command_exists_in_path, detect_bsd_package_managers, detect_linux_package_managers,
    detect_macos_package_managers, detect_windows_package_managers, get_commands_for_manager,
    get_path_dirs,
};
pub use time::{NtpStatus, TimeInfo, detect_ntp_status, detect_timezone};

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
// OS Information
// ============================================================================

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
    /// System hostname
    pub hostname: String,
    /// System uptime in seconds
    pub uptime_seconds: u64,
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
/// println!("Kernel: {}", os.kernel);
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
        hostname: System::host_name().unwrap_or_default(),
        uptime_seconds: System::uptime(),
        system_package_managers,
        locale,
        time,
    })
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
            hostname: "myhost".to_string(),
            uptime_seconds: 3600, // 1 hour for test
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
}
