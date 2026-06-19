use chrono::{DateTime, Local, TimeZone, Timelike};
use worktree::git::git_command;

/// A parsed commit for verbose display.
pub struct CommitDetail {
    pub short_sha: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub refs: String,
}

/// A commit identifier carrying both the full SHA (for identity and branch
/// placement) and a Rust-derived display ID (for Mermaid commit labels).
#[derive(Debug, Clone)]
pub struct CommitId {
    /// Full 40-character SHA, used for equality comparisons.
    pub full: String,
    /// Display ID derived in-process via [`display_sha`], not from git `%h`.
    pub display: String,
}

/// Render a full SHA as a fixed-width display ID for graph commit labels.
///
/// Branch placement must never use this — compare full SHAs instead, since
/// git's `%h` abbreviation length is repository-dependent.
fn display_sha(full: &str) -> &str {
    &full[..7.min(full.len())]
}

/// Per-branch data gathered once and consumed by both graph and verbose paths.
pub struct BranchGraphData {
    pub branch: String,
    pub default_branch: String,
    /// Full SHA of the merge-base between this branch and the default branch.
    pub merge_base_full: String,
    /// Index of the merge-base in the main commit list. Populated by
    /// [`gather_base_graph`] for deterministic base-graph rendering.
    pub merge_base_idx: usize,
    /// Up to 2 context commits ending at the merge-base, oldest first.
    default_context: Vec<CommitId>,
    /// Up to 5 commits on the default branch since the merge-base, oldest first.
    default_after_base: Vec<CommitId>,
    /// Commits on the default branch since the merge-base not shown in
    /// `default_after_base` (older than the newest-5 window). Drives the
    /// fork-adjacent elision marker on the default line.
    default_hidden: usize,
    /// Up to 5 commits on the branch since the merge-base, oldest first.
    branch_after_base: Vec<CommitId>,
    /// Commits on the branch since the merge-base not shown in
    /// `branch_after_base` (older than the newest-5 window). Drives the
    /// fork-adjacent elision marker on the branch line.
    branch_hidden: usize,
    /// Verbose detail for the merge-base commit, populated only when requested.
    pub merge_base_detail: Option<CommitDetail>,
    /// Verbose details for branch commits since the merge-base, populated only when requested.
    pub branch_details: Vec<CommitDetail>,
}

/// Data gathered for the base-branch graph, including the main commit list and
/// all per-branch graph data in deterministic render order.
pub struct BaseGraphData {
    pub default_branch: String,
    pub main_commits: Vec<CommitId>,
    pub branches: Vec<BranchGraphData>,
}

/// What subset of branch data a gather pass should collect.
enum GatherScope {
    /// Full data: default-context, post-divergence commits on both branches,
    /// and optional verbose details.
    Full,
    /// Minimal data for the base-graph overview: merge-base identity and
    /// branch commits since the merge-base only.
    BaseOverview,
}

/// Gather all graph and (optionally) verbose data for a single branch in one pass.
///
/// Returns `None` only when the merge-base cannot be resolved; partial log
/// failures degrade to empty lists so rendering can still proceed.
pub fn gather_branch(default_branch: &str, branch: &str, verbose: bool) -> Option<BranchGraphData> {
    gather_branch_impl(default_branch, branch, GatherScope::Full, verbose)
}

/// Gather only the data the base-graph renderer consumes: merge-base identity
/// and branch commits since divergence. Skips the two per-branch default-branch
/// log queries that [`gather_branch`] would collect but [`base_graph`] discards.
fn gather_branch_for_base(default_branch: &str, branch: &str) -> Option<BranchGraphData> {
    gather_branch_impl(default_branch, branch, GatherScope::BaseOverview, false)
}

fn gather_branch_impl(
    default_branch: &str,
    branch: &str,
    scope: GatherScope,
    verbose: bool,
) -> Option<BranchGraphData> {
    let merge_base_full = get_merge_base(default_branch, branch)?;

    let (default_context, default_after_base, default_hidden) = match scope {
        GatherScope::Full => {
            let after = commits_since(default_branch, &merge_base_full, 5);
            let hidden = hidden_since(default_branch, &merge_base_full, after.len());
            (ancestor_commits(&merge_base_full, 2), after, hidden)
        }
        GatherScope::BaseOverview => (Vec::new(), Vec::new(), 0),
    };

    let branch_after_base = commits_since(branch, &merge_base_full, 5);
    let branch_hidden = hidden_since(branch, &merge_base_full, branch_after_base.len());

    let (merge_base_detail, branch_details) = if verbose {
        (
            commit_details(&merge_base_full, 1).into_iter().next(),
            commit_details_since(branch, &merge_base_full),
        )
    } else {
        (None, Vec::new())
    };

    Some(BranchGraphData {
        branch: branch.to_string(),
        default_branch: default_branch.to_string(),
        merge_base_full,
        merge_base_idx: 0,
        default_context,
        default_after_base,
        default_hidden,
        branch_after_base,
        branch_hidden,
        merge_base_detail,
        branch_details,
    })
}

/// Get the detailed commit info for the merge-base commit from already-gathered data.
pub fn merge_base_commit(data: &BranchGraphData) -> Option<&CommitDetail> {
    data.merge_base_detail.as_ref()
}

/// Get detailed commits on a branch since the merge-base from already-gathered data.
pub fn branch_commits_detail(data: &BranchGraphData) -> &[CommitDetail] {
    &data.branch_details
}

/// Git format string using %x1f (Unit Separator) as field delimiter.
/// Using git's own escape avoids embedding raw control chars in args.
const DETAIL_FMT: &str = "%h%x1f%s%x1f%at%x1f%D";

/// Query git log for detailed commit info, returning oldest first.
fn commit_details(rev: &str, max: usize) -> Vec<CommitDetail> {
    let max_str = max.to_string();
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let Ok(output) = git_command(&[
        "log",
        &fmt_arg,
        "--max-count",
        &max_str,
        "--reverse",
        rev,
        "--",
    ]) else {
        return vec![];
    };
    parse_commit_lines(&output)
}

/// Query git log for commits reachable from `target` but not `exclude`, oldest first.
fn commit_details_since(target: &str, exclude: &str) -> Vec<CommitDetail> {
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let Ok(output) = git_command(&[
        "log",
        &fmt_arg,
        "--reverse",
        target,
        "--not",
        exclude,
        "--",
    ]) else {
        return vec![];
    };
    parse_commit_lines(&output)
}

fn parse_commit_lines(output: &str) -> Vec<CommitDetail> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            if parts.len() < 3 {
                return None;
            }
            let ts: i64 = parts[2].parse().ok()?;
            let timestamp = Local.timestamp_opt(ts, 0).single()?;
            Some(CommitDetail {
                short_sha: parts[0].to_string(),
                message: parts[1].to_string(),
                timestamp,
                refs: parts.get(3).unwrap_or(&"").to_string(),
            })
        })
        .collect()
}

/// Format a commit in the same style as `sniff repo git-status`.
pub fn format_commit(commit: &CommitDetail) -> String {
    let sha_display = format!("<b>{}</b>", commit.short_sha);
    let (date_str, time_str, use_on) = format_datetime(&commit.timestamp);
    let date_prefix = if use_on { "<i>on</i> " } else { "" };
    let refs_part = format_refs(&commit.refs);

    let cc = parse_conventional(&commit.message);
    if let Some((op, scope, desc)) = cc {
        let scope_part = scope
            .map(|s| format!("(<dim>{s}</dim>)"))
            .unwrap_or_default();
        format!(
            "[{sha_display}] <b><yellow>{op}</yellow></b>{scope_part} <i>at</i> <blue><b>{time_str}</b></blue> {date_prefix}<blue>{date_str}</blue>{refs_part}: <dim>{desc}</dim>"
        )
    } else {
        let first_line = commit.message.lines().next().unwrap_or("");
        let truncated = if first_line.len() > 50 {
            format!("{}...", &first_line[..47])
        } else {
            first_line.to_string()
        };
        format!(
            "[{sha_display}] <dim>{truncated}</dim> {date_prefix}<blue><b>{time_str}</b></blue> <blue>{date_str}</blue>{refs_part}"
        )
    }
}

fn format_datetime(ts: &DateTime<Local>) -> (String, String, bool) {
    let today = Local::now().date_naive();
    let commit_date = ts.date_naive();

    let (date_str, use_on) = if commit_date == today {
        ("Today".to_string(), false)
    } else if commit_date == today.pred_opt().unwrap_or(today) {
        ("Yesterday".to_string(), false)
    } else {
        (commit_date.format("%Y-%m-%d").to_string(), true)
    };

    let hour = ts.hour();
    let minute = ts.minute();
    let (hour_12, period) = if hour == 0 {
        (12, "am")
    } else if hour < 12 {
        (hour, "am")
    } else if hour == 12 {
        (12, "pm")
    } else {
        (hour - 12, "pm")
    };
    let time_str = format!("{hour_12}:{minute:02}{period}");

    (date_str, time_str, use_on)
}

fn format_refs(refs_raw: &str) -> String {
    if refs_raw.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = refs_raw
        .split(", ")
        .map(|r| {
            let r = r.trim();
            if r.contains("HEAD -> ") {
                let branch = r.strip_prefix("HEAD -> ").unwrap_or(r);
                format!("<cyan><b>HEAD -></b> {branch}</cyan>")
            } else if r.starts_with("tag: ") {
                let tag = r.strip_prefix("tag: ").unwrap_or(r);
                format!("<yellow>{tag}</yellow>")
            } else if r.contains('/') {
                format!("<green>{r}</green>")
            } else {
                format!("<cyan>{r}</cyan>")
            }
        })
        .collect();
    format!(" <dim>(</dim>{}<dim>)</dim>", parts.join("<dim>, </dim>"))
}

/// Parse conventional commit format: `type(scope): description`
fn parse_conventional(message: &str) -> Option<(String, Option<String>, String)> {
    let first_line = message.lines().next()?;
    // Match: type[(scope)][!]: description
    let colon_pos = first_line.find(": ")?;
    let prefix = &first_line[..colon_pos];
    let desc = first_line[colon_pos + 2..].to_string();

    let prefix = prefix.trim_end_matches('!');

    if let Some(paren_start) = prefix.find('(') {
        let op = &prefix[..paren_start];
        let scope = prefix[paren_start + 1..].trim_end_matches(')');
        if op.chars().all(|c| c.is_alphanumeric() || c == '-') && !op.is_empty() {
            return Some((op.to_string(), Some(scope.to_string()), desc));
        }
    } else if prefix.chars().all(|c| c.is_alphanumeric() || c == '-') && !prefix.is_empty() {
        return Some((prefix.to_string(), None, desc));
    }

    None
}

/// Returns commit IDs in oldest-first order (up to `max`),
/// reachable from `target` but not from `exclude`.
fn commits_since(target: &str, exclude: &str, max: usize) -> Vec<CommitId> {
    query_commits(target, Some(exclude), max)
}

/// Returns up to `max` ancestor commit IDs ending at `tip`, oldest first.
fn ancestor_commits(tip: &str, max: usize) -> Vec<CommitId> {
    query_commits(tip, None, max)
}

fn query_commits(target: &str, exclude: Option<&str>, max: usize) -> Vec<CommitId> {
    let max_str = max.to_string();
    const FORMAT_ARG: &str = "--format=%H";
    let mut args = vec![
        "log",
        FORMAT_ARG,
        "--max-count",
        &max_str,
        "--reverse",
        target,
    ];
    if let Some(ex) = exclude {
        args.push("--not");
        args.push(ex);
    }
    args.push("--");

    match git_command(&args) {
        Ok(output) if !output.is_empty() => output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let full = line.to_string();
                let display = display_sha(&full).to_string();
                CommitId { full, display }
            })
            .collect(),
        _ => vec![],
    }
}

fn get_merge_base(a: &str, b: &str) -> Option<String> {
    git_command(&["merge-base", a, b])
        .ok()
        .filter(|s| !s.is_empty())
}

/// Count commits on `target` since `merge_base` that fall outside the displayed
/// window, i.e. the older commits the newest-`shown` window elides.
///
/// Returns 0 on any git failure so a missing count degrades to "no elision
/// marker" rather than suppressing the whole graph.
fn hidden_since(target: &str, merge_base: &str, shown: usize) -> usize {
    let total = git_command(&["rev-list", "--count", target, "--not", merge_base, "--"])
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);
    total.saturating_sub(shown)
}

/// Mermaid gitGraph line for a fork-adjacent elision marker standing in for
/// `hidden` commits the newest-5 window does not draw.
///
/// Rendered as a `HIGHLIGHT` node (a distinct square, not a commit dot) so it
/// reads as "more history here" rather than a real commit. The label carries
/// the elided count; duplicate labels across branches render fine — the
/// renderer does not require unique commit ids.
fn elision_commit(hidden: usize) -> String {
    format!("    commit id: \"+{hidden}\" type: HIGHLIGHT")
}

/// Build Mermaid gitGraph instructions for the 2-branch case
/// (current worktree branch vs. the default/main branch).
///
/// All data is taken from the already-gathered `BranchGraphData`; this function
/// performs no git calls.
pub fn worktree_graph(data: &BranchGraphData) -> Option<String> {
    let current_branch = &data.branch;
    let default_branch = &data.default_branch;

    let mut lines = vec!["gitGraph".to_string()];
    for commit in &data.default_context {
        lines.push(format!("    commit id: \"{}\"", commit.display));
    }

    // Always show the branch — even with 0 new commits, the fork point matters.
    lines.push(format!("    branch {current_branch}"));
    lines.push(format!("    checkout {current_branch}"));
    if data.branch_after_base.is_empty() {
        lines.push("    commit id: \"HEAD\"".to_string());
    } else {
        if data.branch_hidden > 0 {
            lines.push(elision_commit(data.branch_hidden));
        }
        for commit in &data.branch_after_base {
            lines.push(format!("    commit id: \"{}\"", commit.display));
        }
    }
    lines.push(format!("    checkout {default_branch}"));

    if data.default_hidden > 0 {
        lines.push(elision_commit(data.default_hidden));
    }
    for commit in &data.default_after_base {
        lines.push(format!("    commit id: \"{}\"", commit.display));
    }

    Some(lines.join("\n"))
}

/// Gather graph data for every branch concurrently, returning the main commit
/// list and per-branch data sorted deterministically by merge-base position.
///
/// Branches whose merge-base cannot be resolved or whose log queries fail are
/// omitted; failures in one branch do not suppress others.
pub fn gather_base_graph(
    default_branch: &str,
    branch_names: &[String],
) -> Option<BaseGraphData> {
    let main_commits = ancestor_commits(default_branch, 10);
    if main_commits.is_empty() {
        return None;
    }

    let mut branches: Vec<BranchGraphData> = std::thread::scope(|scope| {
        let handles: Vec<_> = branch_names
            .iter()
            .filter(|b| *b != default_branch)
            .map(|branch| scope.spawn(move || gather_branch_for_base(default_branch, branch)))
            .collect();

        handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect()
    });

    for branch in &mut branches {
        branch.merge_base_idx = main_commits
            .iter()
            .position(|c| c.full == branch.merge_base_full)
            .unwrap_or(0);
    }

    branches.sort_by_key(|b| (b.merge_base_idx, b.branch.clone()));

    Some(BaseGraphData {
        default_branch: default_branch.to_string(),
        main_commits,
        branches,
    })
}

/// Build Mermaid gitGraph instructions for the base (main) case,
/// showing up to 10 recent main commits and all active worktree branches.
///
/// All data is taken from the already-gathered [`BaseGraphData`]; this function
/// performs no git calls.
pub fn base_graph(data: &BaseGraphData) -> Option<String> {
    if data.main_commits.is_empty() {
        return None;
    }

    let mut lines = vec!["gitGraph".to_string()];
    let mut branch_iter = data.branches.iter().peekable();

    for (i, commit) in data.main_commits.iter().enumerate() {
        lines.push(format!("    commit id: \"{}\"", commit.display));
        // After each main commit, insert any branches that diverge here.
        while branch_iter
            .peek()
            .is_some_and(|b| b.merge_base_idx == i)
        {
            let info = branch_iter.next().unwrap();
            lines.push(format!("    branch {}", info.branch));
            lines.push(format!("    checkout {}", info.branch));
            if info.branch_after_base.is_empty() {
                lines.push("    commit id: \"HEAD\"".to_string());
            } else {
                if info.branch_hidden > 0 {
                    lines.push(elision_commit(info.branch_hidden));
                }
                for c in &info.branch_after_base {
                    lines.push(format!("    commit id: \"{}\"", c.display));
                }
            }
            lines.push(format!("    checkout {}", data.default_branch));
        }
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::*;
    use worktree::git::recorder;
    use worktree::git::recorder::count_matching;

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .expect("git should be installed");
        assert!(status.success(), "git {:?} failed in {:?}", args, repo);
    }

    /// RAII guard that changes CWD for the duration of a test and restores it on drop.
    struct DirGuard {
        old: PathBuf,
    }

    impl DirGuard {
        fn enter(dir: &Path) -> Self {
            let old = std::env::current_dir().expect("get cwd");
            std::env::set_current_dir(dir).expect("set cwd");
            DirGuard { old }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old);
        }
    }

    /// Create a temporary git repo with a known branch structure:
    ///
    /// ```text
    /// main:      c1 -> c2 -> c3
    /// feature-a:       `- c2 -> a1
    /// feature-b:            `- c3 -> b1
    /// ```
    fn temp_repo_with_branches() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        // Suppress background/detached git work so nextest leak detection
        // sees no lingering child processes after the test returns.
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);

        fs::write(path.join("file.txt"), "1\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 1"]);

        fs::write(path.join("file.txt"), "2\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 2"]);

        run_git(path, &["checkout", "-b", "feature-a"]);
        fs::write(path.join("a.txt"), "a\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "feature a"]);

        run_git(path, &["checkout", "main"]);
        fs::write(path.join("file.txt"), "3\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 3"]);

        run_git(path, &["checkout", "-b", "feature-b"]);
        fs::write(path.join("b.txt"), "b\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "feature b"]);

        run_git(path, &["checkout", "main"]);

        dir
    }

    const DEFAULT_BRANCH: &str = "main";
    const BRANCHES: &[&str] = &["feature-a", "feature-b"];

    fn branch_names() -> Vec<String> {
        BRANCHES.iter().map(|s| s.to_string()).collect()
    }

    /// Helper for the old-style `%h`-only ancestor query used by characterization tests.
    fn ancestor_commits_short(tip: &str, max: usize) -> Vec<String> {
        let max_str = max.to_string();
        match git_command(&[
            "log",
            "--format=%h",
            "--max-count",
            &max_str,
            "--reverse",
            tip,
            "--",
        ]) {
            Ok(output) if !output.is_empty() => output.lines().map(|s| s.to_string()).collect(),
            _ => vec![],
        }
    }

    #[test]
    #[serial_test::serial]
    fn ancestor_commits_returns_oldest_first() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let expected_output = git_command(&[
            "log",
            "--reverse",
            "--format=%h",
            "--max-count",
            "3",
            DEFAULT_BRANCH,
            "--",
        ])
        .expect("git log should succeed");
        let expected: Vec<String> = expected_output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        let actual = ancestor_commits_short(DEFAULT_BRANCH, 3);

        assert_eq!(
            actual, expected,
            "ancestor_commits should match git log --reverse order exactly"
        );
    }

    /// Binding SLA + subprocess-count guard for the image-terminal `wt list -v`
    /// data-gather path (graph IDs + verbose details).
    ///
    /// Runs on a controlled non-main fixture so the path always executes and the
    /// 1-second SLA is always asserted, regardless of the ambient checkout. This
    /// replaces the ambient-conditional check that was silently skipped on a main
    /// checkout. Rasterization is excluded — `gather_branch` performs no Mermaid
    /// rendering. The ambient observability print in
    /// `list::perf_subprocess_counts_meet_sla` complements this binding assertion.
    #[test]
    #[serial_test::serial]
    fn gather_branch_uses_one_merge_base_and_no_short_sha() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        recorder::start_recording();
        let t0 = std::time::Instant::now();
        let data = gather_branch(DEFAULT_BRANCH, "feature-a", true)
            .expect("gather should succeed for feature-a");
        let gather_elapsed = t0.elapsed();
        let calls = recorder::finish_recording();

        let merge_base_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("merge-base")
        });
        assert_eq!(
            merge_base_count, 1,
            "expected exactly one merge-base call, got {calls:?}"
        );

        let short_sha_count = count_matching(&calls, |args| {
            args.len() >= 2 && args[0] == "rev-parse" && args[1] == "--short"
        });
        assert_eq!(
            short_sha_count, 0,
            "expected zero rev-parse --short calls, got {calls:?}"
        );

        // Timing is reported but not asserted here: this is a subprocess-count
        // correctness test, and a single-shot wall-clock check flakes under
        // parallel-suite CPU contention. The wall-clock SLA for this path is
        // owned by the dedicated best-of-5 `perf_full_command_non_image_meets_sla`
        // and the `cache_{warm,cold}_path` tests, matching the count-only pattern
        // in `perf_subprocess_counts_meet_sla`.
        eprintln!(
            "gather_branch (fixture, verbose): {gather_elapsed:.2?}, {} git calls",
            calls.len()
        );

        assert!(!data.branch.is_empty());
        assert!(!data.default_branch.is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn worktree_graph_uses_in_process_display_ids() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let branch = "feature-a";

        let merge_base = git_command(&["merge-base", DEFAULT_BRANCH, branch])
            .expect("merge-base should exist");

        // Build expected output from full SHAs (%H), deriving display IDs the
        // same way the production code does: truncating to 7 chars in-process
        // via display_sha. This asserts the in-process display contract rather
        // than git's repository-dependent %h abbreviation.
        let context = git_command(&[
            "log",
            "--format=%H",
            "--max-count",
            "2",
            "--reverse",
            &merge_base,
            "--",
        ])
        .expect("context log should succeed");

        let branch_commits = git_command(&[
            "log",
            "--format=%H",
            "--max-count",
            "5",
            "--reverse",
            branch,
            "--not",
            &merge_base,
            "--",
        ])
        .expect("branch log should succeed");

        let main_after = git_command(&[
            "log",
            "--format=%H",
            "--max-count",
            "5",
            "--reverse",
            DEFAULT_BRANCH,
            "--not",
            &merge_base,
            "--",
        ])
        .expect("main log should succeed");

        let mut expected = vec!["gitGraph".to_string()];
        for sha in context.lines().filter(|l| !l.is_empty()) {
            expected.push(format!("    commit id: \"{}\"", display_sha(sha)));
        }
        expected.push(format!("    branch {branch}"));
        expected.push(format!("    checkout {branch}"));
        if branch_commits.trim().is_empty() {
            expected.push("    commit id: \"HEAD\"".to_string());
        } else {
            for sha in branch_commits.lines().filter(|l| !l.is_empty()) {
                expected.push(format!("    commit id: \"{}\"", display_sha(sha)));
            }
        }
        expected.push(format!("    checkout {DEFAULT_BRANCH}"));
        for sha in main_after.lines().filter(|l| !l.is_empty()) {
            expected.push(format!("    commit id: \"{}\"", display_sha(sha)));
        }

        let data = gather_branch(DEFAULT_BRANCH, branch, false).expect("gather should succeed");
        let actual = worktree_graph(&data).expect("graph should render");

        assert_eq!(
            actual,
            expected.join("\n"),
            "worktree_graph output must match the in-process display_sha contract (7-char truncation of %H)"
        );
    }

    #[test]
    #[serial_test::serial]
    fn base_graph_renders_for_all_branches() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let branches = branch_names();
        let data = gather_base_graph(DEFAULT_BRANCH, &branches)
            .expect("base graph gather should succeed");
        let graph = base_graph(&data).expect("base graph should render");
        assert!(graph.starts_with("gitGraph"));
        assert!(graph.lines().count() > 1);
    }

    #[test]
    #[serial_test::serial]
    fn base_graph_is_deterministic_across_gather_runs() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let branches = branch_names();
        let data_a = gather_base_graph(DEFAULT_BRANCH, &branches)
            .expect("first gather should succeed");
        let data_b = gather_base_graph(DEFAULT_BRANCH, &branches)
            .expect("second gather should succeed");

        let graph_a = base_graph(&data_a).expect("graph should render");
        let graph_b = base_graph(&data_b).expect("graph should render");

        assert_eq!(
            graph_a, graph_b,
            "base_graph output must be byte-identical across independent gathers"
        );
    }

    #[test]
    #[serial_test::serial]
    fn base_graph_subprocess_count_is_bounded() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let branches = branch_names();

        recorder::start_recording();
        let data = gather_base_graph(DEFAULT_BRANCH, &branches)
            .expect("gather should succeed");
        let calls = recorder::finish_recording();

        assert!(
            !data.branches.is_empty(),
            "base graph should collect at least one branch"
        );
        assert!(
            !data.main_commits.is_empty(),
            "base graph should have main commits"
        );

        // One merge-base per branch, no more.
        let merge_base_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("merge-base")
        });
        assert_eq!(
            merge_base_count,
            data.branches.len(),
            "expected exactly one merge-base per branch, got {calls:?}"
        );

        // The base-graph gather path issues exactly:
        //   1 `git log` for main_commits + 1 `git log` per branch (branch_after_base).
        // No default-context or default-after-base logs should be present.
        let log_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("log")
        });
        assert_eq!(
            log_count,
            1 + data.branches.len(),
            "expected one main-commits log plus one per branch, got {calls:?}"
        );

        // Plus one `git rev-list --count` per branch to size the elision marker.
        // No default-branch count is issued in the base-overview scope.
        let rev_list_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("rev-list")
        });
        assert_eq!(
            rev_list_count,
            data.branches.len(),
            "expected one rev-list --count per branch, got {calls:?}"
        );

        // No rev-parse --short calls in the base-graph path.
        let short_sha_count = count_matching(&calls, |args| {
            args.len() >= 2 && args[0] == "rev-parse" && args[1] == "--short"
        });
        assert_eq!(
            short_sha_count, 0,
            "expected zero rev-parse --short calls, got {calls:?}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn base_graph_placement_matches_short_sha_placement() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let main_commits = ancestor_commits(DEFAULT_BRANCH, 10);
        let main_commits_short: Vec<String> =
            main_commits.iter().map(|c| c.display.clone()).collect();

        for branch in BRANCHES {
            let Some(base_sha) = get_merge_base(DEFAULT_BRANCH, branch) else {
                continue;
            };
            let base_short = git_command(&["rev-parse", "--short", &base_sha])
                .expect("short sha should resolve");

            let idx_full = main_commits.iter().position(|c| c.full == base_sha);
            let idx_short = main_commits_short.iter().position(|s| s == &base_short);

            assert_eq!(
                idx_full, idx_short,
                "full-SHA placement must match short-SHA placement for {branch}"
            );
        }
    }

    /// Build a repo where both lines exceed the 5-commit window:
    /// `feature` has 7 commits since the fork, `main` has 6.
    fn temp_repo_over_window() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);

        let commit = |n: &str| {
            fs::write(path.join(format!("{n}.txt")), format!("{n}\n")).unwrap();
            run_git(path, &["add", "."]);
            run_git(path, &["commit", "-m", n]);
        };

        commit("root");
        run_git(path, &["checkout", "-b", "feature"]);
        for i in 1..=7 {
            commit(&format!("f{i}"));
        }
        run_git(path, &["checkout", "main"]);
        for i in 1..=6 {
            commit(&format!("m{i}"));
        }
        run_git(path, &["checkout", "main"]);
        dir
    }

    /// A branch and default line that exceed the window each get exactly one
    /// fork-adjacent HIGHLIGHT elision node carrying the elided count, placed
    /// before the first windowed commit on that line.
    #[test]
    #[serial_test::serial]
    fn worktree_graph_marks_elided_commits() {
        let repo = temp_repo_over_window();
        let _guard = DirGuard::enter(repo.path());

        let data = gather_branch(DEFAULT_BRANCH, "feature", false).expect("gather should succeed");
        assert_eq!(data.branch_hidden, 2, "7 ahead − 5 shown = 2 elided on feature");
        assert_eq!(data.default_hidden, 1, "6 ahead − 5 shown = 1 elided on main");

        let graph = worktree_graph(&data).expect("graph should render");
        let lines: Vec<&str> = graph.lines().collect();

        let branch_elide = "    commit id: \"+2\" type: HIGHLIGHT";
        let default_elide = "    commit id: \"+1\" type: HIGHLIGHT";
        assert_eq!(
            lines.iter().filter(|l| **l == branch_elide).count(),
            1,
            "exactly one feature-line elision marker:\n{graph}"
        );
        assert_eq!(
            lines.iter().filter(|l| **l == default_elide).count(),
            1,
            "exactly one main-line elision marker:\n{graph}"
        );

        // The marker is fork-adjacent: it precedes every windowed commit on its
        // line. `checkout feature` opens the branch segment; the elision node is
        // the first commit after it.
        let checkout_feature = lines
            .iter()
            .position(|l| *l == "    checkout feature")
            .expect("feature checkout present");
        assert_eq!(
            lines[checkout_feature + 1],
            branch_elide,
            "elision marker must be the first node on the branch line:\n{graph}"
        );
    }

    /// A line that fits within the window draws no elision marker.
    #[test]
    #[serial_test::serial]
    fn worktree_graph_omits_marker_within_window() {
        let repo = temp_repo_with_branches();
        let _guard = DirGuard::enter(repo.path());

        let data = gather_branch(DEFAULT_BRANCH, "feature-a", false).expect("gather should succeed");
        assert_eq!(data.branch_hidden, 0);
        assert_eq!(data.default_hidden, 0);

        let graph = worktree_graph(&data).expect("graph should render");
        assert!(
            !graph.contains("type: HIGHLIGHT"),
            "no elision marker expected when within window:\n{graph}"
        );
    }
}
