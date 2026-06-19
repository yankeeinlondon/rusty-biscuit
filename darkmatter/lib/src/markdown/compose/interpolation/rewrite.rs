//! Shared string rewrite helper for interpolation.
//!
//! Provides `interpolate_text`, which scans a string for `{{ }}` expressions,
//! evaluates them against an [`EvaluationLookup`] implementation, and
//! returns the rewritten string. Supports both markdown-aware scanning
//! (skipping code regions) and plain-text scanning.

use super::{EvalResult, Evaluator, ExpressionFinder, ExpressionLocation, parse};
use crate::markdown::compose::expression::{EvaluationLookup, UNKNOWN_FUNCTION_PREFIX};
use crate::markdown::compose::ComposeWarning;
use crate::markdown::types::MarkdownError;
use serde_json::Value;

/// Whether an evaluation error is fatal even in non-fail-fast mode.
///
/// An unknown function is an authoring mistake, not a data-dependent miss
/// (unlike an undefined variable, which resolves to an empty string by design).
/// Tolerating it would leave the literal `{{ … }}` text in place to poison a
/// later consumer with an unrelated error, so it is always surfaced here.
fn is_fatal_eval_error(message: &str) -> bool {
    message.starts_with(UNKNOWN_FUNCTION_PREFIX)
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
/// - `fail_fast` — if `true`, return an error on the first parse/eval failure
/// - `warning_stage` — label attached to any warnings produced
pub(crate) fn interpolate_text<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, MarkdownError> {
    let mut output = input.to_string();
    let mut total_count = 0;
    let mut all_warnings = Vec::new();

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
            match parse(&loc.expression) {
                Ok(expr) => {
                    let mut ctx_warnings = evaluator.collect_context_warnings(
                        &expr,
                        warning_stage,
                    );
                    warnings.append(&mut ctx_warnings);
                    match evaluator.eval(&expr) {
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
                        output.replace_range(loc.start..loc.end, &replacement);
                        count += 1;
                    }
                    EvalResult::Error { message, .. }
                        if fail_fast || is_fatal_eval_error(&message) =>
                    {
                        return Err(MarkdownError::Transform(format!(
                            "Interpolation evaluation failed for '{}': {}",
                            loc.expression, message
                        )));
                    }
                    EvalResult::Error { message, original } => {
                        warnings.push(ComposeWarning::new(
                            warning_stage,
                            format!("failed to evaluate '{}': {}", original, message),
                        ));
                    }
                }
                }
                Err(e) if fail_fast => {
                    return Err(MarkdownError::Transform(format!(
                        "Interpolation parse failed for '{}': {}",
                        loc.expression, e
                    )));
                }
                Err(e) => {
                    warnings.push(ComposeWarning::new(
                        warning_stage,
                        format!("failed to parse '{}': {}", loc.expression, e),
                    ));
                }
            }
        }

        total_count += count;
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

    Ok(InterpolationRewrite {
        output,
        replacements: total_count,
        warnings: all_warnings,
    })
}

/// Interpolates a single frontmatter value, preserving scalar type when the
/// whole value is one `{{ expr }}`.
///
/// When `input` is exactly one interpolation expression (ignoring surrounding
/// whitespace) that evaluates to a boolean, number, or null, the typed
/// `serde_json::Value` is returned so `{{ false }}` stays the boolean `false`
/// (falsy) rather than the string `"false"` (truthy), and `{{ file_index(x) }}`
/// stays a number for downstream predicates like `is_number`. Strings, arrays,
/// objects, mixed text (`"a {{ x }}"`), parse/eval failures, and unresolved
/// (e.g. shell-pending) templates fall through to [`interpolate_text`], keeping
/// the established string-rewrite behavior — including leaving an unresolved
/// `{{ … }}` in place for a later pass.
///
/// ## Errors
///
/// Propagates the same `MarkdownError` as [`interpolate_text`] when `fail_fast`
/// is set or a fatal evaluation error occurs on the string path.
pub(crate) fn interpolate_value<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError> {
    if let Some((typed, expr)) = whole_value_scalar(input, evaluator) {
        // The scalar fast-path bypasses `interpolate_text`, so it must still run
        // the context-typo check on its single parsed expression — otherwise
        // `phase: "{{ ctx.toady }}"` (resolving to null) would warn in body text
        // but stay silent in frontmatter.
        let warnings = evaluator.collect_context_warnings(&expr, warning_stage);
        return Ok((typed, 1, warnings));
    }
    let result = interpolate_text(input, evaluator, ScanMode::Plain, fail_fast, warning_stage)?;
    Ok((Value::String(result.output), result.replacements, result.warnings))
}

/// Returns the typed scalar value and its parsed expression when `input` is a
/// single whole-value `{{ expr }}` that evaluates to a boolean, number, or null.
///
/// Returns `None` (string path) when the value is mixed text, holds more than
/// one expression, evaluates to a string/array/object, or fails to parse or
/// evaluate. Restricting to `Bool`/`Number`/`Null` keeps the change to the
/// value kinds literal frontmatter already produces (`yolo: true`, `phase: 1`),
/// and leaves string/array/object results on the proven string path.
///
/// The parsed [`Expr`] is returned alongside the value so the caller can run
/// AST-based diagnostics (context-typo detection) on the same expression
/// without reparsing.
fn whole_value_scalar<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
) -> Option<(Value, super::Expr)> {
    let locations = ExpressionFinder::find_all_plain(input);
    let [loc] = locations.as_slice() else {
        return None;
    };
    if !input[..loc.start].trim().is_empty() || !input[loc.end..].trim().is_empty() {
        return None;
    }
    let expr = parse(&loc.expression).ok()?;
    match evaluator.eval_json(&expr) {
        Ok(value @ (Value::Bool(_) | Value::Number(_) | Value::Null)) => Some((value, expr)),
        _ => None,
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
            interpolate_text("`{{ name }}`", &evaluator, ScanMode::Plain, false, "test").unwrap();
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
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "`Alice`");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn markdown_aware_skips_fenced_code_blocks() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let input = "before {{ name }}\n\n```\n{{ name }}\n```\nafter";
        let result =
            interpolate_text(input, &evaluator, ScanMode::MarkdownAware, false, "test").unwrap();
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
            false,
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
        let result = interpolate_text("{{ > invalid }}", &evaluator, ScanMode::Plain, true, "test");
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
            false,
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
            false,
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
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "no expressions here");
        assert_eq!(result.replacements, 0);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn nested_ternary_in_true_branch_via_interpolate_text() {
        let state = make_state(json!({"a": true, "b": true}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? b ? 'inner-true' : 'inner-false' : 'outer-false' }}",
            &evaluator,
            ScanMode::Plain,
            false,
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
            false,
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
            false,
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
            false,
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
            false,
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
            false,
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
            false,
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
            false,
            "test",
        )
        .unwrap();
        assert!(!result.output.is_empty());
        assert!(!result.warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    // ── Whole-value scalar frontmatter context-typo coverage ──────────────
    //
    // `interpolate_value`'s scalar fast-path bypasses `interpolate_text`, so
    // the same ctx-typo diagnostic must still fire when the whole value is a
    // single `{{ expr }}` resolving to a scalar (Bool/Number/Null).

    #[test]
    fn whole_value_scalar_typo_emits_warning_with_suggestion() {
        // `ctx.toady` is unknown and resolves to null (a scalar), taking the
        // fast-path — it must still warn.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let (value, replacements, warnings) =
            interpolate_value("{{ ctx.toady }}", &evaluator, false, "frontmatter-interpolation")
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
            false,
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
            interpolate_value("{{ number(ctx.year) }}", &evaluator, false, "frontmatter-interpolation")
                .unwrap();
        assert!(matches!(value, Value::Number(_)));
        assert_eq!(replacements, 1);
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }
}
