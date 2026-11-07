//! Locates `{{ … }}` expressions, `{{{ … }}}` literals, and `{{!data:…}}`
//! literal tokens in text.
//!
//! The scan knows nothing about Markdown: a host that must skip code blocks
//! computes their byte ranges itself and passes them to [`scan`].
//! [`scan_plain`] excludes nothing.
//!
//! An expression ends at the first `}}` that sits outside every quoted string
//! (the expression lexer's string and escape rules, so strings may span
//! lines) and closes every single `{` opened inside it. That span replaces
//! the older "first unnested `}}`" span only when the two differ, the older
//! span's text does not parse, and the new span's text does
//! ([`parse`](crate::expression::parse)). So no span that parses under the
//! older rule changes, and a changed span always parses; prose apostrophes
//! and unbalanced quotes keep the older span. The trade-off: an expression
//! with a real syntax error and a `}}` inside a string still ends at that
//! `}}`.
//!
//! [`expression_region_at`] answers the editor question "is this offset inside
//! a complete or still-open expression?" with the same lexical rules, without
//! the parse gate.
//!
//! An opener preceded by an odd run of backslashes (`\{{`) is not a span; an
//! even run (`\\{{`) leaves it active. Scanning never removes the backslashes.
//!
//! ## Examples
//!
//! ```
//! use simplified_schema::expression::scan::{scan, scan_plain};
//!
//! let result = scan_plain("Hello {{ name }} and {{{ kept }}}");
//! assert_eq!(result.expressions[0].expression, "name");
//! assert_eq!(result.literals[0].content, " kept ");
//!
//! // Bytes 0..9 are excluded, so only the second span is found.
//! let result = scan("{{ a }}  {{ b }}", &[(0, 9)]);
//! assert_eq!(result.expressions.len(), 1);
//! assert_eq!(result.expressions[0].expression, "b");
//! ```

use tracing::debug;

use crate::expression::parse;

/// What follows `{{` in a literal token (`{{!data:…}}`).
const LITERAL_TOKEN_BODY_PREFIX: &str = "!data:";

/// Location of an interpolation expression in scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionLocation {
    /// Byte offset of the first `{` in the source.
    pub start: usize,

    /// Byte offset after the last `}` in the source.
    pub end: usize,

    /// The expression content between `{{` and `}}`, trimmed.
    pub expression: String,
}

/// Location of an interpolation literal (`{{{ ... }}}`) in content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationLiteral {
    /// Byte offset of the first `{` in the source.
    pub start: usize,

    /// Byte offset after the last `}` in the source.
    pub end: usize,

    /// The literal content between `{{{` and `}}}`, preserved verbatim.
    pub content: String,
}

/// Location of a literal token (`{{!data:…}}`) in content.
///
/// The scanner only finds the token; whether it is well formed and allowed
/// where it sits is decided by the caller (see [`pending`](crate::pending)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralTokenLocation {
    /// Byte offset of the first `{` in the source.
    pub start: usize,

    /// Byte offset after the first `}}` following the opener, or the end of
    /// the content when there is none.
    pub end: usize,
}

/// Result of scanning content for interpolation expressions and literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionScanResult {
    /// Interpolation expressions found in the content.
    pub expressions: Vec<ExpressionLocation>,

    /// Interpolation literals found in the content.
    pub literals: Vec<InterpolationLiteral>,

    /// Literal tokens found in the content. A token is never an expression.
    pub tokens: Vec<LiteralTokenLocation>,
}

/// Scans `content` for expressions, literals, and literal tokens, skipping
/// any opener whose first `{` falls inside one of `excluded` (half-open byte
/// ranges `start..end`).
#[must_use]
pub fn scan(content: &str, excluded: &[(usize, usize)]) -> ExpressionScanResult {
    let is_excluded = |pos: usize| excluded.iter().any(|(start, end)| pos >= *start && pos < *end);
    let mut expressions = Vec::new();
    let mut literals = Vec::new();
    let mut tokens = Vec::new();
    let mut pos = 0;
    let bytes = content.as_bytes();
    let len = bytes.len();

    while pos < len.saturating_sub(3) {
        // An odd run of backslashes escapes the opener. The backslashes stay
        // in the text; the Markdown renderer removes the visual escape.
        if bytes[pos] == b'{' && bytes[pos + 1] == b'{' && is_escaped_opener(bytes, pos) {
            while pos < len && bytes[pos] == b'{' {
                pos += 1;
            }
            continue;
        }

        // Check for literal opener before regular expression opener.
        // A literal opener is exactly three consecutive `{` characters.
        if bytes[pos] == b'{'
            && pos + 1 < len
            && bytes[pos + 1] == b'{'
            && pos + 2 < len
            && bytes[pos + 2] == b'{'
            && (pos + 3 >= len || bytes[pos + 3] != b'{')
        {
            if is_excluded(pos) {
                pos += 1;
                continue;
            }

            // Find the first subsequent }}}.
            let start = pos;
            let mut close_pos = pos + 3;
            let mut found_close = false;
            while close_pos + 2 < len {
                if bytes[close_pos] == b'}' && bytes[close_pos + 1] == b'}' && bytes[close_pos + 2] == b'}' {
                    let end = close_pos + 3;
                    let literal = content[start + 3..close_pos].to_string();
                    literals.push(InterpolationLiteral {
                        start,
                        end,
                        content: literal,
                    });
                    pos = end;
                    found_close = true;
                    break;
                }
                close_pos += 1;
            }

            if !found_close {
                // Unclosed {{{ - fall back to legacy {{ scanning at the
                // same byte position to preserve existing behavior.
                let (expr, next_pos) = scan_expression(content, start);
                if let Some(expr) = expr {
                    expressions.push(expr);
                }
                pos = next_pos;
            }
            continue;
        }

        // Look for opening {{
        if bytes[pos] == b'{' && pos + 1 < len && bytes[pos + 1] == b'{' {
            if is_excluded(pos) {
                pos += 2;
                continue;
            }

            let start = pos;
            // A literal token is classified before expression parsing,
            // where `!` would lex as the unary operator. It ends at the
            // first `}}`; its payload alphabet holds no brace.
            if content[start + 2..].starts_with(LITERAL_TOKEN_BODY_PREFIX) {
                let body = start + 2 + LITERAL_TOKEN_BODY_PREFIX.len();
                let end = content[body..].find("}}").map_or(len, |close| body + close + 2);
                tokens.push(LiteralTokenLocation { start, end });
                pos = end;
                continue;
            }
            let (expr, next_pos) = scan_expression(content, start);
            if let Some(expr) = expr {
                expressions.push(expr);
            }
            pos = next_pos;
            continue;
        }

        pos += 1;
    }

    debug!(
        expression_count = expressions.len(),
        literal_count = literals.len(),
        token_count = tokens.len(),
        "interpolation: scan complete"
    );
    ExpressionScanResult {
        expressions,
        literals,
        tokens,
    }
}

/// [`scan`] with nothing excluded.
#[must_use]
pub fn scan_plain(content: &str) -> ExpressionScanResult {
    scan(content, &[])
}

/// The expressions [`scan_plain`] finds, without literals or tokens.
#[must_use]
pub fn find_all_plain(content: &str) -> Vec<ExpressionLocation> {
    scan_plain(content).expressions
}

/// Where an editor offset sits relative to the `{{ … }}` expressions of a text.
///
/// Uses the scanner's lexical rules only (openers, escapes, excluded ranges,
/// literals, tokens, strings, and single-brace depth) and never applies the
/// parse gate, so a complete region may differ from [`scan`]'s span for text
/// that does not parse. One editor-only rule is added: a `{{` at depth 0
/// outside a string starts a new expression, so the cursor in
/// `{{ don't }} it's fine {{ x` belongs to the second opener. A prose
/// apostrophe after a broken expression can still read as an open string;
/// that ambiguity is inherent in unfinished text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionRegion {
    /// Byte offset of the opening `{{`.
    pub start: usize,

    /// Byte offset after the closing `}}`, or `None` while the expression is
    /// still open at the end of the text or at a following `{{`.
    pub end: Option<usize>,

    /// Byte offset of the quote opening the string that holds the offset.
    pub string_start: Option<usize>,

    /// Single `{` opened outside strings between the opener and the offset
    /// and not yet closed.
    pub brace_depth: usize,
}

/// Finds the expression region holding the byte `offset` of `text`, skipping
/// openers whose first `{` falls inside one of `excluded` (as in [`scan`]).
///
/// Returns `None` when the offset is in prose, on or between the braces of a
/// delimiter, or inside a `{{{ … }}}` literal or `{{!data:…}}` token. An
/// offset past the end of `text` is `None`.
///
/// ## Examples
///
/// ```
/// use simplified_schema::expression::scan::expression_region_at;
///
/// let text = r#"{{ load_data(f, "{a: {b: string}}") "#;
/// let region = expression_region_at(text, text.len(), &[]).unwrap();
/// assert_eq!((region.start, region.end), (0, None));
/// assert_eq!(region.string_start, None);
/// ```
#[must_use]
pub fn expression_region_at(text: &str, offset: usize, excluded: &[(usize, usize)]) -> Option<ExpressionRegion> {
    let is_excluded = |pos: usize| excluded.iter().any(|(start, end)| pos >= *start && pos < *end);
    let bytes = text.as_bytes();
    let len = bytes.len();
    if offset > len {
        return None;
    }
    let mut pos = 0;

    while pos < offset && pos + 1 < len {
        if !(bytes[pos] == b'{' && bytes[pos + 1] == b'{') {
            pos += 1;
            continue;
        }
        if is_escaped_opener(bytes, pos) {
            while pos < len && bytes[pos] == b'{' {
                pos += 1;
            }
            continue;
        }

        let triple = pos + 2 < len && bytes[pos + 2] == b'{' && (pos + 3 >= len || bytes[pos + 3] != b'{');
        if triple {
            if is_excluded(pos) {
                pos += 1;
                continue;
            }
            if pos + 2 > offset {
                return None;
            }
            // An unclosed `{{{` is scanned as a `{{` expression, as in `scan`.
            if let Some(close) = text[pos + 3..].find("}}}") {
                let end = pos + 3 + close + 3;
                if offset < end {
                    return None;
                }
                pos = end;
                continue;
            }
        } else if is_excluded(pos) {
            pos += 2;
            continue;
        }

        if pos + 2 > offset {
            return None;
        }
        if !triple && bytes[pos + 2..].starts_with(LITERAL_TOKEN_BODY_PREFIX.as_bytes()) {
            let body = pos + 2 + LITERAL_TOKEN_BODY_PREFIX.len();
            let end = text[body..].find("}}").map_or(len, |close| body + close + 2);
            if offset < end {
                return None;
            }
            pos = end;
            continue;
        }

        match region_from(bytes, pos, offset) {
            RegionWalk::Found(region) => return Some(region),
            RegionWalk::Resume(next) => pos = next,
            RegionWalk::Outside => return None,
        }
    }
    None
}

/// Outcome of lexing one opener for [`expression_region_at`].
enum RegionWalk {
    /// The offset lies in this expression.
    Found(ExpressionRegion),
    /// The expression ended before the offset; keep walking here.
    Resume(usize),
    /// The offset sits between the two braces of the closing `}}`.
    Outside,
}

fn region_from(bytes: &[u8], start: usize, offset: usize) -> RegionWalk {
    let mut state = Delimiters::default();
    let mut at_offset = None;
    let mut i = start + 2;
    let found = |at: Option<(Option<usize>, usize)>, end| {
        let (string_start, brace_depth) = at.unwrap_or_default();
        RegionWalk::Found(ExpressionRegion {
            start,
            end,
            string_start,
            brace_depth,
        })
    };

    loop {
        if at_offset.is_none() && i >= offset {
            at_offset = Some((state.string_start, state.depth));
        }
        if i >= bytes.len() {
            return found(at_offset, None);
        }
        if state.string_start.is_none() && state.depth == 0 && bytes[i] == b'{' && bytes.get(i + 1) == Some(&b'{') {
            if at_offset.is_some() {
                return found(at_offset, None);
            }
            return RegionWalk::Resume(i);
        }
        match state.advance(bytes, i) {
            Step::Next(next) => i = next,
            Step::Close => {
                if at_offset.is_some() {
                    return found(at_offset, Some(i + 2));
                }
                return if i + 2 <= offset {
                    RegionWalk::Resume(i + 2)
                } else {
                    RegionWalk::Outside
                };
            }
        }
    }
}

/// Lexical state between an opener and its closer: the open string, if any,
/// and the single `{` opened outside strings.
#[derive(Default)]
struct Delimiters {
    string_start: Option<usize>,
    depth: usize,
}

enum Step {
    Next(usize),
    /// The byte is a `}` at depth 0 outside a string, followed by `}`.
    Close,
}

impl Delimiters {
    /// Consumes the byte at `i`. Every delimiter is ASCII and a UTF-8
    /// continuation byte never equals one, so skipping the byte after a `\`
    /// lands at worst on a continuation byte, which is then ordinary text.
    fn advance(&mut self, bytes: &[u8], i: usize) -> Step {
        if let Some(open) = self.string_start {
            match bytes[i] {
                b'\\' => return Step::Next(i + 2),
                quote if quote == bytes[open] => self.string_start = None,
                _ => {}
            }
            return Step::Next(i + 1);
        }
        match bytes[i] {
            b'"' | b'\'' => self.string_start = Some(i),
            b'{' => self.depth += 1,
            b'}' if self.depth > 0 => self.depth -= 1,
            b'}' if bytes.get(i + 1) == Some(&b'}') => return Step::Close,
            _ => {}
        }
        Step::Next(i + 1)
    }
}

/// Byte offset of the `}}` closing the expression opened at `start`, under
/// the quote- and brace-balanced rule; `None` at EOF or in an unterminated
/// string.
fn balanced_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut state = Delimiters::default();
    let mut i = start + 2;
    while i < bytes.len() {
        match state.advance(bytes, i) {
            Step::Next(next) => i = next,
            Step::Close => return Some(i),
        }
    }
    None
}

/// Scans the `{{ … }}` expression opened at `start`, applying the parse gate
/// described in the module docs. Returns the expression (if any) and the
/// position at which scanning continues.
fn scan_expression(content: &str, start: usize) -> (Option<ExpressionLocation>, usize) {
    let legacy = scan_legacy_expression(content, start);
    let Some(close) = balanced_close(content.as_bytes(), start) else {
        return legacy;
    };
    let end = close + 2;
    if legacy.1 == end {
        return legacy;
    }
    if legacy.0.as_ref().is_some_and(|old| parse(&old.expression).is_ok()) {
        return legacy;
    }
    let expression = content[start + 2..close].trim();
    if expression.is_empty() || parse(expression).is_err() {
        return legacy;
    }
    (
        Some(ExpressionLocation {
            start,
            end,
            expression: expression.to_string(),
        }),
        end,
    )
}

/// Scans a legacy `{{ ... }}` expression starting at `start`.
///
/// Returns the discovered expression (if any) and the position at which
/// scanning should continue.
fn scan_legacy_expression(content: &str, start: usize) -> (Option<ExpressionLocation>, usize) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut pos = start + 2; // Skip past {{
    let mut depth = 1;

    while pos + 1 < len {
        if bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            depth += 1;
            pos += 2;
        } else if bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
            depth -= 1;
            if depth == 0 {
                let end = pos + 2;
                let expr_start = start + 2;
                let expr_end = pos;
                if expr_start < expr_end {
                    let expression = content[expr_start..expr_end].trim();
                    if !expression.is_empty() {
                        return (
                            Some(ExpressionLocation {
                                start,
                                end,
                                expression: expression.to_string(),
                            }),
                            end,
                        );
                    }
                }
                return (None, end);
            }
            pos += 2;
        } else {
            pos += 1;
        }
    }

    // Unclosed {{ - skip it
    (None, start + 2)
}

/// Returns `true` when the brace at `pos` follows an odd-length run of
/// backslashes, which escapes it; an even run escapes itself instead.
fn is_escaped_opener(bytes: &[u8], pos: usize) -> bool {
    let run = bytes[..pos].iter().rev().take_while(|&&b| b == b'\\').count();
    run % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expressions(content: &str) -> Vec<String> {
        find_all_plain(content).into_iter().map(|loc| loc.expression).collect()
    }

    #[test]
    fn expression_offsets_cover_both_braces() {
        let result = scan_plain("Hi {{ name }}!");
        assert_eq!(
            result.expressions,
            vec![ExpressionLocation {
                start: 3,
                end: 13,
                expression: "name".to_string()
            }]
        );
        assert!(result.literals.is_empty());
        assert!(result.tokens.is_empty());
    }

    #[test]
    fn nested_openers_close_at_the_matching_depth() {
        assert_eq!(expressions("{{ a {{ b }} c }}"), vec!["a {{ b }} c"]);
    }

    #[test]
    fn empty_and_unclosed_spans_find_nothing() {
        assert!(expressions("{{   }}").is_empty());
        assert!(expressions("{{ open").is_empty());
        // An unclosed outer opener is skipped and scanning resumes inside it.
        assert_eq!(expressions("{{ open {{ x }}"), vec!["x"]);
    }

    #[test]
    fn a_triple_opener_is_a_verbatim_literal() {
        let result = scan_plain("a {{{ {{ x }} }}} b");
        assert!(result.expressions.is_empty());
        assert_eq!(result.literals.len(), 1);
        assert_eq!(result.literals[0].content, " {{ x }} ");
        assert_eq!((result.literals[0].start, result.literals[0].end), (2, 17));
    }

    #[test]
    fn an_unclosed_triple_opener_falls_back_to_expression_scanning() {
        let result = scan_plain("{{{ x }}");
        assert!(result.literals.is_empty());
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].expression, "{ x");
    }

    #[test]
    fn a_literal_token_is_never_an_expression() {
        let result = scan_plain("{{!data:v1:YQ}} and {{ x }}");
        assert_eq!(result.tokens, vec![LiteralTokenLocation { start: 0, end: 15 }]);
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].expression, "x");
    }

    #[test]
    fn an_unterminated_literal_token_runs_to_the_end() {
        let result = scan_plain("{{!data:v1:YQ");
        assert_eq!(result.tokens, vec![LiteralTokenLocation { start: 0, end: 13 }]);
    }

    #[test]
    fn backslash_run_parity_decides_the_opener() {
        for (content, expected) in [
            (r"\{{ x }}", Vec::<&str>::new()),
            (r"\\{{ x }}", vec!["x"]),
            (r"\\\{{ x }}", vec![]),
            (r"\\\\{{ x }}", vec!["x"]),
        ] {
            assert_eq!(expressions(content), expected, "for {content:?}");
        }
        let escaped = scan_plain(r"\{{{ x }}}");
        assert!(escaped.expressions.is_empty() && escaped.literals.is_empty());
    }

    #[test]
    fn excluded_ranges_hide_only_the_openers_inside_them() {
        let content = "{{ a }} {{{ b }}} {{ c }}";
        let result = scan(content, &[(0, 7), (8, 17)]);
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].expression, "c");
        assert!(result.literals.is_empty());
        // The range end is exclusive: an opener at `end` is scanned.
        assert_eq!(scan(content, &[(0, 0)]).expressions.len(), 2);
    }

    #[test]
    fn the_balanced_close_skips_strings_and_single_braces() {
        assert_eq!(balanced_close(br#"{{ "}}" }}"#, 0), Some(8));
        assert_eq!(balanced_close(b"{{ {a: {b: 1}} }}", 0), Some(15));
        assert_eq!(balanced_close(b"{{ a { }} b", 0), None);
        assert_eq!(balanced_close(br#"{{ "open }}"#, 0), None);
        // A backslash at the end of an open string runs past the text.
        assert_eq!(balanced_close(br#"{{ "\"#, 0), None);
    }

    #[test]
    fn the_gate_keeps_a_parsing_older_span() {
        // Older span `a {{ b }} c` parses and the balanced span is the same.
        assert_eq!(scan_expression("{{ a {{ b }} c }}", 0).1, 17);
        // Older span `"` fails; the balanced span parses and replaces it.
        let (found, next) = scan_expression(r#"{{ "}}" }} z"#, 0);
        assert_eq!((found.unwrap().expression.as_str(), next), (r#""}}""#, 10));
        // Neither parses: the older span stays, including its resume point.
        let (found, next) = scan_expression("{{ don't }} it's }}", 0);
        assert_eq!((found.unwrap().expression.as_str(), next), ("don't", 11));
    }

    #[test]
    fn offsets_are_bytes_of_the_supplied_text_including_crlf() {
        let content = "a: C:\\dir\r\nb: {{ x }}\r\n";
        let result = scan_plain(content);
        let found = &result.expressions[0];
        assert_eq!(&content[found.start..found.end], "{{ x }}");
        assert_eq!(found.start, 14);
    }
}
