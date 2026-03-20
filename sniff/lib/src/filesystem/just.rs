use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A recipe extracted from a justfile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustRecipe {
    /// Recipe name (e.g., "build", "test", "_private")
    pub name: String,
    /// Recipe parameters as raw text (e.g., "*args=\"\"")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<String>,
    /// Whether this is a private recipe (starts with `_`)
    pub private: bool,
    /// The full recipe body (everything after the recipe line)
    pub body: String,
    /// xxHash of the recipe body
    pub hash: u64,
}

/// Information about a single justfile and its recipes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustfileInfo {
    /// Absolute path to the justfile
    pub path: PathBuf,
    /// Path relative to the scope root (repo root or base dir)
    pub relative: String,
    /// Recipes found in this justfile
    pub recipes: Vec<JustRecipe>,
}

/// Detect all justfiles and their recipes.
///
/// Scope is determined by git: if `base_dir` is inside a git repo, all
/// justfiles in the repo are found. Otherwise, all justfiles under
/// `base_dir` are found.
///
/// Justfile reads and recipe parsing are parallelized with rayon.
///
/// ## Arguments
///
/// * `base_dir` - Starting directory for discovery
/// * `filters` - Optional path substring filters (OR logic: justfile path
///   must contain at least one filter string)
pub fn detect_justfiles(base_dir: &Path, filters: &[String]) -> Result<Vec<JustfileInfo>> {
    let scope_root = find_scope_root(base_dir);
    let justfile_paths = find_justfiles(&scope_root);

    let filtered = if filters.is_empty() {
        justfile_paths
    } else {
        justfile_paths
            .into_iter()
            .filter(|path| {
                let path_str = path.display().to_string().to_lowercase();
                filters.iter().any(|f| path_str.contains(&f.to_lowercase()))
            })
            .collect()
    };

    let results: Vec<JustfileInfo> = filtered
        .par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let relative = path
                .strip_prefix(&scope_root)
                .unwrap_or(path)
                .display()
                .to_string();
            let recipes = parse_recipes(&content);
            Some(JustfileInfo {
                path: path.clone(),
                relative,
                recipes,
            })
        })
        .collect();

    Ok(results)
}

/// Determine scope root: git repo root if in a repo, otherwise base_dir.
fn find_scope_root(base_dir: &Path) -> PathBuf {
    match git2::Repository::discover(base_dir) {
        Ok(repo) => repo.workdir().unwrap_or(base_dir).to_path_buf(),
        Err(_) => base_dir.to_path_buf(),
    }
}

/// Walk a directory tree and collect all justfile paths.
fn find_justfiles(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden dirs and common non-source dirs
            if e.file_type().is_dir() {
                return !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "vendor";
            }
            true
        });

    for entry in walker.flatten() {
        let name = entry.file_name().to_string_lossy();
        if entry.file_type().is_file()
            && (name == "justfile" || name == "Justfile" || name == ".justfile")
        {
            results.push(entry.into_path());
        }
    }
    results
}

/// Parse recipes from justfile content.
///
/// A recipe starts with a line matching `name params:` (not indented,
/// not a comment, not a variable assignment). The body is all subsequent
/// indented lines.
fn parse_recipes(content: &str) -> Vec<JustRecipe> {
    let mut recipes = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip empty lines, comments, settings, variable assignments, and indented lines
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(' ')
            || line.starts_with('\t')
            || line.starts_with("set ")
            || line.starts_with("import ")
            || line.starts_with("mod ")
            || line.starts_with("export ")
            || line.starts_with("alias ")
        {
            i += 1;
            continue;
        }

        // Variable assignment (NAME := value)
        if line.contains(":=") {
            i += 1;
            continue;
        }

        // Look for recipe pattern: name [params]:
        if let Some((head, _after_colon)) = line.split_once(':') {
            // Must not be empty before colon
            let head = head.trim();
            if head.is_empty() {
                i += 1;
                continue;
            }

            // Extract recipe name and params
            let (name, params) = parse_recipe_head(head);

            // Skip if name contains invalid characters
            if name.is_empty() || !is_valid_recipe_name(&name) {
                i += 1;
                continue;
            }

            // Collect body lines (indented lines following the recipe declaration)
            let mut body_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                let body_line = lines[i];
                if body_line.starts_with(' ')
                    || body_line.starts_with('\t')
                    || body_line.is_empty()
                {
                    // Empty lines within a recipe body are part of it,
                    // but only if followed by more indented lines
                    if body_line.is_empty() {
                        // Look ahead to see if there are more body lines
                        let has_more = lines[i + 1..]
                            .iter()
                            .take(1)
                            .any(|l| l.starts_with(' ') || l.starts_with('\t'));
                        if has_more {
                            body_lines.push(body_line);
                            i += 1;
                            continue;
                        }
                        break;
                    }
                    body_lines.push(body_line);
                    i += 1;
                } else {
                    break;
                }
            }

            let body = body_lines.join("\n");
            let hash = biscuit_hash::xx_hash(&body);
            let private = name.starts_with('_');

            recipes.push(JustRecipe {
                name,
                params,
                private,
                body,
                hash,
            });
        } else {
            i += 1;
        }
    }

    recipes
}

/// Parse the head portion of a recipe line (before the colon) into name and params.
fn parse_recipe_head(head: &str) -> (String, Option<String>) {
    // Head could be: "name", "name param", "name *args=\"\"", "[group] name", etc.
    // Strip any attributes like [group('foo')]
    let head = if head.starts_with('[') {
        // Find the recipe name after the attribute - it's on a separate line in just
        // but may be on the same line in some cases. We'll just skip attributes.
        head.trim()
    } else {
        head
    };

    let parts: Vec<&str> = head.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), None);
    }

    let name = parts[0].to_string();
    let params = if parts.len() > 1 {
        Some(parts[1..].join(" "))
    } else {
        None
    };

    (name, params)
}

/// Check if a string is a valid just recipe name.
fn is_valid_recipe_name(name: &str) -> bool {
    let Some(first) = name.chars().next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_' || first == '-')
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_recipe() {
        let content = "build:\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "build");
        assert!(!recipes[0].private);
        assert!(recipes[0].params.is_none());
        assert_eq!(recipes[0].body, "    cargo build");
    }

    #[test]
    fn parse_recipe_with_params() {
        let content = "test *args=\"\":\n    cargo test {{args}}\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "test");
        assert_eq!(recipes[0].params.as_deref(), Some("*args=\"\""));
    }

    #[test]
    fn parse_private_recipe() {
        let content = "_helper:\n    echo \"internal\"\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert!(recipes[0].private);
    }

    #[test]
    fn parse_multiple_recipes() {
        let content = "\
build:\n    cargo build\n\ntest:\n    cargo test\n\n_lint:\n    cargo clippy\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 3);
        assert_eq!(recipes[0].name, "build");
        assert_eq!(recipes[1].name, "test");
        assert_eq!(recipes[2].name, "_lint");
    }

    #[test]
    fn skips_variables_and_settings() {
        let content = "\
set shell := [\"bash\"]\n\nFOO := \"bar\"\n\nbuild:\n    echo $FOO\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "build");
    }

    #[test]
    fn skips_comments() {
        let content = "# a comment\nbuild:\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
    }

    #[test]
    fn hash_is_consistent() {
        let content = "build:\n    cargo build\n";
        let r1 = parse_recipes(content);
        let r2 = parse_recipes(content);
        assert_eq!(r1[0].hash, r2[0].hash);
    }

    #[test]
    fn hash_differs_for_different_bodies() {
        let c1 = "build:\n    cargo build\n";
        let c2 = "build:\n    cargo build --release\n";
        let r1 = parse_recipes(c1);
        let r2 = parse_recipes(c2);
        assert_ne!(r1[0].hash, r2[0].hash);
    }

    #[test]
    fn multiline_body() {
        let content = "build:\n    #!/usr/bin/env bash\n    set -euo pipefail\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert!(recipes[0].body.contains("#!/usr/bin/env bash"));
        assert!(recipes[0].body.contains("cargo build"));
    }

    #[test]
    fn recipe_with_dependency() {
        let content = "all: build test\n    echo done\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "all");
    }

    #[test]
    fn is_valid_recipe_name_checks() {
        assert!(is_valid_recipe_name("build"));
        assert!(is_valid_recipe_name("_private"));
        assert!(is_valid_recipe_name("test-unit"));
        assert!(is_valid_recipe_name("build_all"));
        assert!(!is_valid_recipe_name("123"));
        assert!(!is_valid_recipe_name(""));
    }

    #[test]
    fn detect_justfiles_finds_repo_files() {
        // Run from this repo's root
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let result = detect_justfiles(&base, &[]).unwrap();
        // Should find at least the root justfile plus some package area justfiles
        assert!(
            result.len() > 1,
            "Expected multiple justfiles, found {}",
            result.len()
        );
        // All should have at least one recipe
        for jf in &result {
            assert!(
                !jf.recipes.is_empty(),
                "Justfile {} has no recipes",
                jf.relative
            );
        }
    }

    #[test]
    fn detect_justfiles_with_filter() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let result = detect_justfiles(&base, &["sniff".to_string()]).unwrap();
        assert!(
            !result.is_empty(),
            "Should find at least one justfile matching 'sniff'"
        );
        for jf in &result {
            assert!(
                jf.relative.to_lowercase().contains("sniff")
                    || jf.path.display().to_string().to_lowercase().contains("sniff"),
                "Justfile {} should match filter 'sniff'",
                jf.relative
            );
        }
    }
}
