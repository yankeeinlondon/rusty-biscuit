use serde_json::{Map, Value};

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["branch", "worktree", "merge_conflicts"];

pub(super) fn populate_git(cap: &ContextCapture, values: &mut Map<String, Value>) {
    values.insert(
        "branch".into(),
        cap.git_branch.clone().map(Value::String).unwrap_or(Value::Null),
    );
    values.insert(
        "worktree".into(),
        cap.git_worktree
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    values.insert(
        "merge_conflicts".into(),
        string_array(
            cap.merge_conflicts
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
        ),
    );
}
