//! Type definitions for program detection and installation.
//!
//! This module provides:
//! - `ProgramDetails`: Metadata about a program including installation methods
//! - `ProgramDetector`: Trait for structs that detect and manage installed programs
//! - `InstallationMethod`: Enum describing how to install a program

use std::path::PathBuf;

use crate::{error::SniffInstallationError, os::OsType, programs::schema::ProgramMetadata};

/// Describes an installation method for installing some piece of software.
///
/// This installation takes two broad forms:
///
/// 1. Using a package manager (OS level _or_ Language specific)
/// 2. Downloading a bash script and executing it locally
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallationMethod {
    // Language Package Managers
    /// Default Node.js package manager. [Website](https://www.npmjs.com)
    Npm(&'static str),
    /// Disk-efficient Node.js package manager. [Website](https://pnpm.io)
    Pnpm(&'static str),
    /// Alternative Node.js package manager. [Website](https://yarnpkg.com)
    Yarn(&'static str),
    /// All-in-one JS runtime with built-in package manager. [Website](https://bun.sh)
    Bun(&'static str),
    /// Official Rust package manager and build tool. [Website](https://doc.rust-lang.org/cargo)
    Cargo(&'static str),
    /// Built-in Go dependency system. [Website](https://go.dev/ref/mod)
    GoModules(&'static str),
    /// Dependency manager for modern PHP applications. [Website](https://getcomposer.org)
    Composer(&'static str),
    /// Official Swift dependency manager. [Website](https://www.swift.org/package-manager)
    SwiftPm(&'static str),
    /// Standard package manager for Lua modules. [Website](https://luarocks.org)
    LuaRocks(&'static str),
    /// Cross-platform C/C++ dependency manager. [Website](https://vcpkg.io)
    VcPkg(&'static str),
    /// Decentralized C/C++ package manager. [Website](https://conan.io)
    Conan(&'static str),
    /// Official package manager for .NET and C#. [Website](https://www.nuget.org)
    Nuget(&'static str),
    /// Package manager for the BEAM ecosystem. [Website](https://hex.pm)
    Hex(&'static str),
    /// Traditional Python package installer. [Website](https://pip.pypa.io)
    Pip(&'static str),
    /// High-performance Python package manager. [Website](https://astral.sh/uv)
    Uv(&'static str),
    /// Python dependency manager with lockfile support. [Website](https://python-poetry.org)
    Poetry(&'static str),
    /// Canonical archive and installer for Perl modules. [Website](https://www.cpan.org)
    Cpan(&'static str),
    /// Lightweight, scriptable CPAN client. [Website](https://metacpan.org/pod/App::cpanminus)
    Cpanm(&'static str),

    // OS Package Managers
    /// Debian/Ubuntu primary package manager. [Website](https://tracker.debian.org/pkg/apt)
    Apt(&'static str),
    /// Modern apt frontend with parallel downloads. [Website](https://github.com/volitank/nala)
    Nala(&'static str),
    /// macOS/Linux community package manager. [Website](https://brew.sh)
    Brew(&'static str),
    /// Fedora/RHEL primary package manager. [Website](https://github.com/rpm-software-management/dnf)
    Dnf(&'static str),
    /// Arch Linux package manager. [Website](https://archlinux.org/pacman/)
    Pacman(&'static str),
    /// Windows Package Manager. [Website](https://github.com/microsoft/winget-cli)
    Winget(&'static str),
    /// Windows community package manager. [Website](https://chocolatey.org)
    Chocolatey(&'static str),
    /// Windows command-line installer. [Website](https://scoop.sh)
    Scoop(&'static str),
    /// Nix package manager. [Website](https://nixos.org)
    Nix(&'static str),

    /// Install by downloading a bash script from a URL and then
    /// piping it to the host's `bash` command for installation.
    RemoteBash(&'static str),
}

impl InstallationMethod {
    /// Returns the package name for this installation method.
    pub fn package_name(&self) -> &'static str {
        match self {
            // Language package managers
            InstallationMethod::Npm(pkg) => pkg,
            InstallationMethod::Pnpm(pkg) => pkg,
            InstallationMethod::Yarn(pkg) => pkg,
            InstallationMethod::Bun(pkg) => pkg,
            InstallationMethod::Cargo(pkg) => pkg,
            InstallationMethod::GoModules(pkg) => pkg,
            InstallationMethod::Composer(pkg) => pkg,
            InstallationMethod::SwiftPm(pkg) => pkg,
            InstallationMethod::LuaRocks(pkg) => pkg,
            InstallationMethod::VcPkg(pkg) => pkg,
            InstallationMethod::Conan(pkg) => pkg,
            InstallationMethod::Nuget(pkg) => pkg,
            InstallationMethod::Hex(pkg) => pkg,
            InstallationMethod::Pip(pkg) => pkg,
            InstallationMethod::Uv(pkg) => pkg,
            InstallationMethod::Poetry(pkg) => pkg,
            InstallationMethod::Cpan(pkg) => pkg,
            InstallationMethod::Cpanm(pkg) => pkg,
            // OS package managers
            InstallationMethod::Apt(pkg) => pkg,
            InstallationMethod::Nala(pkg) => pkg,
            InstallationMethod::Brew(pkg) => pkg,
            InstallationMethod::Dnf(pkg) => pkg,
            InstallationMethod::Pacman(pkg) => pkg,
            InstallationMethod::Winget(pkg) => pkg,
            InstallationMethod::Chocolatey(pkg) => pkg,
            InstallationMethod::Scoop(pkg) => pkg,
            InstallationMethod::Nix(pkg) => pkg,
            // Remote bash
            InstallationMethod::RemoteBash(url) => url,
        }
    }

    /// Returns the package manager name for this installation method.
    pub fn manager_name(&self) -> &'static str {
        match self {
            InstallationMethod::Npm(_) => "npm",
            InstallationMethod::Pnpm(_) => "pnpm",
            InstallationMethod::Yarn(_) => "yarn",
            InstallationMethod::Bun(_) => "bun",
            InstallationMethod::Cargo(_) => "cargo",
            InstallationMethod::GoModules(_) => "go",
            InstallationMethod::Composer(_) => "composer",
            InstallationMethod::SwiftPm(_) => "swift",
            InstallationMethod::LuaRocks(_) => "luarocks",
            InstallationMethod::VcPkg(_) => "vcpkg",
            InstallationMethod::Conan(_) => "conan",
            InstallationMethod::Nuget(_) => "nuget",
            InstallationMethod::Hex(_) => "mix",
            InstallationMethod::Pip(_) => "pip",
            InstallationMethod::Uv(_) => "uv",
            InstallationMethod::Poetry(_) => "poetry",
            InstallationMethod::Cpan(_) => "cpan",
            InstallationMethod::Cpanm(_) => "cpanm",
            InstallationMethod::Apt(_) => "apt",
            InstallationMethod::Nala(_) => "nala",
            InstallationMethod::Brew(_) => "brew",
            InstallationMethod::Dnf(_) => "dnf",
            InstallationMethod::Pacman(_) => "pacman",
            InstallationMethod::Winget(_) => "winget",
            InstallationMethod::Chocolatey(_) => "choco",
            InstallationMethod::Scoop(_) => "scoop",
            InstallationMethod::Nix(_) => "nix",
            InstallationMethod::RemoteBash(_) => "bash",
        }
    }

    /// Returns true if this is an OS-level package manager.
    pub fn is_os_package_manager(&self) -> bool {
        matches!(
            self,
            InstallationMethod::Apt(_)
                | InstallationMethod::Nala(_)
                | InstallationMethod::Brew(_)
                | InstallationMethod::Dnf(_)
                | InstallationMethod::Pacman(_)
                | InstallationMethod::Winget(_)
                | InstallationMethod::Chocolatey(_)
                | InstallationMethod::Scoop(_)
                | InstallationMethod::Nix(_)
        )
    }

    /// Returns true if this is a remote bash installation.
    pub fn is_remote_bash(&self) -> bool {
        matches!(self, InstallationMethod::RemoteBash(_))
    }
}

/// Details about a program including installation methods.
///
/// This struct provides the metadata for a program to support detection
/// and installation via the `ProgramDetector` trait.
///
/// ## Notes
///
/// All fields use `'static` lifetime references to allow embedding in
/// static arrays without allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramDetails {
    /// The name of the software.
    pub name: &'static str,
    /// A description of the software.
    pub description: &'static str,
    /// The operating systems this software can run on.
    pub os_availability: &'static [OsType],
    /// The primary website describing the program.
    pub website: &'static str,
    /// The repo for the program (if available).
    pub repo: Option<&'static str>,
    /// Describes various methods for installing this software.
    pub installation_methods: &'static [InstallationMethod],
}

impl ProgramDetails {
    /// Creates a new `ProgramDetails` with required fields.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        website: &'static str,
    ) -> Self {
        Self {
            name,
            description,
            os_availability: &[OsType::MacOS, OsType::Linux, OsType::Windows],
            website,
            repo: None,
            installation_methods: &[],
        }
    }

    /// Creates a `ProgramDetails` with full configuration.
    pub const fn full(
        name: &'static str,
        description: &'static str,
        os_availability: &'static [OsType],
        website: &'static str,
        repo: Option<&'static str>,
        installation_methods: &'static [InstallationMethod],
    ) -> Self {
        Self {
            name,
            description,
            os_availability,
            website,
            repo,
            installation_methods,
        }
    }
}

/// Trait for structs that detect and manage programs of a specific category.
///
/// Implementors track installation status for a set of related programs
/// (e.g., editors, utilities, TTS clients) and provide methods to query
/// metadata, check installation status, and install programs.
///
/// ## Associated Type
///
/// The `Program` associated type specifies the enum type representing
/// the programs in this category. It must implement `ProgramMetadata`
/// for metadata access and `Copy` for efficient parameter passing.
///
/// ## Examples
///
/// ```ignore
/// use sniff_lib::programs::{ProgramDetector, InstalledEditors, Editor};
///
/// let editors = InstalledEditors::new();
/// if editors.is_installed(Editor::Vim) {
///     println!("Vim is installed at {:?}", editors.path(Editor::Vim));
/// }
/// ```
pub trait ProgramDetector {
    /// The enum type representing programs in this category.
    type Program: ProgramMetadata + Copy;

    /// Re-check program availability and update internal state.
    fn refresh(&mut self);

    /// Returns true if the specified program is installed.
    fn is_installed(&self, program: Self::Program) -> bool;

    /// Returns the path to the specified program's binary if installed.
    fn path(&self, program: Self::Program) -> Option<PathBuf>;

    /// Returns the version of the specified program if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The program is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    fn version(&self, program: Self::Program) -> Result<String, crate::programs::ProgramError>;

    /// Returns the official website URL for the specified program.
    fn website(&self, program: Self::Program) -> &'static str;

    /// Returns a one-line description of the specified program.
    fn description(&self, program: Self::Program) -> &'static str;

    /// Returns the description formatted for terminal display.
    ///
    /// The description uses OSC8 hyperlinks for clickable URLs and
    /// ANSI escape codes for styling (bold/blue for the program name).
    ///
    /// ## Notes
    ///
    /// Assumes the terminal supports OSC8 and ANSI color codes.
    fn description_for_terminal(&self, program: Self::Program) -> String {
        let info = program.info();
        let name = info.display_name;
        let url = info.website;
        let desc = info.description;

        // OSC8 hyperlink: \x1b]8;;URL\x07TEXT\x1b]8;;\x07
        // Bold blue: \x1b[1;34mTEXT\x1b[0m
        format!(
            "\x1b]8;;{url}\x07\x1b[1;34m{name}\x1b[0m\x1b]8;;\x07 - {desc}",
            url = url,
            name = name,
            desc = desc
        )
    }

    /// Returns a list of all installed programs in this category.
    fn installed(&self) -> Vec<Self::Program>;

    /// Returns true if the specified program can be installed on this system.
    ///
    /// Checks:
    /// - OS compatibility
    /// - Available package managers
    /// - Defined installation methods
    fn installable(&self, program: Self::Program) -> bool;

    /// Attempts to install the program onto the host.
    ///
    /// ## Notes
    ///
    /// - Language package managers install globally (to PATH)
    /// - OS package managers are preferred over language package managers
    /// - Package managers are preferred over bash script installations
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - No installation method is available for this OS
    /// - The required package manager is not installed
    /// - The installation command fails
    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError>;

    /// Attempts to install a specific version of the program.
    ///
    /// ## Notes
    ///
    /// Remote bash installations do NOT support versioned installs and will
    /// return an error.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - Version-specific installation is not supported
    /// - The installation fails
    fn install_version(
        &self,
        program: Self::Program,
        version: &str,
    ) -> Result<(), SniffInstallationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_method_package_name() {
        assert_eq!(InstallationMethod::Brew("ripgrep").package_name(), "ripgrep");
        assert_eq!(InstallationMethod::Cargo("bat").package_name(), "bat");
        assert_eq!(
            InstallationMethod::RemoteBash("https://example.com/install.sh").package_name(),
            "https://example.com/install.sh"
        );
    }

    #[test]
    fn test_installation_method_manager_name() {
        assert_eq!(InstallationMethod::Brew("ripgrep").manager_name(), "brew");
        assert_eq!(InstallationMethod::Cargo("bat").manager_name(), "cargo");
        assert_eq!(InstallationMethod::Npm("typescript").manager_name(), "npm");
    }

    #[test]
    fn test_installation_method_is_os_package_manager() {
        assert!(InstallationMethod::Brew("ripgrep").is_os_package_manager());
        assert!(InstallationMethod::Apt("ripgrep").is_os_package_manager());
        assert!(!InstallationMethod::Cargo("bat").is_os_package_manager());
        assert!(!InstallationMethod::Npm("typescript").is_os_package_manager());
    }

    #[test]
    fn test_program_details_new() {
        let details = ProgramDetails::new("ripgrep", "Fast grep", "https://github.com/BurntSushi/ripgrep");
        assert_eq!(details.name, "ripgrep");
        assert_eq!(details.description, "Fast grep");
        assert_eq!(details.website, "https://github.com/BurntSushi/ripgrep");
        assert!(details.repo.is_none());
        assert!(details.installation_methods.is_empty());
    }

    #[test]
    fn test_program_details_full() {
        static METHODS: &[InstallationMethod] = &[
            InstallationMethod::Brew("ripgrep"),
            InstallationMethod::Cargo("ripgrep"),
        ];

        let details = ProgramDetails::full(
            "ripgrep",
            "Fast grep",
            &[OsType::MacOS, OsType::Linux],
            "https://github.com/BurntSushi/ripgrep",
            Some("https://github.com/BurntSushi/ripgrep"),
            METHODS,
        );

        assert_eq!(details.name, "ripgrep");
        assert_eq!(details.os_availability.len(), 2);
        assert_eq!(details.installation_methods.len(), 2);
        assert!(details.repo.is_some());
    }
}
