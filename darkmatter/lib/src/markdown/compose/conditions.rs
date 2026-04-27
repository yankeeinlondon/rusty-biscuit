//! Shared condition evaluation for `when=` expressions.
//!
//! This module provides the condition evaluator used by both page blocks
//! and transclusion `when` clauses. It was promoted from
//! `transclusion/conditions.rs` to avoid cross-module coupling.

use super::EffectiveState;
use super::expression::{evaluate, is_truthy, parse_condition};
use std::ops::Range;
use tracing::{debug, trace};

/// Errors from condition parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum ConditionError {
    /// Failed to parse a condition expression.
    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    Parse {
        expr: String,
        line: usize,
        message: String,
        /// Byte range in `expr` that the parser identified as problematic.
        ///
        /// When the parser reports only a single byte position, darkmatter
        /// records a single-point span at that offset. If no position can be
        /// recovered, the span falls back to the full expression.
        span: Range<usize>,
    },
    /// Failed to evaluate a condition expression.
    #[error("Failed to evaluate condition '{expr}' at line {line}: {message}")]
    Eval {
        expr: String,
        line: usize,
        message: String,
    },
}

impl biscuit_terminal::errors::BlockError for ConditionError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        let operator_hint = "Operators: <cyan>&&  ||  !  ==  !=  >  >=  <</cyan> | Helpers: <cyan>HasKey, Contains, Length, number, round</cyan>";

        match self {
            ConditionError::Parse {
                expr,
                line,
                message,
                span,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ConditionError", "parse failed"))
                .body(format!(
                    "<dim>Expression:</dim>\n  <cyan>{expr}</cyan>\n{}\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}",
                    caret_marker(expr, span.start)
                ))
                .hint(operator_hint),

            ConditionError::Eval { expr, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ConditionError", "evaluation failed"))
                .body(format!(
                    "<dim>Expression:</dim> <cyan>{expr}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Confirm every variable referenced is present in the effective state."),
        }
    }
}

/// Evaluates a `when` condition expression.
pub fn evaluate_condition(
    expr: &str,
    state: &EffectiveState,
    line: usize,
) -> Result<bool, ConditionError> {
    trace!(expr = %expr, line, "conditions: evaluating");

    let parsed = parse_condition(expr).map_err(|e| ConditionError::Parse {
        expr: expr.to_string(),
        line,
        message: e.message.clone(),
        span: parse_error_span(expr, e.position),
    })?;

    let value = evaluate(&parsed, state).map_err(|message| ConditionError::Eval {
        expr: expr.to_string(),
        line,
        message,
    })?;

    let result = is_truthy(&value);
    debug!(expr = %expr, result, "conditions: evaluated");

    Ok(result)
}

fn parse_error_span(expr: &str, position: usize) -> Range<usize> {
    if expr.is_empty() {
        return 0..0;
    }

    let start = position.min(expr.len());
    let end = (start.saturating_add(1)).min(expr.len());
    if start == end {
        0..expr.len()
    } else {
        start..end
    }
}

fn caret_marker(input: &str, byte_offset: usize) -> String {
    let clamped = byte_offset.min(input.len());
    let column = input
        .char_indices()
        .take_while(|(idx, _)| *idx < clamped)
        .count();
    format!("  {}^", " ".repeat(column))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{ComposeContext, EffectiveStateBuilder};
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn test_state(data: Value) -> EffectiveState {
        let fm: HashMap<String, Value> = match data {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    #[test]
    fn evaluates_unary_not() {
        let state = test_state(json!({}));
        assert!(evaluate_condition("!missing", &state, 1).unwrap());
    }

    #[test]
    fn evaluates_has_key() {
        let state = test_state(json!({ "user": {"name": "Alice"} }));
        assert!(evaluate_condition(r#"HasKey(user, "name")"#, &state, 1).unwrap());
    }

    #[test]
    fn evaluates_and_or() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("And(a, b)", &state, 1).unwrap());
        assert!(evaluate_condition("Or(a, b)", &state, 1).unwrap());
    }

    #[test]
    fn numeric_comparison_coerces_non_numeric_to_zero() {
        let state = test_state(json!({ "name": "Alice" }));
        assert!(evaluate_condition("name >= 0", &state, 1).unwrap());
    }

    #[test]
    fn null_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a == missing_b", &state, 1).unwrap());
    }

    #[test]
    fn null_not_equal_null_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("missing_a != missing_b", &state, 1).unwrap());
    }

    #[test]
    fn defined_equal_null_is_false() {
        let state = test_state(json!({ "color": "red" }));
        assert!(!evaluate_condition("color == missing", &state, 1).unwrap());
    }

    #[test]
    fn defined_not_equal_null_is_true() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color != missing", &state, 1).unwrap());
    }

    #[test]
    fn equality_with_string_literal() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition(r#"color == "red""#, &state, 1).unwrap());
        assert!(!evaluate_condition(r#"color == "blue""#, &state, 1).unwrap());
    }

    #[test]
    fn equality_with_single_quoted_string() {
        let state = test_state(json!({ "color": "red" }));
        assert!(evaluate_condition("color == 'red'", &state, 1).unwrap());
        assert!(!evaluate_condition("color == 'blue'", &state, 1).unwrap());
    }

    #[test]
    fn env_equality_with_string_literal() {
        let mut ctx = ComposeContext::fixed_for_testing();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build()
            .unwrap();

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
    }

    #[test]
    fn unset_env_equality_with_string_literal_is_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
    }

    #[test]
    fn mutual_exclusion_pattern() {
        let mut ctx = ComposeContext::fixed_for_testing();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build()
            .unwrap();

        assert!(evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
        assert!(!evaluate_condition("!env.AGENT", &state, 1).unwrap());
    }

    #[test]
    fn mutual_exclusion_pattern_unset() {
        let state = test_state(json!({}));

        assert!(!evaluate_condition("env.AGENT == 'claude'", &state, 1).unwrap());
        assert!(!evaluate_condition("env.AGENT == 'opencode'", &state, 1).unwrap());
        assert!(evaluate_condition("!env.AGENT", &state, 1).unwrap());
    }

    // ── Infix `&&` / `||` coverage ────────────────────────────────────────

    #[test]
    fn infix_and_both_true() {
        let state = test_state(json!({ "a": true, "b": true }));
        assert!(evaluate_condition("a && b", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_one_false() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("a && b", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_both_false() {
        let state = test_state(json!({ "a": false, "b": false }));
        assert!(!evaluate_condition("a || b", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_one_true() {
        let state = test_state(json!({ "a": false, "b": true }));
        assert!(evaluate_condition("a || b", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_binds_tighter_than_or() {
        // (false && true) || true => true
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("a && b || c", &state, 1).unwrap());

        // false || (true && false) => false
        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("a || b && c", &state, 1).unwrap());
    }

    #[test]
    fn infix_parenthesized_or_then_and() {
        // (a || b) && c
        let state = test_state(json!({ "a": false, "b": true, "c": true }));
        assert!(evaluate_condition("(a || b) && c", &state, 1).unwrap());

        let state = test_state(json!({ "a": false, "b": true, "c": false }));
        assert!(!evaluate_condition("(a || b) && c", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_with_literal() {
        // a || (missing || "default") — Or short-circuits on `a`.
        let state = test_state(json!({ "a": true }));
        assert!(evaluate_condition(r#"a || (missing || "default")"#, &state, 1).unwrap());
    }

    #[test]
    fn legacy_and_or_function_still_works() {
        let state = test_state(json!({ "a": true, "b": false }));
        assert!(!evaluate_condition("And(a, b)", &state, 1).unwrap());
        assert!(evaluate_condition("Or(a, b)", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_short_circuits_on_false() {
        // UnknownFn would raise `Unknown function` — but short-circuit on
        // leading `false` means the rhs is never evaluated.
        let state = test_state(json!({}));
        assert!(!evaluate_condition("false_flag && UnknownFn(x)", &state, 1).unwrap());
    }

    #[test]
    fn infix_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(evaluate_condition("truthy_flag || UnknownFn(x)", &state, 1).unwrap());
    }

    #[test]
    fn function_and_short_circuits_on_false() {
        let state = test_state(json!({}));
        assert!(!evaluate_condition("And(false_flag, UnknownFn(x))", &state, 1).unwrap());
    }

    #[test]
    fn function_or_short_circuits_on_true() {
        let state = test_state(json!({ "truthy_flag": true }));
        assert!(evaluate_condition("Or(truthy_flag, UnknownFn(x))", &state, 1).unwrap());
    }

    #[test]
    fn infix_and_without_short_circuit_propagates_eval_error() {
        // When short-circuit doesn't kick in, unknown functions must surface
        // as an evaluation error.
        let state = test_state(json!({ "truthy_flag": true }));
        let result = evaluate_condition("truthy_flag && UnknownFn(x)", &state, 1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConditionError::Eval { .. }));
    }

    #[test]
    fn infix_with_comparison_operands() {
        let state = test_state(json!({ "count": 5, "name": "alice" }));
        assert!(evaluate_condition(r#"count > 0 && name == "alice""#, &state, 1).unwrap());
        assert!(!evaluate_condition(r#"count > 0 && name == "bob""#, &state, 1).unwrap());
        assert!(evaluate_condition(r#"count > 10 || name == "alice""#, &state, 1).unwrap());
    }
}
