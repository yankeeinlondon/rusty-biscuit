//! Type definitions for program detection and installation.
//!
//! This module provides:
//! - `ExecutableSource`: Describes where a program executable was discovered
//! - `ProgramDetector`: Trait for structs that detect and manage installed programs
//! - `InstallationMethod`: Enum describing how to install a program
//! - `CategoryDetector<E>`: Generic detector for any category enum

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::programs::contract::{CategoryEnum, ExecutableSource, InstallationMethod, ProgramError};
use crate::executable_index::{ExecutableIndex, find_programs_with_source_from_index};
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::schema::ProgramMetadata;


/// Generic program detector for any category enum.
///
/// Stores detection results (path + source) indexed by enum variant ordinal.
/// Replaces the per-category `InstalledEditors`, `InstalledUtilities`, etc.
/// structs with a single generic implementation.
#[derive(Debug, Clone)]
pub struct CategoryDetector<E: CategoryEnum + ProgramMetadata> {
    results: Vec<Option<(PathBuf, ExecutableSource)>>,
    _phantom: PhantomData<E>,
}

impl<E: CategoryEnum + ProgramMetadata> Default for CategoryDetector<E> {
    fn default() -> Self {
        Self {
            results: vec![None; E::COUNT],
            _phantom: PhantomData,
        }
    }
}

impl<E: CategoryEnum + ProgramMetadata> PartialEq for CategoryDetector<E> {
    fn eq(&self, other: &Self) -> bool {
        self.results == other.results
    }
}

impl<E: CategoryEnum + ProgramMetadata> Eq for CategoryDetector<E> {}

impl<E: CategoryEnum + ProgramMetadata> Serialize for CategoryDetector<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(E::COUNT))?;
        for variant in E::iter() {
            let key = variant.serde_key();
            let info = variant.info();
            let entry = match self.path_with_source(variant) {
                Some((path, source)) => {
                    crate::programs::schema::ProgramEntry::installed(info, path, source)
                }
                None => crate::programs::schema::ProgramEntry::not_installed(info),
            };
            map.serialize_entry(key, &entry)?;
        }
        map.end()
    }
}

/// Helper for deserializing both boolean and ProgramEntry values.
#[derive(Deserialize)]
#[serde(untagged)]
enum BoolOrEntry {
    Bool(bool),
    Entry { installed: bool },
}

impl<'de, E: CategoryEnum + ProgramMetadata> Deserialize<'de> for CategoryDetector<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CategoryDetectorVisitor::<E>(PhantomData))
    }
}

struct CategoryDetectorVisitor<E>(PhantomData<E>);

impl<'de, E: CategoryEnum + ProgramMetadata> serde::de::Visitor<'de> for CategoryDetectorVisitor<E> {
    type Value = CategoryDetector<E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a map of program names to booleans or program entries"
        )
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        // Build key -> variant index lookup
        let key_to_index: HashMap<&'static str, usize> = E::iter()
            .map(|v| (v.serde_key(), v.variant_index()))
            .collect();

        let mut results = vec![None; E::COUNT];

        while let Some(key) = map.next_key::<String>()? {
            if let Some(&idx) = key_to_index.get(key.as_str()) {
                let value: BoolOrEntry = map.next_value()?;
                let installed = match value {
                    BoolOrEntry::Bool(b) => b,
                    BoolOrEntry::Entry { installed } => installed,
                };
                if installed {
                    results[idx] = Some((PathBuf::new(), ExecutableSource::Path));
                }
            } else {
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }

        Ok(CategoryDetector {
            results,
            _phantom: PhantomData,
        })
    }
}

/// Collect every binary name (primary + alternates) for a category enum,
/// preserving first-seen order while removing duplicates.
///
/// Categories share aliases (e.g. `node`, `npm`), and a duplicate name causes
/// the underlying `which`/`PATH` lookup to fire twice. Deduplicating up front
/// keeps bulk detection one-PATH-walk-per-program.
fn collect_unique_names<E: CategoryEnum + ProgramMetadata>() -> Vec<&'static str> {
    let mut seen: HashSet<&'static str> = HashSet::new();
    let mut names: Vec<&'static str> = Vec::new();
    for variant in E::iter() {
        let info = variant.info();
        if seen.insert(info.binary_name) {
            names.push(info.binary_name);
        }
        for alt in info.alternate_binary_names {
            if seen.insert(*alt) {
                names.push(*alt);
            }
        }
    }
    names
}

impl<E: CategoryEnum + ProgramMetadata> CategoryDetector<E> {
    /// Detect installed programs by scanning PATH.
    pub fn new() -> Self {
        let names_to_search = collect_unique_names::<E>();
        let found = find_programs_with_source_parallel(&names_to_search);
        Self::from_search_results(&found)
    }

    /// Detect installed programs using a pre-built executable index.
    pub fn new_with_index(index: &ExecutableIndex) -> Self {
        let names_to_search = collect_unique_names::<E>();
        let found = find_programs_with_source_from_index(index, &names_to_search);
        Self::from_search_results(&found)
    }

    /// Construct from search results HashMap.
    fn from_search_results(found: &HashMap<String, Option<(PathBuf, ExecutableSource)>>) -> Self {
        let mut results = vec![None; E::COUNT];

        for variant in E::iter() {
            let idx = variant.variant_index();

            // Check platform override first (e.g., Windows SAPI)
            if let Some(override_result) = variant.platform_override() {
                results[idx] = Some(override_result);
                continue;
            }

            // Try primary binary name
            let info = variant.info();
            if let Some(result) = found.get(info.binary_name).and_then(|r| r.clone()) {
                results[idx] = Some(result);
                continue;
            }

            // Try alternate binary names
            for alt in info.alternate_binary_names {
                if let Some(result) = found.get(*alt).and_then(|r| r.clone()) {
                    results[idx] = Some(result);
                    break;
                }
            }
        }

        Self {
            results,
            _phantom: PhantomData,
        }
    }

    /// Re-check program availability and update internal state.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns true if the specified program is installed.
    pub fn is_installed(&self, program: E) -> bool {
        self.results[program.variant_index()].is_some()
    }

    /// Returns the path to the specified program's binary if installed.
    pub fn path(&self, program: E) -> Option<PathBuf> {
        self.results[program.variant_index()]
            .as_ref()
            .map(|(p, _)| p.clone())
    }

    /// Returns the path and source of the specified program if installed.
    pub fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        self.results[program.variant_index()].clone()
    }

    /// Returns the version of the specified program if available.
    ///
    /// Uses the executable path discovered during detection so version probing
    /// does not re-scan PATH.
    ///
    /// ## Errors
    ///
    /// Returns an error if the program is not installed or version detection fails.
    pub fn version(&self, program: E) -> Result<String, ProgramError> {
        let (path, _) = self
            .path_with_source(program)
            .ok_or_else(|| ProgramError::NotFound(program.binary_name().to_string()))?;
        program.version_from_path(&path)
    }

    /// Returns the official website URL for the specified program.
    pub fn website(&self, program: E) -> &'static str {
        program.website()
    }

    /// Returns a one-line description of the specified program.
    pub fn description(&self, program: E) -> &'static str {
        program.description()
    }

    /// Returns a list of all installed programs in this category.
    pub fn installed(&self) -> Vec<E> {
        E::iter().filter(|p| self.is_installed(*p)).collect()
    }

    /// Builder method: mark a program as installed with given path and source.
    ///
    /// Useful for testing.
    pub fn with_program(mut self, program: E, path: PathBuf, source: ExecutableSource) -> Self {
        self.results[program.variant_index()] = Some((path, source));
        self
    }
}

impl<E: CategoryEnum + ProgramMetadata> ProgramDetector for CategoryDetector<E> {
    type Program = E;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: E) -> bool {
        CategoryDetector::is_installed(self, program)
    }

    fn path(&self, program: E) -> Option<PathBuf> {
        CategoryDetector::path(self, program)
    }

    fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        CategoryDetector::path_with_source(self, program)
    }

    fn version(&self, program: E) -> Result<String, crate::programs::ProgramError> {
        CategoryDetector::version(self, program)
    }

    fn website(&self, program: E) -> &'static str {
        CategoryDetector::website(self, program)
    }

    fn description(&self, program: E) -> &'static str {
        CategoryDetector::description(self, program)
    }

    fn installed(&self) -> Vec<E> {
        CategoryDetector::installed(self)
    }

    fn installable(&self, program: E) -> bool {
        self.install_plan(program).successful
    }

    fn install(&self, program: E) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        if !plan.successful {
            return Err(SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            });
        }
        let _ = plan.execute(&crate::programs::installer::InstallOptions::default())?;
        Ok(())
    }

    fn install_version(&self, program: E, version: &str) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        let chosen = plan
            .chosen()
            .ok_or_else(|| SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            })?;

        if matches!(
            chosen.kind,
            InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
        ) {
            let url = match &chosen.kind {
                InstallationMethod::RemoteBash(u) => (*u).to_string(),
                InstallationMethod::UvWithInstall(_) => {
                    crate::programs::installer::astral_installer_url().to_string()
                }
                _ => unreachable!(),
            };
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: program.display_name().to_string(),
                url,
            });
        }

        let _ = crate::programs::installer::execute_versioned_install(
            &chosen.kind,
            version,
            &crate::programs::installer::InstallOptions::default(),
        )?;
        Ok(())
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
        use crate::programs::installer::method_available;

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
    fn install_plan(&self, program: Self::Program) -> crate::programs::install_plan::InstallPlan {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::install_plan::build_install_plan;

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
    }

    #[test]
    fn test_executable_source_deserialize_invalid_json() {
        // Invalid JSON value should fail to deserialize
        let result: Result<ExecutableSource, _> = serde_json::from_str("\"invalid_source\"");
        assert!(result.is_err(), "Invalid source should fail to deserialize");
    }

    // ============================================
    // CategoryDetector tests
    // ============================================

    use crate::programs::enums::Editor;

    #[test]
    fn test_category_detector_default_has_nothing_installed() {
        let detector = CategoryDetector::<Editor>::default();
        assert!(detector.installed().is_empty());
        assert!(!detector.is_installed(Editor::Vim));
        assert!(detector.path(Editor::Vim).is_none());
        assert!(detector.path_with_source(Editor::Vim).is_none());
    }

    #[test]
    fn test_category_detector_with_program_marks_installed() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::Neovim));
        assert_eq!(
            detector.path(Editor::Vim),
            Some(PathBuf::from("/usr/bin/vim"))
        );
    }

    #[test]
    fn test_category_detector_installed_returns_only_installed() {
        let detector = CategoryDetector::<Editor>::default()
            .with_program(
                Editor::Vim,
                PathBuf::from("/usr/bin/vim"),
                ExecutableSource::Path,
            )
            .with_program(
                Editor::Neovim,
                PathBuf::from("/usr/bin/nvim"),
                ExecutableSource::Path,
            );
        let installed = detector.installed();
        assert_eq!(installed.len(), 2);
        assert!(installed.contains(&Editor::Vim));
        assert!(installed.contains(&Editor::Neovim));
    }

    // ============================================
    // CategoryDetector Serialization/Deserialization tests
    // ============================================

    #[test]
    fn test_category_detector_serialize_produces_program_entries() {
        let detector = CategoryDetector::<Editor>::default();
        let json = serde_json::to_string(&detector).unwrap();
        // Should produce ProgramEntry objects with full metadata
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"name\":\"Vim\""));
    }

    #[test]
    fn test_category_detector_deserialize_from_booleans() {
        let json = r#"{"vim": true, "vscode": false}"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_category_detector_deserialize_partial_json() {
        let json = r#"{"vim": true}"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::Neovim));
    }

    #[test]
    fn test_category_detector_serialize_includes_path_and_source() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );
        let json = serde_json::to_string(&detector).unwrap();
        // Vim should be installed
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"installed\":true"));
        assert!(json.contains("\"path\":\"/usr/bin/vim\""));
        assert!(json.contains("\"source\":\"path\""));
        // VSCode should not be installed
        assert!(json.contains("\"vscode\":{"));
        // Should contain at least one installed:false
        assert!(json.contains("\"installed\":false"));
    }

    #[test]
    fn test_category_detector_roundtrip_serialization() {
        let detector1 = CategoryDetector::<Editor>::default()
            .with_program(
                Editor::Vim,
                PathBuf::from("/usr/bin/vim"),
                ExecutableSource::Path,
            )
            .with_program(
                Editor::Neovim,
                PathBuf::from("/usr/bin/nvim"),
                ExecutableSource::Path,
            );

        let json = serde_json::to_string(&detector1).unwrap();
        let detector2: CategoryDetector<Editor> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            detector1.is_installed(Editor::Vim),
            detector2.is_installed(Editor::Vim)
        );
        assert_eq!(
            detector1.is_installed(Editor::Neovim),
            detector2.is_installed(Editor::Neovim)
        );
        assert_eq!(
            detector1.is_installed(Editor::VSCode),
            detector2.is_installed(Editor::VSCode)
        );
    }

    #[test]
    fn test_category_detector_deserialize_from_program_entries() {
        let json = r#"{
            "vim": {
                "installed": true,
                "name": "Vim",
                "description": "Classic modal text editor",
                "website": "https://www.vim.org",
                "path": "/usr/bin/vim",
                "source": "path"
            },
            "vscode": {
                "installed": false,
                "name": "Visual Studio Code",
                "description": "Modern code editor by Microsoft",
                "website": "https://code.visualstudio.com"
            }
        }"#;
        let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
        assert!(detector.is_installed(Editor::Vim));
        assert!(!detector.is_installed(Editor::VSCode));
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

    // ============================================
    // CategoryDetector ProgramDetector trait tests
    // ============================================

    #[test]
    fn test_category_detector_program_detector_trait() {
        let detector = CategoryDetector::<Editor>::default().with_program(
            Editor::Vim,
            PathBuf::from("/usr/bin/vim"),
            ExecutableSource::Path,
        );

        // Test through ProgramDetector trait interface
        let pd: &dyn ProgramDetector<Program = Editor> = &detector;
        assert!(pd.is_installed(Editor::Vim));
        assert!(!pd.is_installed(Editor::Neovim));
        assert_eq!(pd.path(Editor::Vim), Some(PathBuf::from("/usr/bin/vim")));
        let installed = pd.installed();
        assert_eq!(installed, vec![Editor::Vim]);
    }

    #[test]
    fn category_detector_known_methods_matches_metadata() {
        let detector = CategoryDetector::<Editor>::default();
        let methods = detector.known_methods(Editor::Vim);
        assert_eq!(methods, Editor::Vim.info().installation_methods);
    }

    #[test]
    fn category_detector_available_methods_filters_by_os() {
        // On the current host, VSCode's methods should produce a deterministic
        // subset — we just assert the call compiles and returns a Vec.
        let detector = CategoryDetector::<Editor>::default();
        let _available = detector.available_methods(Editor::VSCode);
    }

    #[test]
    fn category_detector_install_plan_returns_plan_for_program() {
        let detector = CategoryDetector::<Editor>::default();
        let plan = detector.install_plan(Editor::Vim);
        assert_eq!(plan.program, Editor::Vim.display_name());
    }

    #[test]
    fn installable_mirrors_plan_successful() {
        use strum::IntoEnumIterator;
        let detector = CategoryDetector::<Editor>::default();
        for editor in Editor::iter() {
            let plan = detector.install_plan(editor);
            assert_eq!(
                detector.installable(editor),
                plan.successful,
                "installable() must mirror install_plan().successful for {:?}",
                editor
            );
        }
    }
}

#[cfg(test)]
mod prereq_type_tests {
    use super::*;
    use crate::programs::contract::{PrereqProbe, SystemPrerequisite};

    #[test]
    fn system_prerequisite_is_constructible_as_const() {
        const PREREQ: SystemPrerequisite = SystemPrerequisite {
            name: "PortAudio",
            probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
            methods: &[InstallationMethod::Apt("libportaudio2")],
        };
        assert_eq!(PREREQ.name, "PortAudio");
        assert!(matches!(PREREQ.probe, PrereqProbe::SharedLibrary(_)));
    }

    #[test]
    fn prereq_probe_binary_variant() {
        const PROBE: PrereqProbe = PrereqProbe::Binary("ffmpeg");
        assert!(matches!(PROBE, PrereqProbe::Binary("ffmpeg")));
    }
}
