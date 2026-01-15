//! Linux distribution detection and classification.
//!
//! This module provides functionality for detecting and classifying Linux
//! distributions based on system files like `/etc/os-release`, `/etc/lsb-release`,
//! and `/etc/system-release`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::detect_os_type;
use super::OsType;

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
pub(crate) fn detect_linux_distro_from_paths(
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

#[cfg(test)]
mod tests {
    use super::*;

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
            let mut file = std::fs::File::create(&os_release_path).expect("should create file");
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
            let mut file = std::fs::File::create(&lsb_release_path).expect("should create file");
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
            let mut file =
                std::fs::File::create(&system_release_path).expect("should create file");
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
}
