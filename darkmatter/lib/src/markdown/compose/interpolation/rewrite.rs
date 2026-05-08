//! Shared string rewrite helper for interpolation.
//!
//! Provides `interpolate_text`, which scans a string for `{{ }}` expressions,
//! evaluates them against an [`EvaluationLookup`] implementation, and
//! returns the rewritten string. Supports both markdown-aware scanning
//! (skipping code regions) and plain-text scanning.

use super::{EvalResult, Evaluator, ExpressionFinder, ExpressionLocation, parse};
use crate::markdown::compose::expression::EvaluationLookup;
use crate::markdown::compose::types::ComposeWarning;
use crate::markdown::types::MarkdownError;

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
                Ok(expr) => match evaluator.eval(&expr) {
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
                    EvalResult::Error { message, .. } if fail_fast => {
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
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::state::EffectiveStateBuilder;
    use crate::markdown::compose::types::ComposeContext;
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
    fn loop_depth_protection() {
        // A self-referencing expression would loop forever without depth protection.
        // We simulate this by having a frontmatter value that resolves to itself.
        let state = make_state(json!({"self_ref": "{{self_ref}}"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ self_ref }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        // After 10 iterations the depth limit is hit and a warning is emitted.
        assert_eq!(result.replacements, 10);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("depth limit"));
    }
}
