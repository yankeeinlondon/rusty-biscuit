use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
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
}

/// Detects programming languages in a directory tree.
///
/// Walks the directory tree starting from `root`, excluding common
/// build/dependency directories, and uses hyperpolyglot to detect
/// the programming language of each file.
///
/// ## Returns
///
/// Returns a `LanguageBreakdown` containing statistics about detected languages,
/// sorted by file count in descending order.
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
    let mut language_counts: HashMap<String, usize> = HashMap::new();
    let mut total_files = 0;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_excluded_dir(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .take(MAX_FILES)
    {
        total_files += 1;
        if let Ok(Some(detection)) = hyperpolyglot::detect(entry.path()) {
            *language_counts
                .entry(detection.language().to_string())
                .or_insert(0) += 1;
        }
    }

    let languages = calculate_stats(&language_counts, total_files);
    let primary = languages.first().map(|s| s.language.clone());

    Ok(LanguageBreakdown {
        languages,
        primary,
        total_files,
    })
}

/// Checks if a directory entry should be excluded from language detection.
///
/// Excludes common build artifacts, dependency directories, and version control
/// directories to improve performance and accuracy.
fn is_excluded_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    matches!(
        entry.file_name().to_str(),
        Some("node_modules" | "target" | ".git" | "vendor" | "dist" | "build" | "__pycache__")
    )
}

/// Calculates language statistics from raw counts.
///
/// Converts a HashMap of language counts into a sorted vector of LanguageStats,
/// computing percentages and sorting by file count in descending order.
fn calculate_stats(counts: &HashMap<String, usize>, total: usize) -> Vec<LanguageStats> {
    let mut stats: Vec<_> = counts
        .iter()
        .map(|(lang, &count)| LanguageStats {
            language: lang.clone(),
            file_count: count,
            percentage: if total > 0 {
                (count as f64 / total as f64) * 100.0
            } else {
                0.0
            },
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

        // Test with WalkDir entries
        for entry in WalkDir::new(dir.path()).into_iter().filter_map(|e| e.ok()) {
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
        let mut counts = HashMap::new();
        counts.insert("Rust".to_string(), 7);
        counts.insert("JavaScript".to_string(), 3);

        let stats = calculate_stats(&counts, 10);

        assert_eq!(stats.len(), 2);
        // Should be sorted by count (Rust first)
        assert_eq!(stats[0].language, "Rust");
        assert_eq!(stats[0].file_count, 7);
        assert!((stats[0].percentage - 70.0).abs() < 0.01);

        assert_eq!(stats[1].language, "JavaScript");
        assert_eq!(stats[1].file_count, 3);
        assert!((stats[1].percentage - 30.0).abs() < 0.01);
    }
}
