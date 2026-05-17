use chrono::{DateTime, Local, TimeZone, Timelike};
use worktree::git::git_command;

/// A parsed commit for verbose display.
pub struct CommitDetail {
    pub short_sha: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub refs: String,
}

/// Get detailed commit info for the merge-base commit.
pub fn merge_base_commit(default_branch: &str, current_branch: &str) -> Option<CommitDetail> {
    let base_sha = get_merge_base(default_branch, current_branch)?;
    let commits = commit_details(&base_sha, 1);
    commits.into_iter().next()
}

/// Get detailed commits on a branch since the merge-base.
pub fn branch_commits_detail(current_branch: &str, default_branch: &str) -> Vec<CommitDetail> {
    let Some(base_sha) = get_merge_base(default_branch, current_branch) else {
        return vec![];
    };
    commit_details_since(current_branch, &base_sha)
}

/// Git format string using %x1f (Unit Separator) as field delimiter.
/// Using git's own escape avoids embedding raw control chars in args.
const DETAIL_FMT: &str = "%h%x1f%s%x1f%at%x1f%D";

/// Query git log for detailed commit info, returning oldest first.
fn commit_details(rev: &str, max: usize) -> Vec<CommitDetail> {
    let max_str = max.to_string();
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let Ok(output) = git_command(&["log", &fmt_arg, "--max-count", &max_str, rev, "--"]) else {
        return vec![];
    };
    parse_commit_lines(&output)
}

/// Query git log for commits reachable from `target` but not `exclude`, oldest first.
fn commit_details_since(target: &str, exclude: &str) -> Vec<CommitDetail> {
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let Ok(output) = git_command(&["log", &fmt_arg, target, "--not", exclude, "--"]) else {
        return vec![];
    };
    parse_commit_lines(&output)
}

fn parse_commit_lines(output: &str) -> Vec<CommitDetail> {
    let mut commits: Vec<CommitDetail> = output
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
        .collect();
    commits.reverse(); // oldest first
    commits
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

/// Returns commit short SHAs in oldest-first order (up to `max`),
/// reachable from `target` but not from `exclude`.
fn commits_since(target: &str, exclude: &str, max: usize) -> Vec<String> {
    let max_str = max.to_string();
    match git_command(&[
        "log",
        "--format=%h",
        "--max-count",
        &max_str,
        target,
        "--not",
        exclude,
        "--",
    ]) {
        Ok(output) if !output.is_empty() => {
            let mut v: Vec<String> = output.lines().map(|s| s.to_string()).collect();
            v.reverse();
            v
        }
        _ => vec![],
    }
}

/// Returns up to `max` ancestor commits ending at `tip`, oldest first.
fn ancestor_commits(tip: &str, max: usize) -> Vec<String> {
    let max_str = max.to_string();
    match git_command(&["log", "--format=%h", "--max-count", &max_str, tip, "--"]) {
        Ok(output) if !output.is_empty() => {
            let mut v: Vec<String> = output.lines().map(|s| s.to_string()).collect();
            v.reverse();
            v
        }
        _ => vec![],
    }
}

fn get_merge_base(a: &str, b: &str) -> Option<String> {
    git_command(&["merge-base", a, b])
        .ok()
        .filter(|s| !s.is_empty())
}

fn short_sha(full: &str) -> String {
    git_command(&["rev-parse", "--short", full])
        .unwrap_or_else(|_| full[..7.min(full.len())].to_string())
}

/// Build Mermaid gitGraph instructions for the 2-branch case
/// (current worktree branch vs. the default/main branch).
pub fn worktree_graph(current_branch: &str, default_branch: &str) -> Option<String> {
    let base_sha = get_merge_base(default_branch, current_branch)?;

    // Up to 2 context commits ending at the merge-base (oldest first)
    let context = ancestor_commits(&base_sha, 2);
    // Commits on the feature branch since divergence (cap at 5)
    let branch_commits = commits_since(current_branch, &base_sha, 5);
    // Commits on main since divergence (cap at 5)
    let main_after = commits_since(default_branch, &base_sha, 5);

    let mut lines = vec!["gitGraph".to_string()];
    for sha in &context {
        lines.push(format!("    commit id: \"{sha}\""));
    }

    // Always show the branch — even with 0 new commits, the fork point matters
    lines.push(format!("    branch {current_branch}"));
    lines.push(format!("    checkout {current_branch}"));
    if branch_commits.is_empty() {
        lines.push("    commit id: \"HEAD\"".to_string());
    } else {
        for sha in &branch_commits {
            lines.push(format!("    commit id: \"{sha}\""));
        }
    }
    lines.push(format!("    checkout {default_branch}"));

    for sha in &main_after {
        lines.push(format!("    commit id: \"{sha}\""));
    }

    Some(lines.join("\n"))
}

struct BranchData {
    name: String,
    /// Index of the merge-base commit in the main commit list
    merge_base_idx: usize,
    commits: Vec<String>,
}

/// Build Mermaid gitGraph instructions for the base (main) case,
/// showing up to 10 recent main commits and all active worktree branches.
pub fn base_graph(branch_names: &[String], default_branch: &str) -> Option<String> {
    // Last 10 commits on main, oldest first
    let main_commits = ancestor_commits(default_branch, 10);
    if main_commits.is_empty() {
        return None;
    }

    let mut branch_data: Vec<BranchData> = Vec::new();
    for branch in branch_names {
        if branch == default_branch {
            continue;
        }
        let Some(base_sha) = get_merge_base(default_branch, branch) else {
            continue;
        };
        let base_short = short_sha(&base_sha);
        // If the merge-base is outside our window, anchor it at position 0
        let merge_base_idx = main_commits
            .iter()
            .position(|s| s == &base_short)
            .unwrap_or(0);
        let commits = commits_since(branch, &base_sha, 10);
        branch_data.push(BranchData {
            name: branch.clone(),
            merge_base_idx,
            commits,
        });
    }

    // Sort so branches appear in the order their divergence point appears in main
    branch_data.sort_by_key(|b| b.merge_base_idx);

    let mut lines = vec!["gitGraph".to_string()];
    let mut branch_iter = branch_data.iter().peekable();

    for (i, sha) in main_commits.iter().enumerate() {
        lines.push(format!("    commit id: \"{sha}\""));
        // After each main commit, insert any branches that diverge here
        while branch_iter.peek().is_some_and(|b| b.merge_base_idx == i) {
            let info = branch_iter.next().unwrap();
            lines.push(format!("    branch {}", info.name));
            lines.push(format!("    checkout {}", info.name));
            if info.commits.is_empty() {
                lines.push("    commit id: \"HEAD\"".to_string());
            } else {
                for c in &info.commits {
                    lines.push(format!("    commit id: \"{c}\""));
                }
            }
            lines.push(format!("    checkout {default_branch}"));
        }
    }

    Some(lines.join("\n"))
}
