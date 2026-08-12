//! Sub-document region projection (frontmatter today, code fences later).
//!
//! A region keeps its three ranges distinct — opening delimiter line,
//! content, closing delimiter line — so a delimiter diagnostic can never be
//! confused with a content diagnostic. Projection composes **byte offsets
//! only**; line/column addition is deliberately absent because it breaks on
//! CRLF and astral characters (R-2).

use darkmatter::markdown::FrontmatterExtraction;
use darkmatter::markdown::span::SourceSpan;

/// A sub-document region of the host document, in host byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The opening delimiter line (frontmatter `---`, or a fence's
    /// info-string line), including its terminator.
    pub opening_delimiter: SourceSpan,
    /// The region content. Zero-width when the region is empty (e.g. empty
    /// frontmatter: the width-0 span between the delimiter lines).
    pub content: SourceSpan,
    /// The closing delimiter line including its terminator, or `None` when
    /// the closing delimiter is missing (an unclosed fence ends at EOF; no
    /// delimiter range is fabricated).
    pub closing_delimiter: Option<SourceSpan>,
}

impl Region {
    /// Builds the frontmatter region from a Phase-1 library extraction.
    pub fn from_frontmatter(extraction: &FrontmatterExtraction<'_>) -> Self {
        Self {
            opening_delimiter: extraction.block_span.start..extraction.yaml_span.start,
            content: extraction.yaml_span.clone(),
            closing_delimiter: Some(extraction.yaml_span.end..extraction.block_span.end),
        }
    }

    /// Projects a content-relative byte offset into a host byte offset.
    ///
    /// ## Returns
    ///
    /// `None` when the relative offset overruns the content (region-relative
    /// diagnostics must never land on a delimiter line).
    pub fn project_offset(&self, relative: usize) -> Option<usize> {
        let len = self.content.end - self.content.start;
        (relative <= len).then_some(self.content.start + relative)
    }

    /// Projects a content-relative byte span into a host byte span.
    pub fn project_span(&self, relative: SourceSpan) -> Option<SourceSpan> {
        if relative.end < relative.start {
            return None;
        }
        let start = self.project_offset(relative.start)?;
        let end = self.project_offset(relative.end)?;
        Some(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::{PositionEncoding, SourceMap};
    use darkmatter::markdown::extract_frontmatter_block;
    use lsp_types::Position;
    use std::sync::Arc;

    fn frontmatter_region(source: &str) -> Region {
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();
        Region::from_frontmatter(&extraction)
    }

    fn source_map(text: &str) -> SourceMap {
        let uri: lsp_types::Uri = "file:///test/doc.md".parse().unwrap();
        SourceMap::new(uri, 1, PositionEncoding::Utf16, Arc::from(text))
    }

    #[test]
    fn test_frontmatter_with_one_key() {
        let source = "---\ntitle: X\n---\nbody\n";
        let region = frontmatter_region(source);
        assert_eq!(region.opening_delimiter, 0..4);
        assert_eq!(region.content, 4..13);
        assert_eq!(region.closing_delimiter, Some(13..17));
        assert_eq!(&source[region.content.clone()], "title: X\n");
    }

    #[test]
    fn test_empty_frontmatter_zero_width_content() {
        let source = "---\n---\nbody\n";
        let region = frontmatter_region(source);
        assert_eq!(region.content, 4..4);
        assert_eq!(region.project_offset(0), Some(4));
        assert_eq!(region.project_offset(1), None);
    }

    #[test]
    fn test_frontmatter_without_trailing_body_newline() {
        let source = "---\na: 1\n---";
        let region = frontmatter_region(source);
        assert_eq!(region.content, 4..9);
        assert_eq!(region.closing_delimiter, Some(9..12));
    }

    #[test]
    fn test_frontmatter_with_crlf_delimiters() {
        let source = "---\r\ntitle: X\r\n---\r\nbody\r\n";
        let region = frontmatter_region(source);
        assert_eq!(region.opening_delimiter, 0..5);
        assert_eq!(&source[region.content.clone()], "title: X\r\n");
        assert_eq!(region.closing_delimiter, Some(15..20));
    }

    #[test]
    fn test_diagnostic_on_first_yaml_content_character() {
        // A parser diagnostic at YAML offset 0 maps to document line 1,
        // character 0 — never onto the opening delimiter.
        let source = "---\ntitle: X\n---\nbody\n";
        let sm = source_map(source);
        let region = frontmatter_region(source);
        let range = sm.region_range_to_lsp(&region, 0..5).unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(1, 5));
    }

    #[test]
    fn test_diagnostic_on_last_yaml_content_character() {
        // End of the last YAML content line maps before the closing
        // delimiter's line, not onto the closing delimiter.
        let source = "---\ntitle: X\n---\nbody\n";
        let sm = source_map(source);
        let region = frontmatter_region(source);
        // "title: X" is 8 bytes; span the final character.
        let range = sm.region_range_to_lsp(&region, 7..8).unwrap();
        assert_eq!(range.start, Position::new(1, 7));
        assert_eq!(range.end, Position::new(1, 8));
    }

    #[test]
    fn test_diagnostic_spanning_multiple_frontmatter_lines() {
        let source = "---\na: 1\nb: 2\n---\n";
        let sm = source_map(source);
        let region = frontmatter_region(source);
        // Span from "a" through "2": YAML-relative 0..9.
        let range = sm.region_range_to_lsp(&region, 0..9).unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(2, 4));
    }

    #[test]
    fn test_projection_cannot_land_on_delimiters() {
        let source = "---\na: 1\n---\n";
        let region = frontmatter_region(source);
        let content_len = region.content.end - region.content.start;
        // Relative offsets beyond the content overrun into the closing
        // delimiter and are rejected.
        assert_eq!(region.project_offset(content_len + 1), None);
        assert_eq!(region.project_span(0..content_len + 1), None);
    }

    #[test]
    fn test_code_fence_body_maps_after_opening_fence_line() {
        // Fence regions are hand-built until the library exposes fence
        // spans; the projection contract is the same as frontmatter.
        let source = "```rust\nlet x = 1;\n```\n";
        let region = Region {
            opening_delimiter: 0..8,
            content: 8..19,
            closing_delimiter: Some(19..23),
        };
        let sm = source_map(source);
        let range = sm.region_range_to_lsp(&region, 0..3).unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(1, 3));
    }

    #[test]
    fn test_info_string_diagnostic_on_opening_fence_line() {
        let source = "```rust\nlet x = 1;\n```\n";
        let region = Region {
            opening_delimiter: 0..8,
            content: 8..19,
            closing_delimiter: Some(19..23),
        };
        // Info-string diagnostics belong to the opening delimiter span,
        // converted directly (not via content projection).
        let sm = source_map(source);
        let range = sm.byte_range_to_lsp(3..7).unwrap();
        assert_eq!(range.start, Position::new(0, 3));
        assert_eq!(range.end, Position::new(0, 7));
        assert_eq!(region.opening_delimiter, 0..8);
    }

    #[test]
    fn test_missing_closing_fence_ends_at_eof_without_fabricated_delimiter() {
        let source = "```rust\nlet x = 1;";
        let region = Region {
            opening_delimiter: 0..8,
            content: 8..source.len(),
            closing_delimiter: None,
        };
        assert_eq!(region.closing_delimiter, None);
        assert_eq!(region.project_offset(10), Some(18));
        let sm = source_map(source);
        let range = sm.region_range_to_lsp(&region, 0..10).unwrap();
        assert_eq!(range.end, Position::new(1, 10));
    }

    #[test]
    fn test_projection_with_multibyte_yaml_content() {
        // Byte-base composition stays correct across multibyte YAML.
        let source = "---\nnom: café\n---\n";
        let sm = source_map(source);
        let region = frontmatter_region(source);
        // "nom: café" — é is 2 bytes, so the value span is bytes 5..10.
        let range = sm.region_range_to_lsp(&region, 5..10).unwrap();
        assert_eq!(range.start, Position::new(1, 5));
        // café = 4 UTF-16 units.
        assert_eq!(range.end, Position::new(1, 9));
    }
}
