//! Blast-radius analysis: source-code detection, changed-path collection,
//! and document matching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git2::Repository;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::filesystem::docs::{detect_docs, MarkdownMeta};
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

/// Returns `true` if the path refers to a documentation file.
///
/// Classification uses the file-type registry (exact filename match, then
/// extension match). Paths whose association is `Documentation` are considered
/// documentation. This includes bare filenames like `README`, `CHANGELOG`,
/// and `CONTRIBUTING` (without extension) as well as extension-based matches.
pub fn is_documentation_path(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    if let Some(desc) = lookup_exact_filename(file_name) {
        if matches!(desc.association, FileAssociation::Documentation) {
            return true;
        }
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_ascii_lowercase();
        if matches!(lower.as_str(), "md" | "mdx" | "rst" | "txt" | "adoc") {
            return true;
        }

        if let Some(desc) = lookup_extension(ext) {
            return matches!(desc.association, FileAssociation::Documentation);
        }
    }

    false
}

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
/// Returns `SniffError::NotARepository` if `base_dir` is not inside a git repo.
/// Returns an error if `package` or `package_area` is specified but the repo
/// is not a monorepo.
pub fn collect_changed_paths(
    base_dir: &Path,
    query: &ChangedPathQuery,
) -> Result<ChangedPathResult> {
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
        .map_err(|e| {
            debug!(error = %e, "Failed to get repository HEAD");
            e
        })
        .ok()
        .and_then(|h| {
            h.peel_to_commit()
                .map_err(|e| {
                    debug!(error = %e, "Failed to peel HEAD to commit");
                    e
                })
                .ok()
        })
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

    mod documentation_detection {
        use super::*;

        #[test]
        fn markdown_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/README.md")));
        }

        #[test]
        fn mdx_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/guide.mdx")));
        }

        #[test]
        fn rst_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/index.rst")));
        }

        #[test]
        fn txt_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("LICENSE.txt")));
        }

        #[test]
        fn adoc_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/guide.adoc")));
        }

        #[test]
        fn bare_readme_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("README")));
        }

        #[test]
        fn bare_changelog_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("CHANGELOG")));
        }

        #[test]
        fn bare_contributing_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("CONTRIBUTING")));
        }

        #[test]
        fn readme_md_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("readme.md")));
        }

        #[test]
        fn changelog_md_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("changelog.md")));
        }

        #[test]
        fn rust_file_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("src/lib.rs")));
        }

        #[test]
        fn json_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("package.json")));
        }

        #[test]
        fn png_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("images/logo.png")));
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
        let repo = Repository::init(dir.path()).unwrap();

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

        let repo = Repository::open(repo_path).unwrap();
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

        let repo = Repository::open(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    mod temp_repo_changed_paths {
        use super::*;

        #[test]
        fn staged_scope_only_returns_staged_files() {
            let (_dir, path) = create_temp_repo();
            // Commit both files first
            commit_file(&path, "src/main.rs", "fn main() {}");
            commit_file(&path, "src/lib.rs", "pub fn lib() {}");

            // Stage a modification to main.rs
            stage_file(&path, "src/main.rs", "fn main() { updated }");
            // Modify lib.rs without staging (unstaged only)
            std::fs::write(path.join("src/lib.rs"), "pub fn lib() { modified }").unwrap();
            // Create an untracked file
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
            // Commit and then modify without staging
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { modified }").unwrap();
            // Create an untracked file (should NOT be in unstaged)
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
            // Commit all files first
            commit_file(&path, "src/a.rs", "a");
            commit_file(&path, "src/b.rs", "b");

            // Stage a modification to a.rs
            stage_file(&path, "src/a.rs", "a modified");
            // Modify b.rs without staging (unstaged)
            std::fs::write(path.join("src/b.rs"), "b modified").unwrap();
            // Untracked file
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

            // Delete and stage the deletion
            std::fs::remove_file(path.join("src/old.rs")).unwrap();
            let repo = Repository::open(&path).unwrap();
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
            // sniff/lib → package name "sniff-lib", homelab/lib → "homelab-lib"
            make_workspace(&path, &["sniff/lib", "homelab/lib"]);

            // Dirty files in both packages
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
            // Commit a source file, then dirty it
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            // Create a docs directory with a document referencing src/main.rs
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
            // Dirty a source file
            commit_file(&path, "src/main.rs", "fn main() {}");
            std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

            // Doc references a DIFFERENT file
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
            // Everything is committed, nothing dirty
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
            // Create two source files
            commit_file(&path, "src/a.rs", "a");
            commit_file(&path, "src/b.rs", "b");

            // Doc references both
            let doc_content =
                "---\ntitle: Guide\nblast_radius:\n  - src/a.rs\n  - src/b.rs\n---\n# Guide\n";
            commit_file(&path, "docs/guide.md", doc_content);

            // Only stage changes to a.rs
            stage_file(&path, "src/a.rs", "a modified");
            // Modify b.rs without staging (unstaged only)
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

            // Doc uses ./src/main.rs (should be normalized to src/main.rs)
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
