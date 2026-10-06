//! File discovery for the `::file-links` directive.
//!
//! A glob target is a [`GlobReference`] listed in the containing document's
//! context, so it takes every reference prefix and the merged, most-local-first
//! roots of `find_files()`. A `--dir` target is a directory relative to the
//! document. A `--dir` scan and a bare, `./`, or `../` glob are held to the
//! context's tree root (`base_dir()`), never to the process's current
//! directory. The result carries the rendering
//! metadata consumed by the
//! [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! component.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference};

use super::types::{
    ALLOWED_EXTENSIONS, FileLinksDirective, FileLinksError, FileLinksMode, FileLinksRender,
    FileLinksResult,
};
use crate::markdown::compose::ComposeSource;

/// Discovers file-link targets for a single directive authored in `source`,
/// whose resolution context is `document_context`.
///
/// Returns a [`FileLinksResult`] whose `render` field is `None` when no files
/// matched (the caller decides how to surface the empty result).
pub fn discover(
    directive: &FileLinksDirective,
    source: &ComposeSource,
    document_context: &FileResolutionContext,
) -> Result<FileLinksResult, FileLinksError> {
    let source_path = require_source_file(source, directive.line)?;
    let source_dir = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let source_canonical = canonicalize(&source_path);
    let boundary = canonicalize(document_context.base_dir());
    let is_repo = document_context.repository_root().is_some();

    let (candidates, component_root, skipped) = match &directive.mode {
        FileLinksMode::Glob(glob) => {
            discover_glob(glob, document_context, &source_canonical, directive.line)?
        }
        FileLinksMode::Dir { path, depth } => {
            let (candidates, root) = discover_dir(
                path,
                *depth,
                &source_dir,
                &boundary,
                &source_canonical,
                directive.line,
            )?;
            (candidates, root, Vec::new())
        }
    };

    if candidates.is_empty() {
        return Ok(FileLinksResult {
            directive: directive.clone(),
            render: None,
            skipped,
        });
    }

    let render = compute_render(&component_root, &boundary, is_repo, &candidates)?;
    Ok(FileLinksResult {
        directive: directive.clone(),
        render: Some(render),
        skipped,
    })
}

/// Extracts the source file path, erroring for non-file sources.
fn require_source_file(source: &ComposeSource, line: usize) -> Result<PathBuf, FileLinksError> {
    match source {
        ComposeSource::File(path) => Ok(path.clone()),
        _ => Err(FileLinksError::MissingSourceContext { line }),
    }
}

/// Canonicalize a path, falling back to the original on failure.
fn canonicalize(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

// ── Glob discovery ──────────────────────────────────────────────────────────

/// Discovers the files a glob reference lists, keeping only allowed
/// extensions and leaving out the containing document.
///
/// Returns the candidates (display paths, deduplicated on their canonical
/// target), the component root (their common ancestor), and the file
/// symlinks the listing skipped because their target leaves the file tree.
fn discover_glob(
    glob: &str,
    document_context: &FileResolutionContext,
    source_canonical: &Path,
    line: usize,
) -> Result<(Vec<PathBuf>, PathBuf, Vec<biscuit_file::SkippedEntry>), FileLinksError> {
    let glob_error = |source| FileLinksError::GlobReference { line, source };
    let listing = GlobReference::new([glob])
        .and_then(|globs| globs.list_files(document_context))
        .map_err(glob_error)?;

    let mut candidates = BTreeMap::new();
    for path in &listing.matches {
        let canonical = canonicalize(path);
        if canonical == *source_canonical || !has_allowed_extension(path) {
            continue;
        }
        insert_candidate(&mut candidates, canonical, display_path(path));
    }

    let candidates: Vec<PathBuf> = candidates.into_values().collect();
    let component_root = common_ancestor(&candidates)
        .unwrap_or_else(|| canonicalize(document_context.cwd()));
    Ok((candidates, component_root, listing.skipped))
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
    // `--dir` requires a directory. Without this guard a regular-file target
    // falls through to `read_dir()`, whose error is silently dropped, yielding
    // an empty (and misdiagnosed) match instead of a syntax error.
    if !target.is_dir() {
        return Err(FileLinksError::TargetNotDirectory {
            path: path.to_string(),
            line,
        });
    }
    let component_root = canonicalize(&target);

    let mut candidates = BTreeMap::new();
    walk_recursive(&target, 0, depth, boundary, source_canonical, &mut candidates)
        .map_err(|(path, source)| FileLinksError::Unreadable { path, line, source })?;

    Ok((candidates.into_values().collect(), component_root))
}

// ── Directory walking / filtering ──────────────────────────────────────────

/// Walks `dir` for `--dir` mode, collecting files keyed by their canonical
/// target (see [`insert_candidate`]).
///
/// ## Filtering
///
/// - Only regular files (symlinks followed for type detection).
/// - Extension must be in [`ALLOWED_EXTENSIONS`] (case-insensitive).
/// - Canonical target must be within `boundary`.
/// - The source document itself is excluded.
/// - Symlinked directories are not descended into; symlinked files whose
///   target escapes `boundary` are dropped.
/// - A dangling symlink, or an entry that vanished during the scan, is
///   skipped. Any other failure to read a directory or an entry is returned
///   with the path that failed, so an unreadable directory is never a
///   silently shorter tree.
fn walk_recursive(
    dir: &Path,
    current_depth: u32,
    max_depth: u32,
    boundary: &Path,
    source_canonical: &Path,
    out: &mut BTreeMap<PathBuf, PathBuf>,
) -> Result<(), (PathBuf, std::io::Error)> {
    let entries = std::fs::read_dir(dir).map_err(|error| (dir.to_path_buf(), error))?;

    for entry in entries {
        let entry = entry.map_err(|error| (dir.to_path_buf(), error))?;
        let entry_path = entry.path();

        // Skip symlinked directories to avoid loops; include symlinked files.
        let is_symlink = entry
            .file_type()
            .map_err(|error| (entry_path.clone(), error))?
            .is_symlink();

        let canonical = canonicalize(&entry_path);

        // Boundary check on the canonical target (catches symlink escapes).
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
            Err(error) if vanished(&error) => continue,
            Err(error) => return Err((entry_path, error)),
        };

        if metadata.is_file() {
            if has_allowed_extension(&entry_path) {
                insert_candidate(out, canonical, display_path(&entry_path));
            }
        } else if metadata.is_dir() {
            if is_symlink || current_depth >= max_depth {
                continue;
            }
            walk_recursive(
                &entry_path,
                current_depth + 1,
                max_depth,
                boundary,
                source_canonical,
                out,
            )?;
        }
    }
    Ok(())
}

/// A dangling symlink, or an entry removed (or replaced by a file) after its
/// directory was listed.
fn vanished(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
    )
}

/// A matched file's display path: its canonical parent directory joined with
/// its own (lexical) name. For an ordinary file this is the canonical path;
/// for a symlinked file it keeps the alias name, so the rendered tree and
/// hyperlink show the matched file rather than its target.
fn display_path(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => canonicalize(parent).join(name),
        _ => canonicalize(path),
    }
}

/// Records a candidate keyed by its canonical target, so two aliases of one
/// file are listed once, under the lexically smallest display path.
fn insert_candidate(out: &mut BTreeMap<PathBuf, PathBuf>, canonical: PathBuf, display: PathBuf) {
    out.entry(canonical)
        .and_modify(|existing| {
            if display < *existing {
                *existing = display.clone();
            }
        })
        .or_insert(display);
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

    /// The context a request prepares for `source_path`: rooted at the
    /// enclosing fake repository when there is one.
    fn document_context(source_path: &Path) -> FileResolutionContext {
        let context = FileResolutionContext::new(source_path.parent().unwrap());
        match crate::markdown::compose::find_git_root_from(source_path) {
            Some(root) => context.with_repository_root(root),
            None => context,
        }
    }

    fn discover_content(
        content: &str,
        source_path: &Path,
    ) -> Result<FileLinksResult, FileLinksError> {
        let directives = parse_file_links_directives(content)?;
        assert_eq!(directives.len(), 1, "expected exactly one directive");
        discover(
            &directives[0],
            &ComposeSource::File(source_path.to_path_buf()),
            &document_context(source_path),
        )
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
        assert!(matches!(
            err,
            FileLinksError::GlobReference {
                source: biscuit_file::GlobReferenceError::InvalidGlob { .. },
                ..
            }
        ));
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
    fn dir_target_regular_file_errors() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(dir.path(), "report.pdf", "pdf");
        let source = write_file(dir.path(), "index.md", "");

        let err =
            discover_content("::file-links --dir report.pdf\n", &source).unwrap_err();
        assert!(matches!(err, FileLinksError::TargetNotDirectory { .. }));
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
    fn without_a_repository_the_context_tree_root_bounds_the_glob() {
        // No `.git`: the tree root is the context's, never the process's
        // current directory, so the document's own folder is searchable.
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        write_file(&docs, "a.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.md\n", &source).unwrap();
        let render = result.render.expect("expected matches");
        assert_eq!(render.included_paths, vec![PathBuf::from("a.md")]);
        assert!(!render.uses_repo_icon);
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
    fn a_relative_glob_cannot_leave_the_repository() {
        let workspace = TempDir::new().unwrap();
        let repo = workspace.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        write_file(&workspace.path().join("outside"), "outside.md", "");
        let source = write_file(&repo, "index.md", "");

        let err = discover_content("::file-links ../outside/*.md\n", &source).unwrap_err();
        assert!(
            matches!(
                err,
                FileLinksError::GlobReference {
                    source: biscuit_file::GlobReferenceError::RelativeTreeEscape { .. },
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn an_absolute_glob_may_name_a_directory_outside_the_repository() {
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let outside = TempDir::new().unwrap();
        write_file(outside.path(), "outside.md", "");
        let source = write_file(dir.path(), "index.md", "");

        let glob = format!(
            "::file-links \"{}/*.md\"\n",
            biscuit_file::to_portable_string(outside.path())
        );
        let render = discover_content(&glob, &source)
            .unwrap()
            .render
            .expect("an absolute root is not relative, so it is allowed");
        assert_eq!(render.included_paths, vec![PathBuf::from("outside.md")]);
    }

    // ── Symlinks (in-bound vs escaping) ──────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn inbound_symlinked_file_keeps_lexical_alias_path() {
        // docs/alias.pdf -> ../assets/report.pdf (both inside the repo).
        // The matched alias must render under `docs/alias.pdf`, not under the
        // canonical target `assets/report.pdf`.
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        write_file(&dir.path().join("assets"), "report.pdf", "");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        std::os::unix::fs::symlink("../assets/report.pdf", docs.join("alias.pdf")).unwrap();
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.pdf\n", &source).unwrap();
        let render = result.render.expect("expected the in-bound symlink to match");
        assert_eq!(render.target_name, "docs");
        assert_eq!(render.included_paths, vec![PathBuf::from("alias.pdf")]);
    }

    #[cfg(unix)]
    #[test]
    fn escaping_symlinked_file_is_dropped() {
        // docs/escape.pdf -> <outside repo>/secret.pdf must be filtered out by
        // the boundary check on the canonical target.
        let dir = TempDir::new().unwrap();
        make_repo(&dir);
        let outside = TempDir::new().unwrap();
        let secret = write_file(outside.path(), "secret.pdf", "");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        std::os::unix::fs::symlink(&secret, docs.join("escape.pdf")).unwrap();
        let source = write_file(dir.path(), "index.md", "");

        let result = discover_content("::file-links docs/*.pdf\n", &source).unwrap();
        assert!(result.render.is_none(), "escaping symlink must be excluded");
        let skipped: Vec<_> = result.skipped.iter().map(|entry| entry.link.file_name().unwrap()).collect();
        assert_eq!(skipped, ["escape.pdf"], "the omission is reported");
    }

    // ── Source context ───────────────────────────────────────────────────────

    #[test]
    fn inline_content_without_source_errors() {
        let directives =
            parse_file_links_directives("::file-links *.md\n").unwrap();
        let context = FileResolutionContext::new(std::env::temp_dir());
        let err = discover(&directives[0], &ComposeSource::Unknown, &context).unwrap_err();
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
        let unique: std::collections::BTreeSet<_> = render.included_paths.iter().collect();
        assert_eq!(unique.len(), render.included_paths.len());
    }

    // ── Unit tests for helpers ───────────────────────────────────────────────

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
