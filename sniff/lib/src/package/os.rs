//! Operating system-level package manager types.
//!
//! This module provides the [`OsPackageManager`] enum representing system-level
//! package managers across different operating systems and distributions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Operating system-level package manager identification.
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
pub enum OsPackageManager {
    // ===== Debian family =====
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

    // ===== RedHat family =====
    /// DNF (Dandified YUM) - Fedora/RHEL 8+ package manager
    Dnf,
    /// YUM (Yellowdog Updater Modified) - Legacy RHEL/CentOS package manager
    Yum,
    /// microdnf - Minimal DNF for containers
    Microdnf,
    /// RPM - Low-level RedHat package manager
    Rpm,

    // ===== Arch family =====
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

    // ===== SUSE =====
    /// zypper - SUSE/openSUSE package manager
    Zypper,

    // ===== Gentoo =====
    /// Portage/emerge - Gentoo source-based package manager
    Portage,

    // ===== Alpine =====
    /// apk - Alpine Linux package manager
    Apk,

    // ===== Void =====
    /// xbps - X Binary Package System for Void Linux
    Xbps,

    // ===== Slackware =====
    /// pkgtool - Slackware package manager
    Pkgtool,

    // ===== Cross-distro =====
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

    // ===== macOS =====
    /// Homebrew - macOS/Linux community package manager
    Homebrew,
    /// MacPorts - macOS package manager (formerly DarwinPorts)
    MacPorts,
    /// Fink - Debian-based macOS package manager
    Fink,
    /// softwareupdate - macOS system update tool
    Softwareupdate,

    // ===== Windows =====
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

    // ===== BSD =====
    /// pkg - FreeBSD package manager
    Pkg,
    /// ports - BSD ports collection
    Ports,
    /// pkg_add - OpenBSD package manager
    PkgAdd,
    /// pkgin - NetBSD binary package manager
    Pkgin,
}

impl OsPackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::OsPackageManager;
    ///
    /// assert_eq!(OsPackageManager::Apt.executable_name(), "apt");
    /// assert_eq!(OsPackageManager::Homebrew.executable_name(), "brew");
    /// assert_eq!(OsPackageManager::Portage.executable_name(), "emerge");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            // Debian family
            OsPackageManager::Apt => "apt",
            OsPackageManager::Aptitude => "aptitude",
            OsPackageManager::Dpkg => "dpkg",
            OsPackageManager::Nala => "nala",
            OsPackageManager::AptFast => "apt-fast",
            // RedHat family
            OsPackageManager::Dnf => "dnf",
            OsPackageManager::Yum => "yum",
            OsPackageManager::Microdnf => "microdnf",
            OsPackageManager::Rpm => "rpm",
            // Arch family
            OsPackageManager::Pacman | OsPackageManager::Msys2Pacman => "pacman",
            OsPackageManager::Makepkg => "makepkg",
            OsPackageManager::Yay => "yay",
            OsPackageManager::Paru => "paru",
            OsPackageManager::Pamac => "pamac",
            // SUSE
            OsPackageManager::Zypper => "zypper",
            // Gentoo
            OsPackageManager::Portage => "emerge",
            // Alpine
            OsPackageManager::Apk => "apk",
            // Void
            OsPackageManager::Xbps => "xbps-install",
            // Slackware
            OsPackageManager::Pkgtool => "pkgtool",
            // Cross-distro
            OsPackageManager::Snap => "snap",
            OsPackageManager::Flatpak => "flatpak",
            OsPackageManager::Guix => "guix",
            OsPackageManager::Nix => "nix",
            OsPackageManager::NixEnv => "nix-env",
            // macOS
            OsPackageManager::Homebrew => "brew",
            OsPackageManager::MacPorts => "port",
            OsPackageManager::Fink => "fink",
            OsPackageManager::Softwareupdate => "softwareupdate",
            // Windows
            OsPackageManager::Winget => "winget",
            OsPackageManager::Dism => "dism",
            OsPackageManager::Chocolatey => "choco",
            OsPackageManager::Scoop => "scoop",
            // BSD
            OsPackageManager::Pkg => "pkg",
            OsPackageManager::Ports => "make",
            OsPackageManager::PkgAdd => "pkg_add",
            OsPackageManager::Pkgin => "pkgin",
        }
    }
}

impl fmt::Display for OsPackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.executable_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_package_manager_executable_names() {
        assert_eq!(OsPackageManager::Apt.executable_name(), "apt");
        assert_eq!(OsPackageManager::Homebrew.executable_name(), "brew");
        assert_eq!(OsPackageManager::Pacman.executable_name(), "pacman");
        assert_eq!(OsPackageManager::Portage.executable_name(), "emerge");
        assert_eq!(OsPackageManager::Winget.executable_name(), "winget");
    }

    #[test]
    fn test_os_package_manager_display() {
        assert_eq!(format!("{}", OsPackageManager::Apt), "apt");
        assert_eq!(format!("{}", OsPackageManager::Homebrew), "brew");
        assert_eq!(format!("{}", OsPackageManager::Chocolatey), "choco");
    }

    #[test]
    fn test_serde_roundtrip_os() {
        let mgr = OsPackageManager::Homebrew;
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: OsPackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }
}
