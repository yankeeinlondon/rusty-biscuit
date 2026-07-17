//! File reference resolution for composition sources.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, PathPosition, find_git_root, find_package_area, home_dir};
use darkmatter::markdown::{Markdown, MarkdownError};

use super::error::{CompositionError, MarkdownLoadCause};
use super::types::ResolvedCompositionSource;

/// Map a Markdown load failure to the most actionable `CompositionError`.
///
/// A malformed-frontmatter failure routes to [`CompositionError::FrontmatterParse`]
/// (which carries the typed error for rich rendering); any other failure
/// (e.g. a parse error from `try_from`) routes to
/// [`CompositionError::MarkdownLoad`] carrying the typed
/// [`MarkdownLoadCause`].
fn map_load_error(path: &Path, err: MarkdownError) -> CompositionError {
    match err {
        MarkdownError::FrontmatterParse { .. }
        | MarkdownError::FrontmatterFenceMismatch { .. } => CompositionError::FrontmatterParse(err),
        other => CompositionError::MarkdownLoad {
            path: path.to_path_buf(),
            source: MarkdownLoadCause::Parse(Box::new(other)),
        },
    }
}

/// Resolve a file reference string to a loaded Markdown document.
///
/// Uses `biscuit-file::FileReference` for all path resolution. Validates
/// that the resolved file has a `.md` or `.markdown` extension.
///
/// When invoked from inside a Cargo workspace package area (common for
/// monorepo prompts like `@prompts/commit.md`), the package area and the
/// convention prompt directories are added as prepended magic search roots
/// so a bare `@<file>` resolves to the closest matching prompt.
pub fn resolve_composition_source(
    file_ref: &str,
) -> Result<ResolvedCompositionSource, CompositionError> {
    // Phase 3 (2026-05-09-slow-prep): instrument the file-reference
    // resolution phase so trace inspection / `--perf` reporting can see
    // when the `biscuit-file` resolver dominates compose prep cost.
    let _span = tracing::info_span!("compose_prep.file_reference", file = %file_ref).entered();
    let reference = build_prompt_reference(file_ref)?;

    let resolved_path = reference
        .resolve()
        .map_err(|e| CompositionError::InvalidReference {
            reference: file_ref.to_string(),
            source: e,
        })?
        .ok_or_else(|| CompositionError::FileNotFound(file_ref.to_string()))?;

    // Validate markdown extension
    let ext = resolved_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown") {
        return Err(CompositionError::NotMarkdown(
            resolved_path.display().to_string(),
        ));
    }

    let original_text = fs::read_to_string(&resolved_path).map_err(|e| {
        CompositionError::MarkdownLoad {
            path: resolved_path.clone(),
            source: MarkdownLoadCause::Read(e),
        }
    })?;

    // Parse fallibly so malformed frontmatter surfaces as a real error. The
    // infallible `From<String>` drops a `FrontmatterParse` error and returns an
    // empty-frontmatter document, which downstream looks like a *missing*
    // `prompt` property — hiding the actual YAML syntax error from the user.
    let markdown =
        Markdown::try_from(resolved_path.as_path()).map_err(|e| map_load_error(&resolved_path, e))?;

    Ok(ResolvedCompositionSource {
        original_ref: file_ref.to_string(),
        resolved_path,
        original_text,
        markdown,
    })
}

/// Build a [`FileReference`] for a top-level Claudine prompt argument with the
/// convention prompt directories registered as magic (`@`) search roots.
///
/// The prompt magic roots (package area, `<area>/prompts`, `<repo>/prompts`,
/// `<repo>/.claudine/prompts`, `~/.claudine/prompts`) are the explicit
/// `FileReference` configuration Claudine layers on top of the shared grammar
/// (D1). Every top-level `compose`/`sequence`/`inline-compose` source resolver
/// shares this builder so a bare or `@`-prefixed reference resolves through the
/// identical roots regardless of the file's extension — `@foo.yaml` and
/// `@foo.md` can no longer diverge.
///
/// ## Errors
///
/// Returns [`CompositionError::InvalidReference`] when the reference string is
/// syntactically invalid.
pub fn build_prompt_reference(file_ref: &str) -> Result<FileReference, CompositionError> {
    Ok(with_prompt_magic_paths(FileReference::new(file_ref).map_err(
        |e| CompositionError::InvalidReference {
            reference: file_ref.to_string(),
            source: e,
        },
    )?))
}

/// Register the convention prompt directories as magic (`@`) search roots.
///
/// Mirrors the completion engine's magic scope set so a value the user
/// tab-completed to `@<file>` resolves at launch: the roots are the
/// package-area, repo, and HOME `prompts/` directories, registered
/// **closest-first**. Because [`FileReference::resolve`] returns the first
/// existing candidate, the nearest prompt wins.
///
/// The package area is also registered as a bare root (replacing
/// `with_package_area_magic_path`), so the single `cargo metadata` probe
/// serves both the bare `@<file>` form and the path-shaped
/// `@prompts/<file>` form.
fn with_prompt_magic_paths(reference: FileReference) -> FileReference {
    let Ok(cwd) = std::env::current_dir() else {
        return reference;
    };
    let git_root = find_git_root(&cwd).ok().flatten();
    let package_area = git_root
        .as_deref()
        .and_then(|root| find_package_area(root, &cwd).ok().flatten());

    prompt_magic_roots(
        git_root.as_deref(),
        package_area.as_deref(),
        home_dir().as_deref(),
    )
    .into_iter()
    .fold(reference, |reference, root| {
        reference.add_magic_path(root, PathPosition::Start)
    })
}

/// The convention prompt directories, **closest-first**: package area, then
/// repo (`prompts/` then `.claudine/prompts/`), then HOME `~/.claudine/
/// prompts`. The bare package-area root is included first so the single
/// `cargo metadata` probe also serves the path-shaped `@prompts/<file>` form.
///
/// Pure (no IO) so the ordering is unit-testable; the IO that discovers the
/// anchors lives in [`with_prompt_magic_paths`].
fn prompt_magic_roots(
    git_root: Option<&Path>,
    package_area: Option<&Path>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(area) = package_area {
        roots.push(area.to_path_buf());
        roots.push(area.join("prompts"));
    }
    if let Some(root) = git_root {
        roots.push(root.join("prompts"));
        roots.push(root.join(".claudine").join("prompts"));
    }
    if let Some(home) = home {
        roots.push(home.join(".claudine").join("prompts"));
    }
    roots
}

/// Enrich a source-load error with the authored frontmatter block.
///
/// Source-load failures can happen after the file has resolved and been read
/// but before a [`ResolvedCompositionSource`] exists. This helper reconstructs
/// the resolved source text for the CLI render boundary and leaves the error
/// unchanged when the file cannot be resolved/read again or the error is not
/// frontmatter-rooted.
pub fn enrich_composition_source_load_error(
    file_ref: &str,
    error: CompositionError,
    stderr_is_tty: bool,
) -> CompositionError {
    let Some(source_text) = read_source_text_for_enrichment(file_ref) else {
        return error;
    };
    error.enrich_frontmatter_text(&source_text, stderr_is_tty)
}

fn read_source_text_for_enrichment(file_ref: &str) -> Option<String> {
    // Re-resolve through the SAME prompt magic roots the launch-time resolver
    // used ([`build_prompt_reference`]); resolving through only the package-area
    // root would fail to re-find a file that launched via a `prompts/` root and
    // silently degrade the enriched render.
    let reference = build_prompt_reference(file_ref).ok()?;
    let resolved_path = reference.resolve().ok()??;

    let ext = resolved_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown") {
        return None;
    }

    fs::read_to_string(resolved_path).ok()
}

/// Validate that the resolved file is readable and writable.
///
/// This is a cross-provider pre-flight check: regardless of which agent
/// is used, the inline composition workflow requires the agent to read
/// the file (to understand context) and write back (to update the body).
pub fn validate_file_permissions(path: &Path) -> Result<(), CompositionError> {
    // Try opening for write — the most reliable cross-platform method,
    // delegating the actual permission decision to the OS.
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| CompositionError::InsufficientFilePermissions {
            path: path.to_path_buf(),
            source: e,
        })?;

    Ok(())
}

/// Validate that a path has a markdown extension.
#[allow(dead_code)]
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
