//! File discovery for the `::file-links` directive.
//!
//! Resolves glob and directory targets relative to the containing document,
//! enforces a repository/CWD security boundary, and computes the rendering
//! metadata consumed by the
//! [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! component.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use globset::GlobBuilder;

use super::types::{
    ALLOWED_EXTENSIONS, FileLinksDirective, FileLinksError, FileLinksMode, FileLinksRender,
    FileLinksResult,
};
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::find_git_root_from;

/// Discovers file-link targets for a single directive.
///
/// Returns a [`FileLinksResult`] whose `render` field is `None` when no files
/// matched (the caller decides how to surface the empty result).
pub fn discover(
    directive: &FileLinksDirective,
    source: &ComposeSource,
) -> Result<FileLinksResult, FileLinksError> {
    let source_path = require_source_file(source, directive.line)?;
    let source_dir = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let source_canonical = canonicalize(&source_path);
    let (boundary, is_repo) = resolve_boundary(&source_path)?;

    let (candidates, component_root) = match &directive.mode {
        FileLinksMode::Glob(glob) => {
            discover_glob(glob, &source_dir, &boundary, &source_canonical, directive.line)?
        }
        FileLinksMode::Dir { path, depth } => discover_dir(
            path,
            *depth,
            &source_dir,
            &boundary,
            &source_canonical,
            directive.line,
        )?,
    };

    if candidates.is_empty() {
        return Ok(FileLinksResult {
            directive: directive.clone(),
            render: None,
        });
    }

    let render = compute_render(&component_root, &boundary, is_repo, &candidates)?;
    Ok(FileLinksResult {
        directive: directive.clone(),
        render: Some(render),
    })
}

/// Extracts the source file path, erroring for non-file sources.
fn require_source_file(source: &ComposeSource, line: usize) -> Result<PathBuf, FileLinksError> {
    match source {
        ComposeSource::File(path) => Ok(path.clone()),
        _ => Err(FileLinksError::MissingSourceContext { line }),
    }
}

/// Resolves the security boundary: repo root when inside a repo, CWD otherwise.
///
/// Returns `(canonical_boundary, is_repo)`.
fn resolve_boundary(source_path: &Path) -> Result<(PathBuf, bool), FileLinksError> {
    let (boundary, is_repo) = match find_git_root_from(source_path) {
        Some(root) => (root, true),
        None => (std::env::current_dir()?, false),
    };
    let canonical = canonicalize(&boundary);
    Ok((canonical, is_repo))
}

/// Canonicalize a path, falling back to the original on failure.
fn canonicalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

// ── Glob discovery ──────────────────────────────────────────────────────────

/// Discovers files matching a glob pattern.
///
/// The glob is resolved relative to `source_dir`. The walk is scoped to the
/// literal directory prefix extracted from the glob so sibling subtrees are
/// not traversed. Returns the sorted, deduplicated canonical candidate paths
/// and the derived component root (common ancestor of all matches).
fn discover_glob(
    glob: &str,
    source_dir: &Path,
    boundary: &Path,
    source_canonical: &Path,
    line: usize,
) -> Result<(Vec<PathBuf>, PathBuf), FileLinksError> {
    let matcher = GlobBuilder::new(glob)
        .literal_separator(true)
        .build()
        .map_err(|e| FileLinksError::InvalidGlob {
            pattern: glob.to_string(),
            line,
            message: e.to_string(),
        })?
        .compile_matcher();

    let (literal_prefix, walk_base) = split_glob_prefix(glob, source_dir);

    let mut candidates = BTreeSet::new();
    if walk_base.is_dir() {
        walk_recursive(
            &walk_base,
            &walk_base,
            0,
            None,
            boundary,
            source_canonical,
            &mut candidates,
            &|relative, _canonical| {
                let rel_str = path_to_forward_slashes(relative);
                let full_rel = if literal_prefix.is_empty() {
                    rel_str
                } else {
                    format!("{literal_prefix}/{rel_str}")
                };
                matcher.is_match(&full_rel)
            },
        );
    }

    let candidates_vec: Vec<PathBuf> = candidates.into_iter().collect();
    let component_root = match common_ancestor(&candidates_vec) {
        Some(root) => root,
        None => return Ok((Vec::new(), walk_base)),
    };
    Ok((candidates_vec, component_root))
}

/// Splits a glob into its literal directory prefix and the resolved walk base.
///
/// Returns `(literal_prefix, walk_base)` where `literal_prefix` is the path
/// component before the first wildcard (e.g. `"docs"` for `docs/**/*.md`,
/// empty for `*.md`), and `walk_base` is that prefix resolved relative to
/// `source_dir`.
fn split_glob_prefix(glob: &str, source_dir: &Path) -> (String, PathBuf) {
    let wildcard_pos = glob.chars().position(|c| matches!(c, '*' | '?' | '['));
    let literal_part = match wildcard_pos {
        Some(pos) => &glob[..pos],
        None => glob,
    };
    let prefix = match literal_part.rfind('/') {
        Some(slash_pos) => literal_part[..slash_pos].to_string(),
        None => String::new(),
    };
    let walk_base = if prefix.is_empty() {
        source_dir.to_path_buf()
    } else {
        source_dir.join(&prefix)
    };
    (prefix, walk_base)
}

// ── Directory discovery ─────────────────────────────────────────────────────

/// Discovers files in a directory scan.
///
/// `depth` controls recursion: `0` lists only the immediate directory; `n`
/// descends `n` levels of subdirectories.
fn discover_dir(
    path: &str,
    depth: u32,
    source_dir: &Path,
    boundary: &Path,
    source_canonical: &Path,
    line: usize,
) -> Result<(Vec<PathBuf>, PathBuf), FileLinksError> {
    let target = source_dir.join(path);
    if !target.exists() {
        return Err(FileLinksError::TargetNotFound {
            path: path.to_string(),
            line,
        });
    }
    let component_root = canonicalize(&target);

    let mut candidates = BTreeSet::new();
    collect_files(
        &target,
        Some(depth),
        boundary,
        source_canonical,
        &mut candidates,
        |_relative, _canonical| true,
    );

    Ok((candidates.into_iter().collect(), component_root))
}

// ── Shared walking / filtering ─────────────────────────────────────────────

/// Walks `root` and collects canonical file paths that pass all filters.
///
/// ## Filtering
///
/// - Only regular files (symlinks followed for type detection).
/// - Extension must be in [`ALLOWED_EXTENSIONS`] (case-insensitive).
/// - Canonical path must be within `boundary`.
/// - The source document itself is excluded.
/// - Symlinked directories that resolve outside the boundary are not descended
///   into; symlinked files that escape are dropped.
///
/// `extra_filter` receives the path relative to `root` and the canonical path,
/// and must return `true` for files to include (e.g. glob matching).
fn collect_files(
    root: &Path,
    max_depth: Option<u32>,
    boundary: &Path,
    source_canonical: &Path,
    out: &mut BTreeSet<PathBuf>,
    extra_filter: impl Fn(&Path, &Path) -> bool,
) {
    walk_recursive(
        root,
        root,
        0,
        max_depth,
        boundary,
        source_canonical,
        out,
        &extra_filter,
    );
}

/// Recursive walker. `base` is the original walk root used to compute relative
/// paths for `extra_filter`.
#[allow(clippy::too_many_arguments)]
fn walk_recursive(
    dir: &Path,
    base: &Path,
    current_depth: u32,
    max_depth: Option<u32>,
    boundary: &Path,
    source_canonical: &Path,
    out: &mut BTreeSet<PathBuf>,
    extra_filter: &impl Fn(&Path, &Path) -> bool,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();

        // Skip symlinked directories to avoid loops; include symlinked files.
        let is_symlink = entry
            .file_type()
            .map(|ft| ft.is_symlink())
            .unwrap_or(false);

        let canonical = canonicalize(&entry_path);

        // Boundary check (catches symlink escapes).
        if !is_within_boundary(&canonical, boundary) {
            continue;
        }

        // Self-exclusion.
        if canonical == *source_canonical {
            continue;
        }

        // Determine real file type (follows symlinks).
        let metadata = match std::fs::metadata(&entry_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_file() {
            if !has_allowed_extension(&entry_path) {
                continue;
            }
            let relative = entry_path.strip_prefix(base).unwrap_or(&entry_path);
            if extra_filter(relative, &canonical) {
                out.insert(canonical);
            }
        } else if metadata.is_dir() {
            // Don't follow symlinked directories.
            if is_symlink {
                continue;
            }
            // Respect max_depth (None = unlimited for glob mode).
            if let Some(max) = max_depth
                && current_depth >= max
            {
                continue;
            }
            walk_recursive(
                &entry_path,
                base,
                current_depth + 1,
                max_depth,
                boundary,
                source_canonical,
                out,
                extra_filter,
            );
        }
    }
}

/// Returns `true` when `path` is `path` itself or a descendant of `boundary`.
fn is_within_boundary(path: &Path, boundary: &Path) -> bool {
    path == boundary || path.starts_with(boundary)
}

/// Returns `true` when `path`'s extension is an allowed document type.
fn has_allowed_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            ALLOWED_EXTENSIONS.iter().any(|allowed| *allowed == lower)
        })
        .unwrap_or(false)
}

/// Converts a path to a forward-slash string for glob matching.
fn path_to_forward_slashes(path: &Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

// ── Render metadata ─────────────────────────────────────────────────────────

/// Computes the [`FileLinksRender`] metadata from discovery results.
fn compute_render(
    component_root: &Path,
    boundary: &Path,
    is_repo: bool,
    candidates: &[PathBuf],
) -> Result<FileLinksRender, FileLinksError> {
    let included_paths = candidates
        .iter()
        .map(|c| {
            c.strip_prefix(component_root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| c.clone())
        })
        .collect::<Vec<_>>();

    let relative = component_root.strip_prefix(boundary).unwrap_or(Path::new(""));
    let components: Vec<String> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
        .collect();

    let (dimmed_prefix, target_name) = if components.is_empty() {
        // Component root is the boundary itself.
        let name = component_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string();
        (String::new(), name)
    } else {
        let target = components.last().cloned().unwrap_or_default();
        let parents = &components[..components.len() - 1];
        let prefix = if parents.is_empty() {
            "/".to_string()
        } else {
            format!("/{}/", parents.join("/"))
        };
        (prefix, target)
    };

    Ok(FileLinksRender {
        component_root: component_root.to_path_buf(),
        included_paths,
        dimmed_prefix,
        target_name,
        uses_repo_icon: is_repo,
    })
}

/// Returns the deepest directory that is an ancestor of every path.
fn common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    let mut iter = paths.iter();
    let first = iter.next()?;
    let mut ancestor = first.parent()?.to_path_buf();

    for path in iter {
        ancestor = common_prefix_dir(&ancestor, path);
    }
    Some(ancestor)
}

/// Returns the deepest common ancestor directory of two paths.
fn common_prefix_dir(a: &Path, b: &Path) -> PathBuf {
    let a_comps: Vec<_> = a.components().collect();
    let b_comps: Vec<_> = b.components().collect();
    let mut result = PathBuf::new();
    for (ca, cb) in a_comps.iter().zip(b_comps.iter()) {
        if ca == cb {
            result.push(ca.as_os_str());
        } else {
            break;
        }
    }
    // Ensure we return a directory, not a file prefix.
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::file_links::parser::parse_file_links_directives;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    /// Creates a fake repo root so the boundary resolves to the tempdir.
    fn make_repo(dir: &TempDir) {
        fs::create_dir(dir.path().join(".git")).unwrap();
    }

    fn discover_content(
        content: &str,
        source_path: &Path,
    ) -> Result<FileLinksResult, FileLinksError> {
        let directives = parse_file_links_directives(content)?;
        assert_eq!(directives.len(), 1, "expected exactly one directive");
        discover(&directives[0], &ComposeSource::File(source_path.to_path_buf()))
    }

    // ── Glob mode ────────────────────────────────────────────────────────────

    #[test]
    fn glob_matches_markdown_files() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "# A");
        write_file(&docs, "b.md", "# B");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 2);
        assert!(render.included_paths.iter().any(|p| p == Path::new("a.md")));
        assert!(render.included_paths.iter().any(|p| p == Path::new("b.md")));
    }

    #[test]
    fn glob_recursive_matches_subdirectories() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "top.md", "");
        write_file(&docs.join("sub"), "nested.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/**/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 2);
    }

    #[test]
    fn glob_excludes_unsupported_extensions() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "a.md", "");
        write_file(dir.path(), "b.png", "");
        write_file(dir.path(), "c.rs", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links *.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 1);
        assert_eq!(render.included_paths[0], Path::new("a.md"));
    }

    #[test]
    fn glob_matches_mixed_case_extensions() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "a.MD", "");
        write_file(dir.path(), "b.Pdf", "");
        write_file(dir.path(), "c.TXT", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links *.*\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 3);
    }

    #[test]
    fn glob_excludes_source_document() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let source = write_file(dir.path(), "index.md", "::file-links *.md\n");
        write_file(dir.path(), "other.md", "");

        let result = discover_content("::file-links *.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert!(render.included_paths.iter().all(|p| p != Path::new("index.md")));
        assert!(render.included_paths.iter().any(|p| p == Path::new("other.md")));
    }

    #[test]
    fn glob_empty_result() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links *.md\n", &source).unwrap();
        assert!(result.render.is_none());
    }

    #[test]
    fn glob_invalid_pattern_errors() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let source = write_file(dir.path(), "index.md", "");

        let err = discover_content("::file-links [invalid\n", &source).unwrap_err();
        assert!(matches!(err, FileLinksError::InvalidGlob { .. }));
    }

    #[test]
    fn glob_never_includes_unmatched_sibling() {
        // A glob that matches only .md files must not include a .txt sibling
        // in the same directory.
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "matched.md", "");
        write_file(&docs, "unmatched.txt", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 1);
        assert_eq!(render.included_paths[0], Path::new("matched.md"));
    }

    // ── Directory mode ───────────────────────────────────────────────────────

    #[test]
    fn dir_depth_zero_lists_immediate_files_only() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "top.md", "");
        write_file(&docs.join("sub"), "nested.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links --dir docs\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 1);
        assert_eq!(render.included_paths[0], Path::new("top.md"));
    }

    #[test]
    fn dir_depth_one_includes_one_subdir_level() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "top.md", "");
        write_file(&docs.join("sub"), "nested.md", "");
        write_file(&docs.join("sub").join("deep"), "very_deep.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result =
            discover_content("::file-links --dir docs --depth 1\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 2);
        assert!(render.included_paths.iter().any(|p| p == Path::new("top.md")));
        assert!(render
            .included_paths
            .iter()
            .any(|p| p == Path::new("sub/nested.md")));
        assert!(!render
            .included_paths
            .iter()
            .any(|p| p == Path::new("sub/deep/very_deep.md")));
    }

    #[test]
    fn dir_depth_two_includes_two_levels() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "top.md", "");
        write_file(&docs.join("a"), "a.md", "");
        write_file(&docs.join("a").join("b"), "b.md", "");
        write_file(&docs.join("a").join("b").join("c"), "c.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result =
            discover_content("::file-links --dir docs --depth 2\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        // depth 0: top.md; depth 1: a/a.md; depth 2: a/b/b.md; NOT a/b/c/c.md
        assert_eq!(render.included_paths.len(), 3);
    }

    #[test]
    fn dir_excludes_source_document() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "");
        let source = write_file(&docs, "index.md", "");

        let result = discover_content("::file-links --dir .\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert!(render.included_paths.iter().any(|p| p == Path::new("a.md")));
        assert!(render
            .included_paths
            .iter()
            .all(|p| p != Path::new("index.md")));
    }

    #[test]
    fn dir_not_found_errors() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let source = write_file(dir.path(), "index.md", "");

        let err =
            discover_content("::file-links --dir nonexistent\n", &source).unwrap_err();
        assert!(matches!(err, FileLinksError::TargetNotFound { .. }));
    }

    #[test]
    fn dir_mixed_case_extensions() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "a.PDF", "");
        write_file(&docs, "b.docx", "");
        write_file(&docs, "c.XLSX", "");
        write_file(&docs, "d.txt", "");
        write_file(&docs, "e.bin", "");
        let source = write_file(dir.path(), "index.md", "");

        let result =
            discover_content("::file-links --dir docs\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 4);
    }

    // ── Boundary / security ──────────────────────────────────────────────────

    #[test]
    fn glob_excludes_outside_cwd_boundary() {
        // No .git in tempdir → boundary is CWD (the worktree). Files in the
        // tempdir are outside that boundary, so the glob matches nothing.
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.md\n", &source).unwrap();
        // The tempdir is outside the repo/CWD boundary → no matches.
        assert!(result.render.is_none());
    }

    #[test]
    fn repo_boundary_allows_files_under_repo() {
        // Create a fake repo root with .git so the boundary is the tempdir.
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths.len(), 1);
        assert!(render.uses_repo_icon);
    }

    #[test]
    fn parent_escape_glob_is_filtered_by_boundary() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "inside.md", "");
        let outside = TempDir::new().unwrap();
        write_file(outside.path(), "outside.md", "");

        let source = write_file(dir.path(), "index.md", "");

        // Try to escape via ..
        let parent_glob = format!(
            "::file-links \"{}/*.md\"\n",
            outside.path().display()
        );
        let result = discover_content(&parent_glob, &source).unwrap();
        assert!(result.render.is_none());
    }

    // ── Source context ───────────────────────────────────────────────────────

    #[test]
    fn inline_content_without_source_errors() {
        let directives =
            parse_file_links_directives("::file-links *.md\n").unwrap();
        let err = discover(&directives[0], &ComposeSource::Unknown).unwrap_err();
        assert!(matches!(err, FileLinksError::MissingSourceContext { .. }));
    }

    // ── Render metadata ─────────────────────────────────────────────────────

    #[test]
    fn render_metadata_dir_mode() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs_topics = dir.path().join("docs").join("topics");
        write_file(&docs_topics, "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result =
            discover_content("::file-links --dir docs/topics\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.target_name, "topics");
        assert_eq!(render.dimmed_prefix, "/docs/");
        assert!(render.uses_repo_icon);
        assert_eq!(render.included_paths, vec![PathBuf::from("a.md")]);
    }

    #[test]
    fn render_metadata_target_at_boundary_root() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links --dir .\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        // The target IS the repo root: no dimmed prefix, target is the dir name.
        assert_eq!(render.dimmed_prefix, "");
        assert!(!render.target_name.is_empty());
    }

    #[test]
    fn render_metadata_glob_common_ancestor() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "");
        write_file(&docs.join("sub"), "b.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/**/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        // Common ancestor of docs/a.md and docs/sub/b.md is docs.
        assert_eq!(render.target_name, "docs");
    }

    // ── Deduplication ────────────────────────────────────────────────────────

    #[test]
    fn duplicate_canonical_matches_are_deduplicated() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        // Two globs in one directive isn't supported, but a single broad glob
        // can't produce dupes since we walk each file once. Instead verify the
        // result set is stable and unique.
        let result = discover_content("::file-links *.md\n", &source).unwrap();
        let render = result.render.unwrap();
        let unique: BTreeSet<_> = render.included_paths.iter().collect();
        assert_eq!(unique.len(), render.included_paths.len());
    }

    // ── Unit tests for helpers ───────────────────────────────────────────────

    #[test]
    fn split_glob_prefix_nested_path() {
        let (prefix, base) = split_glob_prefix("docs/**/*.md", Path::new("/src"));
        assert_eq!(prefix, "docs");
        assert_eq!(base, PathBuf::from("/src/docs"));
    }

    #[test]
    fn split_glob_prefix_no_prefix() {
        let (prefix, base) = split_glob_prefix("*.md", Path::new("/src"));
        assert!(prefix.is_empty());
        assert_eq!(base, PathBuf::from("/src"));
    }

    #[test]
    fn split_glob_prefix_parent_prefix() {
        let (prefix, base) = split_glob_prefix("../sibling/*.md", Path::new("/src/a"));
        assert_eq!(prefix, "../sibling");
        assert_eq!(base, PathBuf::from("/src/a/../sibling"));
    }

    #[test]
    fn common_ancestor_single_file() {
        let paths = vec![PathBuf::from("/a/b/c.md")];
        let ancestor = common_ancestor(&paths).unwrap();
        assert_eq!(ancestor, PathBuf::from("/a/b"));
    }

    #[test]
    fn common_ancestor_multiple_files_same_dir() {
        let paths = vec![
            PathBuf::from("/a/b/x.md"),
            PathBuf::from("/a/b/y.md"),
        ];
        let ancestor = common_ancestor(&paths).unwrap();
        assert_eq!(ancestor, PathBuf::from("/a/b"));
    }

    #[test]
    fn common_ancestor_different_subdirs() {
        let paths = vec![
            PathBuf::from("/a/b/x.md"),
            PathBuf::from("/a/c/y.md"),
        ];
        let ancestor = common_ancestor(&paths).unwrap();
        assert_eq!(ancestor, PathBuf::from("/a"));
    }

    #[test]
    fn has_allowed_extension_checks_case_insensitively() {
        assert!(has_allowed_extension(Path::new("file.md")));
        assert!(has_allowed_extension(Path::new("file.MD")));
        assert!(has_allowed_extension(Path::new("file.PdF")));
        assert!(has_allowed_extension(Path::new("file.txt")));
        assert!(!has_allowed_extension(Path::new("file.png")));
        assert!(!has_allowed_extension(Path::new("file.rs")));
        assert!(!has_allowed_extension(Path::new("file")));
    }
}
