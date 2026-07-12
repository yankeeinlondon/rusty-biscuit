//! Byte-offset ↔ LSP-position conversion for one document version.
//!
//! Wraps rust-analyzer's `line-index` crate (the R-2 recommendation) behind a
//! DMLS-owned API. The wrapper owns everything `line-index` does not:
//!
//! - the negotiated [`PositionEncoding`] (UTF-8 or UTF-16),
//! - lone-CR line endings (Markdown needs them; `line-index` scans `\n` only),
//! - protocol-boundary clamping semantics for client-supplied positions,
//! - `(uri, version)` stamping so ranges cannot be mixed across versions,
//! - sub-document [`Region`] projection (frontmatter today, code fences later).
//!
//! Raw `line-index` types never leak out of this module.

mod region;

pub use region::Region;

use std::sync::Arc;

use darkmatter::markdown::span::SourceSpan;
use line_index::{LineIndex, TextSize};
use lsp_types::{Position, Range as LspRange, Uri};

/// Position encoding negotiated with the client at `initialize`.
///
/// DMLS supports UTF-8 and UTF-16. UTF-16 is the LSP compatibility floor and
/// the default when negotiation is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionEncoding {
    /// LSP `character` counts UTF-8 code units (bytes).
    Utf8,
    /// LSP `character` counts UTF-16 code units.
    #[default]
    Utf16,
}

/// Line/offset map for one `(uri, version)` snapshot of a document.
///
/// All byte offsets index the exact text passed to [`SourceMap::new`] — no
/// normalization is applied to the stored text, so spans produced by
/// Darkmatter parsers over the same string are directly convertible.
#[derive(Debug)]
pub struct SourceMap {
    uri: Uri,
    version: i32,
    encoding: PositionEncoding,
    text: Arc<str>,
    // Built over a shadow copy in which each lone `\r` (not part of `\r\n`)
    // is replaced by `\n`. `\r` and `\n` are both one byte, so every byte
    // offset in the shadow text is identical to the original — the index
    // gains LSP-correct lone-CR line breaks without invalidating offsets.
    index: LineIndex,
}

impl SourceMap {
    /// Builds the map for one document snapshot.
    pub fn new(uri: Uri, version: i32, encoding: PositionEncoding, text: Arc<str>) -> Self {
        let index = match lone_cr_shadow(&text) {
            Some(shadow) => LineIndex::new(&shadow),
            None => LineIndex::new(&text),
        };
        Self {
            uri,
            version,
            encoding,
            text,
            index,
        }
    }

    /// URI this map was built for.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Client document version this map was built from.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Encoding LSP `character` values are counted in.
    pub fn encoding(&self) -> PositionEncoding {
        self.encoding
    }

    /// The exact document text the map was built over.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Converts a byte offset into an LSP position.
    ///
    /// ## Returns
    ///
    /// `None` when the offset is out of bounds or points into the middle of
    /// a multi-byte codepoint. Offsets that land on a line terminator clamp
    /// to the end-of-line position (a position can never point between `\r`
    /// and `\n`); an offset just *past* a terminator is line + 1, column 0.
    pub fn byte_to_lsp(&self, offset: usize) -> Option<Position> {
        let size = TextSize::new(u32::try_from(offset).ok()?);
        let line_col = self.index.try_line_col(size)?;
        let (line_start, content_end, _) = self.line_bounds(line_col.line)?;
        let clamped = offset.min(content_end);
        let character = match self.encoding {
            PositionEncoding::Utf8 => (clamped - line_start) as u32,
            PositionEncoding::Utf16 => {
                self.text[line_start..clamped].encode_utf16().count() as u32
            }
        };
        Some(Position::new(line_col.line, character))
    }

    /// Converts an LSP position into a byte offset.
    ///
    /// This is the protocol boundary, so client overshoot is clamped rather
    /// than rejected: a line past the last line maps to end of document, and
    /// a character past the end of a line maps to the end of that line's
    /// content (before its terminator).
    ///
    /// ## Returns
    ///
    /// `None` only for positions that are invalid rather than overshooting —
    /// a UTF-16 column inside a surrogate pair or a UTF-8 column inside a
    /// multi-byte codepoint.
    pub fn lsp_to_byte(&self, position: Position) -> Option<usize> {
        let Some((line_start, content_end, _)) = self.line_bounds(position.line) else {
            return Some(self.text.len());
        };
        let content = &self.text[line_start..content_end];
        match self.encoding {
            PositionEncoding::Utf8 => {
                let column = position.character as usize;
                if column >= content.len() {
                    return Some(content_end);
                }
                content.is_char_boundary(column).then_some(line_start + column)
            }
            PositionEncoding::Utf16 => {
                let target = position.character;
                let mut units: u32 = 0;
                for (idx, ch) in content.char_indices() {
                    if units == target {
                        return Some(line_start + idx);
                    }
                    units += ch.len_utf16() as u32;
                    if units > target {
                        return None;
                    }
                }
                Some(content_end)
            }
        }
    }

    /// Converts a byte span into an LSP range.
    ///
    /// A span whose end sits just past a line terminator projects its end to
    /// the next line, character 0.
    pub fn byte_range_to_lsp(&self, span: SourceSpan) -> Option<LspRange> {
        if span.end < span.start {
            return None;
        }
        let start = self.byte_to_lsp(span.start)?;
        let end = self.byte_to_lsp(span.end)?;
        Some(LspRange::new(start, end))
    }

    /// Converts an LSP range into a byte span (clamping like [`Self::lsp_to_byte`]).
    pub fn lsp_range_to_byte(&self, range: LspRange) -> Option<SourceSpan> {
        let start = self.lsp_to_byte(range.start)?;
        let end = self.lsp_to_byte(range.end)?;
        (start <= end).then_some(start..end)
    }

    /// Projects a region-content-relative byte span into an LSP range.
    ///
    /// Composition is byte-based only (never line/column addition, which
    /// breaks on CRLF and astral characters): the relative span is offset by
    /// the region's content base and then converted through this map.
    pub fn region_range_to_lsp(&self, region: &Region, relative: SourceSpan) -> Option<LspRange> {
        self.byte_range_to_lsp(region.project_span(relative)?)
    }

    /// Splits a byte span into per-line content subspans.
    ///
    /// Each returned subspan is confined to a single line and excludes that
    /// line's terminator (`\r\n`, `\n`, or lone `\r`), so a multi-line span is
    /// projected to one non-empty token per line. Empty fragments (a span that
    /// begins or ends inside a terminator, or covers only line breaks) are
    /// omitted. This is the LSP-safe way to satisfy the semantic-token rule
    /// that every token stays on one line regardless of client multiline
    /// support.
    ///
    /// ## Returns
    ///
    /// An empty vector for an empty/inverted span or a `start` offset that is
    /// out of bounds or mid-codepoint.
    pub fn split_span_by_line(&self, span: SourceSpan) -> Vec<SourceSpan> {
        let mut out = Vec::new();
        if span.end <= span.start {
            return out;
        }
        let Some(start_size) = u32::try_from(span.start).ok().map(TextSize::new) else {
            return out;
        };
        let Some(start_lc) = self.index.try_line_col(start_size) else {
            return out;
        };
        let mut line = start_lc.line;
        while let Some((line_start, content_end, _line_end)) = self.line_bounds(line) {
            if line_start >= span.end {
                break;
            }
            let seg_start = span.start.max(line_start);
            let seg_end = span.end.min(content_end);
            if seg_start < seg_end {
                out.push(seg_start..seg_end);
            }
            line += 1;
        }
        out
    }

    /// `(line_start, content_end, line_end)` byte offsets for `line`.
    ///
    /// `content_end` excludes the line terminator (`\r\n`, `\n`, or lone
    /// `\r`); `line_end` includes it.
    fn line_bounds(&self, line: u32) -> Option<(usize, usize, usize)> {
        let range = self.index.line(line)?;
        let start = usize::from(range.start());
        let end = usize::from(range.end());
        let raw = &self.text[start..end];
        let content_len = if raw.ends_with("\r\n") {
            raw.len() - 2
        } else if raw.ends_with('\n') || raw.ends_with('\r') {
            raw.len() - 1
        } else {
            raw.len()
        };
        Some((start, start + content_len, end))
    }
}

/// Returns a copy of `text` with every lone `\r` replaced by `\n`, or `None`
/// when the text has no lone CR (the common case, which avoids the copy).
fn lone_cr_shadow(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let has_lone_cr = bytes
        .iter()
        .enumerate()
        .any(|(idx, &byte)| byte == b'\r' && bytes.get(idx + 1) != Some(&b'\n'));
    if !has_lone_cr {
        return None;
    }
    let mut shadow = bytes.to_vec();
    for idx in 0..shadow.len() {
        if shadow[idx] == b'\r' && shadow.get(idx + 1) != Some(&b'\n') {
            shadow[idx] = b'\n';
        }
    }
    // Only ASCII `\r` bytes were replaced by ASCII `\n`, so UTF-8 validity
    // is preserved.
    Some(String::from_utf8(shadow).expect("single-byte ASCII substitution preserves UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(text: &str, encoding: PositionEncoding) -> SourceMap {
        let uri: Uri = "file:///test/doc.md".parse().unwrap();
        SourceMap::new(uri, 1, encoding, Arc::from(text))
    }

    fn map16(text: &str) -> SourceMap {
        map(text, PositionEncoding::Utf16)
    }

    fn map8(text: &str) -> SourceMap {
        map(text, PositionEncoding::Utf8)
    }

    fn pos(line: u32, character: u32) -> Position {
        Position::new(line, character)
    }

    #[test]
    fn test_stamping_uri_version_encoding() {
        let sm = map16("hello");
        assert_eq!(sm.uri().as_str(), "file:///test/doc.md");
        assert_eq!(sm.version(), 1);
        assert_eq!(sm.encoding(), PositionEncoding::Utf16);
    }

    #[test]
    fn test_empty_document() {
        let sm = map16("");
        assert_eq!(sm.byte_to_lsp(0), Some(pos(0, 0)));
        assert_eq!(sm.byte_to_lsp(1), None);
        assert_eq!(sm.lsp_to_byte(pos(0, 0)), Some(0));
        // Overshoot clamps at the protocol boundary.
        assert_eq!(sm.lsp_to_byte(pos(0, 5)), Some(0));
        assert_eq!(sm.lsp_to_byte(pos(3, 0)), Some(0));
    }

    #[test]
    fn test_single_ascii_line_without_trailing_newline() {
        let sm = map16("abc");
        assert_eq!(sm.byte_to_lsp(0), Some(pos(0, 0)));
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 3)));
        assert_eq!(sm.lsp_to_byte(pos(0, 3)), Some(3));
        // EOF insertions use the last line's content length.
        assert_eq!(sm.lsp_to_byte(pos(0, 9)), Some(3));
    }

    #[test]
    fn test_single_ascii_line_with_trailing_newline() {
        let sm = map16("abc\n");
        // Offset of the `\n` clamps to end-of-content.
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 3)));
        // Offset past the `\n` is the (empty) final line.
        assert_eq!(sm.byte_to_lsp(4), Some(pos(1, 0)));
        assert_eq!(sm.lsp_to_byte(pos(1, 0)), Some(4));
    }

    #[test]
    fn test_multiple_lf_lines() {
        let sm = map16("one\ntwo\nthree");
        assert_eq!(sm.byte_to_lsp(4), Some(pos(1, 0)));
        assert_eq!(sm.byte_to_lsp(8), Some(pos(2, 0)));
        assert_eq!(sm.lsp_to_byte(pos(2, 5)), Some(13));
    }

    #[test]
    fn test_crlf_lines() {
        let sm = map16("ab\r\ncd\r\n");
        assert_eq!(sm.byte_to_lsp(0), Some(pos(0, 0)));
        // Offset of `\r`: end-of-line position.
        assert_eq!(sm.byte_to_lsp(2), Some(pos(0, 2)));
        // Offset of `\n` inside CRLF also clamps to end-of-content: a
        // position can never point between `\r` and `\n`.
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 2)));
        assert_eq!(sm.byte_to_lsp(4), Some(pos(1, 0)));
        // Character overshoot clamps before the terminator, not into it.
        assert_eq!(sm.lsp_to_byte(pos(0, 99)), Some(2));
        assert_eq!(sm.lsp_to_byte(pos(1, 0)), Some(4));
    }

    #[test]
    fn test_lone_cr_lines() {
        let sm = map16("ab\rcd\ref");
        assert_eq!(sm.byte_to_lsp(3), Some(pos(1, 0)));
        assert_eq!(sm.byte_to_lsp(6), Some(pos(2, 0)));
        assert_eq!(sm.lsp_to_byte(pos(1, 1)), Some(4));
        assert_eq!(sm.lsp_to_byte(pos(2, 2)), Some(8));
        // Offset of the `\r` terminator clamps to end-of-content.
        assert_eq!(sm.byte_to_lsp(2), Some(pos(0, 2)));
    }

    #[test]
    fn test_mixed_line_endings() {
        let sm = map16("a\nb\r\nc\rd");
        assert_eq!(sm.byte_to_lsp(2), Some(pos(1, 0)));
        assert_eq!(sm.byte_to_lsp(5), Some(pos(2, 0)));
        assert_eq!(sm.byte_to_lsp(7), Some(pos(3, 0)));
        assert_eq!(sm.lsp_to_byte(pos(3, 1)), Some(8));
    }

    #[test]
    fn test_final_empty_line_after_newline() {
        let sm = map16("abc\n");
        assert_eq!(sm.lsp_to_byte(pos(1, 0)), Some(4));
        assert_eq!(sm.lsp_to_byte(pos(1, 7)), Some(4));
        assert_eq!(sm.byte_to_lsp(4), Some(pos(1, 0)));
    }

    #[test]
    fn test_final_nonempty_line_without_newline() {
        let sm = map16("abc\ndef");
        assert_eq!(sm.byte_to_lsp(7), Some(pos(1, 3)));
        assert_eq!(sm.lsp_to_byte(pos(1, 3)), Some(7));
    }

    #[test]
    fn test_bmp_multibyte_utf16() {
        // "aéβ宽b": é = 2 bytes / 1 unit, β = 2 bytes / 1 unit, 宽 = 3 bytes / 1 unit.
        let text = "aéβ宽b";
        let sm = map16(text);
        assert_eq!(sm.byte_to_lsp(1), Some(pos(0, 1))); // before é
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 2))); // before β
        assert_eq!(sm.byte_to_lsp(5), Some(pos(0, 3))); // before 宽
        assert_eq!(sm.byte_to_lsp(8), Some(pos(0, 4))); // before b
        assert_eq!(sm.lsp_to_byte(pos(0, 2)), Some(3));
        assert_eq!(sm.lsp_to_byte(pos(0, 4)), Some(8));
    }

    #[test]
    fn test_bmp_multibyte_utf8_byte_columns() {
        let text = "aéβ宽b";
        let sm = map8(text);
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 3)));
        assert_eq!(sm.lsp_to_byte(pos(0, 3)), Some(3));
        // A UTF-8 column inside a multi-byte codepoint is invalid.
        assert_eq!(sm.lsp_to_byte(pos(0, 2)), None);
    }

    #[test]
    fn test_astral_scalar() {
        // "a💡b": 💡 = 4 bytes, 2 UTF-16 units. Position after `b` is
        // byte col 6, UTF-16 col 4 (LSP spec's own example shape).
        let text = "a💡b";
        let sm16 = map16(text);
        assert_eq!(sm16.byte_to_lsp(1), Some(pos(0, 1)));
        assert_eq!(sm16.byte_to_lsp(5), Some(pos(0, 3)));
        assert_eq!(sm16.byte_to_lsp(6), Some(pos(0, 4)));
        assert_eq!(sm16.lsp_to_byte(pos(0, 3)), Some(5));
        // Column 2 is inside the surrogate pair: invalid, not clamped.
        assert_eq!(sm16.lsp_to_byte(pos(0, 2)), None);

        let sm8 = map8(text);
        assert_eq!(sm8.byte_to_lsp(5), Some(pos(0, 5)));
        assert_eq!(sm8.lsp_to_byte(pos(0, 5)), Some(5));
    }

    #[test]
    fn test_cjk_extension_b_scalar() {
        // U+20021 (𠀡-class CJK Ext B): 4 bytes, 2 UTF-16 units.
        let text = "a\u{20021}b";
        let sm = map16(text);
        assert_eq!(sm.byte_to_lsp(5), Some(pos(0, 3)));
        assert_eq!(sm.byte_to_lsp(6), Some(pos(0, 4)));
        assert_eq!(sm.lsp_to_byte(pos(0, 3)), Some(5));
    }

    #[test]
    fn test_combining_sequence_counts_code_units() {
        // e + U+0301: LSP counts code units, not grapheme clusters, so a
        // position may legally fall between the base and the combining mark.
        let text = "e\u{0301}x";
        let sm = map16(text);
        assert_eq!(sm.byte_to_lsp(1), Some(pos(0, 1)));
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 2)));
        assert_eq!(sm.lsp_to_byte(pos(0, 1)), Some(1));
    }

    #[test]
    fn test_bom_is_ordinary_first_character() {
        // BOM is NOT stripped: it is character 0 of line 0 (bytes 0..3,
        // one UTF-16 unit). All Darkmatter parsers see the same text.
        let text = "\u{FEFF}abc";
        let sm = map16(text);
        assert_eq!(sm.byte_to_lsp(0), Some(pos(0, 0)));
        assert_eq!(sm.byte_to_lsp(3), Some(pos(0, 1)));
        assert_eq!(sm.lsp_to_byte(pos(0, 1)), Some(3));
    }

    #[test]
    fn test_mid_codepoint_byte_offsets_rejected() {
        let sm = map16("aéb");
        assert_eq!(sm.byte_to_lsp(2), None); // inside é
        let sm = map16("a💡b");
        assert_eq!(sm.byte_to_lsp(2), None);
        assert_eq!(sm.byte_to_lsp(3), None);
        assert_eq!(sm.byte_to_lsp(4), None);
    }

    #[test]
    fn test_out_of_bounds_byte_offset_rejected() {
        let sm = map16("abc");
        assert_eq!(sm.byte_to_lsp(4), None);
    }

    #[test]
    fn test_range_ending_at_newline_projects_to_next_line_start() {
        let sm = map16("abc\ndef\n");
        // Span 0..4 includes the `\n`: end projects to line 1, character 0.
        let range = sm.byte_range_to_lsp(0..4).unwrap();
        assert_eq!(range.start, pos(0, 0));
        assert_eq!(range.end, pos(1, 0));
    }

    #[test]
    fn test_range_spanning_multibyte_round_trips() {
        let text = "aéβ\n宽💡b\n";
        let sm = map16(text);
        // Span over "éβ\n宽💡" — starts and ends on char boundaries
        // (é=2, β=2, 宽=3, 💡=4 bytes).
        let span = 1..13;
        let range = sm.byte_range_to_lsp(span.clone()).unwrap();
        assert_eq!(sm.lsp_range_to_byte(range), Some(span));
    }

    #[test]
    fn test_inverted_span_rejected() {
        let sm = map16("abc");
        let inverted = SourceSpan { start: 2, end: 1 };
        assert_eq!(sm.byte_range_to_lsp(inverted), None);
    }

    // --- IWES-ported multibyte hit-testing shapes (Apache-2.0, IWE project) ---

    #[test]
    fn test_iwes_greek_text_positions() {
        // Ported case shape: LSP positions after Greek (BMP, 2-byte) text.
        let text = "κόσμε link";
        let sm = map16(text);
        // 5 Greek chars = 10 bytes, 5 UTF-16 units; space at byte 10.
        assert_eq!(sm.byte_to_lsp(10), Some(pos(0, 5)));
        assert_eq!(sm.lsp_to_byte(pos(0, 6)), Some(11));
    }

    #[test]
    fn test_split_span_single_line() {
        let sm = map16("hello world");
        assert_eq!(sm.split_span_by_line(2..7), vec![2..7]);
    }

    #[test]
    fn test_split_span_lf_excludes_terminator() {
        // "ab\ncd\nef": span 0..8 covers all three lines; each subspan is the
        // line's content only (no `\n`).
        let sm = map16("ab\ncd\nef");
        assert_eq!(sm.split_span_by_line(0..8), vec![0..2, 3..5, 6..8]);
    }

    #[test]
    fn test_split_span_crlf_excludes_terminator() {
        // "ab\r\ncd": span 0..6 excludes the two-byte CRLF.
        let sm = map16("ab\r\ncd");
        assert_eq!(sm.split_span_by_line(0..6), vec![0..2, 4..6]);
    }

    #[test]
    fn test_split_span_lone_cr_excludes_terminator() {
        let sm = map16("ab\rcd");
        assert_eq!(sm.split_span_by_line(0..5), vec![0..2, 3..5]);
    }

    #[test]
    fn test_split_span_blank_middle_line_yields_no_fragment() {
        // "ab\n\ncd": the empty middle line contributes nothing.
        let sm = map16("ab\n\ncd");
        assert_eq!(sm.split_span_by_line(0..6), vec![0..2, 4..6]);
    }

    #[test]
    fn test_split_span_starting_inside_line_clips_first_fragment() {
        let sm = map16("abcd\nefgh");
        assert_eq!(sm.split_span_by_line(2..7), vec![2..4, 5..7]);
    }

    #[test]
    fn test_split_span_empty_and_inverted_yield_nothing() {
        let sm = map16("abc");
        assert!(sm.split_span_by_line(2..2).is_empty());
        assert!(sm.split_span_by_line(SourceSpan { start: 3, end: 1 }).is_empty());
    }

    #[test]
    fn test_iwes_astral_prefix_positions() {
        // Ported case shape: completion prefix slicing after an astral char.
        let text = "💡 [lin";
        let sm = map16(text);
        assert_eq!(sm.lsp_to_byte(pos(0, 3)), Some(5)); // after "💡 "
        assert_eq!(sm.lsp_to_byte(pos(0, 7)), Some(9)); // end of line
        assert_eq!(sm.byte_to_lsp(9), Some(pos(0, 7)));
    }
}
