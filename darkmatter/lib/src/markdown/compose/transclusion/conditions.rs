//! Condition evaluation for transclusion `when=` expressions.
//!
//! This module delegates to the shared condition evaluator at
//! `compose::conditions` and converts errors to `TransclusionError`.

use super::types::TransclusionError;
use crate::markdown::compose::EffectiveState;
use crate::markdown::compose::conditions::{self, ConditionError};

/// Evaluates a `when` condition expression.
pub fn evaluate_condition(
    expr: &str,
    state: &EffectiveState,
    line: usize,
) -> Result<bool, TransclusionError> {
    conditions::evaluate_condition(expr, state, line).map_err(|e| e.into())
}

impl From<ConditionError> for TransclusionError {
    fn from(err: ConditionError) -> Self {
        match err {
            ConditionError::Parse {
                expr,
                line,
                message,
            } => TransclusionError::ConditionParse {
                expr,
                line,
                message,
            },
            ConditionError::Eval {
                expr,
                line,
                message,
            } => TransclusionError::ConditionEval {
                expr,
                line,
                message,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{EffectiveStateBuilder, ComposeContext};
    use serde_json::json;
    use std::collections::HashMap;

    fn test_state(data: serde_json::Value) -> EffectiveState {
        let fm: HashMap<String, serde_json::Value> = match data {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
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
        let mut ctx = ComposeContext::fixed_for_testing();
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
        let mut ctx = ComposeContext::fixed_for_testing();
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
