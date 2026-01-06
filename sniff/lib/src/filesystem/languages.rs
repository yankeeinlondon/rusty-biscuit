use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::Result;

/// Maximum files to scan before early termination
const MAX_FILES: usize = 10_000;

/// Language detection statistics for a repository or directory.
///
/// This structure contains information about the programming languages
/// detected in a codebase, including file counts and percentages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageBreakdown {
    /// List of detected languages with their statistics
    pub languages: Vec<LanguageStats>,
    /// The primary language (the one with the most files)
    pub primary: Option<String>,
    /// Total number of files scanned
    pub total_files: usize,
}

/// Statistics for a single programming language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    /// Name of the programming language
    pub language: String,
    /// Number of files written in this language
    pub file_count: usize,
    /// Percentage of total files (0.0 to 100.0)
    pub percentage: f64,
    /// List of files detected for this language (paths relative to scanned root)
    pub files: Vec<PathBuf>,
}

/// Non-programming languages that should not be considered as "primary".
///
/// These are documentation or data formats, not programming languages.
const NON_PROGRAMMING_LANGUAGES: &[&str] = &[
    "Markdown",
    "JSON",
    "YAML",
    "TOML",
    "XML",
    "HTML",
    "CSS",
    "Text",
    "Plain Text",
    "reStructuredText",
    "AsciiDoc",
    "Org",
    "TeX",
    "LaTeX",
    "BibTeX",
    "Diff",
    "Ignore List",
    "INI",
    "EditorConfig",
    "Git Config",
    "Git Attributes",
    "Dockerfile",
    "Makefile",
    "CMake",
    "Meson",
    "Nix",
];

/// Checks if a language is a programming language (not just markup/config/data).
fn is_programming_language(lang: &str) -> bool {
    !NON_PROGRAMMING_LANGUAGES.contains(&lang)
}

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
/// use sniff_lib::filesystem::detect_languages;
///
/// let breakdown = detect_languages(Path::new(".")).unwrap();
/// println!("Primary language: {:?}", breakdown.primary);
/// println!("Total files: {}", breakdown.total_files);
/// ```
pub fn detect_languages(root: &Path) -> Result<LanguageBreakdown> {
    let mut language_files: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut total_files = 0;

    // Use the `ignore` crate which respects .gitignore files
    let walker = WalkBuilder::new(root)
        .hidden(true)           // Skip hidden files (like .git)
        .git_ignore(true)       // Respect .gitignore
        .git_global(true)       // Respect global gitignore
        .git_exclude(true)      // Respect .git/info/exclude
        .filter_entry(|e| !is_excluded_dir(e))
        .build();

    for entry in walker
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .take(MAX_FILES)
    {
        total_files += 1;
        if let Ok(Some(detection)) = hyperpolyglot::detect(entry.path()) {
            // Store path relative to the scanned root
            let relative_path = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_path_buf();
            language_files
                .entry(detection.language().to_string())
                .or_default()
                .push(relative_path);
        }
    }

    let languages = calculate_stats(&language_files, total_files);

    // Primary language must be a programming language, not markup/config
    let primary = languages
        .iter()
        .find(|s| is_programming_language(&s.language))
        .map(|s| s.language.clone());

    Ok(LanguageBreakdown {
        languages,
        primary,
        total_files,
    })
}

/// Checks if a directory entry should be excluded from language detection.
///
/// Excludes common build artifacts and dependency directories to improve
/// performance and accuracy.
fn is_excluded_dir(entry: &ignore::DirEntry) -> bool {
    if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
        return false;
    }
    matches!(
        entry.file_name().to_str(),
        Some("node_modules" | "target" | "vendor" | "dist" | "build" | "__pycache__")
    )
}

/// Calculates language statistics from file lists.
///
/// Converts a HashMap of language file lists into a sorted vector of LanguageStats,
/// computing percentages and sorting by file count in descending order.
/// Files within each language are sorted for consistent output.
fn calculate_stats(
    language_files: &HashMap<String, Vec<PathBuf>>,
    total: usize,
) -> Vec<LanguageStats> {
    let mut stats: Vec<_> = language_files
        .iter()
        .map(|(lang, files)| {
            let count = files.len();
            let mut sorted_files = files.clone();
            sorted_files.sort();
            LanguageStats {
                language: lang.clone(),
                file_count: count,
                percentage: if total > 0 {
                    (count as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
                files: sorted_files,
            }
        })
        .collect();

    // Sort by file count descending
    stats.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_empty_directory_returns_zero_languages() {
        let dir = TempDir::new().unwrap();
        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.total_files, 0);
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
        assert_eq!(result.total_files, 1);
    }

    #[test]
    fn test_detects_rust_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let result = detect_languages(dir.path()).unwrap();
        assert_eq!(result.total_files, 2);
        assert!(result.primary.is_some());
        // Verify the primary language is Rust
        assert_eq!(result.primary.as_deref(), Some("Rust"));
    }

    #[test]
    fn test_excluded_dirs_list() {
        let dir = TempDir::new().unwrap();

        // Create a regular directory (should not be excluded)
        let regular_dir = dir.path().join("src");
        fs::create_dir(&regular_dir).unwrap();

        // Create an excluded directory
        let excluded_dir = dir.path().join("target");
        fs::create_dir(&excluded_dir).unwrap();

        // Test with ignore WalkBuilder entries
        for entry in WalkBuilder::new(dir.path()).build().filter_map(|e| e.ok()) {
            if entry.path() == regular_dir {
                assert!(!is_excluded_dir(&entry), "src should not be excluded");
            }
            if entry.path() == excluded_dir {
                assert!(is_excluded_dir(&entry), "target should be excluded");
            }
        }
    }

    #[test]
    fn test_percentage_calculation() {
        let mut language_files = HashMap::new();
        language_files.insert(
            "Rust".to_string(),
            (0..7).map(|i| PathBuf::from(format!("file{}.rs", i))).collect(),
        );
        language_files.insert(
            "JavaScript".to_string(),
            (0..3).map(|i| PathBuf::from(format!("file{}.js", i))).collect(),
        );

        let stats = calculate_stats(&language_files, 10);

        assert_eq!(stats.len(), 2);
        // Should be sorted by count (Rust first)
        assert_eq!(stats[0].language, "Rust");
        assert_eq!(stats[0].file_count, 7);
        assert!((stats[0].percentage - 70.0).abs() < 0.01);
        assert_eq!(stats[0].files.len(), 7);

        assert_eq!(stats[1].language, "JavaScript");
        assert_eq!(stats[1].file_count, 3);
        assert!((stats[1].percentage - 30.0).abs() < 0.01);
        assert_eq!(stats[1].files.len(), 3);
    }

    #[test]
    fn test_primary_language_is_programming_language() {
        // Markdown should not be considered a programming language
        assert!(!is_programming_language("Markdown"));
        assert!(!is_programming_language("JSON"));
        assert!(!is_programming_language("YAML"));
        assert!(!is_programming_language("TOML"));

        // Real programming languages
        assert!(is_programming_language("Rust"));
        assert!(is_programming_language("JavaScript"));
        assert!(is_programming_language("Python"));
        assert!(is_programming_language("Go"));
    }

    #[test]
    fn test_primary_skips_markdown_for_programming_language() {
        let dir = TempDir::new().unwrap();
        // Create more markdown files than Rust files
        for i in 0..10 {
            fs::write(dir.path().join(format!("doc{}.md", i)), "# Heading").unwrap();
        }
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let result = detect_languages(dir.path()).unwrap();
        // Primary should be Rust, not Markdown
        assert_eq!(result.primary.as_deref(), Some("Rust"));
        // But Markdown should still be in the languages list
        assert!(result.languages.iter().any(|l| l.language == "Markdown"));
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
        // Should only count main.rs, not the file in generated/
        assert_eq!(result.total_files, 1);
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
            .find(|s| s.language == "Rust")
            .expect("Rust should be detected");

        // Verify Rust files are collected
        assert_eq!(rust_stats.file_count, 2);
        assert_eq!(rust_stats.files.len(), 2);

        // Paths should be relative and sorted
        assert!(rust_stats.files.contains(&PathBuf::from("main.rs")));
        assert!(rust_stats.files.contains(&PathBuf::from("src/lib.rs")));

        // Verify files are sorted
        assert!(rust_stats.files[0] < rust_stats.files[1]);

        // Find JavaScript stats
        let js_stats = result
            .languages
            .iter()
            .find(|s| s.language == "JavaScript")
            .expect("JavaScript should be detected");

        assert_eq!(js_stats.file_count, 1);
        assert_eq!(js_stats.files.len(), 1);
        assert_eq!(js_stats.files[0], PathBuf::from("index.js"));
    }
}
