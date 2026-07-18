use serde_json::Value;

use super::{
    EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext,
};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding {
    canonical: "predict_conflicts",
    aliases: &["predictconflicts"],
    evaluation: EvaluationMode::Context,
    handler: Some(FunctionHandler::Context(predict_conflicts_fn)),
}];

fn predict_conflicts_fn(
    args: &[Value],
    context: &ResolutionContext,
) -> Result<Value, ExpressionError> {
    if args.len() != 1 {
        return Err(ExpressionError::Other {
            function: "predict_conflicts".to_string(),
            message: format!(
                "predict_conflicts() requires 1 argument, got {}",
                args.len()
            ),
        });
    }
    if args[0].is_null() {
        return Ok(Value::Null);
    }
    let Value::String(branch) = &args[0] else {
        return Err(ExpressionError::ArgType {
            function: "predict_conflicts",
            index: 0,
            expected: "string",
            actual_type: value_type(&args[0]),
        });
    };
    if branch.trim().is_empty() {
        return Err(ExpressionError::Other {
            function: "predict_conflicts".to_string(),
            message: "branch name must not be empty or whitespace-only".to_string(),
        });
    }

    let caller_dir = context.caller_dir();
    let paths = sniff::filesystem::git::merge_conflicts_with_branch_at(caller_dir, branch)
        .map_err(|error| ExpressionError::Other {
            function: "predict_conflicts".to_string(),
            message: format!(
                "failed for local branch {branch:?} in caller repository {}: {error}",
                caller_dir.display()
            ),
        })?;
    Ok(Value::Array(
        paths
            .into_iter()
            .map(|path| Value::String(path.to_string_lossy().into_owned()))
            .collect(),
    ))
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
