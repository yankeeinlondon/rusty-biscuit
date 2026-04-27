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
    /// Skip expressions inside code spans and fenced code blocks.
    /// Used by body interpolation.
    MarkdownAware,
    /// Scan the entire string with no exclusions.
    /// Used by frontmatter interpolation.
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

/// Scans `input` for `{{ }}` expressions, evaluates them, and returns
/// the rewritten string.
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
    let locations: Vec<ExpressionLocation> = match scan_mode {
        ScanMode::MarkdownAware => ExpressionFinder::new(input).find_all(),
        ScanMode::Plain => ExpressionFinder::find_all_plain(input),
    };

    if locations.is_empty() {
        return Ok(InterpolationRewrite {
            output: input.to_string(),
            replacements: 0,
            warnings: vec![],
        });
    }

    let mut output = input.to_string();
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

    Ok(InterpolationRewrite {
        output,
        replacements: count,
        warnings,
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
    fn markdown_aware_skips_code_spans() {
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
        assert_eq!(result.output, "`{{ name }}`");
        assert_eq!(result.replacements, 0);
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
}
