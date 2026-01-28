//! Operating system and distribution detection.
//!
//! This module provides types and functions for detecting the current operating
//! system, Linux distribution details, and CI environment status.
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::os_detection::{detect_os_type, detect_linux_distro, is_ci, OsType};
//!
//! let os = detect_os_type();
//! match os {
//!     OsType::Linux => {
//!         if let Some(distro) = detect_linux_distro() {
//!             println!("Running on {} ({})", distro.name, distro.family);
//!         }
//!     }
//!     OsType::MacOS => println!("Running on macOS"),
//!     OsType::Windows => println!("Running on Windows"),
//!     _ => println!("Running on {:?}", os),
//! }
//!
//! if is_ci() {
//!     println!("Running in CI environment");
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

/// The detected operating system type.
///
/// Uses `std::env::consts::OS` for detection, with additional variants
/// for BSD and other Unix-like systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsType {
    /// Microsoft Windows
    Windows,
    /// Linux (any distribution)
    Linux,
    /// Apple macOS
    MacOS,
    /// FreeBSD
    FreeBSD,
    /// NetBSD
    NetBSD,
    /// OpenBSD
    OpenBSD,
    /// DragonFly BSD
    DragonFly,
    /// illumos (OpenSolaris derivative)
    Illumos,
    /// Android
    Android,
    /// iOS
    Ios,
    /// Unknown operating system
    Unknown,
}

impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsType::Windows => write!(f, "Windows"),
            OsType::Linux => write!(f, "Linux"),
            OsType::MacOS => write!(f, "macOS"),
            OsType::FreeBSD => write!(f, "FreeBSD"),
            OsType::NetBSD => write!(f, "NetBSD"),
            OsType::OpenBSD => write!(f, "OpenBSD"),
            OsType::DragonFly => write!(f, "DragonFly BSD"),
            OsType::Illumos => write!(f, "illumos"),
            OsType::Android => write!(f, "Android"),
            OsType::Ios => write!(f, "iOS"),
            OsType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Linux distribution family classification.
///
/// Distributions are grouped by their package manager and base system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinuxFamily {
    /// Debian-based: Debian, Ubuntu, Mint, Pop!_OS, elementary OS
    Debian,
    /// Red Hat-based: RHEL, Fedora, CentOS, Rocky Linux, Alma Linux
    RedHat,
    /// Arch-based: Arch Linux, Manjaro, EndeavourOS, Garuda
    Arch,
    /// SUSE-based: openSUSE, SLES
    SUSE,
    /// Alpine Linux (musl-based, minimal)
    Alpine,
    /// Gentoo-based: Gentoo, Calculate Linux
    Gentoo,
    /// Void Linux (independent, runit-based)
    Void,
    /// NixOS (declarative configuration)
    NixOS,
    /// Slackware-based
    Slackware,
    /// Independent distributions that don't fit other categories
    Independent,
    /// Unknown distribution family
    Unknown,
}

impl std::fmt::Display for LinuxFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinuxFamily::Debian => write!(f, "Debian"),
            LinuxFamily::RedHat => write!(f, "Red Hat"),
            LinuxFamily::Arch => write!(f, "Arch"),
            LinuxFamily::SUSE => write!(f, "SUSE"),
            LinuxFamily::Alpine => write!(f, "Alpine"),
            LinuxFamily::Gentoo => write!(f, "Gentoo"),
            LinuxFamily::Void => write!(f, "Void"),
            LinuxFamily::NixOS => write!(f, "NixOS"),
            LinuxFamily::Slackware => write!(f, "Slackware"),
            LinuxFamily::Independent => write!(f, "Independent"),
            LinuxFamily::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Detailed information about a Linux distribution.
///
/// Parsed from `/etc/os-release` or fallback files like `/etc/lsb-release`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxDistro {
    /// Distribution ID (e.g., "ubuntu", "fedora", "arch")
    ///
    /// Lowercase identifier, suitable for programmatic matching.
    pub id: String,
    /// Pretty name (e.g., "Ubuntu 24.04.1 LTS", "Fedora Linux 40")
    ///
    /// Human-readable display name with version info.
    pub name: String,
    /// Version number (e.g., "24.04", "40")
    ///
    /// May be None for rolling release distributions.
    pub version: Option<String>,
    /// Version codename (e.g., "noble", "bookworm")
    ///
    /// May be None if the distribution doesn't use codenames.
    pub codename: Option<String>,
    /// Distribution family for package manager detection.
    pub family: LinuxFamily,
}

/// Detect the current operating system type.
///
/// Uses `std::env::consts::OS` for reliable compile-time detection,
/// mapping to the appropriate `OsType` variant.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::{detect_os_type, OsType};
///
/// let os = detect_os_type();
/// // On macOS:
/// // assert_eq!(os, OsType::MacOS);
/// ```
pub fn detect_os_type() -> OsType {
    match env::consts::OS {
        "windows" => OsType::Windows,
        "linux" => {
            // Check for Android (Linux kernel but different userland)
            if env::var("ANDROID_ROOT").is_ok() || env::var("ANDROID_DATA").is_ok() {
                OsType::Android
            } else {
                OsType::Linux
            }
        }
        "macos" => OsType::MacOS,
        "freebsd" => OsType::FreeBSD,
        "netbsd" => OsType::NetBSD,
        "openbsd" => OsType::OpenBSD,
        "dragonfly" => OsType::DragonFly,
        "illumos" | "solaris" => OsType::Illumos,
        "ios" => OsType::Ios,
        "android" => OsType::Android,
        _ => OsType::Unknown,
    }
}

/// Detect Linux distribution details.
///
/// Parses distribution information from standard files:
/// 1. `/etc/os-release` (most reliable, freedesktop.org standard)
/// 2. `/etc/lsb-release` (fallback for older systems)
/// 3. `/etc/system-release` (fallback for older RHEL/CentOS)
///
/// Returns `None` on non-Linux systems or if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::detect_linux_distro;
///
/// if let Some(distro) = detect_linux_distro() {
///     println!("Distribution: {} ({})", distro.name, distro.id);
///     if let Some(version) = &distro.version {
///         println!("Version: {}", version);
///     }
/// }
/// ```
pub fn detect_linux_distro() -> Option<LinuxDistro> {
    // Only attempt detection on Linux
    if detect_os_type() != OsType::Linux {
        return None;
    }

    // Try /etc/os-release first (most reliable)
    if let Some(distro) = parse_os_release("/etc/os-release") {
        return Some(distro);
    }

    // Fallback to /etc/lsb-release
    if let Some(distro) = parse_lsb_release("/etc/lsb-release") {
        return Some(distro);
    }

    // Fallback to /etc/system-release (older RHEL/CentOS)
    if let Some(distro) = parse_system_release("/etc/system-release") {
        return Some(distro);
    }

    None
}

/// Parse /etc/os-release file format.
///
/// This is the freedesktop.org standard format used by most modern distributions.
fn parse_os_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_os_release_content(&content)
}

/// Parse the content of an os-release formatted file.
fn parse_os_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = None;
    let mut name = None;
    let mut version = None;
    let mut version_codename = None;
    let mut pretty_name = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            // Remove surrounding quotes if present
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "ID" => id = Some(value.to_lowercase()),
                "NAME" => name = Some(value.to_string()),
                "VERSION_ID" => version = Some(value.to_string()),
                "VERSION_CODENAME" => version_codename = Some(value.to_string()),
                "PRETTY_NAME" => pretty_name = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let id = id?;
    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id: id.clone(),
        name: pretty_name.or(name).unwrap_or_else(|| id.clone()),
        version,
        codename: version_codename,
        family,
    })
}

/// Parse /etc/lsb-release file format.
///
/// This is a legacy format still used by some distributions.
fn parse_lsb_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_lsb_release_content(&content)
}

/// Parse the content of an lsb-release formatted file.
fn parse_lsb_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = None;
    let mut description = None;
    let mut release = None;
    let mut codename = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "DISTRIB_ID" => id = Some(value.to_lowercase()),
                "DISTRIB_DESCRIPTION" => description = Some(value.to_string()),
                "DISTRIB_RELEASE" => release = Some(value.to_string()),
                "DISTRIB_CODENAME" => codename = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let id = id?;
    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id: id.clone(),
        name: description.unwrap_or_else(|| id.clone()),
        version: release,
        codename,
        family,
    })
}

/// Parse /etc/system-release file format.
///
/// Used by older RHEL/CentOS systems that don't have os-release.
fn parse_system_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_system_release_content(&content)
}

/// Parse the content of a system-release file.
fn parse_system_release_content(content: &str) -> Option<LinuxDistro> {
    let line = content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }

    // Format is typically: "CentOS Linux release 7.9.2009 (Core)"
    // or "Red Hat Enterprise Linux Server release 7.9 (Maipo)"
    let lower = line.to_lowercase();

    let id = if lower.contains("centos") {
        "centos"
    } else if lower.contains("red hat") || lower.contains("rhel") {
        "rhel"
    } else if lower.contains("fedora") {
        "fedora"
    } else if lower.contains("oracle") {
        "ol"
    } else {
        return None;
    };

    // Try to extract version number
    let version = extract_version_from_release(line);

    // Try to extract codename (usually in parentheses)
    let codename = line
        .rfind('(')
        .and_then(|start| line.rfind(')').map(|end| &line[start + 1..end]))
        .map(|s| s.to_string());

    Some(LinuxDistro {
        id: id.to_string(),
        name: line.to_string(),
        version,
        codename,
        family: LinuxFamily::RedHat,
    })
}

/// Extract version number from a release string.
fn extract_version_from_release(text: &str) -> Option<String> {
    // Look for patterns like "release 7.9" or "7.9.2009"
    let mut chars = text.chars().peekable();
    let mut version = String::new();
    let mut found_digit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            found_digit = true;
            version.push(c);
            // Continue collecting digits and dots
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' {
                    version.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            // Only return if we found something substantial
            if !version.is_empty() {
                // Trim trailing dots
                return Some(version.trim_end_matches('.').to_string());
            }
        }
    }

    if found_digit && !version.is_empty() {
        Some(version.trim_end_matches('.').to_string())
    } else {
        None
    }
}

/// Infer the Linux distribution family from its ID.
///
/// Maps distribution IDs to their family for package manager detection
/// and system-specific behavior.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::{infer_linux_family, LinuxFamily};
///
/// assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
/// assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
/// assert_eq!(infer_linux_family("arch"), LinuxFamily::Arch);
/// assert_eq!(infer_linux_family("alpine"), LinuxFamily::Alpine);
/// ```
pub fn infer_linux_family(id: &str) -> LinuxFamily {
    let id_lower = id.to_lowercase();

    // Debian family (apt/dpkg)
    if matches!(
        id_lower.as_str(),
        "debian"
            | "ubuntu"
            | "linuxmint"
            | "mint"
            | "pop"
            | "pop_os"
            | "elementary"
            | "elementaryos"
            | "zorin"
            | "zorinos"
            | "kali"
            | "parrot"
            | "raspbian"
            | "pureos"
            | "deepin"
            | "mx"
            | "mxlinux"
            | "lmde"
            | "bunsenlabs"
            | "antix"
            | "sparky"
            | "devuan"
            | "tails"
    ) {
        return LinuxFamily::Debian;
    }

    // Red Hat family (dnf/yum/rpm)
    if matches!(
        id_lower.as_str(),
        "fedora"
            | "rhel"
            | "centos"
            | "rocky"
            | "rockylinux"
            | "almalinux"
            | "alma"
            | "ol"
            | "oracle"
            | "oraclelinux"
            | "scientific"
            | "springdale"
            | "clearos"
            | "amazon"
            | "amzn"
            | "mageia"
            | "openmandriva"
            | "nobara"
    ) {
        return LinuxFamily::RedHat;
    }

    // Arch family (pacman)
    if matches!(
        id_lower.as_str(),
        "arch"
            | "archlinux"
            | "manjaro"
            | "endeavouros"
            | "endeavour"
            | "garuda"
            | "garudalinux"
            | "artix"
            | "arcolinux"
            | "blackarch"
            | "archcraft"
            | "rebornos"
            | "bluestar"
            | "cachyos"
    ) {
        return LinuxFamily::Arch;
    }

    // SUSE family (zypper)
    if matches!(
        id_lower.as_str(),
        "opensuse"
            | "opensuse-leap"
            | "opensuse-tumbleweed"
            | "suse"
            | "sles"
            | "sled"
            | "opensuse-microos"
            | "gecko"
    ) {
        return LinuxFamily::SUSE;
    }

    // Alpine (apk)
    if id_lower == "alpine" {
        return LinuxFamily::Alpine;
    }

    // Gentoo family (emerge/portage)
    if matches!(
        id_lower.as_str(),
        "gentoo" | "calculate" | "funtoo" | "sabayon" | "redcore"
    ) {
        return LinuxFamily::Gentoo;
    }

    // Void Linux (xbps)
    if id_lower == "void" || id_lower == "voidlinux" {
        return LinuxFamily::Void;
    }

    // NixOS (nix)
    if id_lower == "nixos" {
        return LinuxFamily::NixOS;
    }

    // Slackware family
    if matches!(
        id_lower.as_str(),
        "slackware" | "salix" | "slackel" | "zenwalk" | "porteus"
    ) {
        return LinuxFamily::Slackware;
    }

    // Independent distributions
    if matches!(
        id_lower.as_str(),
        "solus" | "clear-linux-os" | "clearlinux" | "guix" | "chimera" | "kiss"
    ) {
        return LinuxFamily::Independent;
    }

    LinuxFamily::Unknown
}

/// Check if the process is running in a CI environment.
///
/// Detects common CI/CD platforms by checking their environment variables.
///
/// ## Supported CI Platforms
///
/// - GitHub Actions (`GITHUB_ACTIONS`)
/// - GitLab CI (`GITLAB_CI`)
/// - Travis CI (`TRAVIS`)
/// - CircleCI (`CIRCLECI`)
/// - Jenkins (`JENKINS_URL`)
/// - Azure Pipelines (`TF_BUILD`)
/// - Buildkite (`BUILDKITE`)
/// - Drone CI (`DRONE`)
/// - AppVeyor (`APPVEYOR`)
/// - Bitbucket Pipelines (`BITBUCKET_COMMIT`)
/// - Sourcehut (`SRHT_BUILD_URL`)
/// - TeamCity (`TEAMCITY_VERSION`)
/// - AWS CodeBuild (`CODEBUILD_BUILD_ID`)
/// - Generic CI (`CI`)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::is_ci;
///
/// if is_ci() {
///     println!("Running in CI - disabling interactive features");
/// }
/// ```
pub fn is_ci() -> bool {
    // Generic CI indicator (used by many platforms)
    if env::var("CI").is_ok() {
        return true;
    }

    // GitHub Actions
    if env::var("GITHUB_ACTIONS").is_ok() {
        return true;
    }

    // GitLab CI
    if env::var("GITLAB_CI").is_ok() {
        return true;
    }

    // Travis CI
    if env::var("TRAVIS").is_ok() {
        return true;
    }

    // CircleCI
    if env::var("CIRCLECI").is_ok() {
        return true;
    }

    // Jenkins
    if env::var("JENKINS_URL").is_ok() {
        return true;
    }

    // Azure Pipelines
    if env::var("TF_BUILD").is_ok() {
        return true;
    }

    // Buildkite
    if env::var("BUILDKITE").is_ok() {
        return true;
    }

    // Drone CI
    if env::var("DRONE").is_ok() {
        return true;
    }

    // AppVeyor
    if env::var("APPVEYOR").is_ok() {
        return true;
    }

    // Bitbucket Pipelines
    if env::var("BITBUCKET_COMMIT").is_ok() {
        return true;
    }

    // Sourcehut
    if env::var("SRHT_BUILD_URL").is_ok() {
        return true;
    }

    // TeamCity
    if env::var("TEAMCITY_VERSION").is_ok() {
        return true;
    }

    // AWS CodeBuild
    if env::var("CODEBUILD_BUILD_ID").is_ok() {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_os_type_returns_valid_variant() {
        let os = detect_os_type();
        // Should not be Unknown on macOS/Linux/Windows
        #[cfg(target_os = "macos")]
        assert_eq!(os, OsType::MacOS);
        #[cfg(target_os = "linux")]
        assert_eq!(os, OsType::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(os, OsType::Windows);
    }

    #[test]
    fn test_os_type_display() {
        assert_eq!(OsType::MacOS.to_string(), "macOS");
        assert_eq!(OsType::Linux.to_string(), "Linux");
        assert_eq!(OsType::Windows.to_string(), "Windows");
        assert_eq!(OsType::FreeBSD.to_string(), "FreeBSD");
    }

    #[test]
    fn test_linux_family_display() {
        assert_eq!(LinuxFamily::Debian.to_string(), "Debian");
        assert_eq!(LinuxFamily::RedHat.to_string(), "Red Hat");
        assert_eq!(LinuxFamily::Arch.to_string(), "Arch");
    }

    #[test]
    fn test_infer_linux_family_debian() {
        assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("debian"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("linuxmint"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("pop"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("elementary"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("kali"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("raspbian"), LinuxFamily::Debian);
    }

    #[test]
    fn test_infer_linux_family_redhat() {
        assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rhel"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("centos"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rocky"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("almalinux"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("ol"), LinuxFamily::RedHat);
    }

    #[test]
    fn test_infer_linux_family_arch() {
        assert_eq!(infer_linux_family("arch"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("manjaro"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("endeavouros"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("garuda"), LinuxFamily::Arch);
    }

    #[test]
    fn test_infer_linux_family_suse() {
        assert_eq!(infer_linux_family("opensuse"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-leap"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-tumbleweed"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("sles"), LinuxFamily::SUSE);
    }

    #[test]
    fn test_infer_linux_family_others() {
        assert_eq!(infer_linux_family("alpine"), LinuxFamily::Alpine);
        assert_eq!(infer_linux_family("gentoo"), LinuxFamily::Gentoo);
        assert_eq!(infer_linux_family("void"), LinuxFamily::Void);
        assert_eq!(infer_linux_family("nixos"), LinuxFamily::NixOS);
        assert_eq!(infer_linux_family("slackware"), LinuxFamily::Slackware);
        assert_eq!(infer_linux_family("solus"), LinuxFamily::Independent);
    }

    #[test]
    fn test_infer_linux_family_unknown() {
        assert_eq!(infer_linux_family("unknown_distro"), LinuxFamily::Unknown);
        assert_eq!(infer_linux_family(""), LinuxFamily::Unknown);
        assert_eq!(infer_linux_family("myowndistro"), LinuxFamily::Unknown);
    }

    #[test]
    fn test_infer_linux_family_case_insensitive() {
        assert_eq!(infer_linux_family("Ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("FEDORA"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("ARCH"), LinuxFamily::Arch);
    }

    #[test]
    fn test_is_ci_returns_bool() {
        // Just verify it returns a bool without panicking
        let _ = is_ci();
    }

    #[test]
    fn test_parse_os_release_content_ubuntu() {
        let content = r#"
NAME="Ubuntu"
VERSION="24.04.1 LTS (Noble Numbat)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 24.04.1 LTS"
VERSION_ID="24.04"
VERSION_CODENAME=noble
HOME_URL="https://www.ubuntu.com/"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
        assert_eq!(distro.version, Some("24.04".to_string()));
        assert_eq!(distro.codename, Some("noble".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_os_release_content_fedora() {
        let content = r#"
NAME="Fedora Linux"
VERSION="40 (Workstation Edition)"
ID=fedora
VERSION_ID=40
PRETTY_NAME="Fedora Linux 40 (Workstation Edition)"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "fedora");
        assert_eq!(distro.name, "Fedora Linux 40 (Workstation Edition)");
        assert_eq!(distro.version, Some("40".to_string()));
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
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "arch");
        assert_eq!(distro.name, "Arch Linux");
        assert_eq!(distro.version, None); // Rolling release
        assert_eq!(distro.family, LinuxFamily::Arch);
    }

    #[test]
    fn test_parse_os_release_content_alpine() {
        let content = r#"
NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.19.1
PRETTY_NAME="Alpine Linux v3.19"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "alpine");
        assert_eq!(distro.version, Some("3.19.1".to_string()));
        assert_eq!(distro.family, LinuxFamily::Alpine);
    }

    #[test]
    fn test_parse_os_release_content_missing_id() {
        let content = r#"
NAME="Some Distro"
VERSION="1.0"
"#;
        assert!(parse_os_release_content(content).is_none());
    }

    #[test]
    fn test_parse_lsb_release_content() {
        let content = r#"
DISTRIB_ID=Ubuntu
DISTRIB_RELEASE=22.04
DISTRIB_CODENAME=jammy
DISTRIB_DESCRIPTION="Ubuntu 22.04.3 LTS"
"#;
        let distro = parse_lsb_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.version, Some("22.04".to_string()));
        assert_eq!(distro.codename, Some("jammy".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_system_release_content_centos() {
        let content = "CentOS Linux release 7.9.2009 (Core)";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "centos");
        assert_eq!(distro.version, Some("7.9.2009".to_string()));
        assert_eq!(distro.codename, Some("Core".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_system_release_content_rhel() {
        let content = "Red Hat Enterprise Linux Server release 7.9 (Maipo)";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "rhel");
        assert_eq!(distro.version, Some("7.9".to_string()));
        assert_eq!(distro.codename, Some("Maipo".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_extract_version_from_release() {
        assert_eq!(
            extract_version_from_release("release 7.9.2009"),
            Some("7.9.2009".to_string())
        );
        assert_eq!(
            extract_version_from_release("version 24.04"),
            Some("24.04".to_string())
        );
        assert_eq!(
            extract_version_from_release("no version here"),
            None
        );
    }

    #[test]
    fn test_detect_linux_distro_on_non_linux() {
        // On non-Linux systems, this should return None
        #[cfg(not(target_os = "linux"))]
        assert!(detect_linux_distro().is_none());
    }

    // === Edge case tests ===

    #[test]
    fn test_parse_os_release_content_empty_file() {
        let content = "";
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "Empty file should return None");
    }

    #[test]
    fn test_parse_os_release_content_malformed() {
        let content = "NOT_A_VALID_KEY";
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "Malformed content without = should return None");
    }

    #[test]
    fn test_parse_os_release_content_only_comments() {
        let content = r#"
# This is a comment
# Another comment
# ID=fake
"#;
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "File with only comments should return None");
    }

    #[test]
    fn test_parse_os_release_content_whitespace_only() {
        let content = "   \n\t\n   \n";
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "Whitespace-only file should return None");
    }

    #[test]
    fn test_parse_os_release_content_quoted_values() {
        let content = r#"ID="ubuntu"
NAME="Ubuntu 24.04.1 LTS"
VERSION_ID="24.04"
VERSION_CODENAME="noble"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
        assert_eq!(distro.version, Some("24.04".to_string()));
        assert_eq!(distro.codename, Some("noble".to_string()));
    }

    #[test]
    fn test_parse_os_release_content_single_quoted_values() {
        let content = r#"ID='ubuntu'
NAME='Ubuntu 24.04.1 LTS'
VERSION_ID='24.04'
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
    }

    #[test]
    fn test_parse_os_release_content_mixed_quotes() {
        let content = r#"ID=ubuntu
NAME="Ubuntu 24.04.1 LTS"
VERSION_ID='24.04'
PRETTY_NAME="Ubuntu 24.04.1 LTS"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.version, Some("24.04".to_string()));
    }

    #[test]
    fn test_parse_os_release_content_special_characters() {
        let content = r#"ID=pop
NAME="Pop!_OS"
PRETTY_NAME="Pop!_OS 22.04 LTS"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "pop");
        assert_eq!(distro.name, "Pop!_OS 22.04 LTS");
    }

    #[test]
    fn test_parse_lsb_release_content_empty() {
        let content = "";
        assert!(parse_lsb_release_content(content).is_none());
    }

    #[test]
    fn test_parse_lsb_release_content_missing_id() {
        let content = r#"
DISTRIB_DESCRIPTION="Ubuntu 22.04"
DISTRIB_RELEASE=22.04
"#;
        assert!(parse_lsb_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_empty() {
        let content = "";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_whitespace_only() {
        let content = "   \n";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_unknown_distro() {
        let content = "Some Unknown Distro release 1.0";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_oracle() {
        let content = "Oracle Linux Server release 8.6";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "ol");
        assert_eq!(distro.version, Some("8.6".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_extract_version_from_release_edge_cases() {
        // Version at start
        assert_eq!(
            extract_version_from_release("7.9 release"),
            Some("7.9".to_string())
        );

        // Version with many dots
        assert_eq!(
            extract_version_from_release("version 1.2.3.4.5"),
            Some("1.2.3.4.5".to_string())
        );

        // Trailing dot should be trimmed
        assert_eq!(
            extract_version_from_release("release 7.9."),
            Some("7.9".to_string())
        );

        // Single digit version
        assert_eq!(
            extract_version_from_release("version 8"),
            Some("8".to_string())
        );
    }

    #[test]
    fn test_all_os_type_variants_display() {
        // Ensure all OsType variants have Display implementations
        let variants = [
            OsType::Windows,
            OsType::Linux,
            OsType::MacOS,
            OsType::FreeBSD,
            OsType::NetBSD,
            OsType::OpenBSD,
            OsType::DragonFly,
            OsType::Illumos,
            OsType::Android,
            OsType::Ios,
            OsType::Unknown,
        ];

        for variant in variants {
            let display = format!("{}", variant);
            assert!(!display.is_empty(), "{:?} should have non-empty display", variant);
        }
    }

    #[test]
    fn test_all_linux_family_variants_display() {
        // Ensure all LinuxFamily variants have Display implementations
        let variants = [
            LinuxFamily::Debian,
            LinuxFamily::RedHat,
            LinuxFamily::Arch,
            LinuxFamily::SUSE,
            LinuxFamily::Alpine,
            LinuxFamily::Gentoo,
            LinuxFamily::Void,
            LinuxFamily::NixOS,
            LinuxFamily::Slackware,
            LinuxFamily::Independent,
            LinuxFamily::Unknown,
        ];

        for variant in variants {
            let display = format!("{}", variant);
            assert!(!display.is_empty(), "{:?} should have non-empty display", variant);
        }
    }

    #[test]
    fn test_infer_linux_family_comprehensive() {
        // Test all documented debian family distros
        let debian_ids = [
            "debian", "ubuntu", "linuxmint", "mint", "pop", "pop_os",
            "elementary", "elementaryos", "zorin", "zorinos", "kali",
            "parrot", "raspbian", "pureos", "deepin", "mx", "mxlinux",
            "lmde", "bunsenlabs", "antix", "sparky", "devuan", "tails"
        ];
        for id in debian_ids {
            assert_eq!(infer_linux_family(id), LinuxFamily::Debian, "Failed for {}", id);
        }

        // Test all documented redhat family distros
        let redhat_ids = [
            "fedora", "rhel", "centos", "rocky", "rockylinux", "almalinux",
            "alma", "ol", "oracle", "oraclelinux", "scientific", "springdale",
            "clearos", "amazon", "amzn", "mageia", "openmandriva", "nobara"
        ];
        for id in redhat_ids {
            assert_eq!(infer_linux_family(id), LinuxFamily::RedHat, "Failed for {}", id);
        }

        // Test arch family
        let arch_ids = [
            "arch", "archlinux", "manjaro", "endeavouros", "endeavour",
            "garuda", "garudalinux", "artix", "arcolinux", "blackarch",
            "archcraft", "rebornos", "bluestar", "cachyos"
        ];
        for id in arch_ids {
            assert_eq!(infer_linux_family(id), LinuxFamily::Arch, "Failed for {}", id);
        }

        // Test SUSE family
        let suse_ids = [
            "opensuse", "opensuse-leap", "opensuse-tumbleweed", "suse",
            "sles", "sled", "opensuse-microos", "gecko"
        ];
        for id in suse_ids {
            assert_eq!(infer_linux_family(id), LinuxFamily::SUSE, "Failed for {}", id);
        }
    }
}
