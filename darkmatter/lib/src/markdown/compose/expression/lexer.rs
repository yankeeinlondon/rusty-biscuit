//! Lexer for interpolation expressions.
//!
//! This module provides two main components:
//!
//! 1. `ExpressionFinder` - Locates `{{ ... }}` expressions in markdown content
//!    while skipping code spans and fenced code blocks.
//!
//! 2. `Lexer` - Tokenizes the content inside `{{ ... }}` into a stream of tokens
//!    for parsing.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::expression::{ExpressionFinder, Lexer, Token};
//!
//! // Find expressions in content
//! let content = "Hello {{ name }}!";
//! let finder = ExpressionFinder::new(content);
//! let expressions = finder.find_all();
//!
//! assert_eq!(expressions.len(), 1);
//! assert_eq!(expressions[0].expression, "name");
//!
//! // Tokenize an expression
//! let mut lexer = Lexer::new("name || \"unknown\"");
//! let tokens = lexer.tokenize_all().unwrap();
//!
//! assert!(matches!(&tokens[0], Token::Variable(v) if v == "name"));
//! assert!(matches!(&tokens[1], Token::Pipe));
//! assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "unknown"));
//! ```

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::fmt;
use tracing::debug;

/// Location of an interpolation expression in markdown content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionLocation {
    /// Byte offset of the first `{` in the source.
    pub start: usize,

    /// Byte offset after the last `}` in the source.
    pub end: usize,

    /// The expression content between `{{` and `}}`, trimmed.
    pub expression: String,
}

/// Finds interpolation expressions in markdown content.
///
/// This finder scans content for `{{ ... }}` patterns while respecting
/// markdown structure - expressions inside code spans and fenced code
/// blocks are ignored.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::ExpressionFinder;
///
/// let content = "Hello {{ name }}! Inline: `{{ also_this }}`";
/// let finder = ExpressionFinder::new(content);
/// let expressions = finder.find_all();
///
/// // Both expressions are found — inline code spans are scanned.
/// // Fenced and indented code blocks are still skipped.
/// assert_eq!(expressions.len(), 2);
/// assert_eq!(expressions[0].expression, "name");
/// assert_eq!(expressions[1].expression, "also_this");
/// ```
pub struct ExpressionFinder<'a> {
    content: &'a str,
    code_regions: Vec<(usize, usize)>,
}

impl<'a> ExpressionFinder<'a> {
    /// Creates a new expression finder for the given content.
    ///
    /// Pre-computes fenced and indented code-block regions that should be
    /// excluded from expression scanning. Inline code spans (single backticks)
    /// are intentionally NOT skipped — `{{ }}` inside `` ` ` `` is interpolated.
    pub fn new(content: &'a str) -> Self {
        let code_regions = Self::find_code_regions(content);
        Self {
            content,
            code_regions,
        }
    }

    /// Finds all interpolation expressions in the content.
    ///
    /// Returns a vector of `ExpressionLocation` for each `{{ ... }}` pattern
    /// found outside of code regions.
    pub fn find_all(&self) -> Vec<ExpressionLocation> {
        let mut locations = Vec::new();
        let mut pos = 0;
        let bytes = self.content.as_bytes();

        while pos < bytes.len().saturating_sub(3) {
            // Look for opening {{
            if bytes[pos] == b'{' && pos + 1 < bytes.len() && bytes[pos + 1] == b'{' {
                // Check if we're in a code region
                if self.is_in_code_region(pos) {
                    pos += 2;
                    continue;
                }

                // Find the closing }}
                let start = pos;
                pos += 2; // Skip past {{

                let mut depth = 1;
                let mut found_close = false;

                while pos < bytes.len().saturating_sub(1) {
                    if bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
                        depth += 1;
                        pos += 2;
                    } else if bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            let end = pos + 2;
                            // Extract expression content (between {{ and }})
                            let expr_start = start + 2;
                            let expr_end = pos;
                            if expr_start < expr_end {
                                let expression = self.content[expr_start..expr_end].trim();
                                if !expression.is_empty() {
                                    locations.push(ExpressionLocation {
                                        start,
                                        end,
                                        expression: expression.to_string(),
                                    });
                                }
                            }
                            pos = end;
                            found_close = true;
                            break;
                        }
                        pos += 2;
                    } else {
                        pos += 1;
                    }
                }

                if !found_close {
                    // Unclosed {{ - skip it
                    pos = start + 2;
                }
            } else {
                pos += 1;
            }
        }

        debug!(count = locations.len(), "interpolation: found expressions");
        locations
    }

    /// Checks if a position is within a code region.
    fn is_in_code_region(&self, pos: usize) -> bool {
        self.code_regions
            .iter()
            .any(|(start, end)| pos >= *start && pos < *end)
    }

    /// Finds all `{{ }}` expressions in a plain string with no code-region exclusions.
    pub fn find_all_plain(input: &'a str) -> Vec<ExpressionLocation> {
        let finder = Self {
            content: input,
            code_regions: vec![],
        };
        finder.find_all()
    }

    /// Finds fenced and indented code-block regions in content.
    ///
    /// Inline code spans (single backticks) are intentionally NOT collected
    /// here — interpolation runs inside them so that templated identifiers
    /// like `` `var_{{ phase }}` `` expand as expected.
    fn find_code_regions(content: &str) -> Vec<(usize, usize)> {
        let mut regions = Vec::new();
        let parser = Parser::new_ext(content, Options::all()).into_offset_iter();

        let mut in_code_block = false;
        let mut code_block_start = 0;

        for (event, range) in parser {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    // Start of fenced/indented code block
                    in_code_block = true;
                    code_block_start = range.start;
                }
                // End of code block
                Event::End(TagEnd::CodeBlock) if in_code_block => {
                    regions.push((code_block_start, range.end));
                    in_code_block = false;
                }
                _ => {}
            }
        }

        regions
    }
}

/// Token types for interpolation expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A variable reference (e.g., `foo`, `user.name`).
    ///
    /// Dotted paths are kept as single tokens for simpler AST representation.
    Variable(String),

    /// The `||` fallback operator in interpolation mode.
    Pipe,

    /// The question mark `?` for ternary condition.
    Question,

    /// The colon `:` for ternary else branch.
    Colon,

    /// Left parenthesis `(`.
    LParen,

    /// Right parenthesis `)`.
    RParen,

    /// Comma `,` for function arguments.
    Comma,

    /// Postfix dot `.` used between a non-identifier base and a member name.
    ///
    /// Plain dotted identifier paths (`foo.bar.baz`) are folded into a single
    /// [`Token::Variable`] by the lexer; this token is only emitted when a
    /// `.` cannot be absorbed into a variable, such as after `]` or `)`.
    Dot,

    /// Left bracket `[`.
    LBracket,

    /// Right bracket `]`.
    RBracket,

    /// Arithmetic plus `+`.
    Plus,

    /// Arithmetic minus / unary minus `-`.
    Minus,

    /// Arithmetic multiplication `*`.
    Star,

    /// Arithmetic division `/`.
    Slash,

    /// Arithmetic remainder `%`.
    Percent,

    /// A string literal (content without quotes).
    StringLiteral(String),

    /// A numeric literal.
    NumberLiteral(f64),

    /// A boolean literal: `true` or `false`.
    BoolLiteral(bool),

    /// A comparison operator.
    CompOp(ComparisonOp),

    /// Unary logical not `!`.
    Bang,

    /// Logical AND `&&` (condition mode only).
    AndAnd,

    /// Logical OR `||` (condition mode only).
    OrOr,

    /// End of input.
    Eof,
}

/// Parse mode controlling how `||` and `&&` are tokenized.
///
/// Interpolation mode: `||` maps to [`Token::Pipe`] (fallback operator);
/// bare `|` and `&&` are invalid.
///
/// Condition mode enables infix logical operators for `when="..."` expressions:
/// `||` maps to [`Token::OrOr`] (logical OR), `&&` maps to [`Token::AndAnd`]
/// (logical AND), and bare `|` is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseMode {
    /// Interpolation parsing (default). `||` is fallback; bare `|` and `&&` are invalid.
    #[default]
    Interpolation,

    /// Condition parsing. `||` is logical OR, `&&` is logical AND; bare `|` is invalid.
    Condition,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Variable(name) => write!(f, "{}", name),
            Token::Pipe => write!(f, "|"),
            Token::Question => write!(f, "?"),
            Token::Colon => write!(f, ":"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::NumberLiteral(n) => write!(f, "{}", n),
            Token::BoolLiteral(b) => write!(f, "{}", b),
            Token::CompOp(op) => write!(f, "{}", op),
            Token::Bang => write!(f, "!"),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Eof => write!(f, "<EOF>"),
        }
    }
}

/// Comparison operators supported in interpolation expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    /// Equal `==`
    Equal,
    /// Not equal `!=`
    NotEqual,
    /// Greater than `>`
    GreaterThan,
    /// Greater than or equal `>=`
    GreaterThanOrEqual,
    /// Less than `<`
    LessThan,
    /// Less than or equal `<=`
    LessThanOrEqual,
}

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonOp::Equal => write!(f, "=="),
            ComparisonOp::NotEqual => write!(f, "!="),
            ComparisonOp::GreaterThan => write!(f, ">"),
            ComparisonOp::GreaterThanOrEqual => write!(f, ">="),
            ComparisonOp::LessThan => write!(f, "<"),
            ComparisonOp::LessThanOrEqual => write!(f, "<="),
        }
    }
}

/// Error that can occur during lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    /// Human-readable error message.
    pub message: String,

    /// Byte position in the input where the error occurred.
    pub position: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.message, self.position)
    }
}

impl std::error::Error for LexerError {}

impl LexerError {
    /// Creates a new lexer error.
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

/// Lexer for interpolation expression content.
///
/// Tokenizes the content between `{{` and `}}` into a stream of tokens.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::{Lexer, Token, ComparisonOp};
///
/// let mut lexer = Lexer::new("count > 0 ? \"yes\" : \"no\"");
/// let tokens = lexer.tokenize_all().unwrap();
///
/// assert!(matches!(&tokens[0], Token::Variable(v) if v == "count"));
/// assert!(matches!(&tokens[1], Token::CompOp(ComparisonOp::GreaterThan)));
/// assert!(matches!(&tokens[2], Token::NumberLiteral(n) if *n == 0.0));
/// assert!(matches!(&tokens[3], Token::Question));
/// assert!(matches!(&tokens[4], Token::StringLiteral(s) if s == "yes"));
/// assert!(matches!(&tokens[5], Token::Colon));
/// assert!(matches!(&tokens[6], Token::StringLiteral(s) if s == "no"));
/// assert!(matches!(&tokens[7], Token::Eof));
/// ```
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    mode: ParseMode,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given expression content in interpolation mode.
    ///
    /// The input should be the content between `{{` and `}}`, not including
    /// the delimiters themselves.
    pub fn new(input: &'a str) -> Self {
        Self::with_mode(input, ParseMode::Interpolation)
    }

    /// Creates a new lexer for the given expression content with a specific parse mode.
    ///
    /// Condition mode enables `&&` and `||` as logical operators. Interpolation
    /// mode preserves the legacy behavior where `||` collapses to fallback and
    /// `&&` is rejected.
    pub fn with_mode(input: &'a str, mode: ParseMode) -> Self {
        Self {
            input,
            pos: 0,
            mode,
        }
    }

    /// Returns the current parse mode for this lexer.
    pub fn mode(&self) -> ParseMode {
        self.mode
    }

    /// Returns the next token from the input.
    ///
    /// Returns `Token::Eof` when the input is exhausted.
    ///
    /// ## Errors
    ///
    /// Returns an error for invalid syntax, such as unterminated strings.
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(Token::Eof);
        }

        let start_pos = self.pos;
        let ch = self.current_char().unwrap();

        match ch {
            '|' => {
                self.advance();
                if self.current_char() == Some('|') {
                    self.advance();
                    match self.mode {
                        ParseMode::Interpolation => Ok(Token::Pipe),
                        ParseMode::Condition => Ok(Token::OrOr),
                    }
                } else {
                    Err(LexerError::new(
                        match self.mode {
                            ParseMode::Interpolation => "Unexpected '|'. Use '||' for fallback.",
                            ParseMode::Condition => "Unexpected '|'. Use '||' for logical OR.",
                        },
                        start_pos,
                    ))
                }
            }
            '&' => {
                self.advance();
                if self.current_char() == Some('&') {
                    self.advance();
                    match self.mode {
                        ParseMode::Condition => Ok(Token::AndAnd),
                        ParseMode::Interpolation => {
                            Err(LexerError::new("Unexpected character: '&'", start_pos))
                        }
                    }
                } else {
                    Err(LexerError::new("Unexpected character: '&'", start_pos))
                }
            }
            '?' => {
                self.advance();
                Ok(Token::Question)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            '.' => {
                self.advance();
                Ok(Token::Dot)
            }
            '[' => {
                self.advance();
                Ok(Token::LBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::RBracket)
            }
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '-' => {
                self.advance();
                Ok(Token::Minus)
            }
            '*' => {
                self.advance();
                Ok(Token::Star)
            }
            '/' => {
                self.advance();
                Ok(Token::Slash)
            }
            '%' => {
                self.advance();
                Ok(Token::Percent)
            }
            '"' | '\'' => self.read_string(ch),
            '=' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Ok(Token::CompOp(ComparisonOp::Equal))
                } else {
                    Err(LexerError::new(
                        "Expected '=' after '=' for equality operator",
                        start_pos,
                    ))
                }
            }
            '!' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Ok(Token::CompOp(ComparisonOp::NotEqual))
                } else {
                    Ok(Token::Bang)
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Ok(Token::CompOp(ComparisonOp::GreaterThanOrEqual))
                } else {
                    Ok(Token::CompOp(ComparisonOp::GreaterThan))
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Ok(Token::CompOp(ComparisonOp::LessThanOrEqual))
                } else {
                    Ok(Token::CompOp(ComparisonOp::LessThan))
                }
            }
            _ if ch.is_ascii_digit() => self.read_number(),
            _ if is_identifier_start(ch) => self.read_variable(),
            _ => Err(LexerError::new(
                format!("Unexpected character: '{}'", ch),
                start_pos,
            )),
        }
    }

    /// Tokenizes all remaining input into a vector of tokens.
    ///
    /// The returned vector always ends with `Token::Eof`.
    ///
    /// ## Errors
    ///
    /// Returns the first error encountered during tokenization.
    pub fn tokenize_all(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Returns the current character without advancing.
    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Returns the next character without advancing.
    fn peek_char(&self) -> Option<char> {
        let mut chars = self.input[self.pos..].chars();
        chars.next();
        chars.next()
    }

    /// Advances the position by one character.
    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
        }
    }

    /// Skips whitespace characters.
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Reads a string literal (single or double quoted).
    fn read_string(&mut self, quote: char) -> Result<Token, LexerError> {
        let start_pos = self.pos;
        self.advance(); // Skip opening quote

        let mut value = String::new();

        loop {
            match self.current_char() {
                None => {
                    return Err(LexerError::new("Unterminated string literal", start_pos));
                }
                Some(ch) if ch == quote => {
                    self.advance(); // Skip closing quote
                    return Ok(Token::StringLiteral(value));
                }
                Some('\\') => {
                    self.advance();
                    match self.current_char() {
                        Some('n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some(ch) if ch == quote => {
                            value.push(ch);
                            self.advance();
                        }
                        Some(ch) => {
                            // Unrecognized escape - keep as-is
                            value.push('\\');
                            value.push(ch);
                            self.advance();
                        }
                        None => {
                            return Err(LexerError::new(
                                "Unterminated string after escape",
                                self.pos,
                            ));
                        }
                    }
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
            }
        }
    }

    /// Reads a numeric literal.
    ///
    /// Numbers are always non-negative at the lexer level. Unary minus is
    /// handled in the parser so `5 - 3` tokenizes as three tokens rather than
    /// being collapsed into `5, -3`.
    fn read_number(&mut self) -> Result<Token, LexerError> {
        let start_pos = self.pos;
        let mut value = String::new();

        // Read integer part
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Read decimal part
        if self.current_char() == Some('.') && self.peek_char().is_some_and(|c| c.is_ascii_digit())
        {
            value.push('.');
            self.advance();

            while let Some(ch) = self.current_char() {
                if ch.is_ascii_digit() {
                    value.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        value
            .parse::<f64>()
            .map(Token::NumberLiteral)
            .map_err(|_| LexerError::new(format!("Invalid number: '{}'", value), start_pos))
    }

    /// Reads a variable (identifier with optional dot-separated path).
    fn read_variable(&mut self) -> Result<Token, LexerError> {
        let mut name = String::new();

        // Read first identifier
        while let Some(ch) = self.current_char() {
            if is_identifier_char(ch) {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Read dotted path segments
        while self.current_char() == Some('.') {
            // Check if next char starts an identifier (not a number)
            if let Some(next) = self.peek_char() {
                if is_identifier_start(next) {
                    name.push('.');
                    self.advance(); // consume the dot

                    while let Some(ch) = self.current_char() {
                        if is_identifier_char(ch) {
                            name.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                } else {
                    // Dot not followed by identifier - stop here
                    break;
                }
            } else {
                // Dot at end of input - stop
                break;
            }
        }

        Ok(match name.as_str() {
            "true" => Token::BoolLiteral(true),
            "false" => Token::BoolLiteral(false),
            _ => Token::Variable(name),
        })
    }
}

/// Checks if a character can start an identifier.
fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

/// Checks if a character can continue an identifier.
fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    mod expression_finder {
        use super::*;

        #[test]
        fn finds_simple_expression() {
            let content = "Hello {{ name }}!";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 1);
            assert_eq!(exprs[0].expression, "name");
            assert_eq!(exprs[0].start, 6);
            assert_eq!(exprs[0].end, 16);
        }

        #[test]
        fn finds_multiple_expressions() {
            let content = "{{ greeting }} {{ name }}!";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 2);
            assert_eq!(exprs[0].expression, "greeting");
            assert_eq!(exprs[1].expression, "name");
        }

        #[test]
        fn trims_expression_content() {
            let content = "{{   foo   }}";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 1);
            assert_eq!(exprs[0].expression, "foo");
        }

        #[test]
        fn scans_inline_code_spans() {
            // Inline code spans are NOT skipped — interpolation runs inside
            // single-backtick spans so `` `var_{{ phase }}` `` works.
            let content = "Hello {{ name }}! Code: `{{ also_this }}`";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 2);
            assert_eq!(exprs[0].expression, "name");
            assert_eq!(exprs[1].expression, "also_this");
        }

        #[test]
        fn skips_fenced_code_block() {
            let content = r#"Hello {{ name }}!

```rust
let x = {{ variable }};
```

And {{ another }}."#;
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 2);
            assert_eq!(exprs[0].expression, "name");
            assert_eq!(exprs[1].expression, "another");
        }

        #[test]
        fn skips_indented_code_block() {
            let content = r#"Hello {{ name }}!

    {{ code_example }}

After code {{ end }}."#;
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            // Indented code should be skipped
            // Note: pulldown-cmark needs a blank line before for indented code
            assert!(exprs.iter().all(|e| e.expression != "code_example"));
        }

        #[test]
        fn handles_nested_braces() {
            // This is an edge case - nested {{ }} should work
            let content = "{{ foo }}{{ bar }}";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 2);
            assert_eq!(exprs[0].expression, "foo");
            assert_eq!(exprs[1].expression, "bar");
        }

        #[test]
        fn handles_unclosed_expression() {
            let content = "Hello {{ name and more text";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            // Unclosed expression should be skipped
            assert_eq!(exprs.len(), 0);
        }

        #[test]
        fn handles_empty_expression() {
            let content = "Hello {{}} world";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            // Empty expression should be skipped
            assert_eq!(exprs.len(), 0);
        }

        #[test]
        fn handles_only_whitespace_expression() {
            let content = "Hello {{   }} world";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            // Whitespace-only expression should be skipped
            assert_eq!(exprs.len(), 0);
        }

        #[test]
        fn finds_complex_expression() {
            let content = r#"{{ color || "unknown" }}"#;
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert_eq!(exprs.len(), 1);
            assert_eq!(exprs[0].expression, r#"color || "unknown""#);
        }
    }

    mod lexer_tokens {
        use super::*;

        #[test]
        fn tokenizes_simple_variable() {
            let mut lexer = Lexer::new("foo");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[1], Token::Eof));
        }

        #[test]
        fn tokenizes_dotted_variable() {
            let mut lexer = Lexer::new("user.name");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "user.name"));
        }

        #[test]
        fn tokenizes_deeply_nested_variable() {
            let mut lexer = Lexer::new("ctx.user.profile.name");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "ctx.user.profile.name"));
        }

        #[test]
        fn tokenizes_fallback_operator() {
            let mut lexer = Lexer::new("foo || bar");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "bar"));
        }

        #[test]
        fn tokenizes_ternary() {
            let mut lexer = Lexer::new("foo ? bar : baz");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 6);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[1], Token::Question));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "bar"));
            assert!(matches!(&tokens[3], Token::Colon));
            assert!(matches!(&tokens[4], Token::Variable(v) if v == "baz"));
        }

        #[test]
        fn tokenizes_parentheses() {
            let mut lexer = Lexer::new("length(foo)");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 5);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "length"));
            assert!(matches!(&tokens[1], Token::LParen));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[3], Token::RParen));
        }

        #[test]
        fn tokenizes_comma() {
            let mut lexer = Lexer::new("func(a, b)");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 7);
            assert!(matches!(&tokens[3], Token::Comma));
        }

        #[test]
        fn tokenizes_double_quoted_string() {
            let mut lexer = Lexer::new(r#""hello world""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::StringLiteral(s) if s == "hello world"));
        }

        #[test]
        fn tokenizes_single_quoted_string() {
            let mut lexer = Lexer::new("'hello world'");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::StringLiteral(s) if s == "hello world"));
        }

        #[test]
        fn tokenizes_string_with_escapes() {
            let mut lexer = Lexer::new(r#""hello\nworld""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[0], Token::StringLiteral(s) if s == "hello\nworld"));
        }

        #[test]
        fn tokenizes_string_with_escaped_quote() {
            let mut lexer = Lexer::new(r#""say \"hello\"""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[0], Token::StringLiteral(s) if s == "say \"hello\""));
        }

        #[test]
        fn tokenizes_integer() {
            let mut lexer = Lexer::new("42");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::NumberLiteral(n) if *n == 42.0));
        }

        #[test]
        fn tokenizes_negative_integer() {
            // Unary minus is parser-level. Lexer always emits Minus + positive number.
            let mut lexer = Lexer::new("-42");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 3);
            assert!(matches!(&tokens[0], Token::Minus));
            assert!(matches!(&tokens[1], Token::NumberLiteral(n) if *n == 42.0));
        }

        #[test]
        fn tokenizes_float() {
            let mut lexer = Lexer::new("3.15");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(
                matches!(&tokens[0], Token::NumberLiteral(n) if (*n - 3.15).abs() < f64::EPSILON)
            );
        }

        #[test]
        fn tokenizes_negative_float() {
            let mut lexer = Lexer::new("-3.15");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 3);
            assert!(matches!(&tokens[0], Token::Minus));
            assert!(
                matches!(&tokens[1], Token::NumberLiteral(n) if (*n - 3.15).abs() < f64::EPSILON)
            );
        }

        #[test]
        fn tokenizes_true_literal() {
            let mut lexer = Lexer::new("true");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::BoolLiteral(b) if *b));
        }

        #[test]
        fn tokenizes_false_literal() {
            let mut lexer = Lexer::new("false");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::BoolLiteral(b) if !*b));
        }

        #[test]
        fn tokenizes_bool_literal_in_expression() {
            let mut lexer = Lexer::new("enabled ? true : false");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 6);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "enabled"));
            assert!(matches!(&tokens[1], Token::Question));
            assert!(matches!(&tokens[2], Token::BoolLiteral(b) if *b));
            assert!(matches!(&tokens[3], Token::Colon));
            assert!(matches!(&tokens[4], Token::BoolLiteral(b) if !*b));
        }

        #[test]
        fn tokenizes_equality() {
            let mut lexer = Lexer::new("a == b");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[1], Token::CompOp(ComparisonOp::Equal)));
        }

        #[test]
        fn tokenizes_inequality() {
            let mut lexer = Lexer::new("a != b");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[1], Token::CompOp(ComparisonOp::NotEqual)));
        }

        #[test]
        fn tokenizes_unary_not() {
            let mut lexer = Lexer::new("!enabled");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 3);
            assert!(matches!(&tokens[0], Token::Bang));
            assert!(matches!(&tokens[1], Token::Variable(v) if v == "enabled"));
        }

        #[test]
        fn tokenizes_greater_than() {
            let mut lexer = Lexer::new("a > b");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(
                &tokens[1],
                Token::CompOp(ComparisonOp::GreaterThan)
            ));
        }

        #[test]
        fn tokenizes_greater_than_or_equal() {
            let mut lexer = Lexer::new("a >= b");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(
                &tokens[1],
                Token::CompOp(ComparisonOp::GreaterThanOrEqual)
            ));
        }

        #[test]
        fn tokenizes_less_than() {
            let mut lexer = Lexer::new("a < b");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[1], Token::CompOp(ComparisonOp::LessThan)));
        }

        #[test]
        fn tokenizes_less_than_or_equal() {
            let mut lexer = Lexer::new("a <= b");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(
                &tokens[1],
                Token::CompOp(ComparisonOp::LessThanOrEqual)
            ));
        }

        #[test]
        fn tokenizes_arithmetic_operators() {
            let mut lexer = Lexer::new("a + b - c * d / e % f");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[1], Token::Plus));
            assert!(matches!(&tokens[3], Token::Minus));
            assert!(matches!(&tokens[5], Token::Star));
            assert!(matches!(&tokens[7], Token::Slash));
            assert!(matches!(&tokens[9], Token::Percent));
        }

        #[test]
        fn tokenizes_arithmetic_no_space() {
            let mut lexer = Lexer::new("5-3");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[0], Token::NumberLiteral(n) if *n == 5.0));
            assert!(matches!(&tokens[1], Token::Minus));
            assert!(matches!(&tokens[2], Token::NumberLiteral(n) if *n == 3.0));
        }

        #[test]
        fn tokenizes_brackets() {
            let mut lexer = Lexer::new("items[0]");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 5);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "items"));
            assert!(matches!(&tokens[1], Token::LBracket));
            assert!(matches!(&tokens[2], Token::NumberLiteral(n) if *n == 0.0));
            assert!(matches!(&tokens[3], Token::RBracket));
        }

        #[test]
        fn tokenizes_negative_index() {
            let mut lexer = Lexer::new("items[-1]");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 6);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "items"));
            assert!(matches!(&tokens[1], Token::LBracket));
            assert!(matches!(&tokens[2], Token::Minus));
            assert!(matches!(&tokens[3], Token::NumberLiteral(n) if *n == 1.0));
            assert!(matches!(&tokens[4], Token::RBracket));
        }

        #[test]
        fn tokenizes_string_index() {
            let mut lexer = Lexer::new(r#"config["key"]"#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 5);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "config"));
            assert!(matches!(&tokens[1], Token::LBracket));
            assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "key"));
            assert!(matches!(&tokens[3], Token::RBracket));
        }

        #[test]
        fn tokenizes_postfix_dot_after_paren() {
            // Plain dotted paths are folded into Variable; standalone Dot is
            // emitted only when the dot can't be absorbed (e.g., after `)`).
            let mut lexer = Lexer::new("(foo).bar");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[0], Token::LParen));
            assert!(matches!(&tokens[1], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[2], Token::RParen));
            assert!(matches!(&tokens[3], Token::Dot));
            assert!(matches!(&tokens[4], Token::Variable(v) if v == "bar"));
        }

        #[test]
        fn tokenizes_postfix_dot_after_bracket() {
            let mut lexer = Lexer::new("items[0].name");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[0], Token::Variable(v) if v == "items"));
            assert!(matches!(&tokens[1], Token::LBracket));
            assert!(matches!(&tokens[2], Token::NumberLiteral(_)));
            assert!(matches!(&tokens[3], Token::RBracket));
            assert!(matches!(&tokens[4], Token::Dot));
            assert!(matches!(&tokens[5], Token::Variable(v) if v == "name"));
        }

        #[test]
        fn tokenizes_dotted_variable_then_bracket() {
            let mut lexer = Lexer::new("foo.bar[0]");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 5);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo.bar"));
            assert!(matches!(&tokens[1], Token::LBracket));
        }

        #[test]
        fn tokenizes_dot_followed_by_digit() {
            // foo.0 — dot is not absorbed into the variable because `0` is not
            // an identifier start. Lexer emits Dot + NumberLiteral.
            let mut lexer = Lexer::new("foo.0");
            let tokens = lexer.tokenize_all().unwrap();

            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[1], Token::Dot));
            assert!(matches!(&tokens[2], Token::NumberLiteral(n) if *n == 0.0));
        }
    }

    mod lexer_complex_expressions {
        use super::*;

        #[test]
        fn tokenizes_fallback_with_string() {
            let mut lexer = Lexer::new(r#"color || "unknown""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "color"));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "unknown"));
            assert!(matches!(&tokens[3], Token::Eof));
        }

        #[test]
        fn tokenizes_double_pipe_as_fallback() {
            let mut lexer = Lexer::new(r#"plan || "plan.md""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "plan"));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "plan.md"));
            assert!(matches!(&tokens[3], Token::Eof));
        }

        #[test]
        fn tokenizes_ternary_with_strings() {
            let mut lexer = Lexer::new(r#"color ? "known" : "unknown""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 6);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "color"));
            assert!(matches!(&tokens[1], Token::Question));
            assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "known"));
            assert!(matches!(&tokens[3], Token::Colon));
            assert!(matches!(&tokens[4], Token::StringLiteral(s) if s == "unknown"));
        }

        #[test]
        fn tokenizes_comparison_ternary() {
            let mut lexer = Lexer::new(r#"count > 0 ? "has items" : "empty""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 8);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "count"));
            assert!(matches!(
                &tokens[1],
                Token::CompOp(ComparisonOp::GreaterThan)
            ));
            assert!(matches!(&tokens[2], Token::NumberLiteral(n) if *n == 0.0));
            assert!(matches!(&tokens[3], Token::Question));
            assert!(matches!(&tokens[4], Token::StringLiteral(s) if s == "has items"));
            assert!(matches!(&tokens[5], Token::Colon));
            assert!(matches!(&tokens[6], Token::StringLiteral(s) if s == "empty"));
        }

        #[test]
        fn tokenizes_function_call() {
            let mut lexer = Lexer::new("length(items)");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 5);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "length"));
            assert!(matches!(&tokens[1], Token::LParen));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "items"));
            assert!(matches!(&tokens[3], Token::RParen));
        }

        #[test]
        fn tokenizes_function_with_default() {
            let mut lexer = Lexer::new("number(value, 0)");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 7);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "number"));
            assert!(matches!(&tokens[1], Token::LParen));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "value"));
            assert!(matches!(&tokens[3], Token::Comma));
            assert!(matches!(&tokens[4], Token::NumberLiteral(n) if *n == 0.0));
            assert!(matches!(&tokens[5], Token::RParen));
        }

        #[test]
        fn tokenizes_context_variable() {
            let mut lexer = Lexer::new("ctx.today");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "ctx.today"));
        }

        #[test]
        fn tokenizes_env_variable() {
            let mut lexer = Lexer::new("env.HOME");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "env.HOME"));
        }

        #[test]
        fn tokenizes_env_with_fallback() {
            let mut lexer = Lexer::new(r#"env.FAVORITE_COLOR || "unknown""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(
                &tokens[0],
                Token::Variable(v) if v == "env.FAVORITE_COLOR"
            ));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::StringLiteral(s) if s == "unknown"));
        }
    }

    mod lexer_errors {
        use super::*;

        #[test]
        fn error_unterminated_string() {
            let mut lexer = Lexer::new(r#""hello"#);
            let result = lexer.tokenize_all();

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unterminated string"));
        }

        #[test]
        fn error_unexpected_character() {
            let mut lexer = Lexer::new("@invalid");
            let result = lexer.tokenize_all();

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unexpected character"));
        }

        #[test]
        fn error_single_equals() {
            let mut lexer = Lexer::new("a = b");
            let result = lexer.tokenize_all();

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Expected '='"));
        }

        #[test]
        fn error_single_bang() {
            let mut lexer = Lexer::new("!foo");
            let result = lexer.tokenize_all();

            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert!(matches!(&tokens[0], Token::Bang));
            assert!(matches!(&tokens[1], Token::Variable(v) if v == "foo"));
        }

        #[test]
        fn error_bare_pipe_in_interpolation() {
            let mut lexer = Lexer::new("foo | bar");
            let result = lexer.tokenize_all();

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unexpected '|'"));
            assert!(err.message.contains("fallback"));
        }

        #[test]
        fn error_bare_pipe_in_condition() {
            let mut lexer = Lexer::with_mode("foo | bar", ParseMode::Condition);
            let result = lexer.tokenize_all();

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unexpected '|'"));
            assert!(err.message.contains("logical OR"));
        }

        #[test]
        fn string_literal_with_pipe_is_ok() {
            let mut lexer = Lexer::new(r#""a | b""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0],
                Token::StringLiteral(s) if s == "a | b"
            ));
        }

        #[test]
        fn string_literal_with_double_pipe_is_ok() {
            let mut lexer = Lexer::new(r#""a || b""#);
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(
                &tokens[0],
                Token::StringLiteral(s) if s == "a || b"
            ));
        }
    }

    mod lexer_whitespace {
        use super::*;

        #[test]
        fn handles_leading_whitespace() {
            let mut lexer = Lexer::new("   foo");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
        }

        #[test]
        fn handles_trailing_whitespace() {
            let mut lexer = Lexer::new("foo   ");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
        }

        #[test]
        fn handles_no_whitespace() {
            let mut lexer = Lexer::new("a||b?c:d");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 8);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "a"));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "b"));
            assert!(matches!(&tokens[3], Token::Question));
            assert!(matches!(&tokens[4], Token::Variable(v) if v == "c"));
            assert!(matches!(&tokens[5], Token::Colon));
            assert!(matches!(&tokens[6], Token::Variable(v) if v == "d"));
        }

        #[test]
        fn handles_mixed_whitespace() {
            let mut lexer = Lexer::new("  foo  ||  bar  ");
            let tokens = lexer.tokenize_all().unwrap();

            assert_eq!(tokens.len(), 4);
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "foo"));
            assert!(matches!(&tokens[1], Token::Pipe));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "bar"));
        }
    }

    mod token_display {
        use super::*;

        #[test]
        fn variable_display() {
            assert_eq!(Token::Variable("foo".to_string()).to_string(), "foo");
        }

        #[test]
        fn operators_display() {
            assert_eq!(Token::Pipe.to_string(), "|");
            assert_eq!(Token::Question.to_string(), "?");
            assert_eq!(Token::Colon.to_string(), ":");
            assert_eq!(Token::LParen.to_string(), "(");
            assert_eq!(Token::RParen.to_string(), ")");
            assert_eq!(Token::Comma.to_string(), ",");
        }

        #[test]
        fn literals_display() {
            assert_eq!(
                Token::StringLiteral("hello".to_string()).to_string(),
                "\"hello\""
            );
            assert_eq!(Token::NumberLiteral(42.0).to_string(), "42");
            assert_eq!(Token::BoolLiteral(true).to_string(), "true");
            assert_eq!(Token::BoolLiteral(false).to_string(), "false");
        }

        #[test]
        fn comparison_ops_display() {
            assert_eq!(ComparisonOp::Equal.to_string(), "==");
            assert_eq!(ComparisonOp::NotEqual.to_string(), "!=");
            assert_eq!(ComparisonOp::GreaterThan.to_string(), ">");
            assert_eq!(ComparisonOp::GreaterThanOrEqual.to_string(), ">=");
            assert_eq!(ComparisonOp::LessThan.to_string(), "<");
            assert_eq!(ComparisonOp::LessThanOrEqual.to_string(), "<=");
        }

        #[test]
        fn arithmetic_tokens_display() {
            assert_eq!(Token::Plus.to_string(), "+");
            assert_eq!(Token::Minus.to_string(), "-");
            assert_eq!(Token::Star.to_string(), "*");
            assert_eq!(Token::Slash.to_string(), "/");
            assert_eq!(Token::Percent.to_string(), "%");
        }

        #[test]
        fn bracket_and_dot_tokens_display() {
            assert_eq!(Token::LBracket.to_string(), "[");
            assert_eq!(Token::RBracket.to_string(), "]");
            assert_eq!(Token::Dot.to_string(), ".");
        }

        #[test]
        fn eof_display() {
            assert_eq!(Token::Eof.to_string(), "<EOF>");
        }
    }

    mod lexer_error_display {
        use super::*;

        #[test]
        fn error_display() {
            let err = LexerError::new("Test error", 5);
            assert_eq!(err.to_string(), "Test error at position 5");
        }
    }

    mod condition_mode_tokens {
        use super::*;

        #[test]
        fn condition_mode_double_pipe_is_or_or() {
            let mut lexer = Lexer::with_mode("a || b", ParseMode::Condition);
            let tokens = lexer.tokenize_all().unwrap();
            assert!(matches!(&tokens[1], Token::OrOr));
        }

        #[test]
        fn condition_mode_double_amp_is_and_and() {
            let mut lexer = Lexer::with_mode("a && b", ParseMode::Condition);
            let tokens = lexer.tokenize_all().unwrap();
            assert!(matches!(&tokens[1], Token::AndAnd));
        }

        #[test]
        fn condition_mode_single_pipe_is_error() {
            let mut lexer = Lexer::with_mode("a | b", ParseMode::Condition);
            let result = lexer.tokenize_all();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Unexpected '|'"));
            assert!(err.message.contains("logical OR"));
        }

        #[test]
        fn condition_mode_single_amp_still_errors() {
            let mut lexer = Lexer::with_mode("a & b", ParseMode::Condition);
            let result = lexer.tokenize_all();
            assert!(result.is_err());
        }

        #[test]
        fn interpolation_mode_double_pipe_collapses_to_fallback() {
            let mut lexer = Lexer::with_mode("a || b", ParseMode::Interpolation);
            let tokens = lexer.tokenize_all().unwrap();
            assert!(matches!(&tokens[1], Token::Pipe));
        }

        #[test]
        fn interpolation_mode_double_amp_errors() {
            let mut lexer = Lexer::with_mode("a && b", ParseMode::Interpolation);
            let result = lexer.tokenize_all();
            assert!(result.is_err());
        }

        #[test]
        fn condition_mode_mixed_operators_tokens() {
            let mut lexer = Lexer::with_mode("a && b || c", ParseMode::Condition);
            let tokens = lexer.tokenize_all().unwrap();
            assert!(matches!(&tokens[0], Token::Variable(v) if v == "a"));
            assert!(matches!(&tokens[1], Token::AndAnd));
            assert!(matches!(&tokens[2], Token::Variable(v) if v == "b"));
            assert!(matches!(&tokens[3], Token::OrOr));
            assert!(matches!(&tokens[4], Token::Variable(v) if v == "c"));
        }
    }
}
