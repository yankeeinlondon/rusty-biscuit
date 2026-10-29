use serde_json::Value;

use super::{BoundCall, FunctionBinding, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("predict_conflicts", &["predictconflicts"], predict_conflicts_fn),
    FunctionBinding::context("branch_exists_on_remote", &["branchexistsonremote"], branch_exists_on_remote_fn),
    FunctionBinding::context("remote_vendor", &["remotevendor"], remote_vendor_fn),
    FunctionBinding::context("recent_commits", &[], recent_commits_fn),
];

/// The largest float `count` accepted: every integer up to it is exactly
/// representable, so a larger value may not be the integer the author wrote.
/// `number(integer; min(1))` admits whole floats up to 2^64, so this bound is
/// the one count check binding does not make.
const MAX_EXACT_FLOAT_COUNT: f64 = 9_007_199_254_740_991.0;

/// `recent_commits(count)` — the pair of `ctx.recent_commits` evaluated at
/// call time (R28).
///
/// Every reached call opens the request's file-resolution repository root and
/// walks history; nothing is memoized, so a commit made mid-compose is visible
/// to a later call. Elements use the capture's formatter, so commits that
/// touched no files are omitted and the array can be shorter than `count`.
fn recent_commits_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let count = commit_count(call.args().get(0))?;
    let Some(root) = context.repository_root.as_deref() else {
        return Ok(Value::Array(Vec::new()));
    };
    let set = crate::markdown::compose::context::capture::fetch_recent_commits(root, count).map_err(
        |error| ExpressionError::Other {
            function: "recent_commits".to_string(),
            message: format!("failed to read history of repository {}: {error}", root.display()),
        },
    )?;
    Ok(Value::Array(
        crate::markdown::compose::context::capture::render_recent_commits(&set, count)
            .into_iter()
            .map(Value::String)
            .collect(),
    ))
}

/// The bound count, a whole number of at least 1; a float above the exact
/// range is rejected rather than rounded (R28).
fn commit_count(value: Option<&Value>) -> Result<usize, ExpressionError> {
    let invalid = |message: String| ExpressionError::ContractViolation {
        function: "recent_commits".to_string(),
        message,
    };
    let number = value.and_then(Value::as_number).ok_or_else(|| invalid("count must be a number".to_string()))?;
    let count = match number.as_u64() {
        Some(count) => count,
        None => {
            let float = number.as_f64().unwrap_or(f64::NAN);
            if float.is_nan() || float > MAX_EXACT_FLOAT_COUNT {
                return Err(invalid(format!("count {number} exceeds the supported maximum")));
            }
            float as u64
        }
    };
    usize::try_from(count)
        .map_err(|_| invalid(format!("count {count} exceeds the supported maximum")))
}

fn predict_conflicts_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(branch) = call.args().text(0) else {
        return Ok(Value::Null);
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

/// Overloads in catalog order: `()`, `(branch)`, `(branch, remote)`. An
/// empty `branch` means the caller's current branch, as omitting it does.
fn branch_exists_on_remote_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let branch = call.args().text(0).filter(|value| !value.is_empty()).map(str::to_string);
    let remote = call.args().text(1).map(str::to_string);
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

/// An omitted or empty `remote` selects the preferred remote.
fn remote_vendor_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let remote = call.args().text(0).filter(|value| !value.is_empty()).map(str::to_string);
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::call_function;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    fn call(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
        call_function("recent_commits", args, Some(context)).expect("registered context function")
    }

    /// Ledger row 66 (`recent_commits(count: number(integer; min(1)))`): a
    /// count outside the type is a binding type error and the handler is never
    /// entered; numeric text converts (`"3"` → 3). Only a whole float beyond
    /// the exact range, which the type admits, is the handler's own error.
    #[test]
    fn invalid_counts_are_errors_before_any_git_io() {
        use crate::markdown::compose::expression::functions::test_hooks::{handler_entries, reset_handler_entries};

        // No repository root: a valid count would return `[]`, so an error
        // proves validation ran first.
        let context = ResolutionContext::default();
        reset_handler_entries();
        for count in [json!(0), json!(-3), json!(0.0), json!(1.5), json!(1e300), json!(null), json!(true), json!("0")] {
            let error = call(std::slice::from_ref(&count), &context).unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{count}: {error:?}");
        }
        assert_eq!(handler_entries("recent_commits"), 0);

        let error = call(&[json!(9_007_199_254_740_993.0_f64)], &context).unwrap_err();
        assert!(matches!(error, ExpressionError::ContractViolation { .. }), "{error:?}");
        assert!(error.is_authoring_fatal());
        assert!(error.to_string().contains("exceeds the supported maximum"), "{error}");

        assert_eq!(call(&[json!(3)], &context).unwrap(), json!([]));
        assert_eq!(call(&[json!(3.0)], &context).unwrap(), json!([]));
        assert_eq!(call(&[json!("3")], &context).unwrap(), json!([]));
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
        let context = ResolutionContext::at(temp.path().to_path_buf()).with_repository_root(temp.path());

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
        let context = ResolutionContext::at(temp.path().to_path_buf()).with_repository_root(temp.path());
        let error = call(&[json!(1)], &context).unwrap_err();
        assert!(error.to_string().contains("failed to read history"), "{error}");
    }
}
