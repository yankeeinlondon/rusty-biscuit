use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_all_os_type_variants_display() {
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
            assert!(
                !display.is_empty(),
                "{:?} should have non-empty display",
                variant
            );
        }
    }

    #[test]
    fn test_all_linux_family_variants_display() {
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
            assert!(
                !display.is_empty(),
                "{:?} should have non-empty display",
                variant
            );
        }
    }
}
