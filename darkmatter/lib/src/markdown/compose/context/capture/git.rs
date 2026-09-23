use serde_json::{Map, Value};

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["branch", "worktree", "merge_conflicts"];

/// Keys owned by the recent-history capture. It is separate from the `Git`
/// group because history capture walks commits and their file changes,
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
/// Takes the newest `limit` commits first, then drops those that touched no
/// files, so the result can be shorter than `limit` — the same order of
/// operations as the eager capture always used.
pub(crate) fn render_recent_commits(
    commits: &sniff::filesystem::git::RecentCommits,
    limit: usize,
) -> Vec<String> {
    let options = sniff::filesystem::git::RecentCommitsOptions::new();
    commits
        .commits()
        .iter()
        .zip(commits.plain_blocks(&options))
        .take(limit)
        .filter(|(commit, _)| !commit.files.is_empty())
        .map(|(_, block)| block)
        .collect()
}

/// The newest `count` commits of the repository at `root`.
///
/// `root` must be inside a repository; a directory that is not one is an
/// error here rather than an empty set, matching the eager capture's contract.
pub(crate) fn fetch_recent_commits(
    root: &std::path::Path,
    count: usize,
) -> sniff::Result<sniff::filesystem::git::RecentCommits> {
    let repo = sniff::filesystem::git::GitRepo::discover(root)?
        .ok_or_else(|| sniff::SniffError::NotARepository(root.to_path_buf()))?;
    sniff::filesystem::git::RecentCommits::collect(
        &repo,
        &sniff::filesystem::git::RecentCommitsOptions::new().count(count),
    )
}
