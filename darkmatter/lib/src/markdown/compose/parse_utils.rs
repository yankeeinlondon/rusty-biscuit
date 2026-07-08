//! Shared parsing utilities for directive scanners.
//!
//! Provides a [`Cursor`] for tokenizing directive lines and helpers
//! for detecting code regions that should be skipped during scanning.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

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
        let quote = self.current().ok_or_else(|| CursorError {
            line,
            column: self.current_column(),
            message: "Expected quote".to_string(),
        })?;
        self.advance();

        let mut out = String::new();
        loop {
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
                        self.advance();
                        break;
                    }
                    // Literal quote — keep it in the output
                    out.push(ch);
                    self.advance();
                }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n') => {
                            out.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            out.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            out.push('\r');
                            self.advance();
                        }
                        Some('\\') => {
                            out.push('\\');
                            self.advance();
                        }
                        Some(ch) if ch == quote => {
                            out.push(ch);
                            self.advance();
                        }
                        Some(ch) => {
                            out.push(ch);
                            self.advance();
                        }
                        None => {
                            return Err(CursorError {
                                line,
                                column: self.current_column(),
                                message: "Unterminated escape sequence".to_string(),
                            });
                        }
                    }
                }
                Some(ch) => {
                    out.push(ch);
                    self.advance();
                }
            }
        }

        Ok(out)
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

/// Finds byte ranges of inline code and fenced code blocks.
pub(crate) fn find_code_regions(content: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut in_code_block = false;
    let mut code_block_start = 0;

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Code(_) => {
                regions.push((range.start, range.end));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_start = range.start;
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                regions.push((code_block_start, range.end));
                in_code_block = false;
            }
            _ => {}
        }
    }

    regions
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
