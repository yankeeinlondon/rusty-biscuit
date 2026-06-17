//! Type definitions for program detection.
//!
//! This module defines the `ProgramDetector` trait used by all category
//! detectors. The concrete generic detector lives in
//! [`crate::programs::category_detector`]; the contract types
//! (`ExecutableSource`, `InstallationMethod`, `SystemPrerequisite`,
//! `PrereqProbe`, `ProgramError`) live in [`crate::programs::contract`].

use std::path::PathBuf;

use crate::error::SniffInstallationError;
use crate::programs::contract::{ExecutableSource, InstallationMethod};
use crate::programs::schema::ProgramMetadata;

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
/// use sniff::programs::{ProgramDetector, InstalledEditors, Editor};
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
    /// use sniff::programs::{ProgramDetector, InstalledEditors, Editor, ExecutableSource};
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

    /// Returns every installation method the program declares, ignoring host
    /// constraints. This is the static metadata.
    fn known_methods(&self, program: Self::Program) -> &'static [InstallationMethod] {
        program.info().installation_methods
    }

    /// Returns the subset of known methods whose required package manager is
    /// actually installed on this host and whose program is permitted on the
    /// current OS.
    fn available_methods(&self, program: Self::Program) -> Vec<InstallationMethod> {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::install::method_available;

        let info = program.info();
        let host = HostCapabilities::load_or_detect();

        let os_ok = info.os_availability.is_empty() || info.os_availability.contains(&host.os_type);
        if !os_ok {
            return Vec::new();
        }

        info.installation_methods
            .iter()
            .filter(|m| method_available(m, &host))
            .cloned()
            .collect()
    }

    /// Returns a full install plan for this program against cached host
    /// capabilities.
    fn install_plan(&self, program: Self::Program) -> crate::programs::install::InstallPlan {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::install::build_install_plan;

        let host = HostCapabilities::load_or_detect();
        build_install_plan(&program, &host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::contract::ProgramError;
    use crate::programs::schema::{ProgramInfo, ProgramMetadata};

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
        assert_eq!(
            ExecutableSource::MacOsAppBundle.to_string(),
            "macOS App Bundle"
        );
    }

    #[test]
    fn test_executable_source_debug() {
        assert_eq!(format!("{:?}", ExecutableSource::Path), "Path");
        assert_eq!(
            format!("{:?}", ExecutableSource::MacOsAppBundle),
            "MacOsAppBundle"
        );
    }

    #[test]
    fn test_executable_source_clone_and_copy() {
        let source = ExecutableSource::Path;
        let cloned = source;
        let copied = source; // Copy
        assert_eq!(source, cloned);
        assert_eq!(source, copied);
    }

    #[test]
    fn test_executable_source_equality() {
        assert_eq!(ExecutableSource::Path, ExecutableSource::Path);
        assert_eq!(
            ExecutableSource::MacOsAppBundle,
            ExecutableSource::MacOsAppBundle
        );
        assert_ne!(ExecutableSource::Path, ExecutableSource::MacOsAppBundle);
    }

    #[test]
    fn test_executable_source_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ExecutableSource::Path);
        set.insert(ExecutableSource::MacOsAppBundle);
        set.insert(ExecutableSource::WindowsAppPaths);
        set.insert(ExecutableSource::WindowsInstallRoot);
        set.insert(ExecutableSource::Path); // Duplicate

        assert_eq!(set.len(), 4);
        assert!(set.contains(&ExecutableSource::Path));
        assert!(set.contains(&ExecutableSource::MacOsAppBundle));
        assert!(set.contains(&ExecutableSource::WindowsAppPaths));
        assert!(set.contains(&ExecutableSource::WindowsInstallRoot));
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
        for source in [
            ExecutableSource::Path,
            ExecutableSource::MacOsAppBundle,
            ExecutableSource::WindowsAppPaths,
            ExecutableSource::WindowsInstallRoot,
        ] {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: ExecutableSource = serde_json::from_str(&json).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    #[test]
    fn test_executable_source_windows_variants_serialize() {
        let app_paths = ExecutableSource::WindowsAppPaths;
        let install_root = ExecutableSource::WindowsInstallRoot;

        assert_eq!(
            serde_json::to_string(&app_paths).unwrap(),
            "\"windows_app_paths\""
        );
        assert_eq!(
            serde_json::to_string(&install_root).unwrap(),
            "\"windows_install_root\""
        );
    }

    #[test]
    fn test_executable_source_windows_variants_deserialize() {
        let ap: ExecutableSource = serde_json::from_str("\"windows_app_paths\"").unwrap();
        let ir: ExecutableSource = serde_json::from_str("\"windows_install_root\"").unwrap();

        assert_eq!(ap, ExecutableSource::WindowsAppPaths);
        assert_eq!(ir, ExecutableSource::WindowsInstallRoot);
    }

    #[test]
    fn test_executable_source_windows_variants_display() {
        assert_eq!(
            ExecutableSource::WindowsAppPaths.to_string(),
            "Windows App Paths"
        );
        assert_eq!(
            ExecutableSource::WindowsInstallRoot.to_string(),
            "Windows Install Root"
        );
    }

    #[test]
    fn test_executable_source_windows_variants_not_app_bundle() {
        assert!(!ExecutableSource::WindowsAppPaths.is_app_bundle());
        assert!(!ExecutableSource::WindowsInstallRoot.is_app_bundle());
    }

    #[test]
    fn test_executable_source_is_fallback() {
        assert!(!ExecutableSource::Path.is_fallback());
        assert!(ExecutableSource::MacOsAppBundle.is_fallback());
        assert!(ExecutableSource::WindowsAppPaths.is_fallback());
        assert!(ExecutableSource::WindowsInstallRoot.is_fallback());
    }

    // ============================================
    // InstallationMethod tests
    // ============================================

    #[test]
    fn test_installation_method_package_name() {
        assert_eq!(
            InstallationMethod::Brew("ripgrep").package_name(),
            "ripgrep"
        );
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
    fn test_installation_method_manager_binary() {
        assert_eq!(InstallationMethod::Brew("vim").manager_binary(), "brew");
        assert_eq!(InstallationMethod::Apt("vim").manager_binary(), "apt");
        assert_eq!(
            InstallationMethod::Cargo("ripgrep").manager_binary(),
            "cargo"
        );
        assert_eq!(
            InstallationMethod::Npm("typescript").manager_binary(),
            "npm"
        );
        assert_eq!(
            InstallationMethod::RemoteBash("url").manager_binary(),
            "bash"
        );
        assert_eq!(
            InstallationMethod::Chocolatey("vim").manager_binary(),
            "choco"
        );
        assert_eq!(
            InstallationMethod::Hex("hex_package").manager_binary(),
            "mix"
        );
    }

    #[test]
    fn test_installation_method_is_os_package_manager() {
        assert!(InstallationMethod::Brew("ripgrep").is_os_package_manager());
        assert!(InstallationMethod::Apt("ripgrep").is_os_package_manager());
        assert!(!InstallationMethod::Cargo("bat").is_os_package_manager());
        assert!(!InstallationMethod::Npm("typescript").is_os_package_manager());
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
            assert!(
                !method.is_os_package_manager(),
                "{:?} should not be OS pkg mgr",
                method
            );
            assert!(
                !method.is_remote_bash(),
                "{:?} should not be remote bash",
                method
            );
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
            assert!(
                method.is_os_package_manager(),
                "{:?} should be OS pkg mgr",
                method
            );
            assert!(
                !method.is_remote_bash(),
                "{:?} should not be remote bash",
                method
            );
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
            InstallationMethod::UvWithInstall("x"),
        ];

        for method in &all_methods {
            let name = method.manager_name();
            assert!(
                !name.is_empty(),
                "{:?} should have non-empty manager name",
                method
            );
        }
    }

    #[test]
    fn test_uv_with_install_package_name_and_manager() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        assert_eq!(method.package_name(), "aider-chat");
        assert_eq!(method.manager_name(), "uv");
        assert_eq!(method.manager_binary(), "uv");
        assert!(!method.is_os_package_manager());
        assert!(!method.is_remote_bash());
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
        fn describe_source(source: ExecutableSource) -> &'static str {
            match source {
                ExecutableSource::Path => "path",
                ExecutableSource::MacOsAppBundle => "bundle",
                ExecutableSource::WindowsAppPaths => "app_paths",
                ExecutableSource::WindowsInstallRoot => "install_root",
                ExecutableSource::ProjectLocal => "project_local",
            }
        }

        assert_eq!(describe_source(ExecutableSource::Path), "path");
        assert_eq!(describe_source(ExecutableSource::MacOsAppBundle), "bundle");
        assert_eq!(
            describe_source(ExecutableSource::WindowsAppPaths),
            "app_paths"
        );
        assert_eq!(
            describe_source(ExecutableSource::WindowsInstallRoot),
            "install_root"
        );
        assert_eq!(
            describe_source(ExecutableSource::ProjectLocal),
            "project_local"
        );
    }

    #[test]
    fn test_executable_source_deserialize_invalid_json() {
        // Invalid JSON value should fail to deserialize
        let result: Result<ExecutableSource, _> = serde_json::from_str("\"invalid_source\"");
        assert!(result.is_err(), "Invalid source should fail to deserialize");
    }

    // ============================================
    // InstallationMethod Serialize tests
    // ============================================

    #[test]
    fn test_installation_method_serializes_with_manager_target_shape() {
        let method = InstallationMethod::Brew("ripgrep");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#"{"manager":"brew","target":"ripgrep"}"#);
    }

    #[test]
    fn test_installation_method_serializes_remote_bash_as_tagged_shape() {
        let method = InstallationMethod::RemoteBash("https://sh.rustup.rs");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(
            json,
            r#"{"manager":"remote_bash","target":"https://sh.rustup.rs"}"#
        );
    }

    #[test]
    fn test_installation_method_serializes_cargo_as_tagged_shape() {
        let method = InstallationMethod::Cargo("bat");
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#"{"manager":"cargo","target":"bat"}"#);
    }
}
