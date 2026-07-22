use serde_json::Value;

use super::{
    EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext,
};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding {
        canonical: "predict_conflicts",
        aliases: &["predictconflicts"],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(predict_conflicts_fn)),
    },
    FunctionBinding {
        canonical: "branch_exists_on_remote",
        aliases: &["branchexistsonremote"],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(branch_exists_on_remote_fn)),
    },
    FunctionBinding {
        canonical: "remote_vendor",
        aliases: &["remotevendor"],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(remote_vendor_fn)),
    },
];

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

fn branch_exists_on_remote_fn(
    args: &[Value],
    context: &ResolutionContext,
) -> Result<Value, ExpressionError> {
    if args.len() > 2 {
        return Err(argument_count("branch_exists_on_remote", "zero, one, or two", args.len()));
    }
    let branch = args.first().map(|value| string_arg("branch_exists_on_remote", 0, value)).transpose()?.filter(|value| !value.is_empty());
    let remote = args
        .get(1)
        .map(|value| string_arg("branch_exists_on_remote", 1, value))
        .transpose()?;
    if remote.as_deref() == Some("") {
        return Err(ExpressionError::Other {
            function: "branch_exists_on_remote".to_string(),
            message: "remote must not be empty".to_string(),
        });
    }
    let key = format!(
        "branch_exists_on_remote:{}:{}",
        branch.as_deref().unwrap_or_default(),
        remote.as_deref().unwrap_or_default()
    );
    context.cached_provider_query("branch_exists_on_remote", key, || {
        sniff::filesystem::git::branch_exists_on_remote_at(
            context.caller_dir(),
            branch.as_deref(),
            remote.as_deref(),
            &context.remote_policy(),
        )
        .map(Value::Bool)
        .map_err(|error| super::provider::provider_error("branch_exists_on_remote", error))
    })
}

fn remote_vendor_fn(
    args: &[Value],
    context: &ResolutionContext,
) -> Result<Value, ExpressionError> {
    if args.len() > 1 {
        return Err(argument_count("remote_vendor", "zero or one", args.len()));
    }
    let remote = args.first().map(|value| string_arg("remote_vendor", 0, value)).transpose()?.filter(|value| !value.is_empty());
    let key = format!("remote_vendor:{}", remote.as_deref().unwrap_or_default());
    context.cached_provider_query("remote_vendor", key, || {
        sniff::filesystem::git::remote_vendor_at(
            context.caller_dir(),
            remote.as_deref(),
            &context.remote_policy(),
        )
        .map(Value::String)
        .map_err(|error| super::provider::provider_error("remote_vendor", error))
    })
}

fn argument_count(function: &str, expected: &str, actual: usize) -> ExpressionError {
    ExpressionError::Other {
        function: function.to_string(),
        message: format!("requires {expected} arguments, got {actual}"),
    }
}

fn string_arg(
    function: &'static str,
    index: usize,
    value: &Value,
) -> Result<String, ExpressionError> {
    value.as_str().map(str::to_string).ok_or_else(|| ExpressionError::ArgType {
        function,
        index,
        expected: "string",
        actual_type: value_type(value),
    })
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
