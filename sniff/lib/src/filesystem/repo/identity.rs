//! Lightweight repository identity detection.
//!
//! `detect_repo_identity` is the fast path behind `sniff repo name`. It
//! resolves a stable, human-friendly name plus a few summary attributes for
//! both monorepo and single-package repositories without paying for the
//! full repo + filesystem scan.

use std::path::Path;

use biscuit_file::toml_crate;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::error::SniffError;

use super::cargo::{cargo_package_name, cargo_package_version};
use super::go::go_module_name_from_content;
use super::npm::{npm_package_name, npm_package_version};
use super::python::{pyproject_package_name, pyproject_package_version};
use super::types::detect_repo_structure;

/// Summary identity of the repository at a given directory.
///
/// Produced by [`detect_repo_identity`]. The shape is intentionally narrow:
/// just enough for a one-line description suitable for shell prompts,
/// status lines, or `sniff repo name`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoIdentity {
    /// Display name of the repository.
    ///
    /// Resolved in priority order: root manifest `name` field → git remote
    /// URL basename → git repo root directory name.
    pub name: String,
    /// Version string from a root manifest, when one is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Primary programming language inferred from the root manifest.
    ///
    /// Only populated for single-package repositories — monorepos
    /// intentionally leave this `None` since their language mix is reported
    /// by `sniff repo language`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Whether the repository is a monorepo (cargo/npm/pnpm/yarn/nx/turbo/lerna).
    pub is_monorepo: bool,
    /// Number of packages discovered in the monorepo, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_count: Option<usize>,
}

/// Detect repository identity from any path inside the repository.
///
/// Walks up from `dir` to locate the enclosing git repository, then
/// resolves a display name, version, language, and monorepo summary from
/// root manifests and the git remote.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff::filesystem::repo::detect_repo_identity;
///
/// let identity = detect_repo_identity(Path::new(".")).unwrap();
/// println!("{} v{}", identity.name, identity.version.unwrap_or_default());
/// ```
///
/// ## Errors
///
/// Returns [`SniffError::NotARepository`] if `dir` is not inside a git
/// repository.
pub fn detect_repo_identity(dir: &Path) -> Result<RepoIdentity> {
    // Trust/permission/I/O/corruption failures surface via `?`; only genuine
    // repository absence becomes `NotARepository`.
    let root = crate::filesystem::repo_root(dir)?
        .ok_or_else(|| SniffError::NotARepository(dir.to_path_buf()))?;

    let monorepo = detect_repo_structure(&root)?;
    let (is_monorepo, package_count) = match monorepo {
        Some(info) if info.is_monorepo => (true, info.packages.as_ref().map(Vec::len)),
        _ => (false, None),
    };

    let name = resolve_name(&root);
    let version = resolve_version(&root);
    let language = if is_monorepo {
        None
    } else {
        resolve_language(&root)
    };

    Ok(RepoIdentity {
        name,
        version,
        language,
        is_monorepo,
        package_count,
    })
}

/// Resolve the display name in priority order: manifest → remote → dir name.
fn resolve_name(root: &Path) -> String {
    if let Some(name) = manifest_name(root) {
        return name;
    }
    if let Some(name) = remote_basename(root) {
        return name;
    }
    root.file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .unwrap_or_else(|| "unknown".to_string())
}

fn manifest_name(root: &Path) -> Option<String> {
    if let Some(value) = read_toml(&root.join("Cargo.toml"))
        && let Some(name) = cargo_package_name(&value)
    {
        return Some(name);
    }
    if let Some(value) = read_json(&root.join("package.json"))
        && let Some(name) = npm_package_name(&value)
    {
        return Some(name);
    }
    if let Some(value) = read_toml(&root.join("pyproject.toml"))
        && let Some(name) = pyproject_package_name(&value)
    {
        return Some(name);
    }
    if let Ok(content) = std::fs::read_to_string(root.join("go.mod"))
        && let Some(module) = go_module_name_from_content(&content)
    {
        // `module github.com/owner/repo` → use the trailing segment.
        return Some(
            module
                .rsplit('/')
                .next()
                .unwrap_or(module.as_str())
                .to_string(),
        );
    }
    None
}

fn remote_basename(root: &Path) -> Option<String> {
    // `origin` if present, otherwise the first configured remote with a URL.
    let url = crate::filesystem::preferred_remote_url(root)
        .ok()
        .flatten()?;
    parse_remote_basename(&url)
}

/// Extract the trailing path segment from a git remote URL, stripping `.git`.
///
/// Handles SSH (`git@host:owner/repo.git`), HTTPS (`https://host/owner/repo`),
/// and local paths. Returns `None` only when no usable basename can be
/// extracted (e.g. empty trailing component).
fn parse_remote_basename(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let after_colon = trimmed.rsplit(':').next().unwrap_or(trimmed);
    let last_segment = after_colon.rsplit('/').next().unwrap_or(after_colon);
    let stripped = last_segment.strip_suffix(".git").unwrap_or(last_segment);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

fn resolve_version(root: &Path) -> Option<String> {
    if let Some(value) = read_toml(&root.join("Cargo.toml")) {
        if let Some(v) = value
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Some(v.to_string());
        }
        if let Some(v) = cargo_package_version(&value) {
            return Some(v);
        }
    }
    if let Some(value) = read_json(&root.join("package.json"))
        && let Some(v) = npm_package_version(&value)
    {
        return Some(v);
    }
    if let Some(value) = read_toml(&root.join("pyproject.toml"))
        && let Some(v) = pyproject_package_version(&value)
    {
        return Some(v);
    }
    None
}

fn resolve_language(root: &Path) -> Option<String> {
    if root.join("Cargo.toml").exists() {
        return Some("Rust".to_string());
    }
    if let Some(value) = read_json(&root.join("package.json")) {
        let has_typescript = ["dependencies", "devDependencies", "peerDependencies"]
            .iter()
            .any(|section| {
                value
                    .get(section)
                    .and_then(|s| s.as_object())
                    .is_some_and(|obj| obj.contains_key("typescript"))
            });
        return Some(if has_typescript {
            "TypeScript".to_string()
        } else {
            "JavaScript".to_string()
        });
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        return Some("Python".to_string());
    }
    if root.join("go.mod").exists() {
        return Some("Go".to_string());
    }
    None
}

fn read_toml(path: &Path) -> Option<toml_crate::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    toml_crate::from_str(&content).ok()
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_when_not_in_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let err = detect_repo_identity(tmp.path()).unwrap_err();
        assert!(matches!(err, SniffError::NotARepository(_)));
    }

    #[test]
    fn resolves_name_from_cargo_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();

        let identity = detect_repo_identity(tmp.path()).unwrap();
        assert_eq!(identity.name, "my-crate");
        assert_eq!(identity.version.as_deref(), Some("1.2.3"));
        assert_eq!(identity.language.as_deref(), Some("Rust"));
        assert!(!identity.is_monorepo);
        assert_eq!(identity.package_count, None);
    }

    #[test]
    fn resolves_typescript_from_devdeps() {
        let tmp = tempfile::tempdir().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"obsidian-thing","version":"0.3.0","devDependencies":{"typescript":"^5"}}"#,
        )
        .unwrap();

        let identity = detect_repo_identity(tmp.path()).unwrap();
        assert_eq!(identity.name, "obsidian-thing");
        assert_eq!(identity.version.as_deref(), Some("0.3.0"));
        assert_eq!(identity.language.as_deref(), Some("TypeScript"));
    }

    #[test]
    fn falls_back_to_dir_name_without_manifest_or_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("widget-shop");
        std::fs::create_dir_all(&nested).unwrap();
        git2::Repository::init(&nested).unwrap();

        let identity = detect_repo_identity(&nested).unwrap();
        assert_eq!(identity.name, "widget-shop");
        assert_eq!(identity.language, None);
        assert!(!identity.is_monorepo);
    }
}
