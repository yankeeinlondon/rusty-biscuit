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
    pub message: String,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    pub fn current(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
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
                message: format!("Expected '{}', found '{}'", expected, ch),
            }),
            None => Err(CursorError {
                line,
                message: format!("Expected '{}' at end of directive", expected),
            }),
        }
    }

    pub fn read_identifier(&mut self, line: usize) -> Result<String, CursorError> {
        let mut out = String::new();
        let Some(ch) = self.current() else {
            return Err(CursorError {
                line,
                message: "Unexpected end of directive".to_string(),
            });
        };

        if !is_identifier_start(ch) {
            return Err(CursorError {
                line,
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
            message: "Expected quote".to_string(),
        })?;
        self.advance();

        let mut out = String::new();
        loop {
            match self.current() {
                None => {
                    return Err(CursorError {
                        line,
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
