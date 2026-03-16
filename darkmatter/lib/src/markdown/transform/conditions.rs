//! Shared condition evaluation for `when=` expressions.
//!
//! This module provides the condition evaluator used by both page blocks
//! and transclusion `when` clauses. It was promoted from
//! `transclusion/conditions.rs` to avoid cross-module coupling.

use super::EffectiveState;
use super::interpolation::{ComparisonOp, Expr, parse};
use serde_json::Value;

/// Errors from condition parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum ConditionError {
    /// Failed to parse a condition expression.
    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    Parse {
        expr: String,
        line: usize,
        message: String,
    },
    /// Failed to evaluate a condition expression.
    #[error("Failed to evaluate condition '{expr}' at line {line}: {message}")]
    Eval {
        expr: String,
        line: usize,
        message: String,
    },
}

/// Evaluates a `when` condition expression.
pub fn evaluate_condition(
    expr: &str,
    state: &EffectiveState,
    line: usize,
) -> Result<bool, ConditionError> {
    let parsed = parse(expr).map_err(|e| ConditionError::Parse {
        expr: expr.to_string(),
        line,
        message: e.to_string(),
    })?;

    let value = eval_expr(&parsed, state).map_err(|message| ConditionError::Eval {
        expr: expr.to_string(),
        line,
        message,
    })?;

    Ok(is_truthy(&value))
}

fn eval_expr(expr: &Expr, state: &EffectiveState) -> Result<Value, String> {
    match expr {
        Expr::Variable(path) => Ok(state.get(path).unwrap_or(Value::Null)),
        Expr::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expr::NumberLiteral(n) => {
            let num = serde_json::Number::from_f64(*n)
                .ok_or_else(|| format!("Invalid numeric literal: {n}"))?;
            Ok(Value::Number(num))
        }
        Expr::UnaryNot(inner) => {
            let value = eval_expr(inner, state)?;
            Ok(Value::Bool(!is_truthy(&value)))
        }
        Expr::Fallback { primary, fallback } => {
            let primary = eval_expr(primary, state)?;
            if is_truthy(&primary) {
                Ok(primary)
            } else {
                eval_expr(fallback, state)
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = eval_expr(condition, state)?;
            if is_truthy(&condition) {
                eval_expr(then_branch, state)
            } else {
                eval_expr(else_branch, state)
            }
        }
        Expr::Comparison { left, op, right } => {
            let left = eval_expr(left, state)?;
            let right = eval_expr(right, state)?;

            // Null-safe comparisons: when both sides are Null (undefined),
            // equality and inequality both return false. Comparing two
            // unknown values yields no meaningful result.
            let both_null = left.is_null() && right.is_null();

            let outcome = match op {
                ComparisonOp::Equal => !both_null && scalar_string(&left) == scalar_string(&right),
                ComparisonOp::NotEqual => {
                    !both_null && scalar_string(&left) != scalar_string(&right)
                }
                ComparisonOp::GreaterThan => to_number_coerce(&left) > to_number_coerce(&right),
                ComparisonOp::GreaterThanOrEqual => {
                    to_number_coerce(&left) >= to_number_coerce(&right)
                }
                ComparisonOp::LessThan => to_number_coerce(&left) < to_number_coerce(&right),
            };
            Ok(Value::Bool(outcome))
        }
        Expr::FunctionCall { name, args } => eval_function(name, args, state),
    }
}

fn eval_function(name: &str, args: &[Expr], state: &EffectiveState) -> Result<Value, String> {
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        "and" => {
            let result = args
                .iter()
                .map(|arg| eval_expr(arg, state))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .all(is_truthy);
            Ok(Value::Bool(result))
        }
        "or" => {
            let result = args
                .iter()
                .map(|arg| eval_expr(arg, state))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .any(is_truthy);
            Ok(Value::Bool(result))
        }
        "haskey" | "has_key" => {
            if args.len() < 2 {
                return Err("HasKey() requires 2 arguments".to_string());
            }
            let object = eval_expr(&args[0], state)?;
            let key = scalar_string(&eval_expr(&args[1], state)?);
            let has = object
                .as_object()
                .map(|obj| obj.contains_key(&key))
                .unwrap_or(false);
            Ok(Value::Bool(has))
        }
        "contains" => {
            if args.len() < 2 {
                return Err("Contains() requires 2 arguments".to_string());
            }
            let haystack = eval_expr(&args[0], state)?;
            let needle = eval_expr(&args[1], state)?;
            let found = match haystack {
                Value::Array(values) => values
                    .iter()
                    .any(|value| scalar_string(value) == scalar_string(&needle)),
                Value::Object(values) => values
                    .values()
                    .any(|value| scalar_string(value) == scalar_string(&needle)),
                Value::String(value) => value.contains(&scalar_string(&needle)),
                value => scalar_string(&value).contains(&scalar_string(&needle)),
            };
            Ok(Value::Bool(found))
        }
        "length" => {
            if args.is_empty() {
                return Err("Length() requires 1 argument".to_string());
            }
            let value = eval_expr(&args[0], state)?;
            let len = match value {
                Value::String(s) => s.chars().count(),
                Value::Array(arr) => arr.len(),
                Value::Object(obj) => obj.len(),
                Value::Number(n) => n.to_string().chars().count(),
                Value::Bool(_) => 0,
                Value::Null => 0,
            };
            Ok(Value::Number(serde_json::Number::from(len)))
        }
        "number" => {
            if args.is_empty() {
                return Err("number() requires at least 1 argument".to_string());
            }
            let value = eval_expr(&args[0], state)?;
            let default = if args.len() > 1 {
                to_number_coerce(&eval_expr(&args[1], state)?)
            } else {
                0.0
            };
            let number = to_number(&value).unwrap_or(default);
            let json_number = serde_json::Number::from_f64(number)
                .ok_or_else(|| "Unable to represent number".to_string())?;
            Ok(Value::Number(json_number))
        }
        "round" => {
            if args.is_empty() {
                return Err("round() requires at least 1 argument".to_string());
            }
            let value = eval_expr(&args[0], state)?;
            let default = if args.len() > 1 {
                to_number_coerce(&eval_expr(&args[1], state)?)
            } else {
                0.0
            };
            let number = to_number(&value).unwrap_or(default).round() as i64;
            Ok(Value::Number(serde_json::Number::from(number)))
        }
        _ => Err(format!("Unknown function: {name}")),
    }
}

fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Null => None,
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn to_number_coerce(value: &Value) -> f64 {
    to_number(value).unwrap_or(0.0)
}

fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::transform::{EffectiveStateBuilder, TransformContext};
    use serde_json::json;
    use std::collections::HashMap;

    fn test_state(data: Value) -> EffectiveState {
        let fm: HashMap<String, Value> = match data {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(TransformContext::fixed_for_testing())
            .build()
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
        let mut ctx = TransformContext::fixed_for_testing();
        ctx.env.insert("AGENT".to_string(), "claude".to_string());

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build();

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
        let mut ctx = TransformContext::fixed_for_testing();
        ctx.env.insert("AGENT".to_string(), "claude".to_string());
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build();

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
}
