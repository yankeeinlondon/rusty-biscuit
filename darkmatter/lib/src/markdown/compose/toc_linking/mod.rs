//! `::toc-linking` directive — generates markdown links to headings in
//! referenced files.
//!
//! This is a transclusion-phase compose step. It scans the document for
//! `::toc-linking` directives, reads the referenced markdown file(s), extracts
//! their TOC from raw source content, applies level/glob filtering and text
//! cleanup, and replaces the directive with a bullet list of markdown links.
//!
//! ## Indentation Preservation
//!
//! The directive preserves the indentation context of the line it appears on.
//! Each generated link line is prefixed with the same leading whitespace as the
//! directive itself. When the directive is at column 1 (no leading whitespace),
//! the parser scans backward to the previous non-empty line and uses its
//! indentation as a fallback. This ensures that `::toc-linking` inside list
//! items and other whitespace-indented containers produces correctly nested
//! output.
//!
//! ## Limitations
//!
//! Directives inside blockquotes (`> ::toc-linking ...`) are not recognized
//! because the parser requires the line to start with `::toc-linking` after
//! trimming whitespace. Blockquote support would require changes to the
//! directive detection logic.
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
pub(crate) use types::{TocLinkingDirective, TocLinkingOptions};

#[cfg(test)]
use crate::markdown::Markdown;
use crate::markdown::compose::transclusion::{DirectiveKind, resolve_path};
use crate::markdown::compose::{ComposeSource, TransclusionOptions};
use crate::markdown::toc::MarkdownTocNode;
use biscuit_terminal::errors::SourceContext;
use filter::HeadingFilter;
use parser::parse_toc_linking_directives;
use render::render_toc_links;

pub(crate) use render::indent_text;
use tracing::trace;

#[cfg(test)]
use tracing::debug;

/// Parses all `::toc-linking` directives from document content.
pub(crate) fn parse_directives(content: &str) -> Result<Vec<TocLinkingDirective>, TocLinkingError> {
    parse_toc_linking_directives(content)
}

/// Resolves the first existing file in a toc-linking fallback chain.
pub(crate) fn resolve_target_chain(
    directive: &TocLinkingDirective,
    source: &ComposeSource,
    transclusion_options: &TransclusionOptions,
    ctx: SourceContext,
) -> Result<Option<(String, std::path::PathBuf)>, TocLinkingError> {
    trace!(target = %directive.targets.join(" > "), "toc_linking: resolving target chain");

    for target in &directive.targets {
        match resolve_file(
            target,
            transclusion_options,
            source,
            directive.line,
            ctx.clone(),
        ) {
            Ok(path) => return Ok(Some((target.clone(), path))),
            Err(_) => continue,
        }
    }

    if directive.suppress_not_found || directive.targets.is_empty() {
        Ok(None)
    } else {
        Err(TocLinkingError::FileNotFound {
            path: directive.targets.join(", "),
            line: directive.line,
        })
    }
}

/// Renders the markdown replacement for one resolved toc-linking directive.
///
/// The `indent` parameter is the leading whitespace of the directive line.
/// When `indent` is empty, `inferred_indent` is used as a fallback (e.g.
/// when the directive is at column 1 inside a list item continuation).
pub(crate) fn render_resolved_directive(
    display_target: &str,
    headings: &[MarkdownTocNode],
    options: &TocLinkingOptions,
    line: usize,
    indent: &str,
    inferred_indent: Option<&str>,
) -> Result<String, TocLinkingError> {
    let heading_filter =
        HeadingFilter::new(&options.keep_patterns, &options.filter_patterns, line)?;
    let heading_refs: Vec<&MarkdownTocNode> = headings.iter().collect();
    Ok(render_toc_links(
        &heading_refs,
        display_target,
        options,
        &heading_filter,
        indent,
        inferred_indent,
    ))
}

/// Processes all `::toc-linking` directives in the content.
///
/// Returns the composed content and the number of directives expanded.
#[cfg(test)]
pub(crate) fn process_toc_linking(
    content: &str,
    source: &ComposeSource,
    transclusion_options: &TransclusionOptions,
    _fail_fast: bool,
) -> Result<(String, usize), TocLinkingError> {
    debug!("toc_linking: processing directives");

    let directives = parse_toc_linking_directives(content)?;
    if directives.is_empty() {
        return Ok((content.to_string(), 0));
    }

    let mut replacements: Vec<(std::ops::Range<usize>, String)> = Vec::new();

    let ctx = match source {
        ComposeSource::File(path) => {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            SourceContext::new(path.clone(), path.clone(), content)
        }
        _ => SourceContext::new(
            std::path::PathBuf::from("<unknown>"),
            std::path::PathBuf::from("<unknown>"),
            content.to_string(),
        ),
    };
    for directive in &directives {
        if let Some((target, path)) =
            resolve_target_chain(directive, source, transclusion_options, ctx.clone())?
        {
            let file_content = std::fs::read_to_string(&path)?;
            let md: Markdown = file_content.as_str().into();
            let headings = md
                .toc()
                .all_headings()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();

            let replacement = render_resolved_directive(
                &target,
                &headings,
                &directive.options,
                directive.line,
                &directive.indent,
                directive.inferred_indent.as_deref(),
            )?;
            replacements.push((directive.span.clone(), replacement));
        } else {
            let empty_text = directive.options.empty_text.clone().unwrap_or_default();
            let replacement = indent_text(
                &empty_text,
                &directive.indent,
                directive.inferred_indent.as_deref(),
            );
            replacements.push((directive.span.clone(), replacement));
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
    ctx: SourceContext,
) -> Result<std::path::PathBuf, TocLinkingError> {
    let path =
        resolve_path(target, DirectiveKind::File, options, source, line, ctx).map_err(|_| {
            TocLinkingError::FileNotFound {
                path: target.to_string(),
                line,
            }
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

        let content = "Before\n\n::toc-linking ./api.md\n\nAfter\n".to_string();
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

    #[test]
    fn indented_toc_linking_nested_two_levels() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "- Item\n    - Subitem\n        ::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        // Every generated bullet should have 8 leading spaces (two levels deep)
        for line in result.lines() {
            if line.contains("[Getting Started]") || line.contains("[Configuration]") {
                assert!(
                    line.starts_with("        "),
                    "Expected line to start with 8 spaces: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn indented_toc_linking_at_column_one_in_list() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        // True column-1 directive inside a CommonMark lazy continuation
        // of a list item — the directive has no leading whitespace, but
        // the previous line is `- Item`, so the output must indent to
        // the item's content column (2 spaces).
        let content = "- Item\n::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        for line in result.lines() {
            if line.contains("[Getting Started]") || line.contains("[Configuration]") {
                assert!(
                    line.starts_with("  "),
                    "Expected line to be indented to list content column: {:?}",
                    line
                );
            }
        }

        use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
        let parser = Parser::new_ext(&result, Options::all());
        let events: Vec<Event> = parser.collect();

        let list_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::Start(Tag::List(_))))
            .collect();
        assert!(
            list_starts.len() >= 2,
            "Expected at least 2 list levels (outer item + nested TOC), got {}.\nResult:\n{}",
            list_starts.len(),
            result
        );

        let mut list_depth = 0;
        let mut found_toc_link_in_nested_list = false;
        for event in &events {
            match event {
                Event::Start(Tag::List(_)) => list_depth += 1,
                Event::End(TagEnd::List(_)) => list_depth -= 1,
                Event::Start(Tag::Link { dest_url, .. })
                    if list_depth >= 2 && dest_url.contains("api.md") =>
                {
                    found_toc_link_in_nested_list = true;
                }
                _ => {}
            }
        }
        assert!(
            found_toc_link_in_nested_list,
            "Expected TOC entries to be children of the outer list item, not siblings.\nResult:\n{}",
            result
        );
    }

    #[test]
    fn toc_linking_at_root_unchanged() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        // Output should start at column 1, unchanged from current behavior
        for line in result.lines() {
            if line.contains("[Getting Started]") {
                assert!(
                    line.starts_with("- ["),
                    "Expected line to start at column 1: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn indented_toc_linking_with_tabs() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "- Item\n\t::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);
        for line in result.lines() {
            if line.contains("[Getting Started]") {
                assert!(
                    line.starts_with('\t'),
                    "Expected line to start with tab: {:?}",
                    line
                );
            }
        }
    }

    #[test]
    fn multiple_directives_at_different_indentation_levels() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "::toc-linking ./api.md\n\n- Item\n    ::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 2);

        let mut root_found = false;
        let mut indented_found = false;

        for line in result.lines() {
            if line.contains("[Getting Started]") || line.contains("[Configuration]") {
                if line.starts_with("- [") {
                    root_found = true;
                } else if line.starts_with("    ") {
                    indented_found = true;
                }
            }
        }

        assert!(root_found, "Expected at least one root-level link");
        assert!(indented_found, "Expected at least one indented link");
    }

    #[test]
    fn empty_text_with_indentation() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "api.md", "# API\n");

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content =
            "- Item\n  ::toc-linking ./api.md level=h2 empty=\"no results\"\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, count) = process_toc_linking(&content, &source, &options, false).unwrap();
        assert_eq!(count, 1);

        let lines: Vec<&str> = result.lines().collect();
        let toc_line = lines.iter().find(|l| l.contains("no results")).unwrap();
        assert!(
            toc_line.starts_with("  "),
            "Expected empty_text to be indented: {:?}",
            toc_line
        );
    }

    #[test]
    fn cache_reuse_preserves_per_directive_indentation() {
        // Two directives targeting the same file with the same options at
        // different indentation levels must each render at their own
        // indentation, even when the second lookup is a cache hit.
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n",
        );

        let source_content =
            "::toc-linking ./api.md\n\n- Item\n    ::toc-linking ./api.md\n".to_string();
        let source_path = write_file(dir.path(), "source.md", &source_content);

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

        let (result, report) = md.compose_with(options).unwrap();
        assert_eq!(report.toc_links_generated, 2);

        let mut root_count = 0usize;
        let mut indented_count = 0usize;
        for line in result.content.lines() {
            if line.contains("[Getting Started]") || line.contains("[Configuration]") {
                if line.starts_with("    - [") {
                    indented_count += 1;
                } else if line.starts_with("- [") {
                    root_count += 1;
                } else {
                    panic!(
                        "TOC line has unexpected indentation: {:?}\nFull output:\n{}",
                        line, result.content
                    );
                }
            }
        }
        assert!(
            root_count >= 2,
            "Expected the root-level directive to emit unindented links, got {root_count}.\nFull output:\n{}",
            result.content
        );
        assert!(
            indented_count >= 2,
            "Expected the indented directive to emit 4-space-indented links, got {indented_count}.\nFull output:\n{}",
            result.content
        );
    }

    #[test]
    fn ac4_roundtrip_commonmark_list_nesting() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "api.md",
            "# API\n\n## Getting Started\n\nIntro.\n\n## Configuration\n\nConfig.\n",
        );

        let source_path = dir.path().join("source.md");
        write_file(dir.path(), "source.md", "");

        let content = "- Item\n    - Subitem\n        ::toc-linking ./api.md\n".to_string();
        let source = ComposeSource::File(source_path);
        let options = TransclusionOptions {
            source: source.clone(),
            ..Default::default()
        };

        let (result, _) = process_toc_linking(&content, &source, &options, false).unwrap();

        use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

        let parser = Parser::new_ext(&result, Options::all());
        let events: Vec<Event> = parser.collect();

        // Expect 3 list levels: outer (- Item), inner (- Subitem), toc links
        let list_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::Start(Tag::List(_))))
            .collect();
        assert!(
            list_starts.len() >= 3,
            "Expected at least 3 list levels (outer, subitem, toc), found {}. Result:\n{}",
            list_starts.len(),
            result
        );

        // Verify that a TOC link appears inside a deeply nested list
        let mut list_depth = 0;
        let mut found_toc_link_in_nested_list = false;
        for event in &events {
            match event {
                Event::Start(Tag::List(_)) => list_depth += 1,
                Event::End(TagEnd::List(_)) => list_depth -= 1,
                Event::Start(Tag::Link { dest_url, .. })
                    if list_depth >= 3 && dest_url.contains("api.md") =>
                {
                    found_toc_link_in_nested_list = true;
                }
                _ => {}
            }
        }
        assert!(
            found_toc_link_in_nested_list,
            "Expected TOC links to be nested inside at least 3 levels of lists. Result:\n{}",
            result
        );
    }
}
