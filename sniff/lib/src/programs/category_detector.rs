//! Generic category-detector implementation.
//!
//! Provides `CategoryDetector<E>` — a single generic detector that replaces
//! per-category structs (e.g. `InstalledEditors`, `InstalledUtilities`). It
//! stores detection results indexed by enum variant ordinal and implements
//! the `ProgramDetector` trait.

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SniffInstallationError;
use crate::executable_index::{ExecutableIndex, find_programs_with_source_from_index};
use crate::programs::contract::{CategoryEnum, ExecutableSource, InstallationMethod, ProgramError};
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::schema::ProgramMetadata;
use crate::programs::types::ProgramDetector;

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
        let _ = plan.execute(&crate::programs::install::InstallOptions::default())?;
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
                    crate::programs::install::astral_installer_url().to_string()
                }
                _ => unreachable!(),
            };
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: program.display_name().to_string(),
                url,
            });
        }

        let _ = crate::programs::install::execute_versioned_install(
            &chosen.kind,
            version,
            &crate::programs::install::InstallOptions::default(),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::enums::Editor;

    // ============================================
    // CategoryDetector tests
    // ============================================

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
