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
                span: _,
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
