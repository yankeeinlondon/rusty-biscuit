//! Blast-radius analysis: source-code detection, changed-path collection,
//! and document matching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::filesystem::docs::{MarkdownMeta, detect_blast_radius_docs};
use crate::filesystem::git::{GitRepo, get_commit_files};
pub use crate::filesystem::path_kind::{is_documentation_path, is_source_code_path};
use crate::filesystem::repo::detect_repo;
use crate::{Result, SniffError};

// ---------------------------------------------------------------------------
// Changed-path collection
// ---------------------------------------------------------------------------

/// Which set of changes to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeScope {
    /// Staged + Modified + Both + Untracked (deduplicated).
    Dirty,
    /// Staged + Both only.
    Staged,
    /// Modified + Both only (excludes untracked).
    Unstaged,
    /// Untracked files only (new files not yet under version control).
    Untracked,
    /// Files changed in the HEAD commit.
    LastCommit,
}

/// Whether to return all files or only source code files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
/// Returns `SniffError::NotARepository` if `base_dir` is not inside a git repo,
/// or if the repository is bare. Returns an error if `package` or
/// `package_area` is specified but the repo is not a monorepo.
pub fn collect_changed_paths(
    base_dir: &Path,
    query: &ChangedPathQuery,
) -> Result<ChangedPathResult> {
    let git_repo = GitRepo::discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    // Every scope here is defined against a checkout; a bare repository has no
    // working tree to diff and no package layout to filter by.
    if git_repo.is_bare() {
        return Err(SniffError::NotARepository(base_dir.to_path_buf()));
    }
    let repo_root = git_repo.repo_root().to_path_buf();

    let raw_paths = match query.scope {
        ChangeScope::LastCommit => {
            let repo = crate::filesystem::git::open::trusted_open(&repo_root)?;
            collect_last_commit_paths(&repo)
        }
        _ => collect_working_tree_paths(&git_repo, query.scope)?,
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
        let repo_info =
            detect_repo(&repo_root)?.ok_or_else(|| SniffError::NotAMonorepo(repo_root.clone()))?;

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
                        // Prefix semantics: --package-area foo matches foo, foo/bar, etc.
                        let pkg_area = pkg.package_area.to_ascii_lowercase();
                        let target = area.to_ascii_lowercase();
                        pkg_area == target || pkg_area.starts_with(&format!("{target}/"))
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

            // Validate that the package/area name matched at least one package
            if matching_roots.is_empty() {
                if let Some(ref name) = query.package {
                    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
                    names.sort();
                    names.dedup();
                    return Err(SniffError::UnknownPackage {
                        name: name.clone(),
                        valid: names.join(", "),
                    });
                }
                if let Some(ref area) = query.package_area {
                    let mut areas: Vec<&str> =
                        packages.iter().map(|p| p.package_area.as_str()).collect();
                    areas.sort();
                    areas.dedup();
                    return Err(SniffError::UnknownPackageArea {
                        area: area.clone(),
                        valid: areas.join(", "),
                    });
                }
            }

            paths.retain(|p| matching_roots.iter().any(|root| p.starts_with(root)));
        }
    }

    // Apply substring filters (OR logic, case-insensitive)
    if !query.filters.is_empty() {
        let lowered_filters: Vec<String> = query.filters.iter().map(|f| f.to_lowercase()).collect();
        paths.retain(|p| {
            let p_str = p.to_string_lossy().to_lowercase();
            lowered_filters.iter().any(|f| p_str.contains(f.as_str()))
        });
    }

    // Sort and deduplicate
    paths.sort();
    paths.dedup();

    Ok(ChangedPathResult { repo_root, paths })
}

/// Collect paths from working tree status (Dirty/Staged/Unstaged).
///
/// Uses the shared gix status implementation so blast-radius and detailed git
/// status share a single classification source of truth.
fn collect_working_tree_paths(repo: &GitRepo, scope: ChangeScope) -> Result<Vec<PathBuf>> {
    use crate::filesystem::git::FileStatus;

    let file_changes = repo.file_changes()?;
    let mut seen = HashSet::new();
    let mut paths = Vec::new();

    for change in file_changes {
        let include = matches!(
            (scope, change.status),
            (ChangeScope::Dirty, _)
                | (ChangeScope::Staged, FileStatus::Staged | FileStatus::Both)
                | (
                    ChangeScope::Unstaged,
                    FileStatus::Modified | FileStatus::Both | FileStatus::Conflicted,
                )
                | (ChangeScope::Untracked, FileStatus::Untracked)
        );

        if include && seen.insert(change.path.clone()) {
            paths.push(change.path);
        }
    }

    Ok(paths)
}

/// Collect paths from the HEAD commit.
fn collect_last_commit_paths(repo: &gix::Repository) -> Vec<PathBuf> {
    let Ok(head_id) = repo.head_id() else {
        return Vec::new();
    };

    get_commit_files(repo, head_id.into())
        .into_iter()
        .map(|(path, _kind)| path)
        .collect()
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
#[instrument(skip_all)]
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

    if changed_set.is_empty() {
        return Ok(Vec::new());
    }

    // Use the blast-radius-only parser: documents are streamed until the
    // closing frontmatter delimiter and only the `blast_radius` key is parsed.
    // Docs without `blast_radius` are never returned by this scanner.
    let docs = match detect_blast_radius_docs(&result.repo_root) {
        Some(docs) => docs,
        None => return Ok(Vec::new()),
    };

    let mut matched: Vec<MarkdownMeta> = docs
        .into_iter()
        .filter(|doc| match &doc.blast_radius {
            Some(paths) => paths.iter().any(|p| changed_set.contains(p)),
            None => false,
        })
        .collect();

    matched.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod changed_path_collection {
        use super::*;

        #[test]
        fn collect_dirty_returns_paths_in_real_repo() {
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
                    path.to_string_lossy()
                        .to_lowercase()
                        .contains("blast_radius"),
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

    /// Helper: create a temp git repo with an initial commit.
    /// Returns (TempDir, repo_path).
    fn create_temp_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Configure git identity for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        config.set_str("user.name", "Test").unwrap();

        // Create initial commit so HEAD exists
        let sig = repo.signature().unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let path = dir.path().to_path_buf();
        (dir, path)
    }

    /// Helper: write a file, add to index, and commit.
    fn commit_file(repo_path: &Path, relative: &str, content: &str) {
        let full = repo_path.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();

        let repo = git2::Repository::open(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();

        let sig = repo.signature().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add file", &tree, &[&head])
            .unwrap();
    }

    /// Helper: write a file and stage it (but don't commit).
    fn stage_file(repo_path: &Path, relative: &str, content: &str) {
        let full = repo_path.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();

        let repo = git2::Repository::open(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    mod temp_repo_changed_paths {
        use super::*;

        #[test]
        fn staged_scope_only_returns_staged_files() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            commit_file(&path, "src/lib.rs", "pub fn lib() {}");

            stage_file(&path, "src/main.rs", "fn main() { updated }");
            std::fs::write(path.join("src/lib.rs"), "pub fn lib() { modified }").unwrap();
            std::fs::write(path.join("src/new.rs"), "fn new() {}").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Staged,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert_eq!(result.paths, vec![PathBuf::from("src/main.rs")]);
        }

        #[test]
        fn unstaged_scope_only_returns_modified_files() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { modified }").unwrap();
            std::fs::write(path.join("src/new.rs"), "fn new() {}").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Unstaged,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert_eq!(result.paths, vec![PathBuf::from("src/main.rs")]);
        }

        #[test]
        fn dirty_scope_returns_staged_unstaged_and_untracked() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/a.rs", "a");
            commit_file(&path, "src/b.rs", "b");

            stage_file(&path, "src/a.rs", "a modified");
            std::fs::write(path.join("src/b.rs"), "b modified").unwrap();
            std::fs::write(path.join("src/c.rs"), "c").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(result.paths.contains(&PathBuf::from("src/a.rs")));
            assert!(result.paths.contains(&PathBuf::from("src/b.rs")));
            assert!(result.paths.contains(&PathBuf::from("src/c.rs")));
        }

        #[test]
        fn last_commit_returns_files_from_head() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/committed.rs", "committed");

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::LastCommit,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(result.paths.contains(&PathBuf::from("src/committed.rs")));
        }

        #[test]
        fn source_code_kind_filters_non_source() {
            let (_dir, path) = create_temp_repo();
            stage_file(&path, "src/main.rs", "fn main() {}");
            stage_file(&path, "docs/readme.md", "# readme");
            stage_file(&path, "data.json", "{}");

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Staged,
                    kind: ChangedPathKind::SourceCode,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert_eq!(result.paths, vec![PathBuf::from("src/main.rs")]);
        }

        #[test]
        fn deleted_file_included_in_staged() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/old.rs", "old code");

            std::fs::remove_file(path.join("src/old.rs")).unwrap();
            let repo = git2::Repository::open(&path).unwrap();
            let mut index = repo.index().unwrap();
            index.remove_path(Path::new("src/old.rs")).unwrap();
            index.write().unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Staged,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(result.paths.contains(&PathBuf::from("src/old.rs")));
        }
    }

    /// Helper: set up the temp repo as a Cargo workspace with given members.
    ///
    /// Each member path like `"sniff/lib"` gets package name `"sniff-lib"` (slashes → dashes).
    fn make_workspace(repo_path: &Path, members: &[&str]) {
        let member_list: Vec<String> = members.iter().map(|m| format!("    \"{m}\"")).collect();
        let cargo_toml = format!("[workspace]\nmembers = [\n{}\n]\n", member_list.join(",\n"));
        commit_file(repo_path, "Cargo.toml", &cargo_toml);

        // Create a Cargo.toml for each member package
        for member in members {
            let name = member.replace('/', "-");
            let pkg_toml =
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
            commit_file(repo_path, &format!("{member}/Cargo.toml"), &pkg_toml);
            commit_file(repo_path, &format!("{member}/src/lib.rs"), "// placeholder");
        }
    }

    mod package_scoping {
        use super::*;

        #[test]
        fn exact_package_match_filters_paths() {
            let (_dir, path) = create_temp_repo();
            make_workspace(&path, &["sniff/lib", "homelab/lib"]);

            std::fs::write(path.join("sniff/lib/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("homelab/lib/src/lib.rs"), "// dirty").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: Some("sniff-lib".to_string()),
                    package_area: None,
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(
                result.paths.iter().all(|p| p.starts_with("sniff/")),
                "All paths should be in sniff/: {:?}",
                result.paths
            );
            assert!(
                !result.paths.iter().any(|p| p.starts_with("homelab/")),
                "Should not include homelab/ paths"
            );
        }

        #[test]
        fn exact_package_area_match() {
            let (_dir, path) = create_temp_repo();
            make_workspace(&path, &["sniff/lib", "sniff/cli", "homelab/lib"]);

            std::fs::write(path.join("sniff/lib/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("sniff/cli/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("homelab/lib/src/lib.rs"), "// dirty").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: Some("sniff".to_string()),
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(
                result.paths.iter().all(|p| p.starts_with("sniff/")),
                "All paths should be in sniff/: {:?}",
                result.paths
            );
            assert!(
                result.paths.len() >= 2,
                "Should match both sniff/lib and sniff/cli"
            );
        }

        #[test]
        fn nested_package_area_prefix_match() {
            let (_dir, path) = create_temp_repo();
            make_workspace(
                &path,
                &["apps/web", "apps/api", "apps/api/workers", "libs/core"],
            );

            std::fs::write(path.join("apps/web/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("apps/api/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("apps/api/workers/src/lib.rs"), "// dirty").unwrap();
            std::fs::write(path.join("libs/core/src/lib.rs"), "// dirty").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: Some("apps".to_string()),
                    filters: Vec::new(),
                },
            )
            .unwrap();

            assert!(
                result.paths.iter().all(|p| p.starts_with("apps/")),
                "All paths should be in apps/: {:?}",
                result.paths
            );
            assert!(
                !result.paths.iter().any(|p| p.starts_with("libs/")),
                "Should not include libs/ paths"
            );
        }

        #[test]
        fn unknown_package_returns_error() {
            let (_dir, path) = create_temp_repo();
            make_workspace(&path, &["sniff/lib", "sniff/cli"]);

            std::fs::write(path.join("sniff/lib/src/lib.rs"), "// dirty").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: Some("nonexistent".to_string()),
                    package_area: None,
                    filters: Vec::new(),
                },
            );

            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("nonexistent") && err.contains("not found"),
                "Error should mention the package name: {err}"
            );
        }

        #[test]
        fn unknown_package_area_returns_error() {
            let (_dir, path) = create_temp_repo();
            make_workspace(&path, &["sniff/lib", "sniff/cli"]);

            std::fs::write(path.join("sniff/lib/src/lib.rs"), "// dirty").unwrap();

            let result = collect_changed_paths(
                &path,
                &ChangedPathQuery {
                    scope: ChangeScope::Dirty,
                    kind: ChangedPathKind::AllFiles,
                    package: None,
                    package_area: Some("bogus".to_string()),
                    filters: Vec::new(),
                },
            );

            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("bogus") && err.contains("not found"),
                "Error should mention the area name: {err}"
            );
        }
    }

    mod blast_radius_document_matching {
        use super::*;

        #[test]
        fn matched_document_returned() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_content = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
            commit_file(&path, "docs/guide.md", doc_content);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert_eq!(matched.len(), 1);
            assert_eq!(matched[0].relative, "docs/guide.md");
        }

        #[test]
        fn unmatched_document_not_returned() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_content = "---\ntitle: Other\nblast_radius:\n  - src/other.rs\n---\n# Other\n";
            commit_file(&path, "docs/other.md", doc_content);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert!(matched.is_empty());
        }

        #[test]
        fn document_with_empty_blast_radius_not_returned() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_content = "---\ntitle: Empty\nblast_radius: []\n---\n# Empty\n";
            commit_file(&path, "docs/empty.md", doc_content);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert!(matched.is_empty());
        }

        #[test]
        fn document_without_blast_radius_not_returned() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_content = "---\ntitle: No BR\n---\n# No BR\n";
            commit_file(&path, "docs/nobr.md", doc_content);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert!(matched.is_empty());
        }

        #[test]
        fn no_changed_files_returns_empty() {
            let (_dir, path) = create_temp_repo();
            let doc_content = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
            commit_file(&path, "docs/guide.md", doc_content);
            commit_file(&path, "src/main.rs", "fn main() {}");

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert!(matched.is_empty());
        }

        #[test]
        fn staged_scope_only_matches_staged_changes() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/a.rs", "a");
            commit_file(&path, "src/b.rs", "b");

            let doc_content =
                "---\ntitle: Guide\nblast_radius:\n  - src/a.rs\n  - src/b.rs\n---\n# Guide\n";
            commit_file(&path, "docs/guide.md", doc_content);

            stage_file(&path, "src/a.rs", "a modified");
            std::fs::write(path.join("src/b.rs"), "b modified").unwrap();

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Staged, None, None).unwrap();
            assert_eq!(matched.len(), 1);
            assert_eq!(matched[0].relative, "docs/guide.md");
        }

        #[test]
        fn normalized_dot_slash_path_matches() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_content = "---\ntitle: Guide\nblast_radius:\n  - ./src/main.rs\n---\n# Guide\n";
            commit_file(&path, "docs/guide.md", doc_content);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert_eq!(matched.len(), 1);
        }

        #[test]
        fn results_sorted_by_relative_path() {
            let (_dir, path) = create_temp_repo();
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            let doc_a = "---\ntitle: Z Doc\nblast_radius:\n  - src/main.rs\n---\n# Z Doc\n";
            let doc_b = "---\ntitle: A Doc\nblast_radius:\n  - src/main.rs\n---\n# A Doc\n";
            commit_file(&path, "docs/z_doc.md", doc_a);
            commit_file(&path, "docs/a_doc.md", doc_b);

            let matched =
                find_blast_radius_documents(&path, ChangeScope::Dirty, None, None).unwrap();
            assert_eq!(matched.len(), 2);
            assert!(matched[0].relative < matched[1].relative);
        }
    }
}
