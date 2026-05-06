//! Type aliases for `CategoryDetector<E>` over each program category enum.
//!
//! All near-empty per-category modules (`editors`, `utilities`,
//! `headless_audio`, `terminal_apps`, `tts_clients`, `ai_cli`, and the
//! language/OS package-manager pair) are consolidated here so callers can
//! refer to a single location for installed-program collection types.

use crate::programs::category_detector::CategoryDetector;
use crate::programs::enums::{
    AiCli, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager, TerminalApp, TtsClient,
    Utility,
};

/// Popular text editors and IDEs found on the system.
pub type InstalledEditors = CategoryDetector<Editor>;

/// Modern command-line utilities found on the system.
pub type InstalledUtilities = CategoryDetector<Utility>;

/// Headless audio players found on the system.
pub type InstalledHeadlessAudio = CategoryDetector<HeadlessAudio>;

/// Terminal emulator applications found on the system.
pub type InstalledTerminalApps = CategoryDetector<TerminalApp>;

/// Text-to-speech clients found on the system.
pub type InstalledTtsClients = CategoryDetector<TtsClient>;

/// AI-powered CLI coding tools found on the system.
pub type InstalledAiClients = CategoryDetector<AiCli>;

/// Language-specific package managers found on the system.
pub type InstalledLanguagePackageManagers = CategoryDetector<LanguagePackageManager>;

/// Operating system package managers found on the system.
pub type InstalledOsPackageManagers = CategoryDetector<OsPackageManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::ProgramError;

    #[test]
    fn editors_default_has_nothing_installed() {
        let editors = InstalledEditors::default();
        assert!(!editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
        assert!(editors.installed().is_empty());
    }

    #[test]
    fn editors_path_with_source_returns_none_when_not_installed() {
        let editors = InstalledEditors::default();
        assert!(editors.path_with_source(Editor::Vim).is_none());
    }

    #[test]
    fn editors_serialize_produces_program_entries() {
        let editors = InstalledEditors::default();
        let json = serde_json::to_string(&editors).unwrap();
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"name\":\"Vim\""));
    }

    #[test]
    fn editors_deserialize_from_boolean_fields() {
        let json = r#"{"vim": true, "vscode": false}"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn editors_version_returns_not_found_for_uninstalled() {
        let editors = InstalledEditors::default();
        let result = editors.version(Editor::Vim);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "vim");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn editors_website_returns_static_str() {
        let editors = InstalledEditors::default();
        let website = editors.website(Editor::Vim);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn editors_default_has_all_none() {
        let editors = InstalledEditors::default();
        use strum::IntoEnumIterator;
        for editor in Editor::iter() {
            assert!(
                !editors.is_installed(editor),
                "{:?} should not be installed",
                editor
            );
        }
    }

    #[test]
    fn editors_clone_produces_equal_struct() {
        let editors = InstalledEditors::default();
        let cloned = editors.clone();
        assert_eq!(editors, cloned);
    }
}
