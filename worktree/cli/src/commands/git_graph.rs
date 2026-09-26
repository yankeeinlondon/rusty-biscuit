//! Graph and verbose data for `wt list`.
//!
//! The graph half gathers typed facts (lines of work with full commit SHAs,
//! ref tips) and hands them to biscuit-terminal's [`GitGraph`], which owns the
//! lane/tag rule, elision, sizing, and the Mermaid text. The verbose half
//! gathers the commit details `wt list -v` prints.

use std::collections::HashSet;

use biscuit_terminal::components::git_graph::{GitGraph, GraphLine, GraphPullRequest, LaneEntry};
use biscuit_terminal::components::terminal_image::ImageWidth;
use chrono::{DateTime, Local, TimeZone, Timelike};
use worktree::fork_origin::ForkOriginStore;
use worktree::git::git_command;
use worktree::listing::RefTips;
use worktree::pull_requests::PrListing;
use worktree::worktree::WorktreeList;

/// Newest commits drawn per line; older ones fold into one `+N` square.
const LINE_WINDOW: usize = 5;
/// Newest default-branch commits in the base view.
const BASE_DEFAULT_WINDOW: usize = 10;
/// Shared commits drawn up to a focused view's fork point.
const CONTEXT_COMMITS: usize = 2;

/// A parsed commit for verbose display.
pub struct CommitDetail {
    pub short_sha: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub refs: String,
}

/// The parsed state graph and verbose gathering start from.
#[derive(Debug, Clone)]
pub struct GatherInput {
    pub default_branch: String,
    /// `None` when the current worktree is detached (no graph, no verbose).
    pub current_branch: Option<String>,
    /// Every worktree's branch.
    pub branch_names: Vec<String>,
    pub refs: RefTips,
    pub forks: ForkOriginStore,
}

impl GatherInput {
    pub fn from_list(list: &WorktreeList) -> Self {
        let entries = list.entries();
        Self {
            default_branch: list.default_branch.clone(),
            current_branch: entries
                .iter()
                .find(|entry| entry.is_current)
                .and_then(|entry| entry.branch.clone()),
            branch_names: entries.iter().filter_map(|entry| entry.branch.clone()).collect(),
            refs: list.refs().clone(),
            forks: list.fork_origins().clone(),
        }
    }

    /// The default branch is checked out: every branch gets a lane.
    pub fn is_base_view(&self) -> bool {
        self.current_branch.as_deref() == Some(self.default_branch.as_str())
    }

    /// A feature branch is checked out, so `-v` has commits to show.
    pub fn has_verbose(&self) -> bool {
        self.current_branch
            .as_deref()
            .is_some_and(|current| current != self.default_branch)
    }
}

/// The typed facts a [`GitGraph`] is built from. PRs are added at build time,
/// since they arrive from another thread.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphFacts {
    pub default_branch: String,
    /// The default lane, oldest first.
    pub default_entries: Vec<LaneEntry>,
    pub lines: Vec<GraphLine>,
    /// Ref tips drawn as tags: the local default branch and `origin/<default>`.
    pub refs: Vec<(String, String)>,
    pub current_branch: String,
}

impl GraphFacts {
    /// The graph, with each open PR from origin's own repository whose head
    /// is a drawn branch, and `width` replacing the scale-derived width.
    pub fn to_git_graph(&self, prs: &PrListing, width: Option<ImageWidth>) -> GitGraph {
        let mut graph = GitGraph::new(self.default_branch.clone(), self.default_entries.clone())
            .with_current_branch(self.current_branch.clone());
        for (name, sha) in &self.refs {
            graph = graph.with_ref(name.clone(), sha.clone());
        }
        let mut drawn = HashSet::new();
        for line in &self.lines {
            drawn.insert(line.branch.clone());
            graph = graph.with_line(line.clone());
        }
        for branch in &drawn {
            // `GitGraph` matches PRs by branch name alone, so the source
            // repository is checked here.
            for pr in prs.for_branch(branch) {
                graph = graph.with_pull_request(GraphPullRequest {
                    number: pr.number,
                    source_branch: pr.source_branch.clone(),
                    target_branch: pr.target_branch.clone(),
                });
            }
        }
        match width {
            Some(width) => graph.with_width(width),
            None => graph,
        }
    }
}

/// The current branch's verbose details.
pub struct VerboseData {
    pub default_branch: String,
    pub branch: String,
    /// The commit the branch forked from on the default lane.
    pub merge_base: Option<CommitDetail>,
    /// Commits on the branch the default branch lacks, oldest first.
    pub branch_commits: Vec<CommitDetail>,
}

/// Where the default lane ends and how `origin/<default>` relates to it.
struct DefaultTips {
    local: Option<String>,
    origin: Option<(String, String)>,
    /// The descendant of the two tips; the local tip when they diverge.
    lane_tip: String,
    /// The fork point when `origin/<default>` has diverged.
    diverged_at: Option<String>,
}

impl DefaultTips {
    /// One `merge-base` when both tips exist and differ.
    fn read(input: &GatherInput) -> Option<Self> {
        let local = input.refs.local(&input.default_branch).map(str::to_string);
        let origin_name = format!("origin/{}", input.default_branch);
        let origin = input
            .refs
            .remote(&origin_name)
            .map(|sha| (origin_name, sha.to_string()));
        let (lane_tip, diverged_at) = match (&local, &origin) {
            (None, None) => return None,
            (Some(local), None) => (local.clone(), None),
            (None, Some((_, origin))) => (origin.clone(), None),
            (Some(local), Some((_, origin))) if local == origin => (local.clone(), None),
            (Some(local), Some((_, origin))) => match merge_base(local, origin) {
                Some(base) if base == *local => (origin.clone(), None),
                Some(base) if base == *origin => (local.clone(), None),
                base => (local.clone(), base),
            },
        };
        Some(Self {
            local,
            origin,
            lane_tip,
            diverged_at,
        })
    }

    /// Tips whose history is the default branch's.
    fn exclusions(&self) -> Vec<String> {
        let mut tips: Vec<String> = self.local.iter().cloned().collect();
        if let Some((_, origin)) = &self.origin
            && !tips.contains(origin)
        {
            tips.push(origin.clone());
        }
        tips
    }

    fn refs(&self, default_branch: &str) -> Vec<(String, String)> {
        let mut refs = Vec::new();
        if let Some(local) = &self.local {
            refs.push((default_branch.to_string(), local.clone()));
        }
        if let Some((name, sha)) = &self.origin {
            refs.push((name.clone(), sha.clone()));
        }
        refs
    }

    /// `origin/<default>` as its own line, only when it has diverged.
    fn diverged_line(&self) -> Option<GraphLine> {
        let fork = self.diverged_at.as_ref()?;
        let (name, sha) = self.origin.as_ref()?;
        let local = self.local.as_deref()?;
        let (entries, last_active) = line_entries(sha, &[local], LINE_WINDOW);
        let mut line = GraphLine::new(name.clone()).forked_at(fork.clone()).with_entries(entries);
        if let Some(time) = last_active {
            line = line.with_last_active(time);
        }
        Some(line)
    }
}

/// Graph and verbose data, gathered in one pass so both share one
/// `merge-base` for the current branch.
pub fn gather(input: &GatherInput, needs_graph: bool, needs_verbose: bool) -> (Option<GraphFacts>, Option<VerboseData>) {
    let Some(current) = input.current_branch.as_deref() else {
        return (None, None);
    };
    if !needs_graph && !(needs_verbose && input.has_verbose()) {
        return (None, None);
    }
    let Some(tips) = DefaultTips::read(input) else {
        return (None, None);
    };

    if input.is_base_view() {
        let graph = needs_graph.then(|| base_view(input, &tips)).flatten();
        return (graph, None);
    }

    let Some(current_tip) = input.refs.local(current) else {
        return (None, None);
    };
    let fork = merge_base(&tips.lane_tip, current_tip);
    let verbose = (needs_verbose && fork.is_some()).then(|| VerboseData {
        default_branch: input.default_branch.clone(),
        branch: current.to_string(),
        merge_base: fork
            .as_deref()
            .and_then(|fork| commit_details(fork, 1).into_iter().next()),
        branch_commits: commit_details_since(current_tip, &tips.exclusions()),
    });
    let graph = if needs_graph {
        fork.as_deref()
            .and_then(|fork| focused_view(input, &tips, current, current_tip, fork))
    } else {
        None
    };
    (graph, verbose)
}

/// The fork parent recorded for `branch`, when it is another existing branch
/// that `accept` allows.
fn recorded_parent<'a>(input: &'a GatherInput, branch: &str, accept: impl Fn(&str) -> bool) -> Option<(&'a str, &'a str)> {
    let parent = input.forks.get(branch)?.base_branch.as_str();
    if parent == input.default_branch || parent == branch || !accept(parent) {
        return None;
    }
    input.refs.local(parent).map(|tip| (parent, tip))
}

fn created_at(input: &GatherInput, branch: &str) -> Option<i64> {
    input
        .forks
        .get(branch)
        .and_then(|origin| i64::try_from(origin.created_at).ok())
}

/// The current branch, its non-default fork parent, and the default lane
/// around their fork point.
fn focused_view(
    input: &GatherInput,
    tips: &DefaultTips,
    current: &str,
    current_tip: &str,
    current_fork: &str,
) -> Option<GraphFacts> {
    let default_exclusions = tips.exclusions();
    let mut lines = Vec::new();
    let mut refs = tips.refs(&input.default_branch);
    let mut default_excludes = vec![current_tip.to_string()];

    let anchor = match recorded_parent(input, current, |_| true) {
        Some((parent, parent_tip)) => {
            let parent_fork = merge_base(&tips.lane_tip, parent_tip)?;
            let excludes: Vec<&str> = default_exclusions.iter().map(String::as_str).collect();
            let (entries, _) = line_entries(parent_tip, &excludes, LINE_WINDOW);
            let mut parent_line = GraphLine::new(parent)
                .forked_at(parent_fork.clone())
                .with_entries(entries);
            if let Some(time) = created_at(input, parent) {
                parent_line = parent_line.with_created_at(time);
            }
            lines.push(parent_line);
            refs.push((parent.to_string(), parent_tip.to_string()));
            default_excludes.push(parent_tip.to_string());

            let current_fork = merge_base(parent_tip, current_tip)?;
            let mut excludes = excludes;
            excludes.push(parent_tip);
            let (entries, _) = line_entries(current_tip, &excludes, LINE_WINDOW);
            lines.push(
                GraphLine::new(current)
                    .with_parent(parent)
                    .forked_at(current_fork)
                    .with_entries(entries),
            );
            parent_fork
        }
        None => {
            let excludes: Vec<&str> = default_exclusions.iter().map(String::as_str).collect();
            let (entries, _) = line_entries(current_tip, &excludes, LINE_WINDOW);
            lines.push(
                GraphLine::new(current)
                    .forked_at(current_fork.to_string())
                    .with_entries(entries),
            );
            current_fork.to_string()
        }
    };
    if let Some(line) = tips.diverged_line() {
        lines.push(line);
    }

    let mut default_entries: Vec<LaneEntry> = log_commits(&anchor, &[], CONTEXT_COMMITS)
        .into_iter()
        .map(|(sha, _)| LaneEntry::Commit(sha))
        .collect();
    let excludes: Vec<&str> = default_excludes.iter().map(String::as_str).collect();
    let (newer, _) = line_entries(&tips.lane_tip, &excludes, LINE_WINDOW);
    default_entries.extend(newer);

    Some(GraphFacts {
        default_branch: input.default_branch.clone(),
        default_entries,
        lines,
        refs,
        current_branch: current.to_string(),
    })
}

/// The default lane's newest commits and one line per worktree branch, each
/// under its fork parent when that parent is also drawn. Branches gather in
/// parallel.
fn base_view(input: &GatherInput, tips: &DefaultTips) -> Option<GraphFacts> {
    let default_entries: Vec<LaneEntry> = log_commits(&tips.lane_tip, &[], BASE_DEFAULT_WINDOW)
        .into_iter()
        .map(|(sha, _)| LaneEntry::Commit(sha))
        .collect();
    if default_entries.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let branches: Vec<&str> = input
        .branch_names
        .iter()
        .map(String::as_str)
        .filter(|branch| *branch != input.default_branch && seen.insert(*branch))
        .collect();
    let drawn: HashSet<&str> = branches.iter().copied().collect();
    let default_exclusions = tips.exclusions();

    let mut lines: Vec<GraphLine> = std::thread::scope(|scope| {
        let handles: Vec<_> = branches
            .iter()
            .map(|branch| {
                let drawn = &drawn;
                let default_exclusions = &default_exclusions;
                scope.spawn(move || {
                    let tip = input.refs.local(branch)?;
                    let parent = recorded_parent(input, branch, |parent| drawn.contains(parent));
                    let mut excludes: Vec<&str> = default_exclusions.iter().map(String::as_str).collect();
                    let fork = match parent {
                        Some((_, parent_tip)) => {
                            excludes.push(parent_tip);
                            merge_base(parent_tip, tip)?
                        }
                        None => merge_base(&tips.lane_tip, tip)?,
                    };
                    let (entries, last_active) = line_entries(tip, &excludes, LINE_WINDOW);
                    let mut line = GraphLine::new(*branch).forked_at(fork).with_entries(entries);
                    if let Some((parent, _)) = parent {
                        line = line.with_parent(parent);
                    }
                    if let Some(time) = created_at(input, branch) {
                        line = line.with_created_at(time);
                    }
                    if let Some(time) = last_active {
                        line = line.with_last_active(time);
                    }
                    Some(line)
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok().flatten())
            .collect()
    });
    if let Some(line) = tips.diverged_line() {
        lines.push(line);
    }

    Some(GraphFacts {
        default_branch: input.default_branch.clone(),
        default_entries,
        lines,
        refs: tips.refs(&input.default_branch),
        current_branch: input.default_branch.clone(),
    })
}

/// A line's newest `window` commits not reachable from `excludes`, oldest
/// first, preceded by a `+N` square for the older ones. Also returns the tip
/// commit's time when the line has commits.
fn line_entries(tip: &str, excludes: &[&str], window: usize) -> (Vec<LaneEntry>, Option<i64>) {
    let shown = log_commits(tip, excludes, window);
    let last_active = shown.last().and_then(|(_, time)| *time);
    let hidden = if shown.len() == window {
        count_commits(tip, excludes).saturating_sub(shown.len())
    } else {
        0
    };
    let mut entries = Vec::with_capacity(shown.len() + 1);
    if hidden > 0 {
        entries.push(LaneEntry::Elided(hidden));
    }
    entries.extend(shown.into_iter().map(|(sha, _)| LaneEntry::Commit(sha)));
    (entries, last_active)
}

/// Up to `max` commits reachable from `tip` but not from `excludes`, oldest
/// first, with their commit times.
fn log_commits(tip: &str, excludes: &[&str], max: usize) -> Vec<(String, Option<i64>)> {
    let max = max.to_string();
    let mut args = vec!["log", "--format=%H %ct", "--max-count", &max, "--reverse", tip];
    if !excludes.is_empty() {
        args.push("--not");
        args.extend_from_slice(excludes);
    }
    args.push("--");
    git_command(&args)
        .map(|output| {
            output
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    let sha = parts.next()?.to_string();
                    Some((sha, parts.next().and_then(|time| time.parse().ok())))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Commits reachable from `tip` but not from `excludes`; 0 on failure, so a
/// missing count only drops the `+N` square.
fn count_commits(tip: &str, excludes: &[&str]) -> usize {
    let mut args = vec!["rev-list", "--count", tip];
    if !excludes.is_empty() {
        args.push("--not");
        args.extend_from_slice(excludes);
    }
    args.push("--");
    git_command(&args)
        .ok()
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or(0)
}

fn merge_base(a: &str, b: &str) -> Option<String> {
    git_command(&["merge-base", a, b]).ok().filter(|sha| !sha.is_empty())
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

/// Query git log for commits reachable from `target` but not `excludes`, oldest first.
fn commit_details_since(target: &str, excludes: &[String]) -> Vec<CommitDetail> {
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let mut args = vec!["log", fmt_arg.as_str(), "--reverse", target];
    if !excludes.is_empty() {
        args.push("--not");
        args.extend(excludes.iter().map(String::as_str));
    }
    args.push("--");
    let Ok(output) = git_command(&args) else {
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

#[cfg(test)]
mod tests;
