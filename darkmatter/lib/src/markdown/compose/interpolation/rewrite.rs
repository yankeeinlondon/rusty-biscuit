//! Shared string rewrite helper for interpolation.
//!
//! Provides `interpolate_text`, which scans a string for `{{ }}` expressions,
//! evaluates them against an [`EvaluationLookup`] implementation, and
//! returns the rewritten string. Supports both markdown-aware scanning
//! (skipping code regions) and plain-text scanning.
//!
//! Every scan is single-pass: text an expression or literal produces is data
//! and is never scanned again.

use super::{EvalResult, Evaluator, ExpressionFinder, ExpressionLocation, parse};
use crate::markdown::compose::body_origin::{DataRanges, TextEdit};
use crate::markdown::compose::expression::InterpolationLiteral;
use crate::markdown::compose::parse_utils::structural_view;
use crate::markdown::compose::expression::lint::whole_value_span;
use crate::markdown::compose::expression::{EvaluationLookup, ExpressionError};
use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::context::report::ExpressionOrigin;
use crate::markdown::literal_token::{TOKEN_PREFIX, TokenError};
use crate::markdown::types::{MarkdownError, SourceRef};
use serde_json::Value;

/// Wraps a typed evaluation `cause` in [`MarkdownError::Interpolation`], the
/// single construction point for the live compose interpolation path.
///
/// The `key` is left `None` here (body interpolation has none); frontmatter
/// whole-value callers attach it afterward via `key_scoped_error`. The `source`
/// starts as [`SourceRef::Effective`] — the on-disk excerpt is layered on at the
/// pipeline boundary where the document's [`SourceContext`] is in scope.
///
/// [`SourceContext`]: biscuit_terminal::errors::SourceContext
fn interpolation_error(expression: &str, cause: ExpressionError) -> MarkdownError {
    MarkdownError::Interpolation {
        key: None,
        expression: expression.to_string(),
        source: Box::new(SourceRef::Effective {
            rendered: expression.to_string(),
            origin_key: None,
        }),
        cause: Box::new(cause),
    }
}

/// A located failure for the literal token (`{{!data:…}}`) at `span` in
/// `input`, which no scan accepts: composition decodes a token only as an
/// entire authored frontmatter value, before any scan.
fn literal_token_failure(
    input: &str,
    span: std::ops::Range<usize>,
    cause: TokenError,
) -> LocatedInterpolationError {
    LocatedInterpolationError {
        error: Box::new(interpolation_error(
            &input[span.clone()],
            ExpressionError::MalformedLiteralToken(cause),
        )),
        span: Some(span),
    }
}

/// An expression whose source holds a token spelling, say inside a string
/// literal, would return that spelling as text; N5 makes it malformed instead.
fn expression_holds_token(loc: &ExpressionLocation) -> Option<LocatedInterpolationError> {
    loc.expression.contains(TOKEN_PREFIX).then(|| LocatedInterpolationError {
        error: Box::new(interpolation_error(
            &loc.expression,
            ExpressionError::MalformedLiteralToken(TokenError::Embedded),
        )),
        span: Some(loc.start..loc.end),
    })
}

/// What a failing `{{ … }}` expression does to the text being rewritten.
///
/// A full-document composition always passes [`Strict`](Self::Strict), whatever
/// `ComposeOptions::fail_fast` says: an expression that cannot be parsed or
/// evaluated is an authoring error there. [`Lenient`](Self::Lenient) is for
/// best-effort callers, `compose_subtree(..., SubtreeStrictness::Lenient)` and
/// preflight command discovery, which must keep going past a bad span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpressionFailurePolicy {
    /// A parse or evaluation failure becomes a coded `ComposeWarning` and the
    /// failing `{{ … }}` stays in the output. Authoring-fatal causes
    /// (`ExpressionError::is_authoring_fatal`) still abort.
    Lenient,
    /// Every parse or evaluation failure aborts the rewrite.
    Strict,
}

/// Controls how `interpolate_text` scans for `{{ }}` expressions.
pub(crate) enum ScanMode {
    /// Skip expressions inside fenced and indented code blocks.
    ///
    /// Inline code spans (single backticks) are still scanned —
    /// the common templating pattern `` `var_{{ phase }}` `` is supported
    /// by default. Used by body interpolation.
    MarkdownAware,
    /// Scan the entire string with no exclusions.
    /// Used by frontmatter interpolation, and by body interpolation when
    /// `interpolate_code_blocks` is enabled.
    Plain,
}

/// Result of rewriting interpolation expressions in a string.
pub(crate) struct InterpolationRewrite {
    /// The rewritten output string.
    pub output: String,
    /// Number of expressions successfully replaced.
    pub replacements: usize,
    /// Warnings generated during rewrite (non-fatal issues).
    pub warnings: Vec<ComposeWarning>,
    /// One data edit per replaced expression and converted literal, in input
    /// order: the text each wrote is data and is never scanned again.
    pub edits: Vec<TextEdit>,
}

/// Converts `{{{ ... }}}` interpolation literals in `input` to the literal
/// text `{{ ... }}`.
///
/// Literals are recognized using the shared scanner and are converted
/// from end to start so byte offsets remain stable. The output is data: a
/// caller never scans it again.
pub(crate) fn convert_literals(input: &str, scan_mode: ScanMode) -> String {
    let scan = match scan_mode {
        ScanMode::MarkdownAware => ExpressionFinder::new(input).scan(),
        ScanMode::Plain => ExpressionFinder::scan_plain(input),
    };
    let mut output = input.to_string();
    for lit in scan.literals.iter().rev() {
        let replacement = format!("{}{}{}", "{{", lit.content, "}}");
        output.replace_range(lit.start..lit.end, &replacement);
    }
    output
}

/// A fatal [`interpolate_text_located`] failure plus where the failing
/// expression sits in the scanned `input`.
#[derive(Debug)]
pub(crate) struct LocatedInterpolationError {
    /// Boxed so the pair stays within clippy's `result_large_err` budget.
    pub error: Box<MarkdownError>,
    /// Byte range of the failing `{{ … }}` in the caller's `input`. `None`
    /// only for a failure that is not about one located expression.
    pub span: Option<std::ops::Range<usize>>,
}

/// Scans `input` for `{{ }}` expressions and `{{{ }}}` literals once,
/// evaluates the expressions, and returns the rewritten string.
///
/// The scan is single-pass: text an expression returns and text a literal
/// produces are data and are never scanned again, even when they contain a
/// valid expression or literal.
///
/// ## Arguments
///
/// - `input` — the text to scan
/// - `evaluator` — evaluates parsed expressions against state
/// - `scan_mode` — whether to respect code regions or scan everything
/// - `policy` — whether a parse/eval failure aborts or becomes a warning
/// - `warning_stage` — label attached to any warnings produced
pub(crate) fn interpolate_text<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, MarkdownError> {
    interpolate_text_located(input, evaluator, scan_mode, policy, warning_stage)
        .map_err(|failure| *failure.error)
}

/// [`interpolate_text`], but a fatal failure also reports the failing
/// expression's span in `input`.
pub(crate) fn interpolate_text_located<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, LocatedInterpolationError> {
    interpolate_text_in(input, None, evaluator, scan_mode, policy, warning_stage)
}

/// [`interpolate_text_located`] over text whose `data` bytes an earlier stage
/// inserted.
///
/// Expressions, literals, and code regions are found in the masked view (see
/// [`DataRanges::masked`]), so data neither contributes an expression nor
/// hides an authored one; a span that straddles authored text and data is not
/// an expression and stays as written.
pub(crate) fn interpolate_text_in<L: EvaluationLookup>(
    input: &str,
    data: Option<&DataRanges>,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, LocatedInterpolationError> {
    let strict = policy == ExpressionFailurePolicy::Strict;
    // Fast path (F14): a `{{ … }}` expression and a `{{{ … }}}` literal both
    // require the `{{` sequence. When the input contains none, the scan — the
    // MarkdownAware pulldown-cmark code-region parse in particular — is
    // provably a no-op. Skip it and return the input verbatim.
    if !input.contains("{{") {
        return Ok(InterpolationRewrite {
            output: input.to_string(),
            replacements: 0,
            warnings: Vec::new(),
            edits: Vec::new(),
        });
    }

    let view = structural_view(input, data);
    let scan = match scan_mode {
        ScanMode::MarkdownAware => ExpressionFinder::new(&view).scan(),
        ScanMode::Plain => ExpressionFinder::scan_plain(&view),
    };
    let straddles = |start: usize, end: usize| data.is_some_and(|data| data.intersects(&(start..end)));
    // A token the masked view still shows was authored here, not as an entire
    // frontmatter value, so it is malformed under either policy.
    if let Some(token) = scan.tokens.first() {
        return Err(literal_token_failure(input, token.start..token.end, TokenError::Embedded));
    }

    // Expressions and literals never overlap; rewrite them end to start so
    // every byte before the current span is still the caller's text.
    enum Found {
        Expression(ExpressionLocation),
        Literal(InterpolationLiteral),
    }
    let mut found: Vec<(usize, usize, Found)> = scan
        .expressions
        .into_iter()
        .filter(|loc| !straddles(loc.start, loc.end))
        .map(|loc| (loc.start, loc.end, Found::Expression(loc)))
        .chain(
            scan.literals
                .into_iter()
                .filter(|lit| !straddles(lit.start, lit.end))
                .map(|lit| (lit.start, lit.end, Found::Literal(lit))),
        )
        .collect();
    found.sort_by_key(|(start, ..)| std::cmp::Reverse(*start));

    let mut output = input.to_string();
    let mut count = 0;
    let mut warnings = Vec::new();
    let mut edits = Vec::new();

    for (start, end, item) in found {
        let loc = match item {
            Found::Literal(lit) => {
                let replacement = format!("{}{}{}", "{{", lit.content, "}}");
                edits.push(TextEdit::data(start..end, replacement.len()));
                output.replace_range(start..end, &replacement);
                continue;
            }
            Found::Expression(loc) => loc,
        };
        if let Some(failure) = expression_holds_token(&loc) {
            return Err(failure);
        }
        let origin = ExpressionOrigin::Authored(loc.start..loc.end);
        match parse(&loc.expression) {
            Ok(expr) => {
                let mut ctx_warnings = evaluator.collect_context_warnings(&expr, warning_stage);
                ctx_warnings.reverse();
                warnings.append(&mut ctx_warnings);
                let mark = evaluator.missing_root_mark();
                let evaluated = evaluator.eval(&expr);
                evaluator.locate_missing_roots(mark, Some(loc.start..loc.end));
                match evaluated {
                    EvalResult::Value(replacement) => {
                        // Inherit line indentation for multiline replacements
                        let replacement = if replacement.contains('\n') {
                            let line_start =
                                output[..loc.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                            let indent: String = output[line_start..loc.start]
                                .chars()
                                .take_while(|c| c.is_whitespace())
                                .collect();
                            if indent.is_empty() {
                                replacement
                            } else {
                                replacement.replace('\n', &format!("\n{indent}"))
                            }
                        } else {
                            replacement
                        };
                        edits.push(TextEdit::data(loc.start..loc.end, replacement.len()));
                        output.replace_range(loc.start..loc.end, &replacement);
                        count += 1;
                    }
                    EvalResult::Error { error, .. } if strict || error.is_authoring_fatal() => {
                        return Err(LocatedInterpolationError {
                            error: Box::new(interpolation_error(&loc.expression, error)),
                            span: Some(loc.start..loc.end),
                        });
                    }
                    EvalResult::Error { error, original } => {
                        warnings.push(ComposeWarning::expression_failure(
                            warning_stage,
                            format!("failed to evaluate '{}': {}", original, error),
                            ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
                            origin,
                        ));
                    }
                }
            }
            Err(e) if strict => {
                return Err(LocatedInterpolationError {
                    error: Box::new(interpolation_error(
                        &loc.expression,
                        ExpressionError::Parse(e.to_string()),
                    )),
                    span: Some(loc.start..loc.end),
                });
            }
            Err(e) => {
                warnings.push(ComposeWarning::expression_failure(
                    warning_stage,
                    format!("failed to parse '{}': {}", loc.expression, e),
                    ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
                    origin,
                ));
            }
        }
    }

    // Spans were visited end to start; report and record them in document
    // order so the first authored occurrence of an issue is the one kept.
    warnings.reverse();
    edits.reverse();

    Ok(InterpolationRewrite {
        output,
        replacements: count,
        warnings,
        edits,
    })
}

/// Interpolates a single frontmatter value.
///
/// When `input`'s trimmed content is exactly one `{{ expr }}` (a *whole-value*
/// interpolation), the value is executable state, not text: it is parsed and
/// evaluated directly, and the typed `serde_json::Value` result is returned —
/// so `{{ false }}` stays the boolean `false` (falsy) rather than the string
/// `"false"` (truthy), `{{ file_index(x) }}` stays a number, and a whole-value
/// expression that yields an array or object is preserved as that typed value.
/// A whole-value parse or evaluation failure is **fatal under either
/// `policy`**, so malformed expansion syntax (e.g. a mismatched paren) can
/// never leak downstream as a raw `{{ … }}` string.
///
/// Mixed text (`"a {{ x }}"`), strings holding more than one expression, and
/// plain strings fall through to [`interpolate_text`] under `policy`.
///
/// ## Errors
///
/// Returns `MarkdownError::Interpolation` when a whole-value expression fails
/// to parse or evaluate, and propagates the same error as [`interpolate_text`]
/// on the string path. Whole-value undefined variables resolve to `null` (not
/// an error), so `{{ missing }}` stays lenient.
pub(crate) fn interpolate_value<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError> {
    interpolate_value_located(input, evaluator, policy, warning_stage).map_err(|failure| *failure.error)
}

/// [`interpolate_value`], but a fatal failure also reports the failing
/// `{{ … }}`'s span in `input` (see [`interpolate_text_located`]).
pub(crate) fn interpolate_value_located<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<(Value, usize, Vec<ComposeWarning>), LocatedInterpolationError> {
    if is_whole_value_literal(input) {
        let result = interpolate_text_located(input, evaluator, ScanMode::Plain, policy, warning_stage)?;
        return Ok((Value::String(result.output), result.replacements, result.warnings));
    }
    if let Some(loc) = whole_value_span(input) {
        let located = |error| LocatedInterpolationError {
            error: Box::new(error),
            span: Some(loc.start..loc.end),
        };
        if let Some(failure) = expression_holds_token(&loc) {
            return Err(failure);
        }
        let expr = parse(&loc.expression).map_err(|e| {
            located(interpolation_error(&loc.expression, ExpressionError::Parse(e.to_string())))
        })?;
        // The whole-value path bypasses `interpolate_text`, so it must still run
        // the context-typo check on its single parsed expression — otherwise
        // `phase: "{{ ctx.toady }}"` (resolving to null) would warn in body text
        // but stay silent in frontmatter.
        let warnings = evaluator.collect_context_warnings(&expr, warning_stage);
        let value = evaluator
            .eval_json(&expr)
            .map_err(|cause| located(interpolation_error(&loc.expression, cause)))?;
        return Ok((value, 1, warnings));
    }
    let result = interpolate_text_located(input, evaluator, ScanMode::Plain, policy, warning_stage)?;
    Ok((Value::String(result.output), result.replacements, result.warnings))
}

/// Returns `true` when `input`'s trimmed content is exactly one interpolation
/// literal (`{{{ ... }}}`).
///
/// Such values take the string-rewrite path rather than the typed whole-value
/// expression path, so they resolve to the literal text `{{ ... }}`.
fn is_whole_value_literal(input: &str) -> bool {
    let scan = ExpressionFinder::scan_plain(input);
    if scan.expressions.is_empty() && scan.literals.len() == 1 {
        let lit = &scan.literals[0];
        input[..lit.start].trim().is_empty() && input[lit.end..].trim().is_empty()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::EffectiveStateBuilder;
    use crate::markdown::compose::ComposeContext;
    use serde_json::json;
    use std::collections::HashMap;

    fn make_state(data: serde_json::Value) -> crate::markdown::compose::EffectiveState {
        let fm: HashMap<String, serde_json::Value> = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => HashMap::new(),
        };
        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    #[test]
    fn plain_mode_does_not_skip_code_spans() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let result =
            interpolate_text("`{{ name }}`", &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Lenient, "test").unwrap();
        assert_eq!(result.output, "`Alice`");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn markdown_aware_scans_inline_code_spans() {
        // Inline code spans are scanned in MarkdownAware mode — only
        // fenced/indented code blocks are skipped.
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "`{{ name }}`",
            &evaluator,
            ScanMode::MarkdownAware,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "`Alice`");
        assert_eq!(result.replacements, 1);
    }

    /// A first-pass failure is located at its own bytes in the caller's input,
    /// even with earlier expressions still unreplaced before it.
    #[test]
    fn located_failure_spans_the_authored_expression() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let input = "a {{ name }}\nb {{ > invalid }}\n";
        let Err(failure) =
            interpolate_text_located(input, &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Strict, "test")
        else {
            panic!("the invalid expression must fail");
        };
        let span = failure.span.expect("a first-pass expression is located");
        assert_eq!(&input[span], "{{ > invalid }}");
    }

    /// A replacement value that spells an invalid expression is data: the
    /// single scan never parses it, even under the strict policy.
    #[test]
    fn replacement_output_is_never_parsed() {
        let state = make_state(json!({"template": "{{ > invalid }}"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text_located(
            "x\ny {{ template }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Strict,
            "test",
        )
        .expect("inserted text is data, not an expression");
        assert_eq!(result.output, "x\ny {{ > invalid }}");
        assert_eq!(result.replacements, 1);
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    }

    /// Each replaced expression and converted literal reports one data edit
    /// at its input range, in input order.
    #[test]
    fn replacements_report_one_data_edit_each() {
        use crate::markdown::compose::body_origin::{EditOrigin, TextEdit};
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "a {{ name }} b {{{ raw }}}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Strict,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "a Alice b {{ raw }}");
        assert_eq!(
            result.edits,
            vec![
                TextEdit { range: 2..12, replacement_len: 5, origin: EditOrigin::Data },
                TextEdit { range: 15..26, replacement_len: 9, origin: EditOrigin::Data },
            ]
        );
    }

    /// Expressions an earlier stage inserted as data are skipped, and one that
    /// straddles authored text and data is not an expression.
    #[test]
    fn data_ranges_are_never_scanned() {
        use crate::markdown::compose::body_origin::DataRanges;
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let input = "{{ name }} {{ name }} {{ name }}";
        // The whole second expression is data, and so is the `name` inside the
        // third, whose braces are authored.
        let data = DataRanges::covering(input, vec![11..21, 25..29]);
        let result = interpolate_text_in(
            input,
            Some(&data),
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Strict,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "Alice {{ name }} {{ name }}");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn markdown_aware_skips_fenced_code_blocks() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let input = "before {{ name }}\n\n```\n{{ name }}\n```\nafter";
        let result =
            interpolate_text(input, &evaluator, ScanMode::MarkdownAware, ExpressionFailurePolicy::Lenient, "test").unwrap();
        assert!(result.output.contains("before Alice"));
        assert!(result.output.contains("```\n{{ name }}\n```"));
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn multiline_indentation_inherited() {
        let state = make_state(json!({"items": "a\nb\nc"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "  list: {{ items }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "  list: a\n  b\n  c");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn fail_fast_returns_error_on_parse_failure() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        // An unparseable expression
        let result = interpolate_text("{{ > invalid }}", &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Strict, "test");
        assert!(result.is_err());
    }

    #[test]
    fn non_fail_fast_records_warnings() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ > invalid }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("failed to parse"));
        // Original is preserved
        assert!(result.output.contains("{{ > invalid }}"));
    }

    #[test]
    fn unknown_function_is_fatal_even_without_fail_fast() {
        // An unrecognized symbol can never resolve; it must surface as an error
        // rather than leaking its literal `{{ … }}` text downstream, even when
        // fail_fast is off.
        let state = make_state(json!({"spec": "a/b/spec.md"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ unknown_fn(spec) }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        );
        let Err(err) = result else {
            panic!("unknown function must be fatal");
        };
        assert!(err.to_string().contains("Unknown function: unknown_fn"));
    }

    #[test]
    fn no_expressions_returns_input_unchanged() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "no expressions here",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "no expressions here");
        assert_eq!(result.replacements, 0);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn whole_value_evaluation_preserves_nullable_target_types() {
        let state = make_state(json!({
            "null_target": null,
            "empty_target": "",
            "concrete_target": "child.md",
        }));
        let evaluator = Evaluator::new(&state);
        for (expression, expected) in [
            ("{{ null_target }}", Value::Null),
            ("{{ empty_target }}", Value::String(String::new())),
            ("{{ concrete_target }}", Value::String("child.md".to_string())),
        ] {
            let (value, replacements, warnings) =
                interpolate_value(expression, &evaluator, ExpressionFailurePolicy::Strict, "target-evaluation").unwrap();
            assert_eq!(value, expected, "{expression}");
            assert_eq!(replacements, 1, "{expression}");
            assert!(warnings.is_empty(), "{expression}: {warnings:?}");
        }
    }

    /// F14 fast-path: input with single braces but no `{{` skips the scan
    /// pipeline entirely and returns verbatim (byte-identical to the scanning
    /// path, which would find zero expressions and zero literals).
    #[test]
    fn single_brace_input_takes_fast_path_verbatim() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        // Contains `{` and `}` but never `{{` — the JSON-ish body must survive
        // untouched under both scan modes.
        let input = "config { key: value } and a lone } brace";
        for mode in [ScanMode::Plain, ScanMode::MarkdownAware] {
            let result = interpolate_text(input, &evaluator, mode, ExpressionFailurePolicy::Lenient, "test").unwrap();
            assert_eq!(result.output, input);
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }
    }

    /// F14 fast-path must NOT swallow `{{{ … }}}` literals: `{{{` contains `{{`,
    /// so the guard falls through to the normal literal-conversion path.
    #[test]
    fn triple_brace_literal_still_converted_despite_fast_path() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text("{{{ x }}}", &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Lenient, "test")
            .unwrap();
        assert_eq!(result.output, "{{ x }}");
        assert_eq!(result.replacements, 0);
    }

    #[test]
    fn nested_ternary_in_true_branch_via_interpolate_text() {
        let state = make_state(json!({"a": true, "b": true}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? b ? 'inner-true' : 'inner-false' : 'outer-false' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "inner-true");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn nested_ternary_in_false_branch_via_interpolate_text() {
        let state = make_state(json!({"a": false, "c": true}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? 'outer-true' : c ? 'inner-true' : 'inner-false' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "inner-true");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn deeply_nested_ternary_via_interpolate_text() {
        let state = make_state(json!({"a": true, "b": true, "c": false}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? b ? c ? 'd' : 'e' : 'f' : 'g' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "e");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn nested_ternary_with_context_variable() {
        // ctx.today is always truthy in test context (returns "2024-06-15")
        let state = make_state(json!({"flag": false}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.today ? ctx.today : 'no date' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "2024-06-15");
        assert_eq!(result.replacements, 1);
    }

    /// A ternary branch's text is the expression's result, so a placeholder
    /// inside it is data and stays literal. Compose with `+` instead.
    #[test]
    fn replacement_text_is_not_rescanned_for_nested_interpolation() {
        let state = make_state(json!({"pkg": "darkmatter"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ pkg ? 'in a package directory: {{pkg}}' : 'not in a package directory' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "in a package directory: {{pkg}}");
        assert_eq!(result.replacements, 1);

        let concatenated = interpolate_text(
            "{{ pkg ? 'in a package directory: ' + pkg : 'not in a package directory' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(concatenated.output, "in a package directory: darkmatter");
    }

    #[test]
    fn false_branch_is_not_rescanned_for_nested_interpolation() {
        let state = make_state(json!({"pkg": null, "fallback": "none"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ pkg ? 'has: {{pkg}}' : 'missing: {{fallback}}' }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "missing: {{fallback}}");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn context_typo_emits_warning_with_suggestion() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.tody }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "");
        assert!(result.warnings.iter().any(|w| {
            w.message.contains("unknown context variable")
                && w.message.contains("today")
        }));
    }

    #[test]
    fn valid_context_variable_emits_no_typo_warning() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.today }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Lenient,
            "test",
        )
        .unwrap();
        assert!(!result.output.is_empty());
        assert!(!result.warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    // ── Whole-value frontmatter context-typo coverage ─────────────────────
    //
    // `interpolate_value`'s whole-value path bypasses `interpolate_text`, so
    // the same ctx-typo diagnostic must still fire when the whole value is a
    // single `{{ expr }}`.

    #[test]
    fn whole_value_scalar_typo_emits_warning_with_suggestion() {
        // `ctx.toady` is unknown and resolves to null, taking the whole-value
        // path — it must still warn.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let (value, replacements, warnings) =
            interpolate_value("{{ ctx.toady }}", &evaluator, ExpressionFailurePolicy::Lenient, "frontmatter-interpolation")
                .unwrap();
        // Silent-null evaluation is unchanged: still null, still one replacement.
        assert_eq!(value, Value::Null);
        assert_eq!(replacements, 1);
        assert!(warnings.iter().any(|w| {
            w.message.contains("unknown context variable") && w.message.contains("today")
        }));
    }

    #[test]
    fn whole_value_scalar_string_literal_does_not_warn() {
        // A string literal that merely *spells* `ctx.toady` must not warn:
        // the check is AST-based, and a string literal evaluates to a String
        // (so it falls through to the string path, never the scalar one) — but
        // either way no ctx reference exists in the AST.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let (_value, _replacements, warnings) = interpolate_value(
            r#"{{ "ctx.toady" }}"#,
            &evaluator,
            ExpressionFailurePolicy::Lenient,
            "frontmatter-interpolation",
        )
        .unwrap();
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    #[test]
    fn whole_value_scalar_valid_ctx_does_not_warn() {
        // A valid `ctx.*` reference that resolves to a scalar (a numeric ctx
        // value) takes the fast-path but must not warn.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        // ctx.year resolves to a number in the fixed test context.
        let (value, replacements, warnings) =
            interpolate_value("{{ number(ctx.year) }}", &evaluator, ExpressionFailurePolicy::Lenient, "frontmatter-interpolation")
                .unwrap();
        assert!(matches!(value, Value::Number(_)));
        assert_eq!(replacements, 1);
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    mod interpolation_literal_tests {
        use super::*;

        #[test]
        fn body_literal_converts_to_literal_text() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ name }}}",
                &evaluator,
                ScanMode::MarkdownAware,
                ExpressionFailurePolicy::Lenient,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ name }}");
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }

        #[test]
        fn inline_code_literal_converts() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "`{{{ name }}}`",
                &evaluator,
                ScanMode::MarkdownAware,
                ExpressionFailurePolicy::Lenient,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "`{{ name }}`");
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn fenced_code_block_literal_is_untouched() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let input = "```\n{{{ name }}}\n```";
            let result = interpolate_text(input, &evaluator, ScanMode::MarkdownAware, ExpressionFailurePolicy::Lenient, "test").unwrap();
            assert_eq!(result.output, input);
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn tight_and_empty_literal_forms() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            for (input, expected) in [
                ("{{{x}}}", "{{x}}"),
                ("{{{}}}", "{{}}"),
                ("{{{ }}}", "{{ }}"),
            ] {
                let result = interpolate_text(input, &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Lenient, "test").unwrap();
                assert_eq!(result.output, expected, "input: {input:?}");
                assert_eq!(result.replacements, 0, "input: {input:?}");
            }
        }

        #[test]
        fn adjacent_expression_and_literal() {
            let state = make_state(json!({"a": "evaluated"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ a }}{{{ b }}}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Lenient,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "evaluated{{ b }}");
            assert_eq!(result.replacements, 1);
        }

        /// Literal conversion applies to authored literals only: a literal a
        /// replacement inserted is data and keeps its three braces.
        #[test]
        fn introduced_literal_is_not_converted() {
            let state = make_state(json!({"tmpl": "{{{ y }}}"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ tmpl }}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Lenient,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{{ y }}}");
            assert_eq!(result.replacements, 1);
        }

        #[test]
        fn fail_fast_over_only_literals_succeeds() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ name }}} {{{ other }}}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Strict,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ name }} {{ other }}");
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }

        #[test]
        fn whole_value_literal_takes_string_path() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let (value, replacements, warnings) =
                interpolate_value("{{{ x }}}", &evaluator, ExpressionFailurePolicy::Lenient, "frontmatter-interpolation").unwrap();
            assert_eq!(value, Value::String("{{ x }}".to_string()));
            assert_eq!(replacements, 0);
            assert!(warnings.is_empty());
        }

        #[test]
        fn literal_containing_expression_is_not_evaluated() {
            let state = make_state(json!({"x": "replaced"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ {{ x }} }}}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Lenient,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ {{ x }} }}");
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn convert_literals_plain_leaves_expressions_intact() {
            let output = convert_literals("{{ a }}{{{ b }}}", ScanMode::Plain);
            assert_eq!(output, "{{ a }}{{ b }}");
        }
    }

    /// Requirement 5: a failing span that survives a pass is one issue, no
    /// matter how many rescans a sibling replacement triggers.
    mod rescan_identity {
        use super::*;

        fn codes(warnings: &[ComposeWarning]) -> Vec<&str> {
            warnings.iter().filter_map(|w| w.code.as_deref()).collect()
        }

        /// The original two-span fixture: one successful replacement plus one
        /// bad expression. Before identity tracking the replacement forced a
        /// second pass that rescanned and re-reported the unchanged failure.
        #[test]
        fn one_replacement_and_one_parse_failure_report_exactly_one_issue() {
            let state = make_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ name }} {{ > invalid }}",
                &evaluator,
                ScanMode::MarkdownAware,
                ExpressionFailurePolicy::Lenient,
                "interpolation",
            )
            .unwrap();

            assert_eq!(result.output, "Alice {{ > invalid }}");
            assert_eq!(result.replacements, 1);
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(codes(&result.warnings), [ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE]);
            assert!(result.warnings[0].message.contains("failed to parse '> invalid'"));
        }

        #[test]
        fn one_replacement_and_one_evaluation_failure_report_exactly_one_issue() {
            let state = make_state(json!({"name": "Alice"}));
            let evaluator = Evaluator::new(&state);
            let result =
                interpolate_text("{{ min(1) }} {{ name }}", &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Lenient, "interpolation")
                    .unwrap();

            assert_eq!(result.output, "{{ min(1) }} Alice");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert_eq!(
                codes(&result.warnings),
                [ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE]
            );
        }

        /// Failures on both sides of a replacement that changes length: the
        /// later one's tracked range must shift or the next pass re-reports it.
        #[test]
        fn failures_around_a_length_changing_replacement_stay_one_each_in_document_order() {
            let state = make_state(json!({"name": "a much longer replacement value"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ > first }} {{ name }} {{ > second }}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Lenient,
                "interpolation",
            )
            .unwrap();

            assert_eq!(
                result.output,
                "{{ > first }} a much longer replacement value {{ > second }}"
            );
            let messages: Vec<&str> = result.warnings.iter().map(|w| w.message.as_str()).collect();
            assert_eq!(messages.len(), 2, "{messages:?}");
            assert!(messages[0].contains("'> first'"), "{messages:?}");
            assert!(messages[1].contains("'> second'"), "{messages:?}");
        }

        /// A replacement that spells a failing expression is data and reports
        /// nothing; only the authored failure is reported.
        #[test]
        fn only_the_authored_failure_is_reported() {
            let state = make_state(json!({"tpl": "{{ > generated }}"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ tpl }} {{ > generated }}",
                &evaluator,
                ScanMode::Plain,
                ExpressionFailurePolicy::Lenient,
                "interpolation",
            )
            .unwrap();

            assert_eq!(result.output, "{{ > generated }} {{ > generated }}");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            let origins: Vec<_> = result
                .warnings
                .iter()
                .map(|w| match &w.identity.as_ref().unwrap().subject {
                    crate::markdown::compose::context::report::WarningSubject::Expression {
                        origin,
                        ..
                    } => origin.clone(),
                    other => panic!("expected an expression identity, got {other:?}"),
                })
                .collect();
            assert_eq!(origins, vec![ExpressionOrigin::Authored(10..27)]);
        }

        /// Ten references to one unknown `ctx.*` group in one text carry one
        /// identity, so the report keeps one warning.
        #[test]
        fn ten_unknown_context_references_share_one_identity() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let input = "{{ ctx.toady }} ".repeat(10);
            let result =
                interpolate_text(&input, &evaluator, ScanMode::Plain, ExpressionFailurePolicy::Lenient, "interpolation").unwrap();

            let mut report = crate::markdown::compose::ComposeReport::new();
            report.add_warnings(result.warnings);
            assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
            assert_eq!(
                report.warnings[0].code.as_deref(),
                Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE)
            );
        }
    }
}
