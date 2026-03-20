use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A structured recipe parameter parsed from a justfile recipe declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustRecipeParam {
    /// Parameter name (without variadic prefix)
    pub name: String,
    /// Whether this is a variadic parameter (`+` or `*` prefix)
    pub variadic: bool,
    /// Whether this parameter is optional (has a default value, or `*` variadic)
    pub optional: bool,
    /// The default value, if present (e.g., `"staging"` for `env="staging"`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// A recipe extracted from a justfile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustRecipe {
    /// Recipe name (e.g., "build", "test", "_private")
    pub name: String,
    /// Structured recipe parameters
    pub params: Vec<JustRecipeParam>,
    /// Whether this is a private recipe (starts with `_`)
    pub private: bool,
    /// Description from comment lines immediately above the recipe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    /// Whether this justfile has a `default` recipe
    pub has_default: bool,
    /// Recipes found in this justfile (excludes the `default` recipe)
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
            let all_recipes = parse_recipes(&content);
            let has_default = all_recipes.iter().any(|r| r.name == "default");
            let recipes = all_recipes
                .into_iter()
                .filter(|r| r.name != "default")
                .collect();
            Some(JustfileInfo {
                path: path.clone(),
                relative,
                has_default,
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

/// Parse a single parameter token into a structured `JustRecipeParam`.
///
/// Handles variadic prefixes (`+`, `*`), default values (`=...`), and
/// quoted/unquoted defaults.
fn parse_param(token: &str) -> JustRecipeParam {
    let (variadic_prefix, rest) = if let Some(stripped) = token.strip_prefix('*') {
        (Some('*'), stripped)
    } else if let Some(stripped) = token.strip_prefix('+') {
        (Some('+'), stripped)
    } else {
        (None, token)
    };

    let variadic = variadic_prefix.is_some();

    if let Some((name, default_raw)) = rest.split_once('=') {
        // Has a default value
        let default_val = default_raw
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        let optional = true;
        JustRecipeParam {
            name: name.to_string(),
            variadic,
            optional,
            default: Some(default_val),
        }
    } else {
        // No default value
        let optional = variadic_prefix == Some('*');
        JustRecipeParam {
            name: rest.to_string(),
            variadic,
            optional,
            default: None,
        }
    }
}

/// Parse recipes from justfile content.
///
/// A recipe starts with a line matching `name params:` (not indented,
/// not a comment, not a variable assignment). The body is all subsequent
/// indented lines. Comment lines (`#`) immediately preceding a recipe
/// are captured as the recipe's description.
fn parse_recipes(content: &str) -> Vec<JustRecipe> {
    let mut recipes = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut pending_comments: Vec<&str> = Vec::new();

    while i < lines.len() {
        let line = lines[i];

        // Collect comment lines as potential descriptions
        if line.starts_with('#') {
            let comment = line.strip_prefix('#').unwrap_or("").trim();
            if !comment.is_empty() {
                pending_comments.push(comment);
            }
            i += 1;
            continue;
        }

        // Skip empty lines, settings, variable assignments, and indented lines
        // Clear pending comments on non-comment, non-recipe lines
        if line.is_empty()
            || line.starts_with(' ')
            || line.starts_with('\t')
            || line.starts_with("set ")
            || line.starts_with("import ")
            || line.starts_with("mod ")
            || line.starts_with("export ")
            || line.starts_with("alias ")
        {
            if !line.starts_with(' ') && !line.starts_with('\t') {
                pending_comments.clear();
            }
            i += 1;
            continue;
        }

        // Variable assignment (NAME := value)
        if line.contains(":=") {
            pending_comments.clear();
            i += 1;
            continue;
        }

        // Skip attribute lines like [group('foo')]
        if line.starts_with('[') {
            // Don't clear pending comments — attributes precede recipes
            i += 1;
            continue;
        }

        // Look for recipe pattern: name [params]:
        if let Some((head, _after_colon)) = line.split_once(':') {
            // Must not be empty before colon
            let head = head.trim();
            if head.is_empty() {
                pending_comments.clear();
                i += 1;
                continue;
            }

            // Extract recipe name and params
            let (name, params) = parse_recipe_head(head);

            // Skip if name contains invalid characters
            if name.is_empty() || !is_valid_recipe_name(&name) {
                pending_comments.clear();
                i += 1;
                continue;
            }

            // Capture description from accumulated comments
            let description = if pending_comments.is_empty() {
                None
            } else {
                Some(pending_comments.join(" "))
            };
            pending_comments.clear();

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
                description,
                body,
                hash,
            });
        } else {
            pending_comments.clear();
            i += 1;
        }
    }

    recipes
}

/// Parse the head portion of a recipe line (before the colon) into name and params.
fn parse_recipe_head(head: &str) -> (String, Vec<JustRecipeParam>) {
    let parts: Vec<&str> = head.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), Vec::new());
    }

    let name = parts[0].to_string();
    let params = if parts.len() > 1 {
        parts[1..].iter().map(|p| parse_param(p)).collect()
    } else {
        Vec::new()
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
        assert!(recipes[0].params.is_empty());
        assert_eq!(recipes[0].body, "    cargo build");
    }

    #[test]
    fn parse_recipe_with_params() {
        let content = "test *args=\"\":\n    cargo test {{args}}\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "test");
        assert_eq!(recipes[0].params.len(), 1);
        assert_eq!(recipes[0].params[0].name, "args");
        assert!(recipes[0].params[0].variadic);
        assert!(recipes[0].params[0].optional);
        assert_eq!(recipes[0].params[0].default.as_deref(), Some(""));
    }

    #[test]
    fn parse_recipe_with_default_value() {
        let content = "deploy env=\"staging\":\n    echo {{env}}\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].params.len(), 1);
        assert_eq!(recipes[0].params[0].name, "env");
        assert!(!recipes[0].params[0].variadic);
        assert!(recipes[0].params[0].optional);
        assert_eq!(recipes[0].params[0].default.as_deref(), Some("staging"));
    }

    #[test]
    fn parse_required_variadic_param() {
        let content = "run +targets:\n    echo {{targets}}\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes[0].params[0].name, "targets");
        assert!(recipes[0].params[0].variadic);
        assert!(!recipes[0].params[0].optional);
        assert!(recipes[0].params[0].default.is_none());
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
    fn captures_description_from_comments() {
        let content = "# Build the project\nbuild:\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].description.as_deref(), Some("Build the project"));
    }

    #[test]
    fn captures_multiline_description() {
        let content = "# Build the project\n# with release optimizations\nbuild:\n    cargo build --release\n";
        let recipes = parse_recipes(content);
        assert_eq!(
            recipes[0].description.as_deref(),
            Some("Build the project with release optimizations")
        );
    }

    #[test]
    fn no_description_when_no_comments() {
        let content = "build:\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert!(recipes[0].description.is_none());
    }

    #[test]
    fn comments_separated_by_blank_line_are_not_description() {
        let content = "# This is a general comment\n\nbuild:\n    cargo build\n";
        let recipes = parse_recipes(content);
        assert!(recipes[0].description.is_none());
    }

    #[test]
    fn default_recipe_tracked_via_has_default() {
        let content = "default:\n    just --list\n\nbuild:\n    cargo build\n";
        let all_recipes = parse_recipes(content);
        let has_default = all_recipes.iter().any(|r| r.name == "default");
        let recipes: Vec<_> = all_recipes
            .into_iter()
            .filter(|r| r.name != "default")
            .collect();
        assert!(has_default);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "build");
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
    fn parse_param_required() {
        let p = parse_param("name");
        assert_eq!(p.name, "name");
        assert!(!p.variadic);
        assert!(!p.optional);
        assert!(p.default.is_none());
    }

    #[test]
    fn parse_param_optional_empty_default() {
        let p = parse_param("*args=\"\"");
        assert_eq!(p.name, "args");
        assert!(p.variadic);
        assert!(p.optional);
        assert_eq!(p.default.as_deref(), Some(""));
    }

    #[test]
    fn parse_param_optional_with_default() {
        let p = parse_param("env=\"staging\"");
        assert_eq!(p.name, "env");
        assert!(!p.variadic);
        assert!(p.optional);
        assert_eq!(p.default.as_deref(), Some("staging"));
    }

    #[test]
    fn parse_param_variadic_required() {
        let p = parse_param("+targets");
        assert_eq!(p.name, "targets");
        assert!(p.variadic);
        assert!(!p.optional);
        assert!(p.default.is_none());
    }

    #[test]
    fn parse_param_variadic_optional() {
        let p = parse_param("*args");
        assert_eq!(p.name, "args");
        assert!(p.variadic);
        assert!(p.optional);
        assert!(p.default.is_none());
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

    #[test]
    fn detect_justfiles_tracks_has_default() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let result = detect_justfiles(&base, &[]).unwrap();
        // At least some justfiles should have a default recipe
        let any_has_default = result.iter().any(|jf| jf.has_default);
        assert!(
            any_has_default,
            "Expected at least one justfile with a default recipe"
        );
        // The default recipe should NOT appear in the recipes list
        for jf in &result {
            assert!(
                !jf.recipes.iter().any(|r| r.name == "default"),
                "Justfile {} should not list 'default' in recipes",
                jf.relative
            );
        }
    }
}
