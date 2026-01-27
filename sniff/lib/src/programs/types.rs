//! Type definitions for program detection and installation.
//!
//! This module provides:
//! - `ExecutableSource`: Describes where a program executable was discovered
//! - `ProgramDetails`: Metadata about a program including installation methods
//! - `ProgramDetector`: Trait for structs that detect and manage installed programs
//! - `InstallationMethod`: Enum describing how to install a program

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{error::SniffInstallationError, os::OsType, programs::schema::ProgramMetadata};

/// Describes where a program executable was discovered.
///
/// This enum distinguishes between traditional PATH-based executables and
/// macOS application bundles, enabling appropriate invocation strategies.
///
/// ## Variants
///
/// - `Path` - Found in system PATH (traditional executable)
/// - `MacOsAppBundle` - Found as a macOS `.app` bundle
///
/// ## Examples
///
/// ```
/// use sniff_lib::programs::ExecutableSource;
///
/// let source = ExecutableSource::Path;
/// assert!(!source.is_app_bundle());
///
/// let bundle = ExecutableSource::MacOsAppBundle;
/// assert!(bundle.is_app_bundle());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    /// Found via PATH lookup (traditional executable).
    Path,
    /// Found as a macOS `.app` bundle.
    MacOsAppBundle,
}

impl ExecutableSource {
    /// Returns true if this source is a macOS app bundle.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::programs::ExecutableSource;
    ///
    /// assert!(!ExecutableSource::Path.is_app_bundle());
    /// assert!(ExecutableSource::MacOsAppBundle.is_app_bundle());
    /// ```
    #[must_use]
    pub fn is_app_bundle(&self) -> bool {
        matches!(self, Self::MacOsAppBundle)
    }
}

impl std::fmt::Display for ExecutableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutableSource::Path => write!(f, "PATH"),
            ExecutableSource::MacOsAppBundle => write!(f, "macOS App Bundle"),
        }
    }
}

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

    /// Returns the path and source of the specified program's binary if installed.
    ///
    /// This extends `path()` by also reporting how the executable was discovered:
    /// - `ExecutableSource::Path` for traditional PATH-based executables
    /// - `ExecutableSource::MacOsAppBundle` for macOS app bundles
    ///
    /// The default implementation wraps `path()` and assumes `ExecutableSource::Path`.
    /// Implementors can override this to provide more accurate source information.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use sniff_lib::programs::{ProgramDetector, InstalledEditors, Editor, ExecutableSource};
    ///
    /// let editors = InstalledEditors::new();
    /// if let Some((path, source)) = editors.path_with_source(Editor::Vscode) {
    ///     match source {
    ///         ExecutableSource::Path => println!("Found in PATH: {}", path.display()),
    ///         ExecutableSource::MacOsAppBundle => println!("Found as macOS app: {}", path.display()),
    ///     }
    /// }
    /// ```
    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        self.path(program).map(|p| (p, ExecutableSource::Path))
    }

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
    use crate::programs::schema::{ProgramError, ProgramInfo, ProgramMetadata};

    // ============================================
    // Mock implementation for testing ProgramDetector trait
    // ============================================

    static MOCK_INSTALLED_INFO: ProgramInfo = ProgramInfo::standard(
        "mock-installed",
        "Mock Installed",
        "A mock installed program",
        "https://example.com/installed",
    );

    static MOCK_NOT_INSTALLED_INFO: ProgramInfo = ProgramInfo::standard(
        "mock-not-installed",
        "Mock Not Installed",
        "A mock not-installed program",
        "https://example.com/not-installed",
    );

    /// Mock program enum for testing the ProgramDetector trait.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockProgram {
        Installed,
        NotInstalled,
    }

    impl ProgramMetadata for MockProgram {
        fn info(&self) -> &'static ProgramInfo {
            match self {
                MockProgram::Installed => &MOCK_INSTALLED_INFO,
                MockProgram::NotInstalled => &MOCK_NOT_INSTALLED_INFO,
            }
        }
    }

    /// Mock detector that implements ProgramDetector for testing default methods.
    struct MockDetector;

    impl ProgramDetector for MockDetector {
        type Program = MockProgram;

        fn refresh(&mut self) {}

        fn is_installed(&self, program: Self::Program) -> bool {
            matches!(program, MockProgram::Installed)
        }

        fn path(&self, program: Self::Program) -> Option<PathBuf> {
            match program {
                MockProgram::Installed => Some(PathBuf::from("/usr/bin/mock-installed")),
                MockProgram::NotInstalled => None,
            }
        }

        fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
            match program {
                MockProgram::Installed => Ok("1.0.0".to_string()),
                MockProgram::NotInstalled => {
                    Err(ProgramError::NotFound("mock-not-installed".to_string()))
                }
            }
        }

        fn website(&self, program: Self::Program) -> &'static str {
            program.info().website
        }

        fn description(&self, program: Self::Program) -> &'static str {
            program.info().description
        }

        fn installed(&self) -> Vec<Self::Program> {
            vec![MockProgram::Installed]
        }

        fn installable(&self, _program: Self::Program) -> bool {
            false
        }

        fn install(&self, _program: Self::Program) -> Result<(), SniffInstallationError> {
            Err(SniffInstallationError::NotInstallableOnOs {
                pkg: "mock".to_string(),
                os: "mock".to_string(),
            })
        }

        fn install_version(
            &self,
            _program: Self::Program,
            _version: &str,
        ) -> Result<(), SniffInstallationError> {
            Err(SniffInstallationError::NotInstallableOnOs {
                pkg: "mock".to_string(),
                os: "mock".to_string(),
            })
        }
    }

    // ============================================
    // ProgramDetector::path_with_source tests
    // ============================================

    #[test]
    fn test_path_with_source_default_returns_path_source_when_installed() {
        let detector = MockDetector;
        let result = detector.path_with_source(MockProgram::Installed);

        assert!(result.is_some());
        let (path, source) = result.unwrap();
        assert_eq!(path, PathBuf::from("/usr/bin/mock-installed"));
        assert_eq!(source, ExecutableSource::Path);
    }

    #[test]
    fn test_path_with_source_default_returns_none_when_not_installed() {
        let detector = MockDetector;
        let result = detector.path_with_source(MockProgram::NotInstalled);

        assert!(result.is_none());
    }

    // ============================================
    // ExecutableSource tests
    // ============================================

    #[test]
    fn test_executable_source_is_app_bundle() {
        assert!(!ExecutableSource::Path.is_app_bundle());
        assert!(ExecutableSource::MacOsAppBundle.is_app_bundle());
    }

    #[test]
    fn test_executable_source_display() {
        assert_eq!(ExecutableSource::Path.to_string(), "PATH");
        assert_eq!(ExecutableSource::MacOsAppBundle.to_string(), "macOS App Bundle");
    }

    #[test]
    fn test_executable_source_debug() {
        assert_eq!(format!("{:?}", ExecutableSource::Path), "Path");
        assert_eq!(format!("{:?}", ExecutableSource::MacOsAppBundle), "MacOsAppBundle");
    }

    #[test]
    fn test_executable_source_clone_and_copy() {
        let source = ExecutableSource::Path;
        let cloned = source.clone();
        let copied = source; // Copy
        assert_eq!(source, cloned);
        assert_eq!(source, copied);
    }

    #[test]
    fn test_executable_source_equality() {
        assert_eq!(ExecutableSource::Path, ExecutableSource::Path);
        assert_eq!(ExecutableSource::MacOsAppBundle, ExecutableSource::MacOsAppBundle);
        assert_ne!(ExecutableSource::Path, ExecutableSource::MacOsAppBundle);
    }

    #[test]
    fn test_executable_source_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ExecutableSource::Path);
        set.insert(ExecutableSource::MacOsAppBundle);
        set.insert(ExecutableSource::Path); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ExecutableSource::Path));
        assert!(set.contains(&ExecutableSource::MacOsAppBundle));
    }

    #[test]
    fn test_executable_source_serialize_json() {
        let path = ExecutableSource::Path;
        let bundle = ExecutableSource::MacOsAppBundle;

        let path_json = serde_json::to_string(&path).unwrap();
        let bundle_json = serde_json::to_string(&bundle).unwrap();

        assert_eq!(path_json, "\"path\"");
        assert_eq!(bundle_json, "\"mac_os_app_bundle\"");
    }

    #[test]
    fn test_executable_source_deserialize_json() {
        let path: ExecutableSource = serde_json::from_str("\"path\"").unwrap();
        let bundle: ExecutableSource = serde_json::from_str("\"mac_os_app_bundle\"").unwrap();

        assert_eq!(path, ExecutableSource::Path);
        assert_eq!(bundle, ExecutableSource::MacOsAppBundle);
    }

    #[test]
    fn test_executable_source_roundtrip() {
        for source in [ExecutableSource::Path, ExecutableSource::MacOsAppBundle] {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: ExecutableSource = serde_json::from_str(&json).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    // ============================================
    // InstallationMethod tests
    // ============================================

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

    // ============================================
    // InstallationMethod comprehensive tests
    // ============================================

    #[test]
    fn test_installation_method_is_remote_bash() {
        assert!(InstallationMethod::RemoteBash("https://example.com/install.sh").is_remote_bash());
        assert!(!InstallationMethod::Brew("ripgrep").is_remote_bash());
        assert!(!InstallationMethod::Cargo("bat").is_remote_bash());
    }

    #[test]
    fn test_installation_method_all_language_managers() {
        let methods = [
            InstallationMethod::Npm("pkg"),
            InstallationMethod::Pnpm("pkg"),
            InstallationMethod::Yarn("pkg"),
            InstallationMethod::Bun("pkg"),
            InstallationMethod::Cargo("pkg"),
            InstallationMethod::GoModules("pkg"),
            InstallationMethod::Composer("pkg"),
            InstallationMethod::SwiftPm("pkg"),
            InstallationMethod::LuaRocks("pkg"),
            InstallationMethod::VcPkg("pkg"),
            InstallationMethod::Conan("pkg"),
            InstallationMethod::Nuget("pkg"),
            InstallationMethod::Hex("pkg"),
            InstallationMethod::Pip("pkg"),
            InstallationMethod::Uv("pkg"),
            InstallationMethod::Poetry("pkg"),
            InstallationMethod::Cpan("pkg"),
            InstallationMethod::Cpanm("pkg"),
        ];

        for method in &methods {
            assert!(!method.is_os_package_manager(), "{:?} should not be OS pkg mgr", method);
            assert!(!method.is_remote_bash(), "{:?} should not be remote bash", method);
            assert_eq!(method.package_name(), "pkg");
        }
    }

    #[test]
    fn test_installation_method_all_os_managers() {
        let methods = [
            InstallationMethod::Apt("pkg"),
            InstallationMethod::Nala("pkg"),
            InstallationMethod::Brew("pkg"),
            InstallationMethod::Dnf("pkg"),
            InstallationMethod::Pacman("pkg"),
            InstallationMethod::Winget("pkg"),
            InstallationMethod::Chocolatey("pkg"),
            InstallationMethod::Scoop("pkg"),
            InstallationMethod::Nix("pkg"),
        ];

        for method in &methods {
            assert!(method.is_os_package_manager(), "{:?} should be OS pkg mgr", method);
            assert!(!method.is_remote_bash(), "{:?} should not be remote bash", method);
            assert_eq!(method.package_name(), "pkg");
        }
    }

    #[test]
    fn test_installation_method_manager_name_all_variants() {
        // Test all manager names are non-empty strings
        let all_methods = [
            InstallationMethod::Npm("x"),
            InstallationMethod::Pnpm("x"),
            InstallationMethod::Yarn("x"),
            InstallationMethod::Bun("x"),
            InstallationMethod::Cargo("x"),
            InstallationMethod::GoModules("x"),
            InstallationMethod::Composer("x"),
            InstallationMethod::SwiftPm("x"),
            InstallationMethod::LuaRocks("x"),
            InstallationMethod::VcPkg("x"),
            InstallationMethod::Conan("x"),
            InstallationMethod::Nuget("x"),
            InstallationMethod::Hex("x"),
            InstallationMethod::Pip("x"),
            InstallationMethod::Uv("x"),
            InstallationMethod::Poetry("x"),
            InstallationMethod::Cpan("x"),
            InstallationMethod::Cpanm("x"),
            InstallationMethod::Apt("x"),
            InstallationMethod::Nala("x"),
            InstallationMethod::Brew("x"),
            InstallationMethod::Dnf("x"),
            InstallationMethod::Pacman("x"),
            InstallationMethod::Winget("x"),
            InstallationMethod::Chocolatey("x"),
            InstallationMethod::Scoop("x"),
            InstallationMethod::Nix("x"),
            InstallationMethod::RemoteBash("x"),
        ];

        for method in &all_methods {
            let name = method.manager_name();
            assert!(!name.is_empty(), "{:?} should have non-empty manager name", method);
        }
    }

    // ============================================
    // ProgramDetails edge case tests
    // ============================================

    #[test]
    fn test_program_details_default_os_availability() {
        let details = ProgramDetails::new("test", "Test program", "https://example.com");
        // Default should include all three major OS types
        assert!(details.os_availability.contains(&OsType::MacOS));
        assert!(details.os_availability.contains(&OsType::Linux));
        assert!(details.os_availability.contains(&OsType::Windows));
    }

    #[test]
    fn test_program_details_empty_installation_methods() {
        let details = ProgramDetails::new("test", "Test program", "https://example.com");
        assert!(details.installation_methods.is_empty());
    }

    // ============================================
    // ExecutableSource additional tests
    // ============================================

    #[test]
    fn test_executable_source_default_is_not_app_bundle() {
        // Verify that Path (the most common case) is not an app bundle
        let source = ExecutableSource::Path;
        assert!(!source.is_app_bundle());
    }

    #[test]
    fn test_executable_source_pattern_matching() {
        // Test that match exhaustiveness is enforced by the compiler
        fn describe_source(source: ExecutableSource) -> &'static str {
            match source {
                ExecutableSource::Path => "path",
                ExecutableSource::MacOsAppBundle => "bundle",
            }
        }

        assert_eq!(describe_source(ExecutableSource::Path), "path");
        assert_eq!(describe_source(ExecutableSource::MacOsAppBundle), "bundle");
    }

    #[test]
    fn test_executable_source_deserialize_invalid_json() {
        // Invalid JSON value should fail to deserialize
        let result: Result<ExecutableSource, _> = serde_json::from_str("\"invalid_source\"");
        assert!(result.is_err(), "Invalid source should fail to deserialize");
    }
}
