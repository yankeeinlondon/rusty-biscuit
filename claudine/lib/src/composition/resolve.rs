//! File reference resolution for composition sources.

use std::path::Path;

use biscuit_file::FileReference;
use darkmatter::markdown::Markdown;

use super::error::CompositionError;
use super::types::ResolvedCompositionSource;

/// Resolve a file reference string to a loaded Markdown document.
///
/// Uses `biscuit-file::FileReference` for all path resolution. Validates
/// that the resolved file has a `.md` or `.markdown` extension.
pub fn resolve_composition_source(
    file_ref: &str,
) -> Result<ResolvedCompositionSource, CompositionError> {
    let reference = FileReference::new(file_ref)
        .map_err(|e| CompositionError::InvalidReference(format!("{file_ref}: {e}")))?;

    let resolved_path = reference
        .resolve()
        .map_err(|e| CompositionError::InvalidReference(format!("{file_ref}: {e}")))?
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

    let markdown = Markdown::try_from(resolved_path.as_path())
        .map_err(|e| CompositionError::MarkdownLoad(format!("{}: {e}", resolved_path.display())))?;

    Ok(ResolvedCompositionSource {
        original_ref: file_ref.to_string(),
        resolved_path,
        markdown,
    })
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
    use tempfile::TempDir;

    #[test]
    fn resolve_absolute_markdown_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(&file, "---\ntitle: Test\n---\n# Hello").unwrap();

        let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
        assert_eq!(result.resolved_path, file);
        assert_eq!(result.original_ref, file.to_str().unwrap());

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
    fn resolve_markdown_extension() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.markdown");
        fs::write(&file, "# Hello").unwrap();

        let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
        assert_eq!(result.resolved_path, file);
    }

    #[test]
    fn is_markdown_path_variants() {
        assert!(is_markdown_path(Path::new("test.md")));
        assert!(is_markdown_path(Path::new("test.markdown")));
        assert!(is_markdown_path(Path::new("test.MD")));
        assert!(!is_markdown_path(Path::new("test.txt")));
        assert!(!is_markdown_path(Path::new("test")));
    }
}
