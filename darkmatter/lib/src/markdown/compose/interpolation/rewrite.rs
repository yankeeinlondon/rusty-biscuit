//! Shared string rewrite helper for interpolation.
//!
//! Provides `interpolate_text`, which scans a string for `{{ }}` expressions,
//! evaluates them against an [`EvaluationLookup`] implementation, and
//! returns the rewritten string. Supports both markdown-aware scanning
//! (skipping code regions) and plain-text scanning.

use super::{EvalResult, Evaluator, ExpressionFinder, ExpressionLocation, parse};
use crate::markdown::compose::expression::{EvaluationLookup, ExpressionError};
use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::context::report::ExpressionOrigin;
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
    /// Every parse or evaluation failure aborts the rewrite, including one found
    /// by a rescan of replacement output.
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
}

/// Maximum number of rescan iterations to prevent infinite loops.
const MAX_INTERPOLATION_DEPTH: usize = 10;

/// Converts `{{{ ... }}}` interpolation literals in `input` to the literal
/// text `{{ ... }}`.
///
/// Literals are recognized using the shared scanner and are converted
/// from end to start so byte offsets remain stable. This runs **after**
/// the final interpolation pass so replacement values that introduce new
/// literals are also converted.
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
    /// Byte range of the failing `{{ … }}` in the caller's `input`, or `None`
    /// when the expression was found by a rescan of replacement output and so
    /// has no position in the text the caller supplied.
    pub span: Option<std::ops::Range<usize>>,
}

/// Scans `input` for `{{ }}` expressions, evaluates them, and returns
/// the rewritten string.
///
/// After each pass of replacements, the output is rescanned for newly
/// introduced `{{ }}` expressions (e.g. from a ternary branch that
/// contains interpolation placeholders).  Loop-depth protection prevents
/// runaway recursion when a replacement re-introduces the same expression.
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
///
/// Expressions evaluated in the first pass are located in `input` itself:
/// replacements run end to start, so every byte before an expression is still
/// the caller's text when it fails. Later passes rescan replacement output, so
/// an expression found there is reported with no span rather than an offset
/// that points at generated text.
pub(crate) fn interpolate_text_located<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    policy: ExpressionFailurePolicy,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, LocatedInterpolationError> {
    let strict = policy == ExpressionFailurePolicy::Strict;
    // Fast path (F14): a `{{ … }}` expression and a `{{{ … }}}` literal both
    // require the `{{` sequence. When the input contains none, no expression or
    // literal can be present, so the whole scan pipeline — the MarkdownAware
    // pulldown-cmark code-region parse in `ExpressionFinder::new`, every rescan
    // pass, and `convert_literals` — is provably a no-op. Skip it and return the
    // input verbatim. Byte-identical: the scan would find zero locations and
    // `convert_literals` zero literals either way.
    if !input.contains("{{") {
        return Ok(InterpolationRewrite {
            output: input.to_string(),
            replacements: 0,
            warnings: Vec::new(),
        });
    }

    let mut output = input.to_string();
    let mut total_count = 0;
    let mut all_warnings = Vec::new();
    // Failures already reported, at their current range in `output`. A failing
    // span is left in place, so when a sibling is replaced the next pass would
    // otherwise evaluate and report it again. The range tracks `output` as
    // replacements land; the origin is the failure's stable identity.
    let mut reported: Vec<(std::ops::Range<usize>, ExpressionOrigin)> = Vec::new();

    for depth in 0..MAX_INTERPOLATION_DEPTH {
        let locations: Vec<ExpressionLocation> = match scan_mode {
            ScanMode::MarkdownAware => ExpressionFinder::new(&output).find_all(),
            ScanMode::Plain => ExpressionFinder::find_all_plain(&output),
        };

        if locations.is_empty() {
            break;
        }

        let mut count = 0;
        let mut warnings = Vec::new();

        for loc in locations.into_iter().rev() {
            if reported.iter().any(|(range, _)| *range == (loc.start..loc.end)) {
                continue;
            }
            let origin = if depth == 0 {
                ExpressionOrigin::Authored(loc.start..loc.end)
            } else {
                ExpressionOrigin::Generated {
                    pass: depth,
                    span: loc.start..loc.end,
                }
            };
            match parse(&loc.expression) {
                Ok(expr) => {
                    let mut ctx_warnings = evaluator.collect_context_warnings(
                        &expr,
                        warning_stage,
                    );
                    ctx_warnings.reverse();
                    warnings.append(&mut ctx_warnings);
                    let mark = evaluator.missing_root_mark();
                    let evaluated = evaluator.eval(&expr);
                    evaluator
                        .locate_missing_roots(mark, (depth == 0).then_some(loc.start..loc.end));
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
                        shift_reported(&mut reported, &loc, replacement.len());
                        output.replace_range(loc.start..loc.end, &replacement);
                        count += 1;
                    }
                    EvalResult::Error { error, .. }
                        if strict || error.is_authoring_fatal() =>
                    {
                        return Err(LocatedInterpolationError {
                            error: Box::new(interpolation_error(&loc.expression, error)),
                            span: (depth == 0).then_some(loc.start..loc.end),
                        });
                    }
                    EvalResult::Error { error, original } => {
                        warnings.push(ComposeWarning::expression_failure(
                            warning_stage,
                            format!("failed to evaluate '{}': {}", original, error),
                            ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE,
                            origin.clone(),
                        ));
                        reported.push((loc.start..loc.end, origin));
                    }
                }
                }
                Err(e) if strict => {
                    return Err(LocatedInterpolationError {
                        error: Box::new(interpolation_error(
                            &loc.expression,
                            ExpressionError::Parse(e.to_string()),
                        )),
                        span: (depth == 0).then_some(loc.start..loc.end),
                    });
                }
                Err(e) => {
                    warnings.push(ComposeWarning::expression_failure(
                        warning_stage,
                        format!("failed to parse '{}': {}", loc.expression, e),
                        ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE,
                        origin.clone(),
                    ));
                    reported.push((loc.start..loc.end, origin));
                }
            }
        }

        total_count += count;
        // Locations were visited end to start; report them in document order
        // so the first authored occurrence of an issue is the one kept.
        warnings.reverse();
        all_warnings.extend(warnings);

        if count == 0 {
            break;
        }

        // If we hit the max depth with replacements still pending, add a warning.
        if depth == MAX_INTERPOLATION_DEPTH - 1 {
            all_warnings.push(ComposeWarning::new(
                warning_stage,
                format!(
                    "interpolation depth limit ({}) reached; possible infinite loop",
                    MAX_INTERPOLATION_DEPTH
                ),
            ));
        }
    }

    // `convert_literals` runs a full expression scan (a pulldown-cmark parse in
    // MarkdownAware mode) plus a copy. A `{{{ … }}}` literal is impossible
    // without the `{{{` sequence, so skip that work entirely when it's absent
    // (F14) — byte-identical: the scan would find no literals either way.
    let output = if output.contains("{{{") {
        convert_literals(&output, scan_mode)
    } else {
        output
    };

    Ok(InterpolationRewrite {
        output,
        replacements: total_count,
        warnings: all_warnings,
    })
}

/// Moves every reported range that sits after `replaced` by the length change
/// of replacing it with `replacement_len` bytes.
///
/// Locations are replaced end to start and never overlap, so a reported range
/// is either wholly after `replaced` or wholly before it.
fn shift_reported(
    reported: &mut [(std::ops::Range<usize>, ExpressionOrigin)],
    replaced: &ExpressionLocation,
    replacement_len: usize,
) {
    let removed = replaced.end - replaced.start;
    for (range, _) in reported.iter_mut().filter(|(range, _)| range.start >= replaced.end) {
        *range = range.start + replacement_len - removed..range.end + replacement_len - removed;
    }
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

/// Returns the single interpolation span when `input`'s trimmed content is
/// exactly one `{{ expr }}` (only whitespace before and after the span).
///
/// Returns `None` for plain strings, mixed text (`"a {{ x }}"`), and strings
/// holding more than one expression — those route to the
/// [`interpolate_text`] string path. Detection is independent of parse/eval
/// outcome, so a malformed whole-value `{{ … }}` is still recognized as
/// whole-value and held to the strict parse-and-evaluate contract.
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

    /// An expression that exists only in a replacement value is found by the
    /// rescan and must not be reported at an offset in the caller's input.
    #[test]
    fn located_failure_from_replacement_output_has_no_span() {
        let state = make_state(json!({"template": "{{ > invalid }}"}));
        let evaluator = Evaluator::new(&state);
        let Err(failure) = interpolate_text_located(
            "x\ny {{ template }}",
            &evaluator,
            ScanMode::Plain,
            ExpressionFailurePolicy::Strict,
            "test",
        ) else {
            panic!("the generated invalid expression must fail");
        };
        assert!(matches!(*failure.error, MarkdownError::Interpolation { .. }));
        assert_eq!(failure.span, None);
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

    #[test]
    fn rescans_replacement_text_for_nested_interpolation() {
        // A ternary branch that contains an interpolation placeholder
        // should be resolved in a subsequent pass.
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
        assert_eq!(result.output, "in a package directory: darkmatter");
        assert_eq!(result.replacements, 2);
    }

    #[test]
    fn rescans_false_branch_for_nested_interpolation() {
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
        assert_eq!(result.output, "missing: none");
        assert_eq!(result.replacements, 2);
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

        #[test]
        fn rescan_loop_converts_introduced_literal() {
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
            assert_eq!(result.output, "{{ y }}");
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

        /// A replacement that generates a failing expression is reported once,
        /// at a generated origin that cannot alias an authored failure.
        #[test]
        fn a_generated_failure_is_reported_once_and_distinct_from_an_authored_one() {
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
            assert_eq!(result.warnings.len(), 2, "{:?}", result.warnings);
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
            assert!(origins.contains(&ExpressionOrigin::Authored(10..27)), "{origins:?}");
            assert!(
                origins.contains(&ExpressionOrigin::Generated { pass: 1, span: 0..17 }),
                "{origins:?}"
            );
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
