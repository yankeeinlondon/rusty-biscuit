use serde_json::{Map, Value};

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["branch", "worktree", "merge_conflicts"];

/// Keys owned by the recent-history capture. It is separate from the `Git`
/// group because `CommitDesc` capture walks commits and their file changes,
/// which `ctx.branch` must never pay for (AC31).
pub(super) const HISTORY_KEYS: &[&str] = &["recent_commits"];

/// Number of commits `ctx.recent_commits` captures (R27).
pub(super) const RECENT_COMMIT_COUNT: usize = 10;

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

pub(super) fn populate_git_history(cap: &ContextCapture, values: &mut Map<String, Value>) {
    values.insert("recent_commits".into(), string_array(cap.recent_commits.clone()));
}

/// Renders each commit through Sniff's canonical per-commit plain formatter.
///
/// Commits that touched no files render nothing, exactly as
/// `sniff repo recent-commits --plain` omits them, so the result can be shorter
/// than `commits`.
pub(super) fn render_recent_commits(commits: &[sniff::filesystem::git::CommitDesc]) -> Vec<String> {
    let today = chrono::Local::now().date_naive();
    commits
        .iter()
        .filter_map(|commit| commit.describe_plain(today))
        .collect()
}
