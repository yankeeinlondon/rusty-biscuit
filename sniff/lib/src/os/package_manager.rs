//! System package manager detection.
//!
//! This module provides functionality for detecting installed system package
//! managers across Linux, macOS, Windows, and BSD operating systems.

use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::path::{Path, PathBuf};

use super::distro::LinuxFamily;
use super::OsType;

// ============================================================================
// Package Manager Detection Infrastructure
// ============================================================================

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
pub(crate) fn determine_linux_primary(
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
    use std::fs;

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
