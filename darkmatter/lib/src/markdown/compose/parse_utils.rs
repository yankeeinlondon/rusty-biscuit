//! Shared parsing utilities for directive scanners.
//!
//! Provides a [`Cursor`] for tokenizing directive lines and helpers
//! for detecting code regions that should be skipped during scanning.

use super::body_origin::DataRanges;
use std::borrow::Cow;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// A decoded quoted value with the map from decoded byte boundaries back to
/// the authored bytes they came from.
///
/// A callback or condition is parsed from the decoded text, but errors and
/// editor ranges must point at what the author typed; an escape such as `\"`
/// is two authored bytes and one decoded byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MappedQuotedValue {
    pub(crate) decoded: String,
    /// `authored[i]` is the authored byte offset of decoded byte `i`; one
    /// trailing entry maps the decoded end. A byte inside a multibyte
    /// character maps to the character's first authored byte.
    authored: Vec<usize>,
}

impl MappedQuotedValue {
    /// Builds the map from `(decoded text, authored start)` pieces, in order,
    /// and the authored offset just past the value.
    pub(crate) fn from_pieces(pieces: &[(String, usize)], authored_end: usize) -> Self {
        let mut decoded = String::new();
        let mut authored = Vec::new();
        for (text, start) in pieces {
            for _ in 0..text.len() {
                authored.push(*start);
            }
            decoded.push_str(text);
        }
        authored.push(authored_end);
        Self { decoded, authored }
    }

    /// The authored offset of decoded byte `boundary`, or `None` past the end.
    pub(crate) fn authored_boundary(&self, boundary: usize) -> Option<usize> {
        self.authored.get(boundary).copied()
    }

    /// The authored range covering decoded `range`, or `None` out of bounds.
    pub(crate) fn authored_range(&self, range: std::ops::Range<usize>) -> Option<std::ops::Range<usize>> {
        Some(self.authored_boundary(range.start)?..self.authored_boundary(range.end)?)
    }
}

/// A lightweight cursor for parsing directive lines character by character.
pub(crate) struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

/// Error from a cursor operation, convertible to module-specific errors.
#[derive(Debug)]
pub(crate) struct CursorError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CursorError {
    /// Converts this error into a [`TransclusionError`](crate::markdown::compose::transclusion::TransclusionError).
    pub fn into_transclusion_error(
        self,
        ctx: biscuit_terminal::errors::SourceContext,
    ) -> crate::markdown::compose::transclusion::TransclusionError {
        crate::markdown::compose::transclusion::TransclusionError::ParseDirective {
            ctx: Box::new(ctx),
            line: self.line,
            message: self.message,
            caret_col: Some(self.column),
        }
    }
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Returns the cursor's current byte offset into its input slice.
    ///
    /// Used by the unified directive scanner to record byte spans for each
    /// keyword, target, and option token without re-tokenizing.
    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn current(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn current_column(&self) -> usize {
        self.input[..self.pos].chars().count() + 1
    }

    pub fn advance(&mut self) {
        if let Some(ch) = self.current() {
            self.pos += ch.len_utf8();
        }
    }

    /// Peeks at the character after the current one (skipping `current_char`).
    fn peek_after(&self, current_char: char) -> Option<char> {
        let next_pos = self.pos + current_char.len_utf8();
        self.input[next_pos..].chars().next()
    }

    pub fn skip_ws(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn expect_literal(&mut self, literal: &str, line: usize) -> Result<(), CursorError> {
        if self.input[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            Ok(())
        } else {
            Err(CursorError {
                line,
                column: self.current_column(),
                message: format!("Expected '{}'", literal),
            })
        }
    }

    pub fn expect_char(&mut self, expected: char, line: usize) -> Result<(), CursorError> {
        match self.current() {
            Some(ch) if ch == expected => {
                self.advance();
                Ok(())
            }
            Some(ch) => Err(CursorError {
                line,
                column: self.current_column(),
                message: format!("Expected '{}', found '{}'", expected, ch),
            }),
            None => Err(CursorError {
                line,
                column: self.current_column(),
                message: format!("Expected '{}' at end of directive", expected),
            }),
        }
    }

    /// Consumes `expected` if it is the next character.
    ///
    /// ## Returns
    ///
    /// `true` if the character was present and consumed, `false` otherwise.
    pub fn try_consume_char(&mut self, expected: char) -> bool {
        if self.current() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Attempts to read a `.NAME` suffix after an identifier.
    ///
    /// Returns `Ok(None)` when the next character is not `.`. When a `.`
    /// is present, consumes it and reads a single identifier segment via
    /// [`read_identifier`](Self::read_identifier). Nested dotted keys such
    /// as `set.author.name` are explicitly rejected because the v1
    /// property-form grammar only permits one segment after `set`.
    ///
    /// ## Errors
    ///
    /// Returns a [`CursorError`] when the identifier segment is missing or
    /// when a second `.` follows the consumed segment.
    pub fn read_dotted_suffix(&mut self, line: usize) -> Result<Option<String>, CursorError> {
        if !self.try_consume_char('.') {
            return Ok(None);
        }

        let name = self.read_identifier(line)?;

        if self.current() == Some('.') {
            return Err(CursorError {
                line,
                column: self.current_column(),
                message: "nested dotted keys are not supported in v1".to_string(),
            });
        }

        Ok(Some(name))
    }

    pub fn read_identifier(&mut self, line: usize) -> Result<String, CursorError> {
        let mut out = String::new();
        let Some(ch) = self.current() else {
            return Err(CursorError {
                line,
                column: self.current_column(),
                message: "Unexpected end of directive".to_string(),
            });
        };

        if !is_identifier_start(ch) {
            return Err(CursorError {
                line,
                column: self.current_column(),
                message: format!("Expected identifier, found '{}'", ch),
            });
        }

        while let Some(ch) = self.current() {
            if is_identifier_char(ch) {
                out.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(out)
    }

    pub fn read_value(&mut self, line: usize) -> Result<String, CursorError> {
        let Some(ch) = self.current() else {
            return Err(CursorError {
                line,
                column: self.current_column(),
                message: "Expected value, found end of directive".to_string(),
            });
        };

        match ch {
            '\'' | '"' => self.read_quoted_value(line),
            '{' | '[' => self.read_balanced_value(ch, line),
            _ => self.read_bare_value(),
        }
    }

    fn read_quoted_value(&mut self, line: usize) -> Result<String, CursorError> {
        self.read_quoted_value_mapped(line).map(|mapped| mapped.decoded)
    }

    /// Reads a quoted value like [`Self::read_value`] does for a quote, but
    /// also returns the map from decoded byte boundaries to authored offsets.
    ///
    /// Authored offsets are relative to this cursor's input, the same space as
    /// [`Self::pos`]. The opening and closing quotes are not part of the
    /// decoded text; the closing quote's offset is the mapped end boundary.
    /// Quote and escape rules are exactly those of `read_value`.
    ///
    /// ## Errors
    ///
    /// Returns an error when the cursor is not on a quote, or the value or an
    /// escape is unterminated.
    pub(crate) fn read_quoted_value_mapped(
        &mut self,
        line: usize,
    ) -> Result<MappedQuotedValue, CursorError> {
        let quote = match self.current() {
            Some(ch @ ('\'' | '"')) => ch,
            _ => {
                return Err(CursorError {
                    line,
                    column: self.current_column(),
                    message: "Expected quote".to_string(),
                });
            }
        };
        self.advance();

        let mut pieces: Vec<(String, usize)> = Vec::new();
        let authored_end;
        loop {
            let start = self.pos;
            match self.current() {
                None => {
                    return Err(CursorError {
                        line,
                        column: self.current_column(),
                        message: "Unterminated quoted value".to_string(),
                    });
                }
                Some(ch) if ch == quote => {
                    // A matching quote is only the *closing* delimiter when
                    // followed by whitespace, EOF, or `=` (start of next
                    // key=value pair). Otherwise treat it as a literal
                    // character, which lets nested same-type quotes work:
                    //   when="stage == "tech-design""
                    let next = self.peek_after(quote);
                    if next.is_none() || next.is_some_and(|c| c.is_whitespace() || c == '=') {
                        authored_end = start;
                        self.advance();
                        break;
                    }
                    pieces.push((ch.to_string(), start));
                    self.advance();
                }
                Some('\\') => {
                    self.advance();
                    let Some(escaped) = self.current() else {
                        return Err(CursorError {
                            line,
                            column: self.current_column(),
                            message: "Unterminated escape sequence".to_string(),
                        });
                    };
                    let decoded = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    };
                    pieces.push((decoded.to_string(), start));
                    self.advance();
                }
                Some(ch) => {
                    pieces.push((ch.to_string(), start));
                    self.advance();
                }
            }
        }

        Ok(MappedQuotedValue::from_pieces(&pieces, authored_end))
    }

    fn read_balanced_value(&mut self, opener: char, line: usize) -> Result<String, CursorError> {
        let closer = if opener == '{' { '}' } else { ']' };
        let mut depth = 0usize;
        let mut out = String::new();
        let mut in_string: Option<char> = None;

        while let Some(ch) = self.current() {
            out.push(ch);
            self.advance();

            match in_string {
                Some(quote) => {
                    if ch == '\\' {
                        if let Some(next) = self.current() {
                            out.push(next);
                            self.advance();
                        }
                    } else if ch == quote {
                        in_string = None;
                    }
                }
                None => {
                    if ch == '\'' || ch == '"' {
                        in_string = Some(ch);
                    } else if ch == opener {
                        depth += 1;
                    } else if ch == closer {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            return Ok(out);
                        }
                    }
                }
            }
        }

        Err(CursorError {
            line,
            column: self.current_column(),
            message: "Unterminated JSON option value".to_string(),
        })
    }

    fn read_bare_value(&mut self) -> Result<String, CursorError> {
        let mut out = String::new();
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                break;
            }
            out.push(ch);
            self.advance();
        }
        Ok(out)
    }
}

/// Returns true if `ch` can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

/// Returns true if `ch` can appear inside an identifier.
pub(crate) fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

/// Byte length of the leading run of block-quote markers and indentation
/// whitespace on `line`.
///
/// This is the prefix a `::shell` / `::shell-block` directive line carries
/// before its directive token — every space, tab, and `>` block-quote marker
/// up to the first other byte. The splice machinery reproduces this exact
/// prefix on each emitted output line so the result stays nested under the same
/// list item or block quote the directive appeared in.
///
/// ## Examples
///
/// ```ignore
/// assert_eq!(directive_prefix_len("::shell x"), 0);
/// assert_eq!(directive_prefix_len("    ::shell x"), 4);
/// assert_eq!(directive_prefix_len("> > ::shell x"), 4);
/// ```
pub(crate) fn directive_prefix_len(line: &str) -> usize {
    line.len() - line.trim_start_matches(['>', ' ', '\t']).len()
}

/// Strips a leading block-quote marker prefix from `line`, returning the bare
/// content after the markers.
///
/// Only strips when the leading run actually contains a `>` marker, so plain
/// whitespace-indented lines (which carry no block-quote markers) are returned
/// untouched — their leading whitespace is semantically significant to callers
/// that join continuation lines. A `>`-led line has its full marker-and-space
/// prefix removed so the bare command remains.
///
/// ## Examples
///
/// ```ignore
/// assert_eq!(strip_blockquote_prefix("> echo hi"), "echo hi");
/// assert_eq!(strip_blockquote_prefix("> > echo hi"), "echo hi");
/// assert_eq!(strip_blockquote_prefix("  echo hi"), "  echo hi");
/// ```
pub(crate) fn strip_blockquote_prefix(line: &str) -> &str {
    let prefix_len = directive_prefix_len(line);
    if line[..prefix_len].contains('>') {
        &line[prefix_len..]
    } else {
        line
    }
}

/// `content` as a scanner that detects Markdown or directive structure must
/// read it: every data byte masked (see [`DataRanges::masked`]), so inserted
/// data can neither open a code fence nor spell a directive.
pub(crate) fn structural_view<'a>(content: &'a str, data: Option<&DataRanges>) -> Cow<'a, str> {
    match data {
        Some(data) => data.masked(content),
        None => Cow::Borrowed(content),
    }
}

/// Whether the directive whose line starts at `line_start` and whose keyword
/// ends at `keyword_end` was authored: neither the line break that starts its
/// line, nor its indentation or container prefix, nor its keyword is data.
pub(crate) fn authored_directive(data: Option<&DataRanges>, line_start: usize, keyword_end: usize) -> bool {
    data.is_none_or(|data| !data.intersects(&(line_start.saturating_sub(1)..keyword_end)))
}

/// Kind of a Markdown container that can hold directive lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ContainerKind {
    /// A block quote (one entry per nesting level).
    Quote,
    /// A list as a whole.
    List,
    /// One item of a list.
    Item,
}

/// One Markdown container from the offset scan, in document (outer-first) order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContainerSpan {
    /// What kind of container this is.
    pub kind: ContainerKind,
    /// Byte range of the container as `pulldown_cmark` reports it.
    pub range: std::ops::Range<usize>,
    /// For an [`ContainerKind::Item`], the byte column (offset from its line
    /// start) where the item's content begins. A later line indented less than
    /// this is a lazy continuation, not a member of the item. Zero otherwise.
    pub content_column: usize,
}

/// One step of a container path: the identity of a container a line sits in.
///
/// Identity is the container's start offset, so two sibling list items, or two
/// block quotes separated by a blank line, are distinct even at equal depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ContainerStep {
    /// What kind of container this step is.
    pub kind: ContainerKind,
    /// Start offset of the container; its identity.
    pub start: usize,
}

/// Byte ranges of the code and the container ancestry in one text, from a
/// single parse.
#[derive(Debug, Clone, Default)]
pub(crate) struct CodeRegions {
    /// Inline code spans and fenced/indented code blocks, in document order.
    pub all: Vec<(usize, usize)>,
    /// Fenced and indented code blocks only.
    pub blocks: Vec<(usize, usize)>,
    /// Block quotes, lists, and list items, in the order they open.
    pub containers: Vec<ContainerSpan>,
}

impl CodeRegions {
    /// The container path of the directive whose first content byte is
    /// `position` on the line starting at `line_start`, outermost first.
    ///
    /// A list item counts only when the line is indented to its content column,
    /// and the n-th enclosing block quote only when the line carries more than
    /// n quote markers, so a lazy continuation line does not belong to the
    /// container it merely trails.
    pub(crate) fn container_path(&self, content: &str, line_start: usize, position: usize) -> Vec<ContainerStep> {
        let markers = content[line_start..position].bytes().filter(|byte| *byte == b'>').count();
        let column = position - line_start;
        let mut quotes_seen = 0usize;
        let mut path = Vec::new();
        for container in &self.containers {
            if !(container.range.start <= position && position < container.range.end) {
                continue;
            }
            match container.kind {
                ContainerKind::Quote => {
                    if quotes_seen < markers {
                        path.push(ContainerStep { kind: container.kind, start: container.range.start });
                    }
                    quotes_seen += 1;
                }
                ContainerKind::Item => {
                    if container.range.start >= line_start || column >= container.content_column {
                        path.push(ContainerStep { kind: container.kind, start: container.range.start });
                    }
                }
                ContainerKind::List => {
                    path.push(ContainerStep { kind: container.kind, start: container.range.start });
                }
            }
        }
        // A list is an ancestor only through an item that kept the line.
        let kept: Vec<bool> = (0..path.len())
            .map(|index| path[index].kind != ContainerKind::List || path.get(index + 1).is_some_and(|next| next.kind == ContainerKind::Item))
            .collect();
        let mut kept = kept.into_iter();
        path.retain(|_| kept.next().unwrap_or(true));
        path
    }
}

/// Byte column (offset from its line start) where a list item's content
/// begins, given the item's marker offset.
fn item_content_column(content: &str, marker_start: usize) -> usize {
    let line_start = content[..marker_start].rfind('\n').map_or(0, |index| index + 1);
    let bytes = content.as_bytes();
    let mut cursor = marker_start;
    if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
    }
    cursor = (cursor + 1).min(bytes.len());
    let spaces = bytes[cursor..].iter().take_while(|byte| matches!(**byte, b' ' | b'\t')).count();
    // Five or more spaces mean indented code after a one-space marker; a
    // marker with nothing after it also contributes one space.
    let width = if spaces == 0 || spaces > 4 { 1 } else { spaces };
    cursor + width - line_start
}

/// Finds the code regions and container ancestry of `content` with one
/// Markdown parse.
///
/// Every directive and expression scanner derives its code regions from
/// here, so a caller that runs several scanners over the same text can
/// compute them once and pass them to each `_with_regions` variant.
pub(crate) fn scan_code_regions(content: &str) -> CodeRegions {
    #[cfg(test)]
    scanner_parse_count::record();
    let mut regions = CodeRegions::default();
    let mut in_code_block = false;
    let mut code_block_start = 0;
    let mut open_containers: Vec<usize> = Vec::new();

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Code(_) => {
                regions.all.push((range.start, range.end));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_start = range.start;
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                regions.all.push((code_block_start, range.end));
                regions.blocks.push((code_block_start, range.end));
                in_code_block = false;
            }
            Event::Start(tag @ (Tag::BlockQuote(_) | Tag::List(_) | Tag::Item)) => {
                let (kind, content_column) = match tag {
                    Tag::BlockQuote(_) => (ContainerKind::Quote, 0),
                    Tag::List(_) => (ContainerKind::List, 0),
                    _ => (ContainerKind::Item, item_content_column(content, range.start)),
                };
                open_containers.push(regions.containers.len());
                regions.containers.push(ContainerSpan { kind, range: range.clone(), content_column });
            }
            Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item) => {
                if let Some(index) = open_containers.pop() {
                    regions.containers[index].range.end = range.end;
                }
            }
            _ => {}
        }
    }

    regions
}

/// Finds byte ranges of inline code and fenced code blocks.
pub(crate) fn find_code_regions(content: &str) -> Vec<(usize, usize)> {
    scan_code_regions(content).all
}

/// Thread-local tally of scanner parses ([`scan_code_regions`]), kept apart
/// from cleanup's own parse count so a cleanup budget cannot hide repeated
/// scanning.
#[cfg(test)]
pub(crate) mod scanner_parse_count {
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn record() {
        COUNT.with(|count| count.set(count.get() + 1));
    }

    /// Runs `operation` and returns its value alongside the number of
    /// scanner parses it performed.
    pub(crate) fn measure<T>(operation: impl FnOnce() -> T) -> (T, usize) {
        let before = COUNT.with(Cell::get);
        let value = operation();
        (value, COUNT.with(Cell::get) - before)
    }
}

/// Returns true if `position` falls inside any of the given code regions.
pub(crate) fn is_in_code_region(position: usize, regions: &[(usize, usize)]) -> bool {
    regions
        .iter()
        .any(|(start, end)| position >= *start && position < *end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_quoted_value_projects_escaped_and_multibyte_ranges() {
        // authored: "é\"x" -> decoded: é"x ; é is 2 bytes, the escape is 2 authored bytes.
        let mapped = MappedQuotedValue::from_pieces(
            &[("é".to_string(), 1), ("\"".to_string(), 3), ("x".to_string(), 5)],
            6,
        );
        assert_eq!(mapped.decoded, "é\"x");
        assert_eq!(mapped.authored_range(0..2), Some(1..3));
        assert_eq!(mapped.authored_range(2..3), Some(3..5));
        assert_eq!(mapped.authored_range(0..4), Some(1..6));
        assert_eq!(mapped.authored_range(0..5), None);
    }

    #[test]
    fn mapped_read_maps_multibyte_and_escaped_bytes() {
        // authored: "é\"x" (quotes included) ; offsets: é=1..3, \"=3..5, x=5
        let mut cursor = Cursor::new(r#""é\"x" tail"#);
        let mapped = cursor.read_quoted_value_mapped(1).unwrap();
        assert_eq!(mapped.decoded, "é\"x");
        assert_eq!(mapped.authored_range(0..2), Some(1..3));
        assert_eq!(mapped.authored_range(2..3), Some(3..5));
        assert_eq!(mapped.authored_range(3..4), Some(5..6));
        assert_eq!(mapped.authored_boundary(4), Some(6));
        assert_eq!(cursor.pos(), 7);
    }

    #[test]
    fn mapped_read_decodes_control_escapes_and_nested_quotes() {
        let mut cursor = Cursor::new(r#""a\tb "q"x \\""#);
        let mapped = cursor.read_quoted_value_mapped(1).unwrap();
        assert_eq!(mapped.decoded, "a\tb \"q\"x \\");
        assert_eq!(mapped.authored_boundary(0), Some(1));
        assert_eq!(mapped.authored_range(1..2), Some(2..4));
    }

    #[test]
    fn mapped_read_matches_read_value_for_quotes() {
        for input in [r#""when == 'x'""#, r#"'it\'s'"#, r#""a "b" c" next"#, r#""é\n""#] {
            let plain = Cursor::new(input).read_value(1).unwrap();
            let mapped = Cursor::new(input).read_quoted_value_mapped(1).unwrap();
            assert_eq!(mapped.decoded, plain, "for {input:?}");
        }
    }

    #[test]
    fn mapped_read_rejects_unquoted_and_unterminated() {
        assert!(Cursor::new("bare").read_quoted_value_mapped(1).is_err());
        assert!(Cursor::new("\"open").read_quoted_value_mapped(1).is_err());
        assert!(Cursor::new("\"trail\\").read_quoted_value_mapped(1).is_err());
    }

    #[test]
    fn directive_prefix_len_column_one_is_zero() {
        assert_eq!(directive_prefix_len("::shell echo hi"), 0);
    }

    #[test]
    fn directive_prefix_len_captures_space_indent() {
        assert_eq!(directive_prefix_len("    ::shell echo hi"), 4);
    }

    #[test]
    fn directive_prefix_len_captures_tab_indent() {
        assert_eq!(directive_prefix_len("\t::shell echo hi"), 1);
    }

    #[test]
    fn directive_prefix_len_captures_blockquote_markers() {
        assert_eq!(directive_prefix_len("> > ::shell echo hi"), 4);
        assert_eq!(directive_prefix_len("> ::shell echo hi"), 2);
        assert_eq!(directive_prefix_len(">::shell echo hi"), 1);
    }

    #[test]
    fn directive_prefix_len_stops_at_first_content_byte() {
        // The internal `>` of a redirection is past the first content byte and
        // must not be folded into the prefix.
        assert_eq!(directive_prefix_len("> echo a > b"), 2);
    }

    #[test]
    fn strip_blockquote_prefix_removes_single_marker() {
        assert_eq!(strip_blockquote_prefix("> echo hi"), "echo hi");
    }

    #[test]
    fn strip_blockquote_prefix_removes_nested_markers() {
        assert_eq!(strip_blockquote_prefix("> > echo hi"), "echo hi");
    }

    #[test]
    fn strip_blockquote_prefix_removes_marker_without_space() {
        assert_eq!(strip_blockquote_prefix(">echo hi"), "echo hi");
    }

    #[test]
    fn strip_blockquote_prefix_leaves_plain_whitespace_untouched() {
        // No `>` marker: leading whitespace is significant to continuation
        // joining and must be preserved verbatim.
        assert_eq!(strip_blockquote_prefix("  echo hi"), "  echo hi");
        assert_eq!(strip_blockquote_prefix("echo hi"), "echo hi");
    }
}
