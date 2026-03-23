//! `::toc-linking` directive — generates markdown links to headings in
//! referenced files.
//!
//! This is a Stage 1 (Preparation) compose step that runs between interpolation
//! and cleanup. It scans the document for `::toc-linking` directives, reads
//! the referenced markdown file(s), extracts their TOC, applies level/glob
//! filtering and text cleanup, and replaces the directive with a bullet list
//! of markdown links.
//!
//! ## Examples
//!
//! ```text
//! ::toc-linking ./api-reference.md level=h2,h3 cleanup=emoji_leader
//! ```
//!
//! Produces links like:
//!
//! ```text
//! - [Getting Started](./api-reference.md#getting-started)
//! - [Configuration](./api-reference.md#configuration)
//! ```

mod cleanup;
mod filter;
mod parser;
mod render;
mod types;

pub use types::TocLinkingError;

use crate::markdown::Markdown;
use crate::markdown::compose::transclusion::resolve_path;
use crate::markdown::compose::{ComposeSource, TransclusionOptions};
use filter::HeadingFilter;
use parser::parse_toc_linking_directives;
use render::render_toc_links;

/// Processes all `::toc-linking` directives in the content.
///
/// Returns the composed content and the number of directives expanded.
pub(crate) fn process_toc_linking(
    content: &str,
    source: &ComposeSource,
    transclusion_options: &TransclusionOptions,
    _fail_fast: bool,
) -> Result<(String, usize), TocLinkingError> {
    let directives = parse_toc_linking_directives(content)?;
    if directives.is_empty() {
        return Ok((content.to_string(), 0));
    }

    let mut replacements: Vec<(std::ops::Range<usize>, String)> = Vec::new();

    for directive in &directives {
        let heading_filter = HeadingFilter::new(
            &directive.options.keep_patterns,
            &directive.options.filter_patterns,
            directive.line,
        )?;

        let mut found = false;

        for target in &directive.targets {
            match resolve_file(target, transclusion_options, source, directive.line) {
                Ok(path) => {
                    let file_content = std::fs::read_to_string(&path)?;
                    let md: Markdown = file_content.as_str().into();
                    let toc = md.toc();
                    let headings = toc.all_headings();

                    let replacement =
                        render_toc_links(&headings, target, &directive.options, &heading_filter);

                    replacements.push((directive.span.clone(), replacement));
                    found = true;
                    break;
                }
                Err(_) => {
                    // Try next fallback in chain
                    continue;
                }
            }
        }

        if !found {
            if directive.suppress_not_found || directive.targets.is_empty() {
                // Suppress: replace with empty_text or empty string
                let replacement = directive.options.empty_text.clone().unwrap_or_default();
                replacements.push((directive.span.clone(), replacement));
            } else {
                let paths = directive.targets.join(", ");
                return Err(TocLinkingError::FileNotFound {
                    path: paths,
                    line: directive.line,
                });
            }
        }
    }

    let count = replacements.len();
    let mut result = content.to_string();

    // Apply replacements in reverse order to preserve byte offsets
    for (span, replacement) in replacements.into_iter().rev() {
        result.replace_range(span, &replacement);
    }

    Ok((result, count))
}

/// Resolves a file path relative to the source document.
fn resolve_file(
    target: &str,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
) -> Result<std::path::PathBuf, TocLinkingError> {
    let path =
        resolve_path(target, options, source, line).map_err(|_| TocLinkingError::FileNotFound {
            path: target.to_string(),
            line,
        })?;

    if !path.exists() {
        return Err(TocLinkingError::FileNotFound {
            path: target.to_string(),
            line,
        });
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeOptions;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn full_pipeline_with_real_file() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n\n### Advanced\n\nDetails.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = format!("Before\n\n::toc-linking ./api.md\n\nAfter\n");
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        assert!(result.contains("- [Getting Started](./api.md#getting-started)"));
        assert!(result.contains("- [Configuration](./api.md#configuration)"));
        assert!(result.contains("- [Advanced](./api.md#advanced)"));
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
        // H1 should be excluded by default (H2-H6)
        assert!(!result.contains("- [API]"));
    }

    #[test]
    fn fallback_chain_first_missing_second_found() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "backup.md", "## Backup Section\n\nContent.\n");
        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "::toc-linking \"./missing.md | ./backup.md\"\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        assert!(result.contains("- [Backup Section](./backup.md#backup-section)"));
    }

    #[test]
    fn suppress_not_found_no_error() {
        let dir = TempDir::new().unwrap();
        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "::toc-linking \"./nope.md | false\"\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        assert_eq!(result.trim(), "");
    }

    #[test]
    fn multiple_directives() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "a.md", "## A Section\n\nContent.\n");
        write_file(dir.path(), "b.md", "## B Section\n\nContent.\n");
        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "::toc-linking ./a.md\n\n::toc-linking ./b.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 2);
        assert!(result.contains("A Section"));
        assert!(result.contains("B Section"));
    }

    #[test]
    fn stage_toggle_disabled() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "api.md", "## Test\n");
        let source_path = write_file(dir.path(), "source.md", "::toc-linking ./api.md\n");

        let md: Markdown = std::fs::read_to_string(&source_path)
            .unwrap()
            .as_str()
            .into();
        let options = ComposeOptions::new()
            .with_source_file(&source_path)
            .disable(crate::markdown::compose::ComposeOperation::TocLinking)
            .disable(crate::markdown::compose::ComposeOperation::PageBlocks)
            .disable(crate::markdown::compose::ComposeOperation::BlockTransclusion)
            .disable(crate::markdown::compose::ComposeOperation::FrontmatterTransclusion)
            .disable(crate::markdown::compose::ComposeOperation::CodeTransclusion);

        let (result, report) = md.compose_with(options).unwrap();
        // Directive should remain in content when stage is disabled
        assert!(result.content.contains("::toc-linking"));
        assert_eq!(report.toc_links_generated, 0);
    }

    #[test]
    fn compose_report_counts() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "api.md", "## Test\n");
        let source_path = write_file(dir.path(), "source.md", "::toc-linking ./api.md\n");

        let md: Markdown = std::fs::read_to_string(&source_path)
            .unwrap()
            .as_str()
            .into();
        let options = ComposeOptions::new()
            .with_source_file(&source_path)
            .disable(crate::markdown::compose::ComposeOperation::PageBlocks)
            .disable(crate::markdown::compose::ComposeOperation::BlockTransclusion)
            .disable(crate::markdown::compose::ComposeOperation::FrontmatterTransclusion)
            .disable(crate::markdown::compose::ComposeOperation::CodeTransclusion);

        let (_, report) = md.compose_with(options).unwrap();
        assert!(report.toc_links_generated > 0);
        assert!(report.has_changes());
        assert!(report.summary().contains("toc-link"));
    }
}
