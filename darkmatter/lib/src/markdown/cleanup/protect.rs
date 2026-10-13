//! Source protection for body cleanup.
//!
//! Darkmatter directive lines, quoted `::shell` lines, and whole
//! `::shell-block` regions are fixed points: cleanup never edits them, and the
//! Markdown parser never sees them. [`ParseView`] is the body with those lines
//! removed plus a map back to source offsets. Removing a line, rather than
//! masking it, is what stops CommonMark's lazy continuation from pulling a
//! directive into the preceding container, while a list that continues across
//! `::block`/`::end-block` stays one list, which matches compose (compose
//! removes those lines too). Recognition is delegated to each family's
//! existing scanner, so directive-looking text inside code stays code.

use std::ops::Range;

use crate::markdown::MarkdownError;
use crate::markdown::compose::block_pairs::{
    BlockOpenKind, BlockPairError, scan_block_pairs_with_code_regions,
};
use crate::markdown::compose::directives_api::scan_darkmatter_directives_with_code_regions;
use crate::markdown::compose::expression::ExpressionFinder;
use crate::markdown::compose::parse_utils::{directive_prefix_len, is_in_code_region, scan_code_regions};
use crate::markdown::render_tree::disclosure_scan::scan_disclosures_with_code_regions;

/// One physical source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Line {
    /// Offset of the first byte of the line.
    pub start: usize,
    /// Offset just past the line's content, before its terminator.
    pub end: usize,
    /// Offset just past the line's terminator (`end` on a final unterminated line).
    pub next: usize,
}

/// The physical lines of a text; `\n`, `\r\n`, and a lone `\r` all end a line,
/// as they do for the Markdown parser.
#[derive(Debug, Clone)]
pub(super) struct Lines {
    lines: Vec<Line>,
}

impl Lines {
    pub(super) fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut index = 0usize;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => {
                    lines.push(Line { start, end: index, next: index + 1 });
                    start = index + 1;
                }
                b'\r' => {
                    let next = if bytes.get(index + 1) == Some(&b'\n') { index + 2 } else { index + 1 };
                    lines.push(Line { start, end: index, next });
                    start = next;
                    index = next;
                    continue;
                }
                _ => {}
            }
            index += 1;
        }
        if start < bytes.len() {
            lines.push(Line { start, end: bytes.len(), next: bytes.len() });
        }
        Self { lines }
    }

    pub(super) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(super) fn get(&self, index: usize) -> Line {
        self.lines[index]
    }

    /// The index of the line containing `offset`; an offset at the very end
    /// belongs to the last line.
    pub(super) fn index_of(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|line| line.next <= offset)
            .min(self.lines.len().saturating_sub(1))
    }

    /// The terminator bytes of line `index` (empty for an unterminated last line).
    pub(super) fn terminator<'a>(&self, text: &'a str, index: usize) -> &'a str {
        let line = self.lines[index];
        &text[line.end..line.next]
    }
}

/// Why the body's directive structure cannot be formatted safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MalformedRegion {
    /// Byte offset of the first affected line.
    pub offset: usize,
    /// What the existing scanner reported.
    pub message: String,
}

/// The protected parts of a body.
#[derive(Debug, Clone, Default)]
pub(super) struct Protection {
    /// Whole protected lines including their terminators, sorted and merged.
    pub lines: Vec<Range<usize>>,
    /// Inline spans (expressions, interpolation literals, literal tokens) whose
    /// spelling must survive byte for byte.
    pub inline: Vec<Range<usize>>,
}

impl Protection {
    /// Scans `source` with the compose recognizers.
    ///
    /// The recognizers share one code-region scan (one scanner parse per
    /// text), so directive-looking text in code stays code without each
    /// scanner re-parsing the body.
    ///
    /// ## Errors
    ///
    /// Returns the first malformed page/shell block or disclosure region, which
    /// the caller leaves unformatted.
    pub(super) fn scan(source: &str) -> Result<Self, MalformedRegion> {
        let code = scan_code_regions(source);
        // Messages name no line: the caller locates the region in its own
        // (document) coordinates from `offset`, and a body-relative line
        // number in the detail would contradict it.
        let pairs = scan_block_pairs_with_code_regions(source, None, &code.all).map_err(|error| {
            let (line, message) = match error {
                BlockPairError::UnmatchedEnd { line } => {
                    (line, "an `::end-block` has no matching opener".to_string())
                }
                BlockPairError::UnterminatedBlock { line, opening_text, .. } => {
                    (line, format!("`{}` is never closed by an `::end-block`", opening_text.trim()))
                }
                BlockPairError::TrailingContent { line, content } => {
                    (line, format!("unexpected content after `::end-block`: '{content}'"))
                }
            };
            MalformedRegion { offset: line_number_offset(source, line), message }
        })?;
        scan_disclosures_with_code_regions(source, &code.all).map_err(|error| match error {
            MarkdownError::MalformedDisclosure { reason, range } => {
                MalformedRegion { offset: range.start, message: reason }
            }
            other => MalformedRegion { offset: 0, message: other.to_string() },
        })?;

        let lines = Lines::new(source);
        let mut protected: Vec<Range<usize>> = scan_darkmatter_directives_with_code_regions(source, None, &code.all)
            .into_iter()
            .map(|directive| {
                let line = lines.get(lines.index_of(directive.span.start));
                line.start..line.next
            })
            .collect();
        protected.extend(
            pairs
                .into_iter()
                .filter(|pair| pair.kind == BlockOpenKind::Shell)
                .map(|pair| pair.span),
        );
        protected.extend(quoted_shell_lines(source, &lines, &code.all));
        protected.sort_by_key(|range| (range.start, range.end));
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(protected.len());
        for range in protected {
            match merged.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }

        let scan = ExpressionFinder::with_code_blocks(source, code.blocks).scan();
        let mut inline: Vec<Range<usize>> = scan
            .expressions
            .iter()
            .map(|location| location.start..location.end)
            .chain(scan.literals.iter().map(|literal| literal.start..literal.end))
            .chain(scan.tokens.iter().map(|token| token.start..token.end))
            .collect();
        inline.sort_by_key(|range| (range.start, range.end));

        Ok(Self { lines: merged, inline })
    }

    /// Whether `range` touches any protected line. An empty range at a
    /// protected line's start does not touch it.
    pub(super) fn touches_line(&self, range: &Range<usize>) -> bool {
        self.lines.iter().any(|line| {
            if range.is_empty() {
                line.start < range.start && range.start < line.end
            } else {
                line.start < range.end && range.start < line.end
            }
        })
    }

    /// Whether any protected line lies inside `range`.
    pub(super) fn line_within(&self, range: &Range<usize>) -> bool {
        self.lines.iter().any(|line| range.start <= line.start && line.start < range.end)
    }

    /// Whether `range` overlaps an inline protected span.
    pub(super) fn touches_inline(&self, range: &Range<usize>) -> bool {
        self.inline
            .iter()
            .any(|span| span.start < range.end.max(range.start + 1) && range.start < span.end)
    }

    /// Whether the line starting at `start` is protected.
    pub(super) fn is_line_protected(&self, start: usize) -> bool {
        self.lines.iter().any(|line| line.start <= start && start < line.end)
    }
}

/// The byte offset of 1-based line `number`.
fn line_number_offset(source: &str, number: usize) -> usize {
    let lines = Lines::new(source);
    if (1..=lines.len()).contains(&number) { lines.get(number - 1).start } else { 0 }
}

/// `::shell` lines behind a block-quote prefix. The shell parser recognizes
/// them; the generic directive scanner does not.
fn quoted_shell_lines(source: &str, lines: &Lines, code_regions: &[(usize, usize)]) -> Vec<Range<usize>> {
    if !source.contains("::shell") {
        return Vec::new();
    }
    (0..lines.len())
        .map(|index| lines.get(index))
        .filter(|line| {
            let text = &source[line.start..line.end];
            let prefix = directive_prefix_len(text);
            text[..prefix].contains('>')
                && text[prefix..].trim_end().starts_with("::shell ")
                && !is_in_code_region(line.start, code_regions)
        })
        .map(|line| line.start..line.next)
        .collect()
}

/// The text the Markdown parser sees: the body minus protected lines.
#[derive(Debug, Clone)]
pub(super) struct ParseView {
    pub text: String,
    /// `(view_start, source_start, len)` for each kept run of source.
    segments: Vec<(usize, usize, usize)>,
}

impl ParseView {
    pub(super) fn new(source: &str, protection: &Protection) -> Self {
        let mut text = String::with_capacity(source.len());
        let mut segments = Vec::new();
        let mut cursor = 0usize;
        let end = source.len()..source.len();
        for range in protection.lines.iter().chain(std::iter::once(&end)) {
            if range.start > cursor {
                segments.push((text.len(), cursor, range.start - cursor));
                text.push_str(&source[cursor..range.start]);
            }
            cursor = cursor.max(range.end);
        }
        if segments.is_empty() {
            segments.push((0, source.len(), 0));
        }
        Self { text, segments }
    }

    /// Maps a view offset that starts something to its source offset. At a
    /// boundary between two kept runs it maps to the later run, past the
    /// removed lines.
    pub(super) fn start_to_source(&self, offset: usize) -> usize {
        let index = self.segments.partition_point(|segment| segment.0 <= offset).saturating_sub(1);
        let (view, source, len) = self.segments[index];
        source + (offset - view).min(len)
    }

    /// Maps an exclusive view end offset to its source offset. At a boundary
    /// it maps to the earlier run, before the removed lines.
    pub(super) fn end_to_source(&self, offset: usize) -> usize {
        let index = self.segments.partition_point(|segment| segment.0 < offset).saturating_sub(1);
        let (view, source, len) = self.segments[index];
        source + (offset.saturating_sub(view)).min(len)
    }

    pub(super) fn range_to_source(&self, range: &Range<usize>) -> Range<usize> {
        let start = self.start_to_source(range.start);
        let end = if range.is_empty() { start } else { self.end_to_source(range.end).max(start) };
        start..end
    }
}

/// The byte offset where the content of `line` at block-quote depth `depth`
/// begins: past any indentation and `depth` `>` markers, each with its one
/// optional following space. `None` when the line lacks the markers (a lazy
/// continuation) or uses a tab or a marker without its space where a column
/// would be ambiguous.
pub(super) fn depth_base(text: &str, line: Line, depth: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut cursor = line.start;
    for _ in 0..depth {
        while cursor < line.end && bytes[cursor] == b' ' {
            cursor += 1;
        }
        if cursor >= line.end || bytes[cursor] != b'>' {
            return None;
        }
        cursor += 1;
        if cursor < line.end && bytes[cursor] == b' ' {
            cursor += 1;
        } else if cursor < line.end && bytes[cursor] == b'\t' {
            return None;
        }
    }
    Some(cursor)
}

/// The prefix a blank line inside `depth` block quotes needs, taken from a
/// neighboring `line`: everything through the `depth`-th `>` marker, with no
/// trailing whitespace.
pub(super) fn blank_line_prefix(text: &str, line: Line, depth: usize) -> Option<String> {
    if depth == 0 {
        return Some(String::new());
    }
    let base = depth_base(text, line, depth)?;
    Some(text[line.start..base].trim_end().to_string())
}

/// Whether `line` is blank inside `depth` block quotes.
pub(super) fn is_blank_at_depth(text: &str, line: Line, depth: usize) -> bool {
    let content = &text[line.start..line.end];
    if content.trim().is_empty() {
        return true;
    }
    if depth == 0 {
        return false;
    }
    let mut rest = content;
    for _ in 0..depth {
        rest = rest.trim_start_matches([' ', '\t']);
        match rest.strip_prefix('>') {
            Some(after) => rest = after,
            None => return false,
        }
    }
    rest.trim().is_empty()
}

/// Whether `line` reads as blank in any container: only whitespace and `>`.
pub(super) fn is_blank_line(text: &str, line: Line) -> bool {
    text[line.start..line.end].chars().all(|c| c == '>' || c.is_whitespace())
}

/// Where the content that an inline event at `offset` belongs to starts on
/// its line. The parser starts an escaped character's event after the
/// backslash, so this steps back over content bytes to the container prefix
/// (spaces, tabs, and `>` markers) or the line start.
pub(super) fn content_start(text: &str, offset: usize, line_start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut start = offset;
    while start > line_start && !matches!(bytes[start - 1], b' ' | b'\t' | b'>') {
        start -= 1;
    }
    start
}
