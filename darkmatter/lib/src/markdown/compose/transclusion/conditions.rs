//! Condition evaluation for transclusion `when=` expressions.
//!
//! This module delegates to the shared condition evaluator at
//! `compose::conditions` and converts errors to `TransclusionError`.

use super::types::TransclusionError;
use crate::markdown::compose::conditions::{self, ConditionError};
use crate::markdown::compose::expression::EvaluationLookup;
use biscuit_terminal::errors::SourceContext;

/// Evaluates a `when` condition expression.
pub fn evaluate_condition<L: EvaluationLookup>(
    expr: &str,
    state: &L,
    line: usize,
    ctx: SourceContext,
) -> Result<bool, TransclusionError> {
    conditions::evaluate_condition(expr, state, line, ctx).map_err(|e| e.into())
}

impl From<ConditionError> for TransclusionError {
    fn from(err: ConditionError) -> Self {
        match err {
            ConditionError::Parse {
                ctx,
                expr,
                line,
                message,
                span: _,
            } => TransclusionError::ConditionParse {
                ctx,
                expr,
                line,
                message,
            },
            ConditionError::Eval {
                ctx,
                expr,
                line,
                message,
            } => TransclusionError::ConditionEval {
                ctx,
                expr,
                line,
                message,
            },
        }
    }
}
