use std::path::Path;
use std::process::Command;

use super::CheckResult;

/// Known source code extensions for dirty-source-code checks.
const SOURCE_EXTENSIONS: &[&str] = &[
    // Rust
    "rs", // JS/TS
    "js", "jsx", "ts", "tsx", "mjs", "cjs", // Python
    "py",  // Go
    "go",  // JVM
    "java", "kt", // Web
    "css", "scss", "html", // Shell
    "sh", "bash", "zsh",
];

/// Known source-adjacent filenames.
const SOURCE_FILENAMES: &[&str] = &["justfile", "Cargo.toml", "package.json"];

pub(crate) fn check_dirty_source_code(root: &Path, expect_dirty: bool) -> CheckResult {
    // Find repo root by walking up from the specified root
    let repo_root = find_git_repo_root(root)
        .ok_or_else(|| format!("no git repository found at or above {}", root.display()))?;

    let output = Command::new("git")
        .args(["status", "--porcelain", "--"])
        .arg(root)
        .current_dir(&repo_root)
        .output()
        .map_err(|e| format!("failed to run git status: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let dirty_files: Vec<Vec<String>> = stdout
        .lines()
        .filter_map(|line| {
            let paths = parse_porcelain_paths(line);
            if paths.iter().any(|p| is_source_file(p)) {
                Some(paths)
            } else {
                None
            }
        })
        .collect();

    if expect_dirty {
        if dirty_files.is_empty() {
            Err("no dirty source code found".to_string())
        } else {
            Ok(())
        }
    } else if dirty_files.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "dirty source code found: {}",
            dirty_files
                .iter()
                .map(|paths| paths.join(" -> "))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

/// Parse a `git status --porcelain` line into one or more file paths.
///
/// Rename entries (`R  old -> new`) yield both paths. Quoted paths
/// (`"path with spaces"`) have their surrounding double quotes stripped.
fn parse_porcelain_paths(line: &str) -> Vec<String> {
    let status = line.get(0..2).unwrap_or("");
    let rest = line.get(3..).unwrap_or("").trim();
    let raw_paths: Vec<&str> = if status.starts_with('R') {
        rest.split(" -> ").collect()
    } else {
        vec![rest]
    };
    raw_paths
        .into_iter()
        .map(|p| p.trim_matches('"').to_string())
        .collect()
}

fn is_source_file(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if SOURCE_FILENAMES.contains(&filename) {
        return true;
    }
    if let Some(ext) = path.rsplit('.').next() {
        SOURCE_EXTENSIONS.contains(&ext)
    } else {
        false
    }
}

fn find_git_repo_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_regular_porcelain_line() {
        assert_eq!(
            parse_porcelain_paths(" M src/main.rs"),
            vec!["src/main.rs".to_string()]
        );
    }

    #[test]
    fn parse_quoted_path_with_spaces() {
        assert_eq!(
            parse_porcelain_paths(" M \"src/my file.rs\""),
            vec!["src/my file.rs".to_string()]
        );
    }

    #[test]
    fn parse_rename_line() {
        assert_eq!(
            parse_porcelain_paths("R  old_name.rs -> new_name.rs"),
            vec!["old_name.rs".to_string(), "new_name.rs".to_string()]
        );
    }

    #[test]
    fn parse_rename_with_quoted_paths() {
        assert_eq!(
            parse_porcelain_paths("R  \"old name.rs\" -> \"new name.rs\""),
            vec!["old name.rs".to_string(), "new name.rs".to_string()]
        );
    }

    #[test]
    fn is_source_file_recognizes_extensions_and_filenames() {
        assert!(is_source_file("src/main.rs"));
        assert!(is_source_file("lib.js"));
        assert!(is_source_file("justfile"));
        assert!(is_source_file("Cargo.toml"));
        assert!(!is_source_file("README.md"));
        assert!(!is_source_file("image.png"));
    }
}
