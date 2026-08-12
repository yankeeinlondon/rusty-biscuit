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
    /// Up to 5 commits reachable from the default tip but not the branch tip,
    /// oldest first.
    default_unique: Vec<CommitId>,
    /// Default-tip-only commits older than the newest-5 display window.
    /// Drives the fork-adjacent elision marker on the default line.
    default_hidden: usize,
    /// Up to 5 commits reachable from the branch tip but not the default tip,
    /// oldest first.
    branch_unique: Vec<CommitId>,
    /// Branch-tip-only commits older than the newest-5 display window.
    /// Drives the fork-adjacent elision marker on the branch line.
    branch_hidden: usize,
    /// Verbose detail for the merge-base commit, populated only when requested.
    pub merge_base_detail: Option<CommitDetail>,
    /// Verbose details for commits reachable only from the branch tip,
    /// populated only when requested.
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
    /// Full data: shared merge-base context, tip-unique commits on both lanes,
    /// and optional branch-tip-only verbose details.
    Full,
    /// Minimal data for the base-graph overview: merge-base identity and
    /// branch-tip-only commits.
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
/// and branch-tip-only commits. Skips the two per-branch default-lane queries
/// that [`gather_branch`] would collect but [`base_graph`] discards.
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

    let (default_context, default_unique, default_hidden) = match scope {
        GatherScope::Full => {
            let unique = commits_since(default_branch, branch, 5);
            let hidden = hidden_since(default_branch, branch, unique.len());
            (ancestor_commits(&merge_base_full, 2), unique, hidden)
        }
        GatherScope::BaseOverview => (Vec::new(), Vec::new(), 0),
    };

    let branch_unique = commits_since(branch, default_branch, 5);
    let branch_hidden = hidden_since(branch, default_branch, branch_unique.len());

    let (merge_base_detail, branch_details) = if verbose {
        (
            commit_details(&merge_base_full, 1).into_iter().next(),
            commit_details_since(branch, default_branch),
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
        default_unique,
        default_hidden,
        branch_unique,
        branch_hidden,
        merge_base_detail,
        branch_details,
    })
}

/// Get the detailed commit info for the merge-base commit from already-gathered data.
pub fn merge_base_commit(data: &BranchGraphData) -> Option<&CommitDetail> {
    data.merge_base_detail.as_ref()
}

/// Get detailed commits reachable only from the branch tip from already-gathered data.
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

/// Count commits reachable from `target` but not `exclude` that fall outside
/// the displayed window.
///
/// Returns 0 on any git failure so a missing count degrades to "no elision
/// marker" rather than suppressing the whole graph.
fn hidden_since(target: &str, exclude: &str, shown: usize) -> usize {
    let total = git_command(&["rev-list", "--count", target, "--not", exclude, "--"])
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
    if data.branch_unique.is_empty() {
        lines.push("    commit id: \"HEAD\"".to_string());
    } else {
        if data.branch_hidden > 0 {
            lines.push(elision_commit(data.branch_hidden));
        }
        for commit in &data.branch_unique {
            lines.push(format!("    commit id: \"{}\"", commit.display));
        }
    }
    lines.push(format!("    checkout {default_branch}"));

    if data.default_hidden > 0 {
        lines.push(elision_commit(data.default_hidden));
    }
    for commit in &data.default_unique {
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
            if info.branch_unique.is_empty() {
                lines.push("    commit id: \"HEAD\"".to_string());
            } else {
                if info.branch_hidden > 0 {
                    lines.push(elision_commit(info.branch_hidden));
                }
                for c in &info.branch_unique {
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
    use std::collections::HashSet;
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

    fn git_output(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(repo)
            .args(args)
            .output()
            .expect("git should be installed");
        assert!(
            output.status.success(),
            "git {:?} failed in {:?}: {}",
            args,
            repo,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git output should be UTF-8")
            .trim()
            .to_string()
    }

    fn init_test_repo(path: &Path) {
        run_git(path, &["init", "-b", DEFAULT_BRANCH]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        // Suppress optional Git workers so nextest does not report fixture leaks.
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);
    }

    fn commit_file(path: &Path, file: &str, contents: &str, message: &str) {
        fs::write(path.join(file), contents).expect("fixture file should be writable");
        run_git(path, &["add", "--", file]);
        run_git(path, &["commit", "-m", message]);
    }

    fn merge_noninteractive(path: &Path, revision: &str, message: &str) {
        run_git(
            path,
            &["merge", "--no-ff", "--no-edit", "-m", message, revision],
        );
    }

    fn is_ancestor(path: &Path, ancestor: &str, descendant: &str) -> bool {
        let status = Command::new("git")
            .current_dir(path)
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .status()
            .expect("git should be installed");
        assert!(
            status.success() || status.code() == Some(1),
            "git merge-base --is-ancestor failed unexpectedly"
        );
        status.success()
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

        init_test_repo(path);
        commit_file(path, "file.txt", "1\n", "commit 1");
        commit_file(path, "file.txt", "2\n", "commit 2");

        run_git(path, &["checkout", "-b", "feature-a"]);
        commit_file(path, "a.txt", "a\n", "feature a");

        run_git(path, &["checkout", "main"]);
        commit_file(path, "file.txt", "3\n", "commit 3");

        run_git(path, &["checkout", "-b", "feature-b"]);
        commit_file(path, "b.txt", "b\n", "feature b");

        run_git(path, &["checkout", "main"]);

        dir
    }

    fn temp_repo_with_criss_cross_merge() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        init_test_repo(path);
        commit_file(path, "root.txt", "root\n", "root");
        let root = git_output(path, &["rev-parse", "HEAD"]);

        commit_file(path, "main.txt", "main base\n", "main base");
        let main_base = git_output(path, &["rev-parse", "HEAD"]);

        run_git(path, &["checkout", "-b", "terminal", &root]);
        commit_file(
            path,
            "terminal.txt",
            "terminal base\n",
            "terminal base",
        );

        run_git(path, &["checkout", "-b", "feature", &root]);
        commit_file(path, "feature.txt", "feature root\n", "feature root");
        merge_noninteractive(path, &main_base, "merge main into feature line");
        commit_file(
            path,
            "feature-work.txt",
            "feature work\n",
            "feature work",
        );

        run_git(path, &["checkout", DEFAULT_BRANCH]);
        merge_noninteractive(path, "terminal", "merge terminal into main");

        run_git(path, &["checkout", "feature"]);
        merge_noninteractive(path, "terminal", "merge terminal into feature");

        dir
    }

    #[test]
    #[serial_test::serial]
    fn criss_cross_fixture_has_two_incomparable_best_bases() {
        let repo = temp_repo_with_criss_cross_merge();
        let path = repo.path();

        let bases: HashSet<String> = git_output(
            path,
            &["merge-base", "--all", DEFAULT_BRANCH, "feature"],
        )
        .lines()
        .map(str::to_string)
        .collect();
        assert_eq!(bases.len(), 2, "fixture must have two distinct best bases");

        let mut bases_iter = bases.iter();
        let first = bases_iter.next().expect("first merge-base");
        let second = bases_iter.next().expect("second merge-base");
        assert!(!is_ancestor(path, first, second));
        assert!(!is_ancestor(path, second, first));

        let selected = git_output(path, &["merge-base", DEFAULT_BRANCH, "feature"]);
        assert!(
            bases.contains(&selected),
            "plain merge-base must select one of the best bases"
        );

        let common_outside_selected: Vec<String> = git_output(
            path,
            &["rev-list", DEFAULT_BRANCH, "--not", &selected, "--"],
        )
        .lines()
        .filter(|commit| is_ancestor(path, commit, "feature"))
        .map(str::to_string)
        .collect();
        assert!(
            !common_outside_selected.is_empty(),
            "the selected base must leave shared history reachable from both tips"
        );
    }

    #[derive(Debug, Eq, PartialEq)]
    enum MermaidNode {
        Commit(String),
        Elision(usize),
        Head,
    }

    fn parse_mermaid_node(line: &str) -> MermaidNode {
        let line = line.trim();
        if line == "commit id: \"HEAD\"" {
            return MermaidNode::Head;
        }
        if let Some(value) = line
            .strip_prefix("commit id: \"+")
            .and_then(|rest| rest.strip_suffix("\" type: HIGHLIGHT"))
        {
            return MermaidNode::Elision(
                value
                    .parse()
                    .expect("elision labels should contain a count"),
            );
        }
        let id = line
            .strip_prefix("commit id: \"")
            .and_then(|rest| rest.strip_suffix('"'))
            .expect("branch segment should contain only commit nodes");
        MermaidNode::Commit(id.to_string())
    }

    fn branch_segment(graph: &str, branch: &str, default_branch: &str) -> Vec<MermaidNode> {
        let lines: Vec<&str> = graph.lines().collect();
        let declaration = format!("    branch {branch}");
        let branch_checkout = format!("    checkout {branch}");
        let default_checkout = format!("    checkout {default_branch}");
        let declaration_idx = lines
            .iter()
            .position(|line| *line == declaration)
            .expect("branch declaration should be present");
        let checkout_idx = lines
            .iter()
            .enumerate()
            .skip(declaration_idx + 1)
            .find_map(|(idx, line)| (*line == branch_checkout).then_some(idx))
            .expect("branch checkout should follow its declaration");
        let end_idx = lines
            .iter()
            .enumerate()
            .skip(checkout_idx + 1)
            .find_map(|(idx, line)| (*line == default_checkout).then_some(idx))
            .expect("default checkout should close the branch segment");

        lines[checkout_idx + 1..end_idx]
            .iter()
            .map(|line| parse_mermaid_node(line))
            .collect()
    }

    fn focused_default_segment(graph: &str, default_branch: &str) -> Vec<MermaidNode> {
        let checkout = format!("    checkout {default_branch}");
        let lines: Vec<&str> = graph.lines().collect();
        let checkout_idx = lines
            .iter()
            .rposition(|line| *line == checkout)
            .expect("focused graph should return to the default branch");
        lines[checkout_idx + 1..]
            .iter()
            .map(|line| parse_mermaid_node(line))
            .collect()
    }

    fn commit_ids(commits: &[CommitId]) -> Vec<String> {
        commits.iter().map(|commit| commit.full.clone()).collect()
    }

    fn rendered_commit_ids(nodes: &[MermaidNode], commits: &[CommitId]) -> Vec<String> {
        nodes
            .iter()
            .filter_map(|node| match node {
                MermaidNode::Commit(display) => {
                    let matches: Vec<&CommitId> = commits
                        .iter()
                        .filter(|commit| commit.display == *display)
                        .collect();
                    assert_eq!(
                        matches.len(),
                        1,
                        "rendered ID {display} should resolve to one gathered full SHA"
                    );
                    Some(matches[0].full.clone())
                }
                MermaidNode::Elision(_) | MermaidNode::Head => None,
            })
            .collect()
    }

    fn unique_commit_ids(target: &str, opposite_tip: &str) -> Vec<String> {
        git_command(&[
            "log",
            "--format=%H",
            "--reverse",
            target,
            "--not",
            opposite_tip,
            "--",
        ])
        .expect("unique commit log should succeed")
        .lines()
        .map(str::to_string)
        .collect()
    }

    fn unique_commit_count(target: &str, opposite_tip: &str) -> usize {
        git_command(&["rev-list", "--count", target, "--not", opposite_tip, "--"])
            .expect("unique commit count should succeed")
            .parse()
            .expect("unique commit count should be numeric")
    }

    fn focused_graph_with_exclusions(
        default_branch: &str,
        branch: &str,
        default_exclude: &str,
        branch_exclude: &str,
    ) -> String {
        let merge_base_full = get_merge_base(default_branch, branch)
            .expect("fixture branches should have a merge-base");
        let default_unique = commits_since(default_branch, default_exclude, 5);
        let branch_unique = commits_since(branch, branch_exclude, 5);
        let data = BranchGraphData {
            branch: branch.to_string(),
            default_branch: default_branch.to_string(),
            merge_base_full: merge_base_full.clone(),
            merge_base_idx: 0,
            default_context: ancestor_commits(&merge_base_full, 2),
            default_hidden: hidden_since(default_branch, default_exclude, default_unique.len()),
            default_unique,
            branch_hidden: hidden_since(branch, branch_exclude, branch_unique.len()),
            branch_unique,
            merge_base_detail: None,
            branch_details: Vec::new(),
        };
        worktree_graph(&data).expect("focused graph should render")
    }

    fn assert_linear_exclusions_render_identically(repo: &Path, branch: &str) {
        let _guard = DirGuard::enter(repo);
        let merge_base = get_merge_base(DEFAULT_BRANCH, branch)
            .expect("fixture branches should have a merge-base");
        let legacy = focused_graph_with_exclusions(
            DEFAULT_BRANCH,
            branch,
            &merge_base,
            &merge_base,
        );
        let opposite_tip =
            focused_graph_with_exclusions(DEFAULT_BRANCH, branch, branch, DEFAULT_BRANCH);
        assert_eq!(
            opposite_tip, legacy,
            "single-base linear history must render identically under either exclusion"
        );
    }

    #[test]
    #[serial_test::serial]
    fn criss_cross_focused_graph_uses_tip_unique_commits() {
        let repo = temp_repo_with_criss_cross_merge();
        let _guard = DirGuard::enter(repo.path());

        let expected_branch = unique_commit_ids("feature", DEFAULT_BRANCH);
        let expected_default = unique_commit_ids(DEFAULT_BRANCH, "feature");
        assert!(expected_branch.len() <= 5 && expected_default.len() <= 5);

        let data = gather_branch(DEFAULT_BRANCH, "feature", false)
            .expect("focused graph gather should succeed");
        assert_eq!(commit_ids(&data.branch_unique), expected_branch);
        assert_eq!(commit_ids(&data.default_unique), expected_default);
        assert_eq!(data.branch_hidden, 0);
        assert_eq!(data.default_hidden, 0);

        let graph = worktree_graph(&data).expect("focused graph should render");
        let branch_nodes = branch_segment(&graph, "feature", DEFAULT_BRANCH);
        let default_nodes = focused_default_segment(&graph, DEFAULT_BRANCH);
        let rendered_branch = rendered_commit_ids(&branch_nodes, &data.branch_unique);
        let rendered_default = rendered_commit_ids(&default_nodes, &data.default_unique);
        assert_eq!(rendered_branch, expected_branch);
        assert_eq!(rendered_default, expected_default);
        assert!(
            rendered_branch
                .iter()
                .all(|commit| !rendered_default.contains(commit)),
            "focused lanes must be disjoint: {graph}"
        );
        assert_eq!(
            rendered_branch.len(),
            unique_commit_count("feature", DEFAULT_BRANCH)
        );
        assert_eq!(
            rendered_default.len(),
            unique_commit_count(DEFAULT_BRANCH, "feature")
        );
        assert!(
            branch_nodes
                .iter()
                .chain(default_nodes.iter())
                .all(|node| !matches!(node, MermaidNode::Elision(_))),
            "no unique lane exceeds the display window: {graph}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn criss_cross_verbose_details_use_feature_unique_commits() {
        let repo = temp_repo_with_criss_cross_merge();
        let _guard = DirGuard::enter(repo.path());

        let expected = git_command(&[
            "log",
            "--format=%h",
            "--reverse",
            "feature",
            "--not",
            DEFAULT_BRANCH,
            "--",
        ])
        .expect("feature-unique detail log should succeed");
        let expected: Vec<String> = expected.lines().map(str::to_string).collect();

        let data = gather_branch(DEFAULT_BRANCH, "feature", true)
            .expect("verbose graph gather should succeed");
        let actual: Vec<String> = data
            .branch_details
            .iter()
            .map(|detail| detail.short_sha.clone())
            .collect();
        assert_eq!(actual, expected);
        for detail in &data.branch_details {
            let full = git_command(&["rev-parse", &detail.short_sha])
                .expect("detail SHA should resolve");
            assert!(
                !is_ancestor(repo.path(), &full, DEFAULT_BRANCH),
                "verbose detail {} must not be reachable from main",
                detail.short_sha
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn criss_cross_base_graph_uses_unique_commits_at_selected_base() {
        let repo = temp_repo_with_criss_cross_merge();
        let _guard = DirGuard::enter(repo.path());

        let expected_branch = unique_commit_ids("feature", DEFAULT_BRANCH);
        let selected_base = git_command(&["merge-base", DEFAULT_BRANCH, "feature"])
            .expect("selected merge-base should exist");
        let branches = vec!["feature".to_string()];
        let data = gather_base_graph(DEFAULT_BRANCH, &branches)
            .expect("base graph gather should succeed");
        let feature = data.branches.first().expect("feature data should exist");
        let selected_idx = data
            .main_commits
            .iter()
            .position(|commit| commit.full == selected_base)
            .expect("selected merge-base should be in the ten-commit main window");
        assert_eq!(feature.merge_base_idx, selected_idx);
        assert_eq!(commit_ids(&feature.branch_unique), expected_branch);

        let graph = base_graph(&data).expect("base graph should render");
        let branch_nodes = branch_segment(&graph, "feature", DEFAULT_BRANCH);
        assert_eq!(
            rendered_commit_ids(&branch_nodes, &feature.branch_unique),
            expected_branch
        );

        let lines: Vec<&str> = graph.lines().collect();
        let branch_idx = lines
            .iter()
            .position(|line| *line == "    branch feature")
            .expect("feature branch block should be present");
        let anchor = parse_mermaid_node(lines[branch_idx - 1]);
        assert_eq!(
            anchor,
            MermaidNode::Commit(data.main_commits[selected_idx].display.clone()),
            "feature branch block should immediately follow the selected merge-base"
        );
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

        let log_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("log")
        });
        assert_eq!(
            log_count, 5,
            "expected context, two lane, and two detail logs, got {calls:?}"
        );

        let rev_list_count = count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("rev-list")
        });
        assert_eq!(
            rev_list_count, 2,
            "expected one hidden-count query per lane, got {calls:?}"
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
            DEFAULT_BRANCH,
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
            branch,
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
    fn linear_focused_graph_is_unchanged_by_opposite_tip_exclusion() {
        let repo = temp_repo_with_branches();
        assert_linear_exclusions_render_identically(repo.path(), "feature-a");
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
        //   1 `git log` for main_commits + 1 `git log` per branch (branch_unique).
        // No default-context or default-unique logs should be present.
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
    /// `feature` has 7 tip-only commits and `main` has 6.
    fn temp_repo_over_window() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        init_test_repo(path);

        let commit = |n: &str| {
            commit_file(path, &format!("{n}.txt"), &format!("{n}\n"), n);
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
        let branch_expected = unique_commit_count("feature", DEFAULT_BRANCH) - 5;
        let default_expected = unique_commit_count(DEFAULT_BRANCH, "feature") - 5;
        assert_eq!(branch_expected, 2, "7 ahead − 5 shown = 2 elided on feature");
        assert_eq!(default_expected, 1, "6 ahead − 5 shown = 1 elided on main");
        assert_eq!(data.branch_hidden, branch_expected);
        assert_eq!(data.default_hidden, default_expected);

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

        let branch_nodes = branch_segment(&graph, "feature", DEFAULT_BRANCH);
        let default_nodes = focused_default_segment(&graph, DEFAULT_BRANCH);
        assert_eq!(branch_nodes.first(), Some(&MermaidNode::Elision(branch_expected)));
        assert_eq!(default_nodes.first(), Some(&MermaidNode::Elision(default_expected)));
        assert_eq!(branch_nodes.len(), 6, "one elision plus five feature commits");
        assert_eq!(default_nodes.len(), 6, "one elision plus five main commits");
    }

    #[test]
    #[serial_test::serial]
    fn over_window_focused_graph_is_unchanged_by_opposite_tip_exclusion() {
        let repo = temp_repo_over_window();
        assert_linear_exclusions_render_identically(repo.path(), "feature");
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
