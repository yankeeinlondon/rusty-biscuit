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
    let dirty_files: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            // git status --porcelain format: XY filename
            let filename = line.get(3..).unwrap_or("").trim();
            is_source_file(filename)
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
                .map(|l| l.get(3..).unwrap_or("").trim())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
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
