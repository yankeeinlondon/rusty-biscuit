//! Editor detection — type alias for `CategoryDetector<Editor>`.

use crate::programs::enums::Editor;
use crate::programs::types::CategoryDetector;

/// Popular text editors and IDEs found on the system.
pub type InstalledEditors = CategoryDetector<Editor>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_path_with_source_returns_none_when_not_installed() {
        let editors = InstalledEditors::default();
        assert!(editors.path_with_source(Editor::Vim).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let editors = InstalledEditors::default();
        assert!(!editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_serialize_produces_program_entries() {
        let editors = InstalledEditors::default();
        let json = serde_json::to_string(&editors).unwrap();
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"vim\":{"));
        assert!(json.contains("\"name\":\"Vim\""));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"vim": true, "vscode": false}"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let editors = InstalledEditors::default();
        assert!(editors.installed().is_empty());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        use crate::programs::schema::ProgramError;
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
    fn test_website_returns_static_str() {
        let editors = InstalledEditors::default();
        let website = editors.website(Editor::Vim);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_deserialize_partial_json() {
        let json = r#"{"vim": true}"#;
        let editors: InstalledEditors = serde_json::from_str(json).unwrap();
        assert!(editors.is_installed(Editor::Vim));
        assert!(!editors.is_installed(Editor::VSCode));
    }

    #[test]
    fn test_default_has_all_none() {
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
    fn test_clone_produces_equal_struct() {
        let editors = InstalledEditors::default();
        let cloned = editors.clone();
        assert_eq!(editors, cloned);
    }
}
