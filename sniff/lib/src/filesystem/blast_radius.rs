//! Blast-radius analysis: source-code detection, changed-path collection,
//! and document matching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git2::Repository;
use serde::{Deserialize, Serialize};

use crate::filesystem::docs::{MarkdownMeta, detect_docs};
use crate::filesystem::file_types::{lookup_exact_filename, lookup_extension};
use crate::filesystem::git::get_commit_files;
use crate::filesystem::repo::detect_repo;
use crate::filesystem::FileAssociation;
use crate::{Result, SniffError};

// ---------------------------------------------------------------------------
// Source-code detection
// ---------------------------------------------------------------------------

/// Returns `true` if the path refers to a source code file.
///
/// Classification uses the file-type registry (exact filename match, then
/// extension match). Paths whose association is `ProgrammingLanguage`,
/// `FrameworkFile`, or `Styling` are considered source code. HTML/HTM files
/// are also accepted as an explicit fallback (they are classified as
/// `Documentation` in the registry but were historically treated as source
/// code in the CLI).
pub fn is_source_code_path(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Try exact filename match first
    if let Some(desc) = lookup_exact_filename(file_name) {
        return matches!(
            desc.association,
            FileAssociation::ProgrammingLanguage
                | FileAssociation::FrameworkFile
                | FileAssociation::Styling
        );
    }

    // Try extension match
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        // Explicit fallback: accept HTML/HTM (classified as Documentation in
        // the registry, but historically treated as source code)
        if ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm") {
            return true;
        }

        if let Some(desc) = lookup_extension(ext) {
            return matches!(
                desc.association,
                FileAssociation::ProgrammingLanguage
                    | FileAssociation::FrameworkFile
                    | FileAssociation::Styling
            );
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Changed-path collection
// ---------------------------------------------------------------------------

/// Which set of changes to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeScope {
    /// Staged + Modified + Both + Untracked (deduplicated).
    Dirty,
    /// Staged + Both only.
    Staged,
    /// Modified + Both only (excludes untracked).
    Unstaged,
    /// Files changed in the HEAD commit.
    LastCommit,
}

/// Whether to return all files or only source code files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangedPathKind {
    AllFiles,
    SourceCode,
}

/// Parameters for collecting changed paths.
#[derive(Debug, Clone)]
pub struct ChangedPathQuery {
    pub scope: ChangeScope,
    pub kind: ChangedPathKind,
    pub package: Option<String>,
    pub package_area: Option<String>,
    pub filters: Vec<String>,
}

/// Result of collecting changed paths.
#[derive(Debug, Clone)]
pub struct ChangedPathResult {
    pub repo_root: PathBuf,
    pub paths: Vec<PathBuf>,
}

/// Collect changed file paths according to the given query.
///
/// ## Errors
///
/// Returns `SniffError::NotARepository` if `base_dir` is not inside a git repo.
/// Returns an error if `package` or `package_area` is specified but the repo
/// is not a monorepo.
pub fn collect_changed_paths(base_dir: &Path, query: &ChangedPathQuery) -> Result<ChangedPathResult> {
    let repo = Repository::discover(base_dir)
        .map_err(|_| SniffError::NotARepository(base_dir.to_path_buf()))?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?
        .to_path_buf();

    let raw_paths = match query.scope {
        ChangeScope::LastCommit => collect_last_commit_paths(&repo),
        _ => collect_working_tree_paths(&repo, query.scope)?,
    };

    let mut paths: Vec<PathBuf> = raw_paths
        .into_iter()
        // Filter by source code if requested
        .filter(|p| match query.kind {
            ChangedPathKind::AllFiles => true,
            ChangedPathKind::SourceCode => is_source_code_path(p),
        })
        .collect();

    // Filter by package or package_area if specified
    if query.package.is_some() || query.package_area.is_some() {
        let repo_info = detect_repo(&repo_root)?
            .ok_or_else(|| SniffError::NotAMonorepo(repo_root.clone()))?;

        if !repo_info.is_monorepo {
            return Err(SniffError::NotAMonorepo(repo_root.clone()));
        }

        if let Some(packages) = &repo_info.packages {
            let matching_roots: Vec<PathBuf> = packages
                .iter()
                .filter(|pkg| {
                    if let Some(ref name) = query.package {
                        pkg.name.eq_ignore_ascii_case(name)
                    } else if let Some(ref area) = query.package_area {
                        pkg.package_area.eq_ignore_ascii_case(area)
                    } else {
                        false
                    }
                })
                .map(|pkg| {
                    pkg.path
                        .strip_prefix(&repo_root)
                        .unwrap_or(&pkg.path)
                        .to_path_buf()
                })
                .collect();

            paths.retain(|p| matching_roots.iter().any(|root| p.starts_with(root)));
        }
    }

    // Apply substring filters (OR logic, case-insensitive)
    if !query.filters.is_empty() {
        paths.retain(|p| {
            let p_str = p.to_string_lossy().to_lowercase();
            query
                .filters
                .iter()
                .any(|f| p_str.contains(&f.to_lowercase()))
        });
    }

    // Sort and deduplicate
    paths.sort();
    paths.dedup();

    Ok(ChangedPathResult { repo_root, paths })
}

/// Collect paths from working tree status (Dirty/Staged/Unstaged).
fn collect_working_tree_paths(repo: &Repository, scope: ChangeScope) -> Result<Vec<PathBuf>> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut seen = HashSet::new();
    let mut paths = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = match entry.path() {
            Some(p) => PathBuf::from(p),
            None => continue,
        };

        let is_staged =
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_modified() || status.is_wt_deleted();
        let is_untracked = status.is_wt_new();

        let include = match scope {
            ChangeScope::Dirty => is_staged || is_unstaged || is_untracked,
            ChangeScope::Staged => is_staged,
            ChangeScope::Unstaged => is_unstaged,
            ChangeScope::LastCommit => unreachable!(),
        };

        if include && seen.insert(path.clone()) {
            paths.push(path);
        }
    }

    Ok(paths)
}

/// Collect paths from the HEAD commit.
fn collect_last_commit_paths(repo: &Repository) -> Vec<PathBuf> {
    let head_sha = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id().to_string());

    match head_sha {
        Some(sha) => get_commit_files(repo, &sha)
            .into_iter()
            .map(|(path, _kind)| path)
            .collect(),
        None => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Blast-radius document matching
// ---------------------------------------------------------------------------

/// Find documents whose `blast_radius` frontmatter intersects with changed
/// source code files.
///
/// ## Errors
///
/// Returns an error if the directory is not inside a git repo.
pub fn find_blast_radius_documents(
    base_dir: &Path,
    scope: ChangeScope,
    package: Option<&str>,
    package_area: Option<&str>,
) -> Result<Vec<MarkdownMeta>> {
    let result = collect_changed_paths(
        base_dir,
        &ChangedPathQuery {
            scope,
            kind: ChangedPathKind::SourceCode,
            package: package.map(String::from),
            package_area: package_area.map(String::from),
            filters: Vec::new(),
        },
    )?;

    let changed_set: HashSet<PathBuf> = result.paths.into_iter().collect();

    let docs = match detect_docs(&result.repo_root) {
        Some(docs) => docs,
        None => return Ok(Vec::new()),
    };

    let mut matched: Vec<MarkdownMeta> = docs
        .into_iter()
        .filter(|doc| {
            if !doc.has_blast_radius {
                return false;
            }
            match &doc.blast_radius {
                Some(paths) => paths.iter().any(|p| changed_set.contains(p)),
                None => false,
            }
        })
        .collect();

    matched.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod source_code_detection {
        use super::*;

        #[test]
        fn rust_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("src/main.rs")));
        }

        #[test]
        fn typescript_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("src/index.ts")));
        }

        #[test]
        fn vue_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("components/App.vue")));
        }

        #[test]
        fn css_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("styles/main.css")));
        }

        #[test]
        fn html_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("public/index.html")));
        }

        #[test]
        fn htm_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("public/page.htm")));
        }

        #[test]
        fn markdown_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("docs/README.md")));
        }

        #[test]
        fn json_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("config.json")));
        }

        #[test]
        fn png_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("images/logo.png")));
        }

        #[test]
        fn no_extension_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("Makefile")));
        }

        #[test]
        fn scss_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("styles/theme.scss")));
        }

        #[test]
        fn python_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("script.py")));
        }

        #[test]
        fn toml_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("Cargo.toml")));
        }

        #[test]
        fn yaml_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("config.yaml")));
        }
    }

    mod changed_path_collection {
        use super::*;

        #[test]
        fn collect_dirty_returns_paths_in_real_repo() {
            // This repo has dirty files (we're on a feature branch with changes)
            let result = collect_changed_paths(
                Path::new("."),
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            );
            assert!(result.is_ok());
            // Paths should be sorted
            let r = result.unwrap();
            let sorted: Vec<_> = {
                let mut v = r.paths.clone();
                v.sort();
                v
            };
            assert_eq!(r.paths, sorted);
        }

        #[test]
        fn source_code_filter_excludes_non_source() {
            let result = collect_changed_paths(
                Path::new("."),
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::SourceCode,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            for path in &result.paths {
                assert!(
                    is_source_code_path(path),
                    "Expected source code path: {:?}",
                    path
                );
            }
        }

        #[test]
        fn substring_filter_or_logic() {
            let result = collect_changed_paths(
                Path::new("."),
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: vec!["blast_radius".to_string()],
                },
            )
            .unwrap();

            for path in &result.paths {
                assert!(
                    path.to_string_lossy().to_lowercase().contains("blast_radius"),
                    "Expected path to match filter: {:?}",
                    path
                );
            }
        }

        #[test]
        fn last_commit_returns_paths() {
            let result = collect_changed_paths(
                Path::new("."),
                &ChangedPathQuery {
                    scope: ChangeScope::LastCommit,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            );
            assert!(result.is_ok());
        }

        #[test]
        fn not_a_repo_returns_error() {
            let result = collect_changed_paths(
                Path::new("/tmp"),
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            );
            assert!(result.is_err());
        }
    }
}
