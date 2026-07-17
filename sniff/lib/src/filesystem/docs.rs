use super::repo::detect_repo_structure;
use super::repo::ownership::PackageOwnershipIndex;
use crate::performance;
use crate::performance::counters;
use crate::{Result, SniffError};
use biscuit_file::serde_yaml_ng;
use biscuit_hash::xx_hash;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

/// Parsing detail level for `parse_markdown_meta_with_mode`.
///
/// Some callers (e.g. blast-radius scans) only need the `blast_radius`
/// frontmatter key. Selecting [`DocParseMode::BlastRadiusOnly`] lets the
/// parser stop reading at the closing frontmatter delimiter, skip body
/// hashing, and skip the title/`last_updated` work that the full parser
/// performs on every document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocParseMode {
    /// Read and parse the full document — frontmatter, body, hash, title, mtime.
    Full,
    /// Read only the frontmatter and extract `blast_radius` plus its presence flag.
    ///
    /// Other [`MarkdownMeta`] fields are filled with cheap defaults so the
    /// returned value still satisfies the public type's invariants but is
    /// only suitable for blast-radius matching.
    BlastRadiusOnly,
    /// Read only the frontmatter — extracts `prompt`, `title` (from frontmatter only),
    /// `frontmatter_keys`, and `has_blast_radius`. Skips body hashing, title scanning
    /// (H1/H2/H3), and mtime resolution. Suitable for filtered queries where only
    /// frontmatter-derived fields are consumed.
    FrontmatterOnly,
}

/// How the document title was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TitleSource {
    FrontmatterTitle,
    H1Heading,
    H2Heading,
    H3Heading,
    None,
}

/// How the last-updated timestamp was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatedSource {
    UpdatedProperty,
    FileMetadata,
}

/// Metadata for a markdown document in a git repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMeta {
    /// Fully qualified file path to the document.
    pub filepath: PathBuf,
    /// Relative file path from the repo root directory.
    pub relative: String,
    /// The package in the monorepo this file resides in.
    /// None if not a monorepo or file is in the root folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// Document title extracted from frontmatter "title" field,
    /// first H1, first H2, first H3, or empty string (in that priority).
    pub title: String,
    /// How the title was resolved.
    pub title_source: TitleSource,
    /// The frontmatter "model" property if defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The frontmatter "prompt" property if defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Last updated timestamp resolved from frontmatter or file mtime.
    pub last_updated: DateTime<Utc>,
    /// How the last-updated timestamp was resolved.
    pub updated_source: UpdatedSource,
    /// xxHash (XXH64) of the document body (content after frontmatter).
    pub content_hash: String,
    /// Whether the frontmatter contains a `blast_radius` key (even if empty).
    #[serde(default)]
    pub has_blast_radius: bool,
    /// Repo-relative paths from the `blast_radius` frontmatter list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blast_radius: Option<Vec<PathBuf>>,
    /// Sorted set of frontmatter keys present in this document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frontmatter_keys: Vec<String>,
}

/// Discovers and analyzes markdown documents in a git repository.
pub struct RepoDocuments {
    /// The git repository root directory.
    repo_root: PathBuf,
    /// Monorepo package directories (name, relative path from repo root).
    packages: Vec<(String, PathBuf)>,
    ownership_index: PackageOwnershipIndex,
}

impl RepoDocuments {
    /// Create a new `RepoDocuments` by discovering the git repo root from
    /// the given directory path.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::NotARepository` if the path is not inside a git repo.
    pub fn new(path: &Path) -> Result<Self> {
        // Trust/permission/I/O/corruption failures surface via `?`; only genuine
        // repository absence becomes `NotARepository`.
        let repo_root = crate::filesystem::repo_root(path)?
            .ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?;

        let packages = detect_repo_packages(&repo_root);
        let ownership_index = PackageOwnershipIndex::from_relative_paths(&repo_root, &packages);

        Ok(Self {
            repo_root,
            packages,
            ownership_index,
        })
    }

    /// Create a `RepoDocuments` from an already-resolved repo root and
    /// pre-computed package list, skipping git discovery and repo detection.
    pub fn from_root(repo_root: PathBuf, packages: Vec<(String, PathBuf)>) -> Self {
        let ownership_index = PackageOwnershipIndex::from_relative_paths(&repo_root, &packages);
        Self {
            repo_root,
            packages,
            ownership_index,
        }
    }

    /// Returns metadata for all markdown documents in the repository.
    pub fn documents(&self) -> Vec<MarkdownMeta> {
        collect_markdown_files_with_index(
            &self.repo_root,
            &self.packages,
            &self.ownership_index,
        )
    }

    /// Returns metadata for markdown documents that have a "prompt" property set.
    pub fn prompt_docs(&self) -> Vec<MarkdownMeta> {
        self.documents()
            .into_iter()
            .filter(|doc| doc.prompt.is_some())
            .collect()
    }
}

/// Detect markdown documents in the given directory (standalone function).
///
/// This matches sniff's detection function pattern. Returns `None` if the
/// directory is not inside a git repository.
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_docs(root: &Path) -> Option<Vec<MarkdownMeta>> {
    let repo_docs = RepoDocuments::new(root).ok()?;
    let docs = repo_docs.documents();
    if docs.is_empty() { None } else { Some(docs) }
}

/// Resolve the monorepo package list using structure-only detection.
///
/// Returns `(package_name, repo_relative_path)` pairs without performing
/// package-manager, dependency, test-runner, feature, language, framework, or
/// file-list enrichment.
///
/// [`RepoRequest::structure`]: crate::request::RepoRequest::structure
pub fn detect_repo_packages(repo_root: &Path) -> Vec<(String, PathBuf)> {
    detect_repo_structure(repo_root)
        .ok()
        .flatten()
        .and_then(|info| info.packages)
        .map(|pkgs| {
            pkgs.into_iter()
                .map(|p| {
                    let rel_path = p
                        .path
                        .strip_prefix(repo_root)
                        .unwrap_or(&p.path)
                        .to_path_buf();
                    (p.name, rel_path)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// Lightweight package discovery within specific directories.
///
/// Unlike [`detect_repo_packages`] which runs full workspace detection across
/// the entire repository, this function only scans `Cargo.toml` files in the
/// given directories (and their immediate children). This is sufficient to
/// populate the `package` field on [`MarkdownMeta`] when the caller already
/// knows the target area (e.g. from `--package-area sniff`).
///
/// Returns `(package_name, repo_relative_path)` pairs.
pub fn detect_packages_in_dirs(repo_root: &Path, dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut packages = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkBuilder::new(dir)
            .max_depth(Some(3))
            .hidden(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path
                .file_name()
                .is_some_and(|n| n == "Cargo.toml" || n == "package.json")
            {
                continue;
            }
            let rel = match path.parent().and_then(|p| p.strip_prefix(repo_root).ok()) {
                Some(r) => r.to_path_buf(),
                None => continue,
            };
            let name = if path.file_name().is_some_and(|n| n == "Cargo.toml") {
                fs::read_to_string(path).ok().and_then(|c| {
                    biscuit_file::toml_crate::from_str::<biscuit_file::toml_crate::Value>(&c)
                        .ok()
                        .and_then(|v| {
                            v.get("package")
                                .and_then(|p| p.get("name"))
                                .and_then(|n| n.as_str().map(|s| s.to_string()))
                        })
                })
            } else {
                fs::read_to_string(path).ok().and_then(|c| {
                    serde_json::from_str::<serde_json::Value>(&c)
                        .ok()
                        .and_then(|v| {
                            v.get("name")
                                .and_then(|n| n.as_str().map(|s| s.to_string()))
                        })
                })
            };
            if let Some(name) = name {
                packages.push((name, rel));
            }
        }
    }
    packages
}

/// Detect markdown documents using a pre-resolved repo root and package list.
///
/// Skips `Repository::discover` and full repo detection, going straight to
/// file collection. Use when the caller already knows the repo root.
pub fn detect_docs_from_root(
    repo_root: &Path,
    packages: &[(String, PathBuf)],
) -> Option<Vec<MarkdownMeta>> {
    let repo_docs = RepoDocuments::from_root(repo_root.to_path_buf(), packages.to_vec());
    let docs = repo_docs.documents();
    if docs.is_empty() { None } else { Some(docs) }
}

/// Detect markdown documents that declare a `blast_radius` frontmatter key.
///
/// Streams each markdown file only until the closing frontmatter delimiter
/// and parses just the `blast_radius` key, skipping content hashing, mtime
/// resolution, and title extraction. Documents without `blast_radius` are
/// not returned at all.
///
/// Returns `None` when the directory is not inside a git repository.
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_blast_radius_docs(root: &Path) -> Option<Vec<MarkdownMeta>> {
    let repo_root = crate::filesystem::repo_root(root).ok().flatten()?;

    let walker = WalkBuilder::new(&repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let paths: Vec<_> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let collector = performance::current_collector();
    let mut docs: Vec<MarkdownMeta> = paths
        .into_par_iter()
        .filter_map(|path| {
            let _worker = performance::pooled_worker(collector.as_ref());
            parse_markdown_meta_with_mode(&path, &repo_root, &[], DocParseMode::BlastRadiusOnly)
        })
        .filter(|doc| doc.has_blast_radius)
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    if docs.is_empty() { None } else { Some(docs) }
}

/// Detect markdown documents using pre-computed package info.
///
/// Avoids the redundant `detect_repo()` call that `detect_docs()` performs
/// internally. Use this when repo/package info has already been gathered.
pub fn detect_docs_with_packages(
    repo_root: &Path,
    packages: &[(String, PathBuf)],
) -> Option<Vec<MarkdownMeta>> {
    let docs = collect_markdown_files(repo_root, packages);
    if docs.is_empty() { None } else { Some(docs) }
}

/// Collect all markdown files from repo root using .gitignore-aware walking.
fn collect_markdown_files(repo_root: &Path, packages: &[(String, PathBuf)]) -> Vec<MarkdownMeta> {
    let ownership_index = PackageOwnershipIndex::from_relative_paths(repo_root, packages);
    collect_markdown_files_with_index(repo_root, packages, &ownership_index)
}

fn collect_markdown_files_with_index(
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    ownership_index: &PackageOwnershipIndex,
) -> Vec<MarkdownMeta> {
    let walker = WalkBuilder::new(repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let paths: Vec<_> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let collector = performance::current_collector();
    let mut docs: Vec<MarkdownMeta> = paths
        .into_par_iter()
        .filter_map(|path| {
            let _worker = performance::pooled_worker(collector.as_ref());
            parse_markdown_meta_with_ownership(
                &path,
                repo_root,
                packages,
                Some(ownership_index),
                DocParseMode::Full,
            )
        })
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    docs
}

pub fn collect_markdown_files_filtered<F>(
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    mode: DocParseMode,
    path_filter: F,
) -> Vec<MarkdownMeta>
where
    F: Fn(&str) -> bool + Sync,
{
    let ownership_index = PackageOwnershipIndex::from_relative_paths(repo_root, packages);
    let walker = WalkBuilder::new(repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let paths: Vec<_> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .filter(|entry| {
            entry
                .path()
                .strip_prefix(repo_root)
                .map(|rel| path_filter(&rel.to_string_lossy()))
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let collector = performance::current_collector();
    let mut docs: Vec<MarkdownMeta> = paths
        .into_par_iter()
        .filter_map(|path| {
            let _worker = performance::pooled_worker(collector.as_ref());
            parse_markdown_meta_with_ownership(
                &path,
                repo_root,
                packages,
                Some(&ownership_index),
                mode,
            )
        })
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    docs
}

/// Collect every markdown file path under `root`, honoring `.gitignore`.
///
/// This is a completion-oriented adapter that returns raw paths only,
/// without parsing frontmatter or building [`MarkdownMeta`]. It reuses the
/// same `ignore::WalkBuilder` configuration as [`RepoDocuments::documents`]
/// (hidden files allowed; repo, global, and local gitignore applied) so
/// there is exactly one exclusion policy for markdown discovery in sniff.
///
/// The returned paths preserve whatever form `root` gives them, so callers
/// that need canonical-path deduplication (e.g. when package-root and
/// package-area-root coincide) should [`canonicalize`] the results
/// themselves.
///
/// Unlike [`RepoDocuments`], this function does not require `root` to be
/// the root of a git repository. Any directory is acceptable; callers pick
/// the scope (repo root, curated subdirectory, etc.) that fits their
/// completion context.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::docs::collect_markdown_paths;
/// use std::path::Path;
///
/// let paths = collect_markdown_paths(Path::new("/path/to/repo"));
/// for path in paths {
///     println!("{}", path.display());
/// }
/// ```
///
/// [`canonicalize`]: std::fs::canonicalize
pub fn collect_markdown_paths(root: &Path) -> Vec<PathBuf> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// Walk one or more root directories and return every markdown file path.
///
/// This is the multi-root counterpart to [`collect_markdown_paths`]. Each
/// directory in `dirs` is walked independently with the same gitignore
/// configuration. Pass a single-element `dirs` containing the repo root to
/// reproduce [`collect_markdown_paths`] behaviour; pass targeted
/// sub-directories (e.g. `["sniff/"]`) to avoid walking the entire
/// repository.
pub fn collect_markdown_paths_from_dirs(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkBuilder::new(dir)
            .hidden(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        let paths: Vec<_> = walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_some_and(|ft| ft.is_file())
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        all.extend(paths);
    }
    all
}

/// Fast path for collecting markdown paths without gitignore checking.
///
/// Intended for targeted directory walks (e.g. `--package-area sniff`)
/// where the directory is known to be curated and the overhead of
/// gitignore resolution is not justified by the small file count.
/// Do NOT use for full-repo walks — without gitignore the walker will
/// descend into `target/`, `.git/`, `node_modules/`, etc.
pub fn collect_markdown_paths_fast(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkBuilder::new(dir)
            .hidden(true)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .build();
        let paths: Vec<_> = walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_some_and(|ft| ft.is_file())
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        all.extend(paths);
    }
    all
}

/// Walk one or more root directories, parse matching markdown files, and
/// return their [`MarkdownMeta`].
///
/// This is the multi-root counterpart to
/// [`collect_markdown_files_filtered`]. When `dirs` contains targeted
/// sub-directories (e.g. derived from `--package-area` flags), only those
/// directories are walked — skipping potentially thousands of unrelated
/// files in the rest of the repository.
pub fn collect_markdown_files_from_dirs<F>(
    dirs: &[PathBuf],
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    mode: DocParseMode,
    path_filter: F,
) -> Vec<MarkdownMeta>
where
    F: Fn(&str) -> bool + Sync,
{
    let ownership_index = PackageOwnershipIndex::from_relative_paths(repo_root, packages);
    let mut all_paths = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkBuilder::new(dir)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        let paths: Vec<_> = walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_some_and(|ft| ft.is_file())
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .filter(|entry| {
                entry
                    .path()
                    .strip_prefix(repo_root)
                    .map(|rel| path_filter(&rel.to_string_lossy()))
                    .unwrap_or(false)
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        all_paths.extend(paths);
    }

    let collector = performance::current_collector();
    let mut docs: Vec<MarkdownMeta> = all_paths
        .into_par_iter()
        .filter_map(|path| {
            let _worker = performance::pooled_worker(collector.as_ref());
            parse_markdown_meta_with_ownership(
                &path,
                repo_root,
                packages,
                Some(&ownership_index),
                mode,
            )
        })
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    docs
}

/// Parse a pre-resolved list of markdown file paths into [`MarkdownMeta`].
///
/// Unlike the `collect_*` functions that walk directories, this function
/// takes an explicit set of file paths — useful when a prior
/// [`FrontmatterOnly`] pass has already narrowed the candidates.
pub fn parse_markdown_files(
    paths: &[PathBuf],
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    mode: DocParseMode,
) -> Vec<MarkdownMeta> {
    let ownership_index = PackageOwnershipIndex::from_relative_paths(repo_root, packages);
    let collector = performance::current_collector();
    let mut docs: Vec<MarkdownMeta> = paths
        .into_par_iter()
        .filter_map(|path| {
            let _worker = performance::pooled_worker(collector.as_ref());
            parse_markdown_meta_with_ownership(
                path,
                repo_root,
                packages,
                Some(&ownership_index),
                mode,
            )
        })
        .collect();
    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    docs
}

/// Parse a single markdown file into its metadata.
pub(crate) fn parse_markdown_meta(
    path: &Path,
    repo_root: &Path,
    packages: &[(String, PathBuf)],
) -> Option<MarkdownMeta> {
    parse_markdown_meta_with_mode(path, repo_root, packages, DocParseMode::Full)
}

/// Parse a single markdown file with the requested level of detail.
///
/// [`DocParseMode::Full`] reproduces the historical behavior of
/// [`parse_markdown_meta`]. [`DocParseMode::BlastRadiusOnly`] streams only
/// until the closing frontmatter delimiter and parses just the `blast_radius`
/// key, skipping hashing, mtime resolution, and full-body title scanning.
pub(crate) fn parse_markdown_meta_with_mode(
    path: &Path,
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    mode: DocParseMode,
) -> Option<MarkdownMeta> {
    let ownership_index = (!packages.is_empty())
        .then(|| PackageOwnershipIndex::from_relative_paths(repo_root, packages));
    parse_markdown_meta_with_ownership(
        path,
        repo_root,
        packages,
        ownership_index.as_ref(),
        mode,
    )
}

fn parse_markdown_meta_with_ownership(
    path: &Path,
    repo_root: &Path,
    packages: &[(String, PathBuf)],
    ownership_index: Option<&PackageOwnershipIndex>,
    mode: DocParseMode,
) -> Option<MarkdownMeta> {
    let relative_path = path.strip_prefix(repo_root).ok()?;
    let relative = relative_path.to_string_lossy().to_string();
    let package = ownership_index
        .and_then(|index| determine_package_with_index(relative_path, packages, index));

    match mode {
        DocParseMode::Full => parse_markdown_meta_full(path, repo_root, package, relative),
        DocParseMode::BlastRadiusOnly => {
            parse_markdown_meta_blast_radius_only(path, repo_root, package, relative)
        }
        DocParseMode::FrontmatterOnly => {
            parse_markdown_meta_frontmatter_only(path, package, relative)
        }
    }
}

fn parse_markdown_meta_full(
    path: &Path,
    repo_root: &Path,
    package: Option<String>,
    relative: String,
) -> Option<MarkdownMeta> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = fs::read_to_string(path)
        .map_err(|e| {
            debug!(path = %path.display(), error = %e, "could not read doc file");
            e
        })
        .ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::FS_DOCS_PARSED, 1);
    let (frontmatter, body) = extract_frontmatter(&content);

    let (title, title_source) = extract_title(&frontmatter, body);
    let model = get_string_field(&frontmatter, "model");
    let prompt = get_string_field(&frontmatter, "prompt");
    let (last_updated, updated_source) = resolve_last_updated(&frontmatter, path);
    let content_hash = format!("{:x}", xx_hash(body));

    let has_blast_radius = frontmatter.contains_key("blast_radius");
    let blast_radius = parse_blast_radius(&frontmatter, repo_root);

    let mut frontmatter_keys: Vec<String> = frontmatter.keys().cloned().collect();
    frontmatter_keys.sort();

    Some(MarkdownMeta {
        filepath: path.to_path_buf(),
        relative,
        package,
        title,
        title_source,
        model,
        prompt,
        last_updated,
        updated_source,
        content_hash,
        has_blast_radius,
        blast_radius,
        frontmatter_keys,
    })
}

fn parse_markdown_meta_blast_radius_only(
    path: &Path,
    repo_root: &Path,
    package: Option<String>,
    relative: String,
) -> Option<MarkdownMeta> {
    let frontmatter = read_frontmatter_only(path)?;
    let has_blast_radius = frontmatter.contains_key("blast_radius");
    let blast_radius = parse_blast_radius(&frontmatter, repo_root);

    Some(MarkdownMeta {
        filepath: path.to_path_buf(),
        relative,
        package,
        title: String::new(),
        title_source: TitleSource::None,
        model: None,
        prompt: None,
        last_updated: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default(),
        updated_source: UpdatedSource::FileMetadata,
        content_hash: String::new(),
        has_blast_radius,
        blast_radius,
        frontmatter_keys: Vec::new(),
    })
}

fn parse_markdown_meta_frontmatter_only(
    path: &Path,
    package: Option<String>,
    relative: String,
) -> Option<MarkdownMeta> {
    let frontmatter = read_frontmatter_only(path)?;

    let title = get_string_field(&frontmatter, "title").unwrap_or_default();
    let title_source = if title.is_empty() {
        TitleSource::None
    } else {
        TitleSource::FrontmatterTitle
    };
    let model = get_string_field(&frontmatter, "model");
    let prompt = get_string_field(&frontmatter, "prompt");
    let has_blast_radius = frontmatter.contains_key("blast_radius");

    let mut frontmatter_keys: Vec<String> = frontmatter.keys().cloned().collect();
    frontmatter_keys.sort();

    Some(MarkdownMeta {
        filepath: path.to_path_buf(),
        relative,
        package,
        title,
        title_source,
        model,
        prompt,
        last_updated: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default(),
        updated_source: UpdatedSource::FileMetadata,
        content_hash: String::new(),
        has_blast_radius,
        blast_radius: None,
        frontmatter_keys,
    })
}

/// Stream-read just the YAML frontmatter from `path` and parse it.
///
/// Stops reading at the closing `---` delimiter, so large documents are not
/// loaded into memory when only frontmatter keys are needed. Returns an empty
/// map when the file has no frontmatter, and `None` when the file cannot be
/// opened or its frontmatter is malformed.
fn read_frontmatter_only(path: &Path) -> Option<HashMap<String, serde_yaml_ng::Value>> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    performance::increment_counter(counters::FS_DOCS_PARSED, 1);
    let file = fs::File::open(path)
        .map_err(|e| {
            debug!(path = %path.display(), error = %e, "could not read doc file");
            e
        })
        .ok()?;
    let mut reader = BufReader::new(file);

    let mut first_line = String::new();
    if reader.read_line(&mut first_line).ok()? == 0 {
        return Some(HashMap::new());
    }
    if first_line.trim_end_matches(['\r', '\n']) != "---" {
        return Some(HashMap::new());
    }

    let mut yaml = String::new();
    let mut found_close = false;
    let mut line = String::new();
    while reader.read_line(&mut line).ok()? > 0 {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            found_close = true;
            break;
        }
        yaml.push_str(&line);
        line.clear();
    }

    if !found_close {
        return Some(HashMap::new());
    }
    if yaml.trim().is_empty() {
        return Some(HashMap::new());
    }

    serde_yaml_ng::from_str::<HashMap<String, serde_yaml_ng::Value>>(&yaml).ok()
}

/// Extract YAML frontmatter from markdown content.
///
/// Returns the parsed frontmatter map and the body (content after frontmatter).
/// If no valid frontmatter is found, returns an empty map and the full content.
fn extract_frontmatter(content: &str) -> (HashMap<String, serde_yaml_ng::Value>, &str) {
    let empty = (HashMap::new(), content);

    if !content.starts_with("---") {
        return empty;
    }

    // Find the closing delimiter (skip the opening "---" line)
    let after_opening = match content.find('\n') {
        Some(idx) => idx + 1,
        None => return empty,
    };

    let rest = &content[after_opening..];

    // Find the closing "---" delimiter.
    // It can be at the start of `rest` (empty frontmatter) or after a newline.
    let (yaml_str, body_start) = if rest.starts_with("---") {
        ("", after_opening + 3)
    } else if let Some(offset) = rest.find("\n---") {
        (&rest[..offset], after_opening + offset + 4)
    } else {
        return empty;
    };

    // Skip the newline after closing delimiter if present
    let body = if body_start < content.len() {
        let remaining = &content[body_start..];
        remaining.strip_prefix('\n').unwrap_or(remaining)
    } else {
        ""
    };

    if yaml_str.trim().is_empty() {
        return (HashMap::new(), body);
    }

    match serde_yaml_ng::from_str::<HashMap<String, serde_yaml_ng::Value>>(yaml_str) {
        Ok(map) => (map, body),
        Err(_) => (HashMap::new(), content),
    }
}

/// Extract a string field from the frontmatter map.
fn get_string_field(
    frontmatter: &HashMap<String, serde_yaml_ng::Value>,
    key: &str,
) -> Option<String> {
    frontmatter.get(key).and_then(|v| match v {
        serde_yaml_ng::Value::String(s) => Some(s.clone()),
        serde_yaml_ng::Value::Bool(b) => Some(b.to_string()),
        serde_yaml_ng::Value::Number(n) => Some(format!("{n}")),
        _ => None,
    })
}

/// Parse the `blast_radius` frontmatter field as a list of repo-relative paths.
///
/// Paths are normalized to repo-relative form:
/// - `./` prefixes are stripped
/// - absolute paths matching `repo_root` are made relative
/// - `.` segments are resolved
///
/// Non-string entries are silently ignored. Returns `None` if the key is missing.
fn parse_blast_radius(
    frontmatter: &HashMap<String, serde_yaml_ng::Value>,
    repo_root: &Path,
) -> Option<Vec<PathBuf>> {
    let value = frontmatter.get("blast_radius")?;
    match value {
        serde_yaml_ng::Value::Sequence(seq) => {
            let paths: Vec<PathBuf> = seq
                .iter()
                .filter_map(|v| {
                    if let serde_yaml_ng::Value::String(s) = v {
                        Some(normalize_blast_radius_path(s, repo_root))
                    } else {
                        None
                    }
                })
                .collect();
            Some(paths)
        }
        _ => Some(Vec::new()),
    }
}

/// Normalize a blast-radius path entry to a clean repo-relative path.
fn normalize_blast_radius_path(raw: &str, repo_root: &Path) -> PathBuf {
    let p = Path::new(raw);

    // Try stripping repo root from absolute paths
    if p.is_absolute()
        && let Ok(relative) = p.strip_prefix(repo_root)
    {
        return normalize_relative_components(relative);
    }

    normalize_relative_components(p)
}

/// Remove `.` and `./` components from a relative path, and strip leading `./`.
fn normalize_relative_components(p: &Path) -> PathBuf {
    use std::path::Component;
    p.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

/// Extract the document title using priority:
/// 1. Frontmatter "title" field
/// 2. First H1 heading (`# ...`)
/// 3. First H2 heading (`## ...`)
/// 4. First H3 heading (`### ...`)
/// 5. Empty string
fn extract_title(
    frontmatter: &HashMap<String, serde_yaml_ng::Value>,
    body: &str,
) -> (String, TitleSource) {
    // 1. Frontmatter title
    if let Some(serde_yaml_ng::Value::String(title)) = frontmatter.get("title")
        && !title.is_empty()
    {
        return (title.clone(), TitleSource::FrontmatterTitle);
    }

    // 2-4. Search for headings in priority order
    let mut first_h2: Option<&str> = None;
    let mut first_h3: Option<&str> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(h1) = trimmed.strip_prefix("# ")
            && !h1.starts_with('#')
        {
            return (h1.trim().to_string(), TitleSource::H1Heading);
        }
        if first_h2.is_none()
            && let Some(h2) = trimmed.strip_prefix("## ")
            && !h2.starts_with('#')
        {
            first_h2 = Some(h2.trim());
        }
        if first_h3.is_none()
            && let Some(h3) = trimmed.strip_prefix("### ")
            && !h3.starts_with('#')
        {
            first_h3 = Some(h3.trim());
        }
    }

    if let Some(h2) = first_h2 {
        return (h2.to_string(), TitleSource::H2Heading);
    }
    if let Some(h3) = first_h3 {
        return (h3.to_string(), TitleSource::H3Heading);
    }

    (String::new(), TitleSource::None)
}

/// Resolve last updated timestamp from frontmatter or file metadata.
///
/// Priority:
/// 1. Frontmatter `last_updated` (date or datetime)
/// 2. Frontmatter `updated_at` (date or datetime)
/// 3. File system last modified time
fn resolve_last_updated(
    frontmatter: &HashMap<String, serde_yaml_ng::Value>,
    path: &Path,
) -> (DateTime<Utc>, UpdatedSource) {
    // Try frontmatter fields in priority order
    for key in &["last_updated", "updated_at"] {
        if let Some(dt) = frontmatter.get(*key).and_then(parse_datetime_value) {
            return (dt, UpdatedSource::UpdatedProperty);
        }
    }

    // Fall back to file modification time
    (file_mtime(path), UpdatedSource::FileMetadata)
}

/// Try to parse a YAML value as a DateTime<Utc>.
fn parse_datetime_value(value: &serde_yaml_ng::Value) -> Option<DateTime<Utc>> {
    let s = match value {
        serde_yaml_ng::Value::String(s) => s.as_str(),
        _ => return None,
    };

    // Try RFC 3339 / ISO 8601 datetime
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try common datetime formats
    let datetime_formats = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M"];
    for fmt in &datetime_formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(ndt.and_utc());
        }
    }

    // Try date-only formats (midnight UTC)
    let date_formats = ["%Y-%m-%d", "%Y/%m/%d"];
    for fmt in &date_formats {
        if let Ok(nd) = NaiveDate::parse_from_str(s, fmt) {
            return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
        }
    }

    None
}

/// Get file modification time as DateTime<Utc>.
fn file_mtime(path: &Path) -> DateTime<Utc> {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
}

/// Determine which monorepo package a file belongs to.
///
/// Matches the file's relative path against known package directories.
/// Returns `None` if the file is in the repo root or not in any package.
#[cfg(test)]
pub(crate) fn determine_package(
    relative_path: &Path,
    packages: &[(String, PathBuf)],
) -> Option<String> {
    let index = PackageOwnershipIndex::from_relative_paths(Path::new("."), packages);
    determine_package_with_index(relative_path, packages, &index)
}

fn determine_package_with_index(
    relative_path: &Path,
    packages: &[(String, PathBuf)],
    index: &PackageOwnershipIndex,
) -> Option<String> {
    let package_index = index.lookup_relative(relative_path)?;
    packages.get(package_index).map(|(name, _)| name.clone())
}

pub fn assign_packages(
    docs: &mut [MarkdownMeta],
    packages: &[(String, PathBuf)],
    repo_root: &Path,
) {
    let ownership_index = PackageOwnershipIndex::from_relative_paths(repo_root, packages);
    for doc in docs {
        let relative_path = doc
            .filepath
            .strip_prefix(repo_root)
            .unwrap_or_else(|_| Path::new(&doc.relative));
        doc.package = determine_package_with_index(relative_path, packages, &ownership_index);
        if doc.filepath.is_relative() {
            doc.filepath = repo_root.join(&doc.relative);
        }
    }
}

pub(crate) fn assign_packages_from_repo(
    docs: &mut [MarkdownMeta],
    repo: &super::repo::RepoInfo,
    ownership_index: &PackageOwnershipIndex,
    repo_root: &Path,
) {
    for doc in docs {
        let relative_path = doc
            .filepath
            .strip_prefix(repo_root)
            .unwrap_or_else(|_| Path::new(&doc.relative));
        doc.package = repo
            .package_for_relative_path_with_index(ownership_index, relative_path)
            .map(|package| package.name.clone());
        if doc.filepath.is_relative() {
            doc.filepath = repo_root.join(&doc.relative);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod frontmatter_parsing {
        use super::*;

        #[test]
        fn parses_valid_yaml_frontmatter() {
            let content = "---\ntitle: Hello\nmodel: gpt-4\n---\n# Body";
            let (fm, body) = extract_frontmatter(content);
            assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
            assert_eq!(fm.get("model").and_then(|v| v.as_str()), Some("gpt-4"));
            assert_eq!(body, "# Body");
        }

        #[test]
        fn returns_empty_map_for_no_frontmatter() {
            let content = "# Just a heading\n\nSome body text.";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, content);
        }

        #[test]
        fn returns_empty_map_for_unclosed_frontmatter() {
            let content = "---\ntitle: Test\n# No closing delimiter";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, content);
        }

        #[test]
        fn handles_empty_frontmatter() {
            let content = "---\n---\n# Content";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, "# Content");
        }

        #[test]
        fn handles_frontmatter_with_prompt() {
            let content = "---\ntitle: Test\nprompt: Generate a summary\n---\nBody text";
            let (fm, body) = extract_frontmatter(content);
            assert_eq!(
                fm.get("prompt").and_then(|v| v.as_str()),
                Some("Generate a summary")
            );
            assert_eq!(body, "Body text");
        }
    }

    mod title_extraction {
        use super::*;

        #[test]
        fn prefers_frontmatter_title() {
            let mut fm = HashMap::new();
            fm.insert(
                "title".to_string(),
                serde_yaml_ng::Value::String("FM Title".to_string()),
            );
            let (title, source) = extract_title(&fm, "# Heading Title");
            assert_eq!(title, "FM Title");
            assert_eq!(source, TitleSource::FrontmatterTitle);
        }

        #[test]
        fn falls_back_to_h1() {
            let fm = HashMap::new();
            let (title, source) = extract_title(&fm, "Some text\n# Main Title\n## Sub");
            assert_eq!(title, "Main Title");
            assert_eq!(source, TitleSource::H1Heading);
        }

        #[test]
        fn falls_back_to_h2_when_no_h1() {
            let fm = HashMap::new();
            let (title, source) = extract_title(&fm, "Some text\n## Section Title\n### Sub");
            assert_eq!(title, "Section Title");
            assert_eq!(source, TitleSource::H2Heading);
        }

        #[test]
        fn falls_back_to_h3_when_no_h1_or_h2() {
            let fm = HashMap::new();
            let (title, source) = extract_title(&fm, "Some text\n### Subsection");
            assert_eq!(title, "Subsection");
            assert_eq!(source, TitleSource::H3Heading);
        }

        #[test]
        fn returns_empty_string_when_no_title() {
            let fm = HashMap::new();
            let (title, source) = extract_title(&fm, "Just some plain text.");
            assert_eq!(title, "");
            assert_eq!(source, TitleSource::None);
        }

        #[test]
        fn h2_not_confused_with_h1() {
            let fm = HashMap::new();
            // "## Title" should not match as H1
            let (title, source) = extract_title(&fm, "## Only H2\n### And H3");
            assert_eq!(title, "Only H2");
            assert_eq!(source, TitleSource::H2Heading);
        }
    }

    mod date_resolution {
        use super::*;

        #[test]
        fn parses_iso_date() {
            let val = serde_yaml_ng::Value::String("2025-06-15".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(
                dt.date_naive(),
                NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
            );
        }

        #[test]
        fn parses_iso_datetime() {
            let val = serde_yaml_ng::Value::String("2025-06-15T10:30:00Z".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(
                dt.date_naive(),
                NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
            );
        }

        #[test]
        fn parses_datetime_without_timezone() {
            let val = serde_yaml_ng::Value::String("2025-06-15 10:30:00".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(
                dt.date_naive(),
                NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
            );
        }

        #[test]
        fn returns_none_for_invalid_date() {
            let val = serde_yaml_ng::Value::String("not-a-date".to_string());
            assert!(parse_datetime_value(&val).is_none());
        }

        #[test]
        fn returns_none_for_non_string() {
            let val = serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42));
            assert!(parse_datetime_value(&val).is_none());
        }
    }

    mod package_detection {
        use super::*;

        #[test]
        fn matches_file_to_package() {
            // Package paths are the actual member paths from detect_repo
            let packages = vec![
                ("research-lib".to_string(), PathBuf::from("research/lib")),
                ("research-cli".to_string(), PathBuf::from("research/cli")),
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
            ];
            let path = Path::new("research/lib/docs/architecture.md");
            assert_eq!(
                determine_package(path, &packages),
                Some("research-lib".to_string())
            );
        }

        #[test]
        fn returns_none_for_root_file() {
            let packages = vec![("research-lib".to_string(), PathBuf::from("research/lib"))];
            let path = Path::new("README.md");
            assert_eq!(determine_package(path, &packages), None);
        }

        #[test]
        fn matches_correct_nested_package() {
            let packages = vec![
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
                ("sniff-cli".to_string(), PathBuf::from("sniff/cli")),
            ];
            let path = Path::new("sniff/lib/docs/design.md");
            assert_eq!(
                determine_package(path, &packages),
                Some("sniff".to_string())
            );
        }

        #[test]
        fn chooses_deepest_package_and_respects_component_boundaries() {
            use crate::performance::{counters, testing};

            let packages = vec![
                ("parent".to_string(), PathBuf::from("crates/pkg-a")),
                (
                    "nested".to_string(),
                    PathBuf::from("crates/pkg-a/nested"),
                ),
                ("sibling".to_string(), PathBuf::from("crates/pkg-a2")),
            ];

            assert_eq!(
                determine_package(
                    Path::new("crates/pkg-a/nested/docs/design.md"),
                    &packages,
                ),
                Some("nested".to_string())
            );
            assert_eq!(
                determine_package(Path::new("crates/pkg-a2/README.md"), &packages),
                Some("sibling".to_string())
            );
            assert_eq!(
                determine_package(Path::new("crates/pkg-a20/README.md"), &packages),
                None
            );

            let index = PackageOwnershipIndex::from_relative_paths(Path::new("."), &packages);
            let ((), counts) = testing::measure(|| {
                assert_eq!(
                    determine_package_with_index(
                        Path::new("crates/pkg-a/nested/docs/design.md"),
                        &packages,
                        &index,
                    ),
                    Some("nested".to_string())
                );
                assert_eq!(
                    determine_package_with_index(
                        Path::new("crates/pkg-a2/README.md"),
                        &packages,
                        &index,
                    ),
                    Some("sibling".to_string())
                );
            });
            assert_eq!(counts.get(counters::FS_CANONICALIZATIONS), 0);
        }

        #[test]
        fn file_between_packages_returns_none() {
            // A file at sniff/README.md doesn't belong to sniff/lib or sniff/cli
            let packages = vec![
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
                ("sniff-cli".to_string(), PathBuf::from("sniff/cli")),
            ];
            let path = Path::new("sniff/README.md");
            assert_eq!(determine_package(path, &packages), None);
        }

        #[test]
        fn hidden_dirs_not_matched() {
            let packages = vec![("research-lib".to_string(), PathBuf::from("research/lib"))];
            let path = Path::new(".ai/plans/some-plan.md");
            assert_eq!(determine_package(path, &packages), None);
        }
    }

    mod content_hashing {
        use super::*;

        #[test]
        fn same_content_produces_same_hash() {
            let hash1 = xx_hash("hello world");
            let hash2 = xx_hash("hello world");
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn different_content_produces_different_hash() {
            let hash1 = xx_hash("hello world");
            let hash2 = xx_hash("goodbye world");
            assert_ne!(hash1, hash2);
        }
    }

    mod blast_radius_parsing {
        use super::*;

        fn dummy_root() -> PathBuf {
            PathBuf::from("/repo")
        }

        #[test]
        fn parses_valid_blast_radius_list() {
            let content = "---\nblast_radius:\n  - src/main.rs\n  - src/lib.rs\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            assert!(fm.contains_key("blast_radius"));
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert_eq!(
                paths,
                vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")]
            );
        }

        #[test]
        fn parses_empty_blast_radius_list() {
            let content = "---\nblast_radius: []\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            assert!(fm.contains_key("blast_radius"));
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert!(paths.is_empty());
        }

        #[test]
        fn missing_blast_radius_returns_none() {
            let content = "---\ntitle: Hello\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            assert!(!fm.contains_key("blast_radius"));
            assert!(parse_blast_radius(&fm, &dummy_root()).is_none());
        }

        #[test]
        fn non_string_entries_silently_dropped() {
            let content = "---\nblast_radius:\n  - src/main.rs\n  - 42\n  - true\n  - src/lib.rs\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert_eq!(
                paths,
                vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")]
            );
        }

        #[test]
        fn has_blast_radius_true_when_key_present() {
            let content = "---\nblast_radius: []\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            assert!(fm.contains_key("blast_radius"));
        }

        #[test]
        fn normalizes_dot_slash_prefix() {
            let content = "---\nblast_radius:\n  - ./src/main.rs\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert_eq!(paths, vec![PathBuf::from("src/main.rs")]);
        }

        #[test]
        fn normalizes_absolute_path_matching_repo_root() {
            let content = "---\nblast_radius:\n  - /repo/src/main.rs\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert_eq!(paths, vec![PathBuf::from("src/main.rs")]);
        }

        #[test]
        fn preserves_absolute_path_not_matching_repo_root() {
            let content = "---\nblast_radius:\n  - /other/src/main.rs\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            let paths = parse_blast_radius(&fm, &dummy_root()).unwrap();
            assert_eq!(paths, vec![PathBuf::from("/other/src/main.rs")]);
        }
    }

    mod frontmatter_keys_extraction {
        use super::*;

        #[test]
        fn extracts_sorted_keys() {
            let content = "---\ntitle: Hello\nmodel: gpt-4\nprompt: Do stuff\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            let mut keys: Vec<String> = fm.keys().cloned().collect();
            keys.sort();
            assert_eq!(keys, vec!["model", "prompt", "title"]);
        }

        #[test]
        fn empty_frontmatter_has_no_keys() {
            let content = "---\n---\n# Body";
            let (fm, _body) = extract_frontmatter(content);
            assert!(fm.is_empty());
        }
    }

    mod updated_source_tracking {
        use super::*;

        #[test]
        fn updated_property_source_from_last_updated() {
            let mut fm = HashMap::new();
            fm.insert(
                "last_updated".to_string(),
                serde_yaml_ng::Value::String("2025-06-15".to_string()),
            );
            let (_dt, source) = resolve_last_updated(&fm, Path::new("nonexistent.md"));
            assert_eq!(source, UpdatedSource::UpdatedProperty);
        }

        #[test]
        fn updated_property_source_from_updated_at() {
            let mut fm = HashMap::new();
            fm.insert(
                "updated_at".to_string(),
                serde_yaml_ng::Value::String("2025-06-15".to_string()),
            );
            let (_dt, source) = resolve_last_updated(&fm, Path::new("nonexistent.md"));
            assert_eq!(source, UpdatedSource::UpdatedProperty);
        }

        #[test]
        fn file_metadata_source_when_no_frontmatter_date() {
            let fm = HashMap::new();
            let (_dt, source) = resolve_last_updated(&fm, Path::new("."));
            assert_eq!(source, UpdatedSource::FileMetadata);
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn packages_detected_in_workspace() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            assert!(
                !repo.packages.is_empty(),
                "Should detect workspace packages. repo_root: {:?}",
                repo.repo_root,
            );
        }

        #[test]
        fn documents_have_correct_packages() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            let docs = repo.documents();

            // Files inside a specific package member should be assigned
            let sniff_lib_docs: Vec<_> = docs
                .iter()
                .filter(|d| d.relative.starts_with("sniff/lib/"))
                .collect();
            for doc in &sniff_lib_docs {
                assert!(
                    doc.package.is_some(),
                    "Doc {} should have a package assigned",
                    doc.relative,
                );
            }

            // Verify some docs are assigned to packages (not all root)
            let with_package = docs.iter().filter(|d| d.package.is_some()).count();
            assert!(
                with_package > 0,
                "At least some documents should have package assignments"
            );
        }

        #[test]
        fn repo_documents_from_current_dir() {
            // This test runs within the rusty-biscuit repo
            let repo = RepoDocuments::new(Path::new("."));
            assert!(repo.is_ok(), "Should detect git repo from current dir");
            let repo = repo.unwrap();
            let docs = repo.documents();
            assert!(!docs.is_empty(), "Should find markdown documents in repo");
        }

        #[test]
        fn all_docs_have_relative_paths() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            for doc in repo.documents() {
                assert!(
                    !doc.relative.starts_with('/'),
                    "Relative path should not start with /: {}",
                    doc.relative
                );
            }
        }

        #[test]
        fn all_docs_have_content_hash() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            for doc in repo.documents() {
                assert!(
                    !doc.content_hash.is_empty(),
                    "Content hash should not be empty for: {}",
                    doc.relative
                );
            }
        }

        #[test]
        fn detect_docs_returns_some_in_repo() {
            let docs = detect_docs(Path::new("."));
            assert!(
                docs.is_some(),
                "detect_docs should return Some in a git repo"
            );
        }
    }

    mod markdown_path_collection {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn write_file(path: &Path, contents: &str) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        #[test]
        fn returns_only_markdown_files() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();

            write_file(&root.join("one.md"), "# One");
            write_file(&root.join("two.txt"), "not markdown");
            write_file(&root.join("nested/three.md"), "# Three");
            write_file(&root.join("nested/four.rs"), "fn main() {}");

            let paths = collect_markdown_paths(root);

            let names: std::collections::BTreeSet<_> = paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            assert!(names.contains("one.md"));
            assert!(names.contains("three.md"));
            assert!(!names.contains("two.txt"));
            assert!(!names.contains("four.rs"));
        }

        #[test]
        fn extension_match_is_case_insensitive() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();

            write_file(&root.join("upper.MD"), "# Upper");
            write_file(&root.join("lower.md"), "# Lower");

            let paths = collect_markdown_paths(root);
            let names: std::collections::BTreeSet<_> = paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            assert!(names.contains("upper.MD"));
            assert!(names.contains("lower.md"));
        }

        #[test]
        fn returns_empty_for_nonexistent_root() {
            let tmp = TempDir::new().unwrap();
            let missing = tmp.path().join("not-here");
            let paths = collect_markdown_paths(&missing);
            assert!(paths.is_empty());
        }

        #[test]
        fn returns_empty_for_directory_without_markdown() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(&root.join("a.txt"), "a");
            write_file(&root.join("nested/b.rs"), "b");

            let paths = collect_markdown_paths(root);
            assert!(paths.is_empty());
        }

        #[test]
        fn honors_local_gitignore() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();

            // Initialize a git repo so ignore::WalkBuilder treats the
            // directory as a repo root and applies .gitignore.
            git2::Repository::init(root).unwrap();

            write_file(&root.join(".gitignore"), "ignored/\nskip.md\n");
            write_file(&root.join("kept.md"), "# Kept");
            write_file(&root.join("skip.md"), "# Skipped");
            write_file(&root.join("ignored/hidden.md"), "# Hidden");
            write_file(&root.join("sub/visible.md"), "# Visible");

            let paths = collect_markdown_paths(root);
            let names: std::collections::BTreeSet<_> = paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();

            assert!(names.contains("kept.md"));
            assert!(names.contains("visible.md"));
            assert!(
                !names.contains("skip.md"),
                "gitignored files must be excluded: {names:?}"
            );
            assert!(
                !names.contains("hidden.md"),
                "files under a gitignored directory must be excluded: {names:?}"
            );
        }

        #[test]
        fn allows_canonical_path_deduplication() {
            // When package-root and package-area-root coincide (or two scopes
            // happen to enumerate the same directory), callers can dedup
            // by canonical path without needing any additional sniff state.
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(&root.join("prompts/a.md"), "# A");
            write_file(&root.join("prompts/b.md"), "# B");

            let first = collect_markdown_paths(&root.join("prompts"));
            let second = collect_markdown_paths(&root.join("prompts"));

            let mut merged: Vec<PathBuf> = first
                .into_iter()
                .chain(second)
                .filter_map(|p| std::fs::canonicalize(&p).ok())
                .collect();
            merged.sort();
            merged.dedup();

            let names: Vec<String> = merged
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            assert_eq!(names, vec!["a.md".to_string(), "b.md".to_string()]);
        }

        #[test]
        fn walks_nested_directories() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();

            write_file(&root.join("top.md"), "# Top");
            write_file(&root.join("a/b/c/deep.md"), "# Deep");

            let paths = collect_markdown_paths(root);
            let names: std::collections::BTreeSet<_> = paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            assert!(names.contains("top.md"));
            assert!(names.contains("deep.md"));
        }
    }

    mod frontmatter_only_mode {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn write_file(path: &Path, contents: &str) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        #[test]
        fn frontmatter_only_extracts_prompt() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(
                &root.join("doc.md"),
                "---\ntitle: Test\nprompt: Do stuff\n---\n# Body",
            );

            let packages: Vec<(String, PathBuf)> = vec![];
            let docs = collect_markdown_files_filtered(
                root,
                &packages,
                DocParseMode::FrontmatterOnly,
                |_| true,
            );
            assert_eq!(docs.len(), 1);
            assert_eq!(docs[0].prompt.as_deref(), Some("Do stuff"));
            assert_eq!(docs[0].title, "Test");
            assert_eq!(docs[0].title_source, TitleSource::FrontmatterTitle);
            assert!(docs[0].content_hash.is_empty());
        }

        #[test]
        fn frontmatter_only_skips_body_hash_and_mtime() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(
                &root.join("doc.md"),
                "---\ntitle: Hello\n---\n# Body content here",
            );

            let packages: Vec<(String, PathBuf)> = vec![];
            let docs = collect_markdown_files_filtered(
                root,
                &packages,
                DocParseMode::FrontmatterOnly,
                |_| true,
            );
            assert_eq!(docs.len(), 1);
            assert!(docs[0].content_hash.is_empty());
            assert_eq!(
                docs[0].last_updated,
                DateTime::<Utc>::from_timestamp(0, 0).unwrap()
            );
        }

        #[test]
        fn frontmatter_only_path_filter_excludes_non_matching() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(&root.join("sniff/lib/doc.md"), "---\ntitle: A\n---\nBody");
            write_file(&root.join("homelab/doc.md"), "---\ntitle: B\n---\nBody");

            let packages: Vec<(String, PathBuf)> = vec![];
            let docs = collect_markdown_files_filtered(
                root,
                &packages,
                DocParseMode::FrontmatterOnly,
                |rel| rel.starts_with("sniff/"),
            );
            assert_eq!(docs.len(), 1);
            assert_eq!(docs[0].title, "A");
        }

        #[test]
        fn frontmatter_only_no_frontmatter_still_returns_doc() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            write_file(&root.join("plain.md"), "# Just a heading");

            let packages: Vec<(String, PathBuf)> = vec![];
            let docs = collect_markdown_files_filtered(
                root,
                &packages,
                DocParseMode::FrontmatterOnly,
                |_| true,
            );
            assert_eq!(docs.len(), 1);
            assert!(docs[0].title.is_empty());
            assert_eq!(docs[0].title_source, TitleSource::None);
        }
    }
}
