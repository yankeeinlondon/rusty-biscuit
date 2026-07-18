//! Whole-document formatting backed by the Darkmatter cleanup pipeline
//! (spec criterion 8).
//!
//! `textDocument/formatting` delegates to the same `Markdown::cleanup` sequence
//! the `md clean` CLI runs, so DMLS formatting is byte-equivalent to the library
//! for the configured options ([`FormattingConfig`](crate::config::FormattingConfig)):
//! the cleanup variant, then an optional fixed-width reflow. Frontmatter and
//! directive lines pass through untouched — cleanup only normalizes body prose,
//! list, and table geometry, and the frontmatter round-trips through the same
//! `as_string` path the CLI uses.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::reflow_to_width;
use lsp_types::{Position, Range, TextEdit};

use super::DocumentContext;
use crate::config::{CleanupVariant, DmlsConfig};

/// Whole-document formatting edits for the current buffer.
///
/// ## Returns
///
/// A single full-document replacement, or an empty vec when the document is
/// already canonical (the client applies nothing) or the end position cannot be
/// mapped.
pub fn formatting(ctx: &DocumentContext) -> Vec<TextEdit> {
    let formatted = format_text(ctx.text, ctx.config);
    if formatted == ctx.text {
        return Vec::new();
    }
    let Some(range) = whole_document_range(ctx) else {
        return Vec::new();
    };
    vec![TextEdit {
        range,
        new_text: formatted,
    }]
}

/// Runs the configured cleanup over `text`.
///
/// Mirrors `md clean`'s `apply_cleanup` (with no explicit indent): the variant
/// cleanup first, then a fixed-width reflow of the body when configured. The
/// document is reassembled through `Markdown::as_string`, exactly as the CLI
/// does, so the output is byte-identical to the library for the same options.
pub fn format_text(text: &str, config: &DmlsConfig) -> String {
    let mut md: Markdown = text.into();
    match config.formatting.cleanup {
        CleanupVariant::Default => {
            md.cleanup();
        }
        CleanupVariant::Compact => {
            md.cleanup_compact();
        }
        CleanupVariant::Loose => {
            md.cleanup_loose();
        }
    }
    if let Some(width) = config.formatting.fixed_width {
        let reflowed = reflow_to_width(md.content(), width as usize);
        *md.content_mut() = reflowed;
    }
    md.as_string()
}

/// The range spanning the whole document (start through the mapped EOF).
fn whole_document_range(ctx: &DocumentContext) -> Option<Range> {
    let end = ctx.source_map.byte_to_lsp(ctx.text.len())?;
    Some(Range::new(Position::new(0, 0), end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DmlsConfig;

    /// The provider output must equal the library cleanup sequence verbatim
    /// (spec criterion 8): DMLS adds no formatting of its own.
    #[test]
    fn test_format_text_is_byte_equivalent_to_library_cleanup() {
        let source = "# Title\nParagraph one.\n## Sub\ntext";
        let config = DmlsConfig::default();

        let mut md: Markdown = source.into();
        md.cleanup();
        let expected = md.as_string();

        assert_eq!(format_text(source, &config), expected);
    }

    #[test]
    fn test_format_text_honors_compact_variant() {
        let source = "- a\n\n- b\n";
        let mut config = DmlsConfig::default();
        config.formatting.cleanup = CleanupVariant::Compact;

        let mut md: Markdown = source.into();
        md.cleanup_compact();
        assert_eq!(format_text(source, &config), md.as_string());
    }

    #[test]
    fn test_format_text_applies_fixed_width_reflow() {
        let source = "One two three four five six seven eight nine ten eleven twelve.\n";
        let mut config = DmlsConfig::default();
        config.formatting.fixed_width = Some(20);

        let mut md: Markdown = source.into();
        md.cleanup();
        let reflowed = reflow_to_width(md.content(), 20);
        *md.content_mut() = reflowed;
        assert_eq!(format_text(source, &config), md.as_string());
    }

    #[test]
    fn test_format_text_reflows_complete_list_paragraph_like_library_cleanup() {
        let source = "- Alpha beta gamma delta\n    epsilon zeta eta theta.\n";
        let mut config = DmlsConfig::default();
        config.formatting.fixed_width = Some(24);

        let mut md: Markdown = source.into();
        md.cleanup();
        let reflowed = reflow_to_width(md.content(), 24);
        *md.content_mut() = reflowed;
        let expected = md.as_string();

        assert_eq!(format_text(source, &config), expected);
        assert_eq!(
            expected,
            "- Alpha beta gamma delta\n  epsilon zeta eta\n  theta.\n"
        );
    }

    #[test]
    fn test_format_text_preserves_frontmatter_and_directive_lines() {
        let source = "---\ntitle: Doc\n---\n\n# Body\n\n::file ./intro.md\n";
        let config = DmlsConfig::default();
        let formatted = format_text(source, &config);
        // A round-trippable frontmatter key and a directive line both survive.
        assert!(formatted.contains("title: Doc"));
        assert!(formatted.contains("::file ./intro.md"));
    }

    #[test]
    fn test_format_text_idempotent_on_canonical_document() {
        let source = "# Title\n\nParagraph.\n";
        let config = DmlsConfig::default();
        let once = format_text(source, &config);
        assert_eq!(format_text(&once, &config), once);
    }
}
