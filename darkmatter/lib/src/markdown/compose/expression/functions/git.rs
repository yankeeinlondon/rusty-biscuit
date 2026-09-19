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
    FunctionBinding {
        canonical: "recent_commits",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(recent_commits_fn)),
    },
];

/// The largest float `count` accepted: every integer up to it is exactly
/// representable, so a larger value may not be the integer the author wrote.
const MAX_EXACT_FLOAT_COUNT: f64 = 9_007_199_254_740_991.0;

/// `recent_commits(count)` — the pair of `ctx.recent_commits` evaluated at
/// call time (R28).
///
/// Every reached call opens the request's file-resolution repository root and
/// walks history; nothing is memoized, so a commit made mid-compose is visible
/// to a later call. Elements use the capture's formatter, so commits that
/// touched no files are omitted and the array can be shorter than `count`.
fn recent_commits_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let [count] = args else {
        return Err(argument_count("recent_commits", "one", args.len()));
    };
    let count = commit_count(count)?;
    let Some(root) = context.repository_root.as_deref() else {
        return Ok(Value::Array(Vec::new()));
    };
    let set = sniff::filesystem::git::get_recent_commits_by_count(root, count).map_err(|error| {
        ExpressionError::Other {
            function: "recent_commits".to_string(),
            message: format!("failed to read history of repository {}: {error}", root.display()),
        }
    })?;
    Ok(Value::Array(
        crate::markdown::compose::context::capture::render_recent_commits(&set.commits)
            .into_iter()
            .map(Value::String)
            .collect(),
    ))
}

fn commit_count(value: &Value) -> Result<usize, ExpressionError> {
    // R28: an out-of-domain count is a compose error on every surface.
    let invalid = |message: String| ExpressionError::ContractViolation {
        function: "recent_commits".to_string(),
        message,
    };
    let Value::Number(number) = value else {
        return Err(ExpressionError::ArgType {
            function: "recent_commits",
            index: 0,
            expected: "number",
            actual_type: value_type(value),
        });
    };
    let count = if let Some(count) = number.as_u64() {
        count
    } else if number.as_i64().is_some() {
        return Err(invalid(format!("count must be at least 1, got {number}")));
    } else {
        let float = number.as_f64().unwrap_or(f64::NAN);
        if !float.is_finite() || float.fract() != 0.0 {
            return Err(invalid(format!("count must be an integer, got {number}")));
        }
        if float > MAX_EXACT_FLOAT_COUNT {
            return Err(invalid(format!("count {number} exceeds the supported maximum")));
        }
        if float < 1.0 {
            return Err(invalid(format!("count must be at least 1, got {number}")));
        }
        float as u64
    };
    if count == 0 {
        return Err(invalid("count must be at least 1, got 0".to_string()));
    }
    usize::try_from(count)
        .map_err(|_| invalid(format!("count {count} exceeds the supported maximum")))
}

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

pub(super) fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::dispatch_fs;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    fn call(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
        dispatch_fs("recent_commits", args, context).expect("registered context function")
    }

    #[test]
    fn invalid_counts_are_errors_before_any_git_io() {
        // No repository root: a valid count would return `[]`, so an error
        // proves validation ran first.
        let context = ResolutionContext::default();
        for (count, expected) in [
            (json!(0), "at least 1"),
            (json!(-3), "at least 1"),
            (json!(0.0), "at least 1"),
            (json!(1.5), "must be an integer"),
            (json!(1e300), "exceeds the supported maximum"),
            (json!(9_007_199_254_740_993.0_f64), "exceeds the supported maximum"),
        ] {
            let error = call(std::slice::from_ref(&count), &context).unwrap_err();
            assert!(matches!(error, ExpressionError::ContractViolation { .. }), "{count}: {error:?}");
            assert!(error.is_authoring_fatal());
            assert!(error.to_string().contains(expected), "{count}: {error}");
        }
        for count in [json!("3"), json!(null), json!(true)] {
            assert!(matches!(
                call(&[count], &context),
                Err(ExpressionError::ArgType { function: "recent_commits", index: 0, .. })
            ));
        }
        assert_eq!(call(&[json!(3)], &context).unwrap(), json!([]));
        assert_eq!(call(&[json!(3.0)], &context).unwrap(), json!([]));
        assert_eq!(call(&[json!(u64::MAX)], &context).unwrap(), json!([]));
    }

    /// Each reached call walks history again: a commit between two calls on
    /// one context is visible to the second.
    #[test]
    fn calls_are_never_cached() {
        let temp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(temp.path()).unwrap();
        let commit = |name: &str| {
            std::fs::write(temp.path().join(name), name).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new(name)).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let signature = git2::Signature::new("fixture", "fixture@example.com", &git2::Time::new(1_577_836_800, 0)).unwrap();
            let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
            let parents: Vec<_> = parent.iter().collect();
            repo.commit(Some("HEAD"), &signature, &signature, name, &tree, &parents).unwrap();
        };
        let context = ResolutionContext::new(temp.path().to_path_buf()).with_repository_root(temp.path());

        // An unborn repository has no history.
        assert_eq!(call(&[json!(5)], &context).unwrap(), json!([]));
        commit("first");
        let first = call(&[json!(5)], &context).unwrap();
        commit("second");
        let second = call(&[json!(5)], &context).unwrap();

        assert_eq!(first.as_array().unwrap().len(), 1);
        assert_eq!(second.as_array().unwrap().len(), 2);
        assert!(second[0].as_str().unwrap().contains("second"), "{second}");
    }

    #[test]
    fn a_captured_root_that_is_not_a_repository_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let context = ResolutionContext::new(temp.path().to_path_buf()).with_repository_root(temp.path());
        let error = call(&[json!(1)], &context).unwrap_err();
        assert!(error.to_string().contains("failed to read history"), "{error}");
    }
}
