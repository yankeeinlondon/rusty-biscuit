//! File reference resolution for composition sources.

use std::fs;
use std::path::Path;

use biscuit_file::FileReference;
use darkmatter::markdown::{Markdown, MarkdownError};

use super::error::CompositionError;
use super::types::ResolvedCompositionSource;

/// Map a Markdown load failure to the most actionable `CompositionError`.
///
/// A malformed-frontmatter failure routes to [`CompositionError::FrontmatterParse`]
/// (which carries the typed error for rich rendering); any other failure
/// (e.g. a read error from `try_from`) falls back to the flat
/// [`CompositionError::MarkdownLoad`] string.
fn map_load_error(path: &Path, err: MarkdownError) -> CompositionError {
    match err {
        MarkdownError::FrontmatterParse { .. }
        | MarkdownError::FrontmatterFenceMismatch { .. } => CompositionError::FrontmatterParse(err),
        other => CompositionError::MarkdownLoad(format!("{}: {other}", path.display())),
    }
}

/// Resolve a file reference string to a loaded Markdown document.
///
/// Uses `biscuit-file::FileReference` for all path resolution. Validates
/// that the resolved file has a `.md` or `.markdown` extension.
///
/// When invoked from inside a Cargo workspace package area (common for
/// monorepo prompts like `@prompts/commit.md`), the package area is added
/// as a prepended magic search root so package-local prompts are found
/// before repo-wide or HOME-scoped ones.
pub fn resolve_composition_source(
    file_ref: &str,
) -> Result<ResolvedCompositionSource, CompositionError> {
    // Phase 3 (2026-05-09-slow-prep): instrument the file-reference
    // resolution phase so trace inspection / `--perf` reporting can see
    // when the `biscuit-file` resolver dominates compose prep cost.
    let _span = tracing::info_span!("compose_prep.file_reference", file = %file_ref).entered();
    let reference = FileReference::new(file_ref)
        .map_err(|e| CompositionError::InvalidReference {
            reference: file_ref.to_string(),
            source: e,
        })?
        .with_package_area_magic_path();

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

    let original_text = fs::read_to_string(&resolved_path)
        .map_err(|e| CompositionError::MarkdownLoad(format!("{}: {e}", resolved_path.display())))?;

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
    let reference = FileReference::new(file_ref)
        .ok()?
        .with_package_area_magic_path();
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
        .map_err(|e| {
            CompositionError::InsufficientFilePermissions(format!("{}: {e}", path.display()))
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
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn resolve_absolute_markdown_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(&file, "---\ntitle: Test\n---\n# Hello").unwrap();

        let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
        assert_eq!(result.resolved_path, file);
        assert_eq!(result.original_ref, file.to_str().unwrap());
        assert_eq!(result.original_text, "---\ntitle: Test\n---\n# Hello");

        let title: Option<String> = result.markdown.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
    }

    #[test]
    fn resolve_rejects_non_markdown() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, CompositionError::NotMarkdown(_)));
    }

    #[test]
    fn resolve_missing_file() {
        let err = resolve_composition_source("/nonexistent/path/test.md").unwrap_err();
        assert!(matches!(err, CompositionError::FileNotFound(_)));
    }

    #[test]
    fn resolve_malformed_frontmatter_reports_parse_error_not_missing_prompt() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("metadata.md");
        // Block-scalar body indented 4 spaces on the first line, then 3 on a
        // later line — YAML closes the scalar early and chokes. Previously this
        // surfaced as a misleading `PromptPropertyMissing`.
        fs::write(
            &file,
            "---\nprompt: |-\n    First line sets indent to four.\n   Three spaces breaks it.\n---\n",
        )
        .unwrap();

        let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
        assert!(
            matches!(err, CompositionError::FrontmatterParse(_)),
            "expected FrontmatterParse, got: {err:?}"
        );
    }

    #[test]
    fn resolve_four_dash_fence_maps_to_frontmatter_parse() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("four-dash.md");
        fs::write(
            &file,
            "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
        )
        .unwrap();

        let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
        assert!(
            matches!(err, CompositionError::FrontmatterParse(_)),
            "expected FrontmatterParse for ---- fence, got: {err:?}"
        );
        assert!(
            !matches!(err, CompositionError::MarkdownLoad(_)),
            "must not fall back to MarkdownLoad: {err:?}"
        );
        assert!(
            !matches!(err, CompositionError::FileNotFound(_)),
            "must not report file not found: {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("----"),
            "error message should name the offending fence: {msg}"
        );
    }

    #[test]
    fn load_error_enrichment_wraps_actual_four_dash_source() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("four-dash.md");
        fs::write(
            &file,
            "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
        )
        .unwrap();

        let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
        let err = enrich_composition_source_load_error(file.to_str().unwrap(), err, true);

        match err {
            CompositionError::WithFrontmatter { inner, excerpt } => {
                assert!(
                    matches!(*inner, CompositionError::FrontmatterParse(_)),
                    "inner error should remain FrontmatterParse"
                );
                assert_eq!(excerpt.highlight_line(), Some(1));
            }
            other => panic!("expected WithFrontmatter, got: {other:?}"),
        }
    }

    #[test]
    fn resolve_markdown_extension() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.markdown");
        fs::write(&file, "# Hello").unwrap();

        let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
        assert_eq!(result.resolved_path, file);
    }

    #[test]
    fn validate_permissions_writable_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(&file, "# Hello").unwrap();
        assert!(validate_file_permissions(&file).is_ok());
    }

    #[test]
    fn validate_permissions_readonly_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("readonly.md");
        fs::write(&file, "# Hello").unwrap();
        let mut perms = fs::metadata(&file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file, perms).unwrap();

        let err = validate_file_permissions(&file).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InsufficientFilePermissions(_)
        ));

        // Cleanup: restore permissions so TempDir can delete
        let mut perms = fs::metadata(&file).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        #[cfg(not(unix))]
        {
            perms.set_readonly(false);
        }
        fs::set_permissions(&file, perms).unwrap();
    }

    #[test]
    fn validate_permissions_nonexistent_file() {
        let err = validate_file_permissions(Path::new("/nonexistent/path.md")).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InsufficientFilePermissions(_)
        ));
    }

    #[test]
    fn is_markdown_path_variants() {
        assert!(is_markdown_path(Path::new("test.md")));
        assert!(is_markdown_path(Path::new("test.markdown")));
        assert!(is_markdown_path(Path::new("test.MD")));
        assert!(!is_markdown_path(Path::new("test.txt")));
        assert!(!is_markdown_path(Path::new("test")));
    }

    /// Acceptance criterion #5: the shipped `prompts/cross-platform.md` prompt
    /// (already fixed to `---` fences) loads as a composition source with
    /// non-empty frontmatter and a body that begins with the real heading. No
    /// YAML keys from the frontmatter may leak into the body.
    #[test]
    fn cross_platform_prompt_composes_cleanly() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .expect("claudine/lib parent")
            .parent()
            .expect("workspace root");
        let path = workspace_root.join("prompts/cross-platform.md");

        let source = resolve_composition_source(path.to_str().unwrap())
            .expect("cross-platform.md should resolve and parse cleanly");

        assert!(
            !source.markdown.frontmatter().is_empty(),
            "frontmatter should be parsed and non-empty"
        );
        let name: Option<String> = source.markdown.fm_get("name").unwrap();
        assert_eq!(name, Some("cross-platform".to_string()));

        let content = source.markdown.content();
        assert!(
            content.starts_with("# Ensuring Cross Platform Support"),
            "body should start with the real heading; got: {content}"
        );
        assert!(
            !content.contains("name: cross-platform"),
            "frontmatter YAML must not leak into body: {content}"
        );
        assert!(
            !content.contains("description:"),
            "frontmatter YAML must not leak into body: {content}"
        );
    }
}
