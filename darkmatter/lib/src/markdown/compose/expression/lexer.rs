//! Locates `{{ ... }}` expressions in Markdown content.
//!
//! [`ExpressionFinder`] skips fenced and indented code blocks (inline code
//! spans are scanned) and hands the remaining text to
//! [`simplified_schema::expression::scan`]. The expression token lexer
//! (`Lexer`, `Token`, …) moved to [`simplified_schema::expression::lexer`] and
//! is re-exported here under its old paths.
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

pub use simplified_schema::expression::scan::{
    ExpressionLocation, ExpressionScanResult, InterpolationLiteral, LiteralTokenLocation,
};
// migration: removed in Phase 7
pub use simplified_schema::expression::lexer::{
    ComparisonOp, Lexer, LexerError, ParseMode, Token, identifier_prefix_start, lex_spanned,
};

/// Finds interpolation expressions in markdown content.
///
/// This finder scans content for `{{ ... }}` patterns while respecting
/// markdown structure - expressions inside fenced and indented code blocks
/// are ignored; inline code spans are scanned.
///
/// An opener preceded by an odd run of backslashes (`\{{`) is not a span in
/// any scan mode; an even run (`\\{{`) leaves it active. Scanning never
/// removes the backslashes.
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

    /// [`Self::new`] with code-block regions the caller already computed for
    /// `content` (the `blocks` of
    /// [`scan_code_regions`](crate::markdown::compose::parse_utils::scan_code_regions)).
    pub(crate) fn with_code_blocks(content: &'a str, code_blocks: Vec<(usize, usize)>) -> Self {
        Self {
            content,
            code_regions: code_blocks,
        }
    }

    /// Finds all interpolation expressions in the content.
    ///
    /// Returns a vector of `ExpressionLocation` for each `{{ ... }}` pattern
    /// found outside of code regions.
    pub fn find_all(&self) -> Vec<ExpressionLocation> {
        self.scan().expressions
    }

    /// Scans content for both interpolation expressions and interpolation
    /// literals (`{{{ ... }}}`).
    ///
    /// This is the primary scanning entry point. `find_all()` and
    /// `find_all_plain()` are convenience wrappers that return only the
    /// expression locations.
    pub fn scan(&self) -> ExpressionScanResult {
        simplified_schema::expression::scan::scan(self.content, &self.code_regions)
    }

    /// The fenced and indented code-block byte ranges this finder skips.
    pub(crate) fn code_regions(&self) -> &[(usize, usize)] {
        &self.code_regions
    }

    /// Finds all `{{ }}` expressions in a plain string with no code-region exclusions.
    pub fn find_all_plain(input: &'a str) -> Vec<ExpressionLocation> {
        simplified_schema::expression::scan::find_all_plain(input)
    }

    /// Scans content for both interpolation expressions and literals with
    /// no code-region exclusions.
    pub fn scan_plain(input: &'a str) -> ExpressionScanResult {
        simplified_schema::expression::scan::scan_plain(input)
    }

    /// Finds fenced and indented code-block regions in content.
    ///
    /// Inline code spans (single backticks) are intentionally NOT collected
    /// here — interpolation runs inside them so that templated identifiers
    /// like `` `var_{{ phase }}` `` expand as expected.
    fn find_code_regions(content: &str) -> Vec<(usize, usize)> {
        crate::markdown::compose::parse_utils::scan_code_regions(content).blocks
    }
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

    mod interpolation_literal {
        use super::*;

        #[test]
        fn finds_simple_literal() {
            let content = "Hello {{{ name }}}!";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.expressions.len(), 0);
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " name ");
            assert_eq!(result.literals[0].start, 6);
            assert_eq!(result.literals[0].end, 18);
        }

        #[test]
        fn finds_literal_inside_inline_code() {
            let content = "Code: `{{{ also_this }}}`";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.expressions.len(), 0);
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " also_this ");
        }

        #[test]
        fn skips_literal_inside_fenced_code_block() {
            let content = r#"Hello {{{ name }}}!

```rust
let x = {{{ variable }}};
```

And {{ another }}."#;
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.expressions.len(), 1);
            assert_eq!(result.expressions[0].expression, "another");
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " name ");
        }

        #[test]
        fn recognizes_tight_and_empty_literals() {
            for (content, expected) in [("{{{x}}}", "x"), ("{{{}}}", ""), ("{{{ }}}", " ")] {
                let finder = ExpressionFinder::new(content);
                let result = finder.scan();

                assert_eq!(result.literals.len(), 1, "for {content:?}");
                assert_eq!(result.literals[0].content, expected, "for {content:?}");
                assert_eq!(result.expressions.len(), 0, "for {content:?}");
            }
        }

        #[test]
        fn handles_adjacent_expression_and_literal() {
            let content = "{{ a }}{{{ b }}}";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.expressions.len(), 1);
            assert_eq!(result.expressions[0].expression, "a");
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " b ");
        }

        #[test]
        fn four_brace_opener_preserves_legacy_behavior() {
            let content = "{{{{ x }}}}";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.literals.len(), 0);
            assert_eq!(result.expressions.len(), 1);
            assert_eq!(result.expressions[0].expression, "{{ x }}");
        }

        #[test]
        fn unclosed_literal_opener_preserves_legacy_behavior() {
            let content = "{{{ x }}";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.literals.len(), 0);
            assert_eq!(result.expressions.len(), 1);
            assert_eq!(result.expressions[0].expression, "{ x");
        }

        #[test]
        fn literal_containing_expression_is_inert() {
            let content = "{{{ {{ x }} }}}";
            let finder = ExpressionFinder::new(content);
            let result = finder.scan();

            assert_eq!(result.expressions.len(), 0);
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " {{ x }} ");
        }

        #[test]
        fn find_all_drops_literals() {
            let content = "Only {{{ literal }}} content";
            let finder = ExpressionFinder::new(content);
            let exprs = finder.find_all();

            assert!(exprs.is_empty());
        }
    }

    mod backslash_escape {
        use super::*;

        fn plain_expressions(content: &str) -> Vec<String> {
            ExpressionFinder::find_all_plain(content)
                .into_iter()
                .map(|loc| loc.expression)
                .collect()
        }

        #[test]
        fn single_backslash_suppresses_opener_in_both_scan_modes() {
            let content = r"literal \{{ x }} here";
            assert!(plain_expressions(content).is_empty());
            assert!(ExpressionFinder::new(content).find_all().is_empty());
        }

        #[test]
        fn fully_escaped_braces_are_not_an_opener() {
            assert!(plain_expressions(r"literal \{\{ x }} here").is_empty());
        }

        #[test]
        fn unescaped_span_is_still_found() {
            assert_eq!(plain_expressions("a {{ x }} b"), vec!["x"]);
        }

        #[test]
        fn backslash_run_parity_decides_the_opener() {
            // An even run escapes itself under Markdown rules, leaving the
            // opener active; an odd run escapes the brace.
            for (content, expected) in [
                (r"\{{ x }}", Vec::<&str>::new()),
                (r"\\{{ x }}", vec!["x"]),
                (r"\\\{{ x }}", vec![]),
                (r"\\\\{{ x }}", vec!["x"]),
            ] {
                assert_eq!(plain_expressions(content), expected, "for {content:?}");
            }
        }

        #[test]
        fn escaped_span_does_not_hide_a_following_span() {
            assert_eq!(plain_expressions(r"\{{ a }} and {{ b }}"), vec!["b"]);
        }

        #[test]
        fn unrelated_backslash_does_not_suppress_opener() {
            assert_eq!(plain_expressions(r"path\to {{ x }}"), vec!["x"]);
            assert_eq!(plain_expressions(r"\ {{ x }}"), vec!["x"]);
        }

        #[test]
        fn escaped_literal_opener_is_neither_literal_nor_expression() {
            let result = ExpressionFinder::scan_plain(r"\{{{ x }}}");
            assert!(result.expressions.is_empty());
            assert!(result.literals.is_empty());
        }

        #[test]
        fn unescaped_literal_opener_is_unchanged() {
            let result = ExpressionFinder::scan_plain(r"\\{{{ x }}}");
            assert!(result.expressions.is_empty());
            assert_eq!(result.literals.len(), 1);
            assert_eq!(result.literals[0].content, " x ");
        }

        #[test]
        fn escaped_unclosed_opener_finds_nothing() {
            assert!(plain_expressions(r"\{{ x").is_empty());
        }

        #[test]
        fn escaped_opener_inside_fenced_code_is_still_skipped() {
            let content = "```\n\\{{ x }}\n{{ y }}\n```\n";
            assert!(ExpressionFinder::new(content).find_all().is_empty());
        }

        #[test]
        fn escaped_inner_opener_does_not_change_depth_counting() {
            // Inside an open span the backslash belongs to expression text; the
            // brace-depth counter still pairs the inner `{{` with its `}}`.
            assert_eq!(
                plain_expressions(r"{{ 'a \{{ b }}' }}"),
                vec![r"'a \{{ b }}'"]
            );
        }
    }
}
