use crate::Result;
use std::path::{Path, PathBuf};

pub use super::file_types::{
    LanguageSummary as LanguageBreakdown, ProgrammingLanguageStats as LanguageStats,
};

/// Detects programming languages in a directory tree.
///
/// Walks the directory tree starting from `root`, respecting `.gitignore` rules
/// and excluding common build/dependency directories. Uses hyperpolyglot to detect
/// the programming language of each file.
///
/// ## Returns
///
/// Returns a `LanguageBreakdown` containing statistics about detected languages,
/// sorted by file count in descending order. The `primary` field only considers
/// programming languages, not markup/documentation formats like Markdown.
///
/// ## Errors
///
/// Returns an error if the directory cannot be traversed or if there are
/// permission issues.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff::filesystem::detect_languages;
///
/// let breakdown = detect_languages(Path::new(".")).unwrap();
/// println!("Primary language: {:?}", breakdown.primary);
/// println!("Scanned files: {}", breakdown.total_files_scanned);
/// println!("Language files: {}", breakdown.total_language_files);
/// ```
pub fn detect_languages(root: &Path) -> Result<LanguageBreakdown> {
    detect_languages_with_exclusions(root, &[])
}

/// Detects programming languages in a directory tree while excluding nested subtrees.
pub(crate) fn detect_languages_with_exclusions(
    root: &Path,
    exclude_roots: &[PathBuf],
) -> Result<LanguageBreakdown> {
    let inventory = super::file_types::scan_file_inventory_with_exclusions(root, exclude_roots)?;
    Ok(super::file_types::summarize_languages(&inventory))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::file_types::{FrameworkKind, ProgrammingLanguage};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_empty_directory_returns_zero_languages() {
        let dir = TempDir::new().unwrap();
        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.total_files_scanned, 0);
        assert!(result.languages.is_empty());
        assert!(result.primary.is_none());
    }

    #[test]
    fn test_excludes_node_modules() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/test.js"), "// js").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let result = detect_languages(dir.path()).unwrap();
        // Should only count main.rs, not the file in node_modules
        assert_eq!(result.total_files_scanned, 1);
    }

    #[test]
    fn test_detects_rust_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.total_files_scanned, 2);
        assert!(result.primary.is_some());
        assert_eq!(result.primary, Some(ProgrammingLanguage::Rust));
    }

    #[test]
    fn test_excludes_nested_package_roots() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::create_dir_all(dir.path().join("frontend/src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            dir.path().join("frontend/src/index.ts"),
            "console.log('hi')",
        )
        .unwrap();

        let result =
            detect_languages_with_exclusions(dir.path(), &[dir.path().join("frontend")]).unwrap();

        assert_eq!(result.total_files_scanned, 1);
        assert_eq!(result.primary, Some(ProgrammingLanguage::Rust));
        assert_eq!(result.languages.len(), 1);
        assert_eq!(result.languages[0].language, ProgrammingLanguage::Rust);
    }

    #[test]
    fn test_markdown_does_not_count_as_language() {
        let dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(dir.path().join(format!("doc{}.md", i)), "# Heading").unwrap();
        }
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.primary, Some(ProgrammingLanguage::Rust));
        assert_eq!(result.total_language_files, 1);
        assert!(
            result
                .languages
                .iter()
                .all(|language| language.language != ProgrammingLanguage::Unknown)
        );
    }

    #[test]
    fn test_respects_gitignore() {
        let dir = TempDir::new().unwrap();

        // Create a .gitignore that ignores generated files
        fs::write(dir.path().join(".gitignore"), "generated/\n").unwrap();

        // Create a directory that should be ignored
        fs::create_dir(dir.path().join("generated")).unwrap();
        fs::write(
            dir.path().join("generated/output.rs"),
            "// This should be ignored",
        )
        .unwrap();

        // Create a file that should be counted
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        // Initialize git repo so .gitignore is respected
        fs::create_dir(dir.path().join(".git")).unwrap();

        let result = detect_languages(dir.path()).unwrap();
        // main.rs plus the visible .gitignore file are scanned, but only main.rs
        // contributes to language selection.
        assert_eq!(result.total_files_scanned, 2);
        assert_eq!(result.total_language_files, 1);
    }

    #[test]
    fn test_files_collected_per_language() {
        let dir = TempDir::new().unwrap();

        // Create a subdirectory
        fs::create_dir(dir.path().join("src")).unwrap();

        // Create Rust files
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn foo() {}").unwrap();

        // Create a JavaScript file
        fs::write(dir.path().join("index.js"), "console.log('hi')").unwrap();

        let result = detect_languages(dir.path()).unwrap();

        // Find Rust stats
        let rust_stats = result
            .languages
            .iter()
            .find(|s| s.language == ProgrammingLanguage::Rust)
            .expect("Rust should be detected");

        assert_eq!(rust_stats.direct_file_count, 2);
        assert_eq!(rust_stats.direct_files.len(), 2);

        assert!(rust_stats.direct_files.contains(&PathBuf::from("main.rs")));
        assert!(
            rust_stats
                .direct_files
                .contains(&PathBuf::from("src/lib.rs"))
        );

        assert!(rust_stats.direct_files[0] < rust_stats.direct_files[1]);

        let js_stats = result
            .languages
            .iter()
            .find(|s| s.language == ProgrammingLanguage::JavaScript)
            .expect("JavaScript should be detected");

        assert_eq!(js_stats.direct_file_count, 1);
        assert_eq!(js_stats.direct_files.len(), 1);
        assert_eq!(js_stats.direct_files[0], PathBuf::from("index.js"));
    }

    #[test]
    fn framework_files_contribute_to_language_summary() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Component.vue"),
            "<script setup lang=\"ts\">const value = 1;</script>",
        )
        .unwrap();

        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.primary, Some(ProgrammingLanguage::TypeScript));
        assert_eq!(result.frameworks.len(), 1);
        assert_eq!(result.frameworks[0].framework, FrameworkKind::Vue);
        assert_eq!(result.languages[0].framework_file_count, 1);
    }
}
