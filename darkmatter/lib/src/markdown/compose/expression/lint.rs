//! Source-aware expression lints.
//!
//! The one lint today is a `{{ … }}` span authored inside a quoted string
//! literal. On a surface evaluated exactly once the span survives as raw text;
//! on a rescanning surface it resolves on the next pass. The lint only
//! describes syntax — callers decide whether the surface they own is
//! single-pass. Design: `claudine/fixes/2026-09-13-better-static-analysis/spec.md` (D1).
//! Literal escape parity is classified after expression-string decoding, with
//! findings mapped back to authored byte ranges.

use std::collections::HashMap;

use super::ast::{BinaryOp, Expr, SpannedExpr, SpannedExprKind};
use super::lexer::{ExpressionFinder, ExpressionLocation, Lexer, ParseMode, Token};
use super::parser::{parse_condition_spanned, parse_spanned};
use crate::markdown::span::SourceSpan;

/// A syntactic defect found in authored expression source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionLint {
    /// What was found.
    pub kind: ExpressionLintKind,
    /// Byte span of the nested `{{ … }}` inside the expression source.
    pub span: SourceSpan,
    /// The complete corrected expression — bare text, no `{{ }}` wrapper and
    /// no YAML quoting. Every lint from one expression carries the same
    /// rewrite, which repairs all of them at once. `None` when no rewrite can
    /// be proven equivalent to the author's intent.
    pub suggestion: Option<String>,
}

/// The kind of defect an [`ExpressionLint`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionLintKind {
    /// A `{{ … }}` span inside an authored string literal.
    NestedSpanInStringLiteral {
        /// Byte span of the whole literal, quotes included.
        literal: SourceSpan,
        /// The nested span's trimmed expression text, as authored.
        nested: String,
    },
}

/// Parses `source` in `mode` and lints it. A parse failure yields no lints;
/// malformed-expression diagnostics own that path.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::{lint_expression, ParseMode};
///
/// let lints = lint_expression(r#"ok ? "done" : "failed in {{ repo }}""#, ParseMode::Interpolation);
/// assert_eq!(lints.len(), 1);
/// assert_eq!(
///     lints[0].suggestion.as_deref(),
///     Some(r#"ok ? "done" : "failed in " + repo"#)
/// );
/// ```
pub fn lint_expression(source: &str, mode: ParseMode) -> Vec<ExpressionLint> {
    let parsed = match mode {
        ParseMode::Interpolation => parse_spanned(source),
        ParseMode::Condition => parse_condition_spanned(source),
    };
    match parsed {
        Ok(expr) => lint_with_modes(source, &expr, &[mode]),
        Err(_) => Vec::new(),
    }
}

/// Lints an already-parsed expression whose spans index into `source`.
///
/// The dialect `expr` was parsed in is recovered by re-parsing `source`; when
/// both dialects produce `expr`, a suggestion must be valid in both.
pub fn lint_spanned(source: &str, expr: &SpannedExpr) -> Vec<ExpressionLint> {
    let modes: Vec<ParseMode> = [ParseMode::Interpolation, ParseMode::Condition]
        .into_iter()
        .filter(|mode| {
            let reparsed = match mode {
                ParseMode::Interpolation => parse_spanned(source),
                ParseMode::Condition => parse_condition_spanned(source),
            };
            reparsed.is_ok_and(|reparsed| &reparsed == expr)
        })
        .collect();
    lint_with_modes(source, expr, &modes)
}

/// True when the trimmed text is exactly one `{{ … }}` span: the shape
/// `interpolate_value` routes to its typed whole-value branch. This classifies
/// syntax only; it does not claim how many times an owning pipeline evaluates
/// the value.
pub fn is_whole_value_span(text: &str) -> bool {
    whole_value_span(text).is_some()
}

/// The single span when `input`'s trimmed content is exactly one `{{ expr }}`.
///
/// Detection is independent of parse outcome, so a malformed whole value is
/// still held to the strict whole-value contract.
pub(crate) fn whole_value_span(input: &str) -> Option<ExpressionLocation> {
    let mut locations = ExpressionFinder::find_all_plain(input);
    if locations.len() != 1 {
        return None;
    }
    let loc = locations.remove(0);
    if !input[..loc.start].trim().is_empty() || !input[loc.end..].trim().is_empty() {
        return None;
    }
    Some(loc)
}

/// A literal holding at least one nested span, with the context the rewrite
/// needs to splice a `+` chain in its place.
struct FlaggedLiteral {
    span: SourceSpan,
    nested: Vec<ExpressionLocation>,
    /// The parent binds tighter than `+`, so a multi-part chain needs parens.
    tight_parent: bool,
    /// Object keys must stay literals; no rewrite exists.
    is_object_key: bool,
}

fn lint_with_modes(source: &str, expr: &SpannedExpr, modes: &[ParseMode]) -> Vec<ExpressionLint> {
    let mut flagged = Vec::new();
    collect_flagged(source, expr, false, &mut flagged);
    if flagged.is_empty() {
        return Vec::new();
    }

    let suggestion = if modes.is_empty() {
        None
    } else {
        rewrite(source, expr, &flagged, modes)
    };

    flagged
        .iter()
        .flat_map(|literal| {
            let inner_start = literal.span.start + 1;
            literal.nested.iter().map(move |loc| (literal, inner_start, loc))
        })
        .map(|(literal, inner_start, loc)| ExpressionLint {
            kind: ExpressionLintKind::NestedSpanInStringLiteral {
                literal: literal.span.clone(),
                nested: loc.expression.clone(),
            },
            span: inner_start + loc.start..inner_start + loc.end,
            suggestion: suggestion.clone(),
        })
        .collect()
}

fn collect_flagged(
    source: &str,
    expr: &SpannedExpr,
    tight_parent: bool,
    out: &mut Vec<FlaggedLiteral>,
) {
    match &expr.kind {
        SpannedExprKind::StringLiteral(value) => {
            flag_literal(source, &expr.span, value, tight_parent, false, out);
        }
        SpannedExprKind::Variable(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => {
            for item in items {
                collect_flagged(source, item, false, out);
            }
        }
        SpannedExprKind::ObjectLiteral(entries) => {
            for (key, value) in entries {
                flag_literal(source, &key.span, &key.value, false, true, out);
                collect_flagged(source, value, false, out);
            }
        }
        SpannedExprKind::UnaryNot(inner) | SpannedExprKind::UnaryMinus(inner) => {
            collect_flagged(source, inner, true, out);
        }
        SpannedExprKind::Paren(inner) => collect_flagged(source, inner, false, out),
        SpannedExprKind::Binary { left, right, .. } => {
            collect_flagged(source, left, true, out);
            collect_flagged(source, right, true, out);
        }
        SpannedExprKind::Index { base, index } => {
            collect_flagged(source, base, true, out);
            collect_flagged(source, index, false, out);
        }
        SpannedExprKind::MemberAccess { base, .. } => collect_flagged(source, base, true, out),
        SpannedExprKind::Fallback { primary, fallback } => {
            collect_flagged(source, primary, false, out);
            collect_flagged(source, fallback, false, out);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_flagged(source, condition, false, out);
            collect_flagged(source, then_branch, false, out);
            collect_flagged(source, else_branch, false, out);
        }
        SpannedExprKind::Comparison { left, right, .. } => {
            collect_flagged(source, left, false, out);
            collect_flagged(source, right, false, out);
        }
        SpannedExprKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_flagged(source, arg, false, out);
            }
        }
    }
}

fn flag_literal(
    source: &str,
    span: &SourceSpan,
    value: &str,
    tight_parent: bool,
    is_object_key: bool,
    out: &mut Vec<FlaggedLiteral>,
) {
    // Bare identifier object keys carry no quotes and cannot hold a span.
    let Some((inner, decoded_to_authored)) = decoded_literal_inner(source, span, value) else {
        return;
    };
    let nested = ExpressionFinder::find_all_plain(value)
        .into_iter()
        .filter_map(|location| {
            let start = *decoded_to_authored.get(location.start)?;
            let end = *decoded_to_authored.get(location.end)?;
            let expression = inner.get(start + 2..end.checked_sub(2)?)?.trim().to_string();
            Some(ExpressionLocation { start, end, expression })
        })
        .collect::<Vec<_>>();
    if nested.is_empty() {
        return;
    }
    out.push(FlaggedLiteral {
        span: span.clone(),
        nested,
        tight_parent,
        is_object_key,
    });
}

/// Decodes a quoted literal while retaining every decoded byte boundary's
/// authored offset. Escape parity belongs to the value the expression lexer
/// produces, while diagnostics and rewrites must continue to select authored
/// bytes.
fn decoded_literal_inner<'a>(
    source: &'a str,
    span: &SourceSpan,
    expected: &str,
) -> Option<(&'a str, Vec<usize>)> {
    let token = source.get(span.clone())?;
    let quote = token.chars().next()?;
    if !matches!(quote, '"' | '\'') || token.len() < 2 || !token.ends_with(quote) {
        return None;
    }
    let inner = &token[1..token.len() - 1];
    let mut decoded = String::with_capacity(inner.len());
    let mut boundaries = vec![0];
    let mut chars = inner.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            boundaries.extend((1..=ch.len_utf8()).map(|offset| start + offset));
            continue;
        }

        let (escaped_start, escaped) = chars.next()?;
        let escaped_end = escaped_start + escaped.len_utf8();
        let replacement = match escaped {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '\\' => Some('\\'),
            ch if ch == quote => Some(ch),
            _ => None,
        };
        if let Some(replacement) = replacement {
            decoded.push(replacement);
            boundaries.push(escaped_end);
        } else {
            decoded.push('\\');
            boundaries.push(escaped_start);
            decoded.push(escaped);
            boundaries.extend(
                (1..=escaped.len_utf8()).map(|offset| escaped_start + offset),
            );
        }
    }
    (decoded == expected).then_some((inner, boundaries))
}

/// Builds the corrected expression and proves it: the rewrite must re-parse,
/// in every candidate dialect, to exactly the tree the author meant. A re-parse
/// that merely succeeds is not enough — a mis-spliced ternary parses cleanly.
fn rewrite(
    source: &str,
    expr: &SpannedExpr,
    flagged: &[FlaggedLiteral],
    modes: &[ParseMode],
) -> Option<String> {
    let mut text_replacements = Vec::with_capacity(flagged.len());
    let mut tree_replacements = HashMap::with_capacity(flagged.len());
    for literal in flagged {
        if literal.is_object_key {
            return None;
        }
        let (text, tree) = rewrite_literal(source, literal)?;
        text_replacements.push((literal.span.clone(), text));
        tree_replacements.insert(literal.span.start, tree);
    }

    let mut rewritten = source.to_string();
    for (span, text) in text_replacements.iter().rev() {
        rewritten.replace_range(span.clone(), text);
    }

    let expected = substitute(expr, &tree_replacements);
    modes
        .iter()
        .all(|mode| {
            let reparsed = match mode {
                ParseMode::Interpolation => parse_spanned(&rewritten),
                ParseMode::Condition => parse_condition_spanned(&rewritten),
            };
            // A lifted span can itself hold a nested literal; a suggestion
            // that leaves a defect behind is not offered.
            reparsed.is_ok_and(|reparsed| {
                reparsed.erase() == expected
                    && lint_with_modes(&rewritten, &reparsed, &[]).is_empty()
            })
        })
        .then_some(rewritten)
}

/// Rewrites one literal as a `+` chain, returning its source text and the tree
/// that text must parse to.
fn rewrite_literal(source: &str, literal: &FlaggedLiteral) -> Option<(String, Expr)> {
    let token = &source[literal.span.clone()];
    let quote = token.chars().next()?;
    let inner = &token[1..token.len() - 1];
    // A `{{{ … }}}` literal escape is converted only by a rescanning pass's
    // final cleanup; a `+` chain would emit its braces verbatim.
    if !ExpressionFinder::scan_plain(inner).literals.is_empty() {
        return None;
    }

    let mut parts: Vec<(String, Expr)> = Vec::new();
    // Once the accumulated string holds a character no number can contain, a
    // following `+` can never take the numeric branch of `evaluate_binary`.
    let mut accumulator_is_non_numeric = false;
    let mut cursor = 0;

    let push_piece = |raw: &str, parts: &mut Vec<(String, Expr)>, flag: &mut bool| {
        if raw.is_empty() {
            return Some(());
        }
        let decoded = decode_literal_text(raw, quote)?;
        if decoded.chars().any(|ch| !is_numeric_text_char(ch)) {
            *flag = true;
        }
        parts.push((format!("{quote}{raw}{quote}"), Expr::StringLiteral(decoded)));
        Some(())
    };

    for loc in &literal.nested {
        push_piece(&inner[cursor..loc.start], &mut parts, &mut accumulator_is_non_numeric)?;

        // The lifted span leaves the literal, so it is decoded exactly as the
        // lexer would have decoded it; it is evaluated with the interpolation
        // grammar a rescanning pass would use.
        let lifted_source = decode_literal_text(&loc.expression, quote)?;
        let lifted = parse_spanned(&lifted_source).ok()?.erase();
        let (mut text, mut tree) = if is_atomic(&lifted) {
            (lifted_source, lifted)
        } else {
            (format!("({lifted_source})"), Expr::Paren(Box::new(lifted)))
        };
        if !accumulator_is_non_numeric {
            text = format!("({quote}{quote} + {text})");
            tree = Expr::Paren(Box::new(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::StringLiteral(String::new())),
                right: Box::new(tree),
            }));
        }
        parts.push((text, tree));
        cursor = loc.end;
    }
    push_piece(&inner[cursor..], &mut parts, &mut accumulator_is_non_numeric)?;

    let mut parts = parts.into_iter();
    let (mut text, mut tree) = parts.next()?;
    let mut part_count = 1;
    for (part_text, part_tree) in parts {
        text = format!("{text} + {part_text}");
        tree = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(tree),
            right: Box::new(part_tree),
        };
        part_count += 1;
    }
    if literal.tight_parent && part_count > 1 {
        text = format!("({text})");
        tree = Expr::Paren(Box::new(tree));
    }
    Some((text, tree))
}

/// Applies the lexer's own string decoding to `raw` literal text.
fn decode_literal_text(raw: &str, quote: char) -> Option<String> {
    let quoted = format!("{quote}{raw}{quote}");
    let mut lexer = Lexer::new(&quoted);
    match (lexer.next_token(), lexer.next_token()) {
        (Ok(Token::StringLiteral(value)), Ok(Token::Eof)) => Some(value),
        _ => None,
    }
}

/// Characters that can appear in text Rust parses as an `f64`, including
/// `inf`, `infinity`, and `nan` in any case.
fn is_numeric_text_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch.to_ascii_lowercase(), '.' | '+' | '-' | 'e' | 'i' | 'n' | 'f' | 't' | 'y' | 'a')
}

fn is_atomic(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Variable(_)
            | Expr::MemberAccess { .. }
            | Expr::Index { .. }
            | Expr::FunctionCall { .. }
            | Expr::StringLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BoolLiteral(_)
            | Expr::ArrayLiteral(_)
            | Expr::ObjectLiteral(_)
            | Expr::Paren(_)
    )
}

/// Erases `expr`, replacing each literal keyed by its span start.
fn substitute(expr: &SpannedExpr, replacements: &HashMap<usize, Expr>) -> Expr {
    if let SpannedExprKind::StringLiteral(_) = expr.kind
        && let Some(replacement) = replacements.get(&expr.span.start)
    {
        return replacement.clone();
    }
    let sub = |child: &SpannedExpr| Box::new(substitute(child, replacements));
    match &expr.kind {
        SpannedExprKind::ArrayLiteral(items) => Expr::ArrayLiteral(
            items.iter().map(|item| substitute(item, replacements)).collect(),
        ),
        SpannedExprKind::ObjectLiteral(entries) => Expr::ObjectLiteral(
            entries
                .iter()
                .map(|(key, value)| (key.value.clone(), substitute(value, replacements)))
                .collect(),
        ),
        SpannedExprKind::UnaryNot(inner) => Expr::UnaryNot(sub(inner)),
        SpannedExprKind::UnaryMinus(inner) => Expr::UnaryMinus(sub(inner)),
        SpannedExprKind::Paren(inner) => Expr::Paren(sub(inner)),
        SpannedExprKind::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: sub(left),
            right: sub(right),
        },
        SpannedExprKind::Index { base, index } => Expr::Index {
            base: sub(base),
            index: sub(index),
        },
        SpannedExprKind::MemberAccess { base, name } => Expr::MemberAccess {
            base: sub(base),
            name: name.clone(),
        },
        SpannedExprKind::Fallback { primary, fallback } => Expr::Fallback {
            primary: sub(primary),
            fallback: sub(fallback),
        },
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => Expr::Ternary {
            condition: sub(condition),
            then_branch: sub(then_branch),
            else_branch: sub(else_branch),
        },
        SpannedExprKind::Comparison { left, op, right } => Expr::Comparison {
            left: sub(left),
            op: *op,
            right: sub(right),
        },
        SpannedExprKind::FunctionCall { name, args } => Expr::FunctionCall {
            name: name.clone(),
            args: args.iter().map(|arg| substitute(arg, replacements)).collect(),
        },
        SpannedExprKind::Variable(_)
        | SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => expr.erase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::compose::context::effective_state::{EffectiveState, EffectiveStateBuilder};
    use crate::markdown::compose::interpolation::{
        EvalResult, Evaluator, ScanMode, interpolate_text,
    };
    use crate::markdown::compose::ComposeContext;
    use crate::markdown::compose::expression::{evaluate, parse};
    use serde_json::{Value, json};

    const REVIEW_SPEC_INLINE: &str = include_str!(
        "../../../../../../claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md"
    );
    const COMMIT: &str = include_str!(
        "../../../../../../claudine/cli/tests/fixtures/nested_span_regression/commit.md"
    );

    fn state(data: Value) -> EffectiveState {
        let Value::Object(map) = data else {
            panic!("state data must be an object");
        };
        EffectiveStateBuilder::new()
            .with_frontmatter(map.into_iter().collect())
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    fn lint(source: &str) -> Vec<ExpressionLint> {
        lint_expression(source, ParseMode::Interpolation)
    }

    fn suggestion(source: &str) -> String {
        let lints = lint(source);
        assert!(!lints.is_empty(), "expected a lint for {source}");
        lints[0]
            .suggestion
            .clone()
            .unwrap_or_else(|| panic!("expected a suggestion for {source}"))
    }

    /// What the author meant: the literal rescanned by a body interpolation.
    fn rescanned(source: &str, data: &Value) -> String {
        let state = state(data.clone());
        let evaluator = Evaluator::new(&state);
        interpolate_text(&format!("{{{{ {source} }}}}"), &evaluator, ScanMode::Plain, true, "test")
            .unwrap()
            .output
    }

    /// The rewrite evaluated exactly once, as a single-pass surface does.
    fn single_pass(source: &str, data: &Value) -> String {
        let state = state(data.clone());
        let evaluator = Evaluator::new(&state);
        match evaluator.eval(&parse(source).unwrap()) {
            EvalResult::Value(value) => value,
            EvalResult::Error { error, .. } => panic!("rewrite failed to evaluate: {error}"),
        }
    }

    fn assert_rewrite_preserves_output(source: &str, data: Value, expected: &str) {
        let rewrite = suggestion(source);
        assert_eq!(rescanned(source, &data), expected, "author intent for {source}");
        assert_eq!(single_pass(&rewrite, &data), expected, "rewrite {rewrite} of {source}");
    }

    /// The whole-value expression text of `event.say` in fixture frontmatter.
    fn fixture_say(fixture: &str, event: &str, key: &str) -> String {
        let markdown: Markdown = fixture.into();
        let map = markdown.frontmatter().as_map();
        let value = map.get(event).unwrap_or_else(|| panic!("fixture has {event}"));
        let text = if key.is_empty() { value } else { &value[key] };
        let text = text.as_str().expect("fixture value is a string");
        whole_value_span(text).expect("whole-value span").expression
    }

    mod lint_model {
        use super::*;

        #[test]
        fn fires_once_per_nested_span_in_both_quote_styles() {
            for source in [r#""a {{ x }} b {{ y }}""#, r#"'a {{ x }} b {{ y }}'"#] {
                let lints = lint(source);
                assert_eq!(lints.len(), 2, "{source}");
                assert_eq!(&source[lints[0].span.clone()], "{{ x }}");
                assert_eq!(&source[lints[1].span.clone()], "{{ y }}");
                for lint in &lints {
                    let ExpressionLintKind::NestedSpanInStringLiteral { literal, .. } = &lint.kind;
                    assert_eq!(literal, &(0..source.len()));
                }
            }
        }

        #[test]
        fn fires_in_both_parse_modes() {
            let source = r#"a || "in {{ x }}""#;
            assert_eq!(lint_expression(source, ParseMode::Interpolation).len(), 1);
            assert_eq!(lint_expression(source, ParseMode::Condition).len(), 1);
        }

        #[test]
        fn fires_in_every_literal_position() {
            for (source, nested) in [
                (r#"c ? "t {{ a }}" : "e {{ b }}""#, vec!["a", "b"]),
                (r#"length("n {{ a }}")"#, vec!["a"]),
                (r#"["i {{ a }}", "plain"]"#, vec!["a"]),
                (r#"{"k {{ a }}": "v {{ b }}"}"#, vec!["a", "b"]),
            ] {
                let found: Vec<String> = lint(source)
                    .into_iter()
                    .map(|lint| match lint.kind {
                        ExpressionLintKind::NestedSpanInStringLiteral { nested, .. } => nested,
                    })
                    .collect();
                assert_eq!(found, nested, "{source}");
            }
        }

        #[test]
        fn quiet_without_a_nested_span() {
            for source in [
                r#""a " + b + " c""#,
                r#""keep {{{ x }}} literal""#,
                r#""lone {{ opener""#,
                r#""escaped \{{ x }}""#,
                "a && b ? c : d",
                "{k: v}",
            ] {
                assert!(lint(source).is_empty(), "{source}");
            }
        }

        #[test]
        fn escaped_opener_parity_is_classified_after_literal_decoding() {
            for (authored_backslashes, flagged) in [
                (1, false),
                (2, false),
                (3, true),
                (4, true),
                (5, false),
                (6, false),
            ] {
                let source = format!("'{}{{{{ name }}}}'", "\\".repeat(authored_backslashes));
                let lints = lint(&source);
                assert_eq!(!lints.is_empty(), flagged, "{source}");
                for found in lints {
                    assert_eq!(&source[found.span.clone()], "{{ name }}", "{source}");
                }
            }
        }

        #[test]
        fn parse_failure_yields_no_lints() {
            assert!(lint(r#""a {{ x }}" +"#).is_empty());
        }

        #[test]
        fn lint_spanned_agrees_with_lint_expression() {
            for source in [
                r#"c ? "t {{ a }}" : 'e {{ b + 1 }}'"#,
                r#"a || "x {{ y || 'd' }}""#,
            ] {
                let expr = parse_spanned(source).unwrap();
                assert_eq!(lint_spanned(source, &expr), lint(source), "{source}");
            }
        }

        #[test]
        fn is_whole_value_span_classifies_exact_span_shape() {
            assert!(is_whole_value_span("{{ x }}"));
            assert!(is_whole_value_span("  {{ x ? 'a' : 'b' }}\n"));
            assert!(is_whole_value_span("{{ not ( valid }}"));
            assert!(!is_whole_value_span("a {{ x }}"));
            assert!(!is_whole_value_span("{{ x }}{{ y }}"));
            assert!(!is_whole_value_span("{{{ x }}}"));
            assert!(!is_whole_value_span(r"\{{ x }}"));
            assert!(!is_whole_value_span("plain"));
        }
    }

    mod rewrite_generator {
        use super::*;

        #[test]
        fn incident_success_branch_rewrites_exactly() {
            let source = fixture_say(REVIEW_SPEC_INLINE, "success", "say");
            let lints = lint(&source);
            assert_eq!(lints.len(), 2);
            let rewrite = lints[0].suggestion.clone().expect("incident suggestion");
            let expected = source
                .replace(
                    r#""The review of the draft specification file in {{ctx.area}} has completed""#,
                    r#""The review of the draft specification file in " + ctx.area + " has completed""#,
                )
                .replace(
                    r#""The review of the draft specification file in the {{ctx.repo_name}} repo has completed""#,
                    r#""The review of the draft specification file in the " + ctx.repo_name + " repo has completed""#,
                );
            assert_eq!(rewrite, expected);
            assert!(lints.iter().all(|lint| lint.suggestion.as_deref() == Some(expected.as_str())));
            let spoken = single_pass(&expected, &json!({}));
            assert_eq!(spoken, rescanned(&source, &json!({})));
            assert!(!spoken.contains("{{"), "{spoken}");
        }

        #[test]
        fn incident_failure_branch_lifts_function_calls() {
            let source = fixture_say(REVIEW_SPEC_INLINE, "failure", "say");
            let rewrite = suggestion(&source);
            assert!(rewrite.contains(
                r#""The inline review " + title_case(without_date(parent_dir(spec))) + " in the " + ctx.repo_name + " repo failed to complete!""#
            ));
        }

        #[test]
        fn commit_fixture_keeps_single_quotes_raw_newline_and_offers_array_rewrite() {
            let source = fixture_say(COMMIT, "resides_in", "");
            let rewrite = suggestion(&source);
            assert!(rewrite.contains(r"'are all part of the ' + ctx.dirty_package_areas + ' package area'"));
            assert!(rewrite.contains(
                r"'are spread across ' + length(ctx.dirty_package_areas) + ':\n ' + as_unordered_list(ctx.dirty_package_areas)"
            ));
            assert!(!rewrite.contains('"'));
        }

        #[test]
        fn rewrite_tree_equals_the_intended_tree() {
            let rewrite = suggestion(r#""a {{ x ? y : z }} b""#);
            assert_eq!(rewrite, r#""a " + (x ? y : z) + " b""#);
            let intended = Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::StringLiteral("a ".into())),
                    right: Box::new(Expr::Paren(Box::new(parse("x ? y : z").unwrap()))),
                }),
                right: Box::new(Expr::StringLiteral(" b".into())),
            };
            assert_eq!(parse(&rewrite).unwrap(), intended);
        }

        #[test]
        fn ternary_span_is_parenthesized() {
            assert_rewrite_preserves_output(
                r#""a {{ x ? y : z }} b""#,
                json!({"x": false, "y": "Y", "z": "Z"}),
                "a Z b",
            );
        }

        #[test]
        fn additive_span_stays_numeric() {
            assert_rewrite_preserves_output(r#""a {{ x + y }}""#, json!({"x": 1, "y": 2}), "a 3");
        }

        #[test]
        fn adjacent_numeric_spans_concatenate() {
            assert_rewrite_preserves_output(r#""{{a}}{{b}}""#, json!({"a": 1, "b": 2}), "12");
            assert_rewrite_preserves_output(r#""{{a}}5""#, json!({"a": 1}), "15");
            assert_rewrite_preserves_output(r#""5{{a}}""#, json!({"a": 1}), "51");
            assert_rewrite_preserves_output(r#""1e{{a}}{{b}}""#, json!({"a": 1, "b": 2}), "1e12");
        }

        #[test]
        fn span_only_literal_stays_a_string() {
            let rewrite = suggestion(r#""{{x}}""#);
            assert_eq!(rewrite, r#"("" + x)"#);
            let lookup = state(json!({"x": 7}));
            assert_eq!(evaluate(&parse(&rewrite).unwrap(), &lookup).unwrap(), json!("7"));
        }

        #[test]
        fn escaped_quotes_decode_in_the_lifted_span_only() {
            let source = r#""a \"{{ length(\"xyz\") }}\" b""#;
            let rewrite = suggestion(source);
            assert_eq!(rewrite, r#""a \"" + length("xyz") + "\" b""#);
            assert_rewrite_preserves_output(source, json!({}), "a \"3\" b");
        }

        #[test]
        fn single_quotes_and_raw_escapes_are_preserved() {
            let source = r"'x {{a}}:\n {{b}}'";
            assert_eq!(suggestion(source), r"'x ' + a + ':\n ' + b");
            assert_rewrite_preserves_output(source, json!({"a": 1, "b": 2}), "x 1:\n 2");
        }

        #[test]
        fn multiple_literals_are_repaired_together() {
            assert_rewrite_preserves_output(
                r#"c ? "yes {{ a }}" : "no {{ b }}""#,
                json!({"c": true, "a": "A", "b": "B"}),
                "yes A",
            );
        }

        #[test]
        fn aggregate_values_render_as_json() {
            assert_rewrite_preserves_output(
                r#""items: {{ list }}""#,
                json!({"list": ["a", "b"]}),
                r#"items: ["a","b"]"#,
            );
            assert_rewrite_preserves_output(
                r#""obj: {{ o }}""#,
                json!({"o": {"k": 1}}),
                r#"obj: {"k":1}"#,
            );
        }

        #[test]
        fn tight_parent_wraps_the_chain() {
            let rewrite = suggestion(r#"!"a {{ x }}""#);
            assert_eq!(rewrite, r#"!("a " + x)"#);
        }

        #[test]
        fn malformed_nested_span_has_no_suggestion() {
            let lints = lint(r#""a {{ x ( }}""#);
            assert_eq!(lints.len(), 1);
            assert_eq!(lints[0].suggestion, None);
        }

        #[test]
        fn object_key_literal_has_no_suggestion() {
            let lints = lint(r#"{"k {{ a }}": 1}"#);
            assert_eq!(lints.len(), 1);
            assert_eq!(lints[0].suggestion, None);
        }

        #[test]
        fn fallback_span_in_condition_dialect_has_no_suggestion() {
            // Lifting `y || 'd'` into condition grammar would turn a fallback
            // into a boolean OR.
            let source = r#"a == "x {{ y || 'd' }}""#;
            assert!(lint_expression(source, ParseMode::Interpolation)[0].suggestion.is_some());
            assert_eq!(lint_expression(source, ParseMode::Condition)[0].suggestion, None);
        }

        #[test]
        fn doubly_nested_literal_has_no_suggestion() {
            let lints = lint(r#""a {{ 'b {{ c }}' }}""#);
            assert_eq!(lints.len(), 1);
            assert_eq!(lints[0].suggestion, None);
        }

        #[test]
        fn literal_escape_alongside_span_has_no_suggestion() {
            let lints = lint(r#""{{{ raw }}} and {{ x }}""#);
            assert_eq!(lints.len(), 1);
            assert_eq!(lints[0].suggestion, None);
        }

        #[test]
        fn suggestion_reparses_in_the_original_dialect() {
            let source = r#"a || "x {{ y }}""#;
            let rewrite = lint_expression(source, ParseMode::Condition)[0]
                .suggestion
                .clone()
                .unwrap();
            assert_eq!(rewrite, r#"a || "x " + y"#);
            assert!(parse_condition_spanned(&rewrite).is_ok());
        }
    }
    mod authority {
        use super::*;
        use crate::markdown::compose::expression::lex_spanned;
        use crate::markdown::compose::interpolation::interpolate_value;
        use proptest::prelude::*;
        use std::collections::BTreeSet;

        fn literal() -> impl Strategy<Value = String> {
            let fragment = prop_oneof![
                Just("a"), Just(" "), Just("{{ x }}"), Just("{{"), Just("}}"),
                Just("\\\\"), Just("\\"), Just("{{{ x }}}"), Just("{"),
            ];
            (prop_oneof![Just('"'), Just('\'')], proptest::collection::vec(fragment, 0..5))
                .prop_map(|(quote, content)| format!("{quote}{}{quote}", content.concat()))
        }

        fn expression_source() -> impl Strategy<Value = String> {
            (literal(), literal(), 0..9usize).prop_map(|(a, b, shape)| match shape {
                0 => a,
                1 => format!("c ? {a} : {b}"),
                2 => format!("f({a}, {b})"),
                3 => format!("[{a}, {b}]"),
                4 => format!("{{{a}: {b}}}"),
                5 => format!("{a} + {b}"),
                6 => format!("m[{a}] == -{b}"),
                7 => format!("({a})[{b}]"),
                _ => format!("!{a} || {b}"),
            })
        }

        proptest! {
            /// Invariant 2, first half: the literals the lint flags are exactly
            /// the lexer's decoded string tokens in which `find_all_plain`
            /// finds a span, independent of the lint's own tree walk.
            #[test]
            fn flagged_literals_equal_scanner_hits(
                source in expression_source(),
                condition in any::<bool>(),
            ) {
                let mode = if condition { ParseMode::Condition } else { ParseMode::Interpolation };
                let parsed = match mode {
                    ParseMode::Interpolation => parse_spanned(&source),
                    ParseMode::Condition => parse_condition_spanned(&source),
                };
                prop_assume!(parsed.is_ok());

                let expected: BTreeSet<(usize, usize, usize)> = lex_spanned(&source, mode)
                    .unwrap()
                    .into_iter()
                    .flat_map(|token| {
                        let Token::StringLiteral(value) = token.value else {
                            return Vec::new().into_iter();
                        };
                        let Some((_, boundaries)) =
                            decoded_literal_inner(&source, &token.span, &value)
                        else {
                            return Vec::new().into_iter();
                        };
                        ExpressionFinder::find_all_plain(&value)
                            .into_iter()
                            .filter_map(move |loc| {
                                Some((
                                    token.span.start,
                                    token.span.end,
                                    token.span.start + 1 + *boundaries.get(loc.start)?,
                                ))
                            })
                            .collect::<Vec<_>>()
                            .into_iter()
                    })
                    .collect();
                let flagged: BTreeSet<(usize, usize, usize)> = lint_expression(&source, mode)
                    .into_iter()
                    .map(|lint| {
                        let ExpressionLintKind::NestedSpanInStringLiteral { literal, .. } = lint.kind;
                        (literal.start, literal.end, lint.span.start)
                    })
                    .collect();
                prop_assert_eq!(flagged, expected);
            }

            /// Invariant 2, second half: `is_whole_value_span` predicts the
            /// branch `interpolate_value` takes. The typed branch yields a
            /// non-string value or a hard error; the text branch yields a string.
            #[test]
            fn whole_value_classifier_matches_interpolate_value_branch(
                pieces in proptest::collection::vec(
                    prop_oneof![
                        Just("{{ 1 }}"), Just("{{ ( }}"), Just("{{{ 1 }}}"), Just(" "),
                        Just("\n"), Just("a"), Just("\\"), Just("{{"), Just("}}"),
                    ],
                    0..5,
                ),
            ) {
                let input = pieces.concat();
                let state = state(json!({}));
                let evaluator = Evaluator::new(&state);
                let took_whole_value_branch = match interpolate_value(&input, &evaluator, false, "test") {
                    Ok((value, _, _)) => !value.is_string(),
                    Err(_) => true,
                };
                prop_assert_eq!(is_whole_value_span(&input), took_whole_value_branch, "{:?}", input);
            }
        }

        #[test]
        fn rescanning_body_still_resolves_what_the_lint_describes() {
            let expression = "pkg ? 'in {{pkg}}' : 'x'";
            assert_eq!(lint(expression).len(), 1, "the lint describes the syntax");

            let markdown: Markdown =
                format!("---\npkg: darkmatter\n---\n{{{{ {expression} }}}}\n").into();
            let (composed, report) = markdown.compose().unwrap();
            assert_eq!(composed.content().trim(), "in darkmatter");
            assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        }
    }
}
