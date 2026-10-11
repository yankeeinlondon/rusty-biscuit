//! The `wt list` graph and verbose rendering.
//!
//! [`worktree::graph`] gathers the facts; this module hands them to
//! biscuit-terminal's [`GitGraph`], which owns the lane/tag rule, elision,
//! sizing, and the Mermaid text, and formats the commit details `wt list -v`
//! prints. See `worktree/docs/git-graph.md`.

use std::collections::HashSet;

use biscuit_terminal::components::git_graph::{self as terminal, GitGraph, GraphPullRequest};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::terminal_image::ImageWidth;
use chrono::{DateTime, Local, Timelike};
use worktree::graph::{CommitDetail, GraphFacts, GraphLine, LaneEntry, LaneMerge};
use worktree::pull_requests::PrListing;

/// The graph, with each open PR from origin's own repository whose head is a
/// drawn branch, and `width` replacing the scale-derived width.
pub fn to_git_graph(facts: &GraphFacts, prs: &PrListing, width: Option<ImageWidth>) -> GitGraph {
    let mut graph = GitGraph::new(facts.default_branch.clone(), lane_entries(&facts.default_entries))
        .with_current_branch(facts.current_branch.clone());
    if facts.incomplete {
        graph = graph.with_incomplete_history();
    }
    for (name, sha) in &facts.refs {
        graph = graph.with_ref(name.clone(), sha.clone());
    }
    // A tip that reached a lane only through another branch's merge has
    // no merge line of its own; the tag says it is merged anyway. Only a
    // drawn tip gets one, so it never becomes an undrawn-label omission.
    for merged in &facts.merged_elsewhere {
        let tip_drawn = facts.lines.iter().any(|line| {
            line.branch == merged.branch && line.entries.iter().any(|entry| matches!(entry, LaneEntry::Commit(sha) if *sha == merged.tip))
        });
        if tip_drawn {
            graph = graph.with_ref(format!("in {}", merged.into), merged.tip.clone());
        }
    }
    let mut drawn = HashSet::new();
    for line in &facts.lines {
        drawn.insert(line.branch.clone());
        graph = graph.with_line(graph_line(line));
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

fn lane_entries(entries: &[LaneEntry]) -> Vec<terminal::LaneEntry> {
    entries
        .iter()
        .map(|entry| match entry {
            LaneEntry::Commit(sha) => terminal::LaneEntry::Commit(sha.clone()),
            LaneEntry::Elided(count) => terminal::LaneEntry::Elided(*count),
        })
        .collect()
}

fn lane_merge(merge: &LaneMerge) -> terminal::LaneMerge {
    terminal::LaneMerge {
        source: merge.source.clone(),
        destination: merge.destination.clone(),
    }
}

fn graph_line(line: &GraphLine) -> terminal::GraphLine {
    terminal::GraphLine {
        branch: line.branch.clone(),
        parent: line.parent.clone(),
        fork_sha: line.fork_sha.clone(),
        tip_sha: line.tip_sha.clone(),
        merges: line.merges.iter().map(lane_merge).collect(),
        entries: lane_entries(&line.entries),
        created_at: line.created_at,
        last_active: line.last_active,
    }
}

/// Format a commit in the same style as `sniff repo git-status`.
pub fn format_commit(commit: &CommitDetail) -> String {
    let sha_display = format!("<b>{}</b>", Prose::escape_text(&commit.short_sha));
    // An out-of-range time reads as the epoch rather than dropping the commit.
    let committed_at = DateTime::from_timestamp(commit.committed_at, 0).unwrap_or_default().with_timezone(&Local);
    let (date_str, time_str, use_on) = format_datetime(&committed_at);
    let date_prefix = if use_on { "<i>on</i> " } else { "" };
    let refs_part = format_refs(&commit.refs);

    let cc = parse_conventional(&commit.message);
    if let Some((op, scope, desc)) = cc {
        let op = Prose::escape_text(&op);
        let desc = Prose::escape_text(&desc);
        let scope_part = scope
            .map(|s| format!("(<dim>{}</dim>)", Prose::escape_text(&s)))
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
        let truncated = Prose::escape_text(&truncated);
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
            let esc = Prose::escape_text;
            if r.contains("HEAD -> ") {
                let branch = r.strip_prefix("HEAD -> ").unwrap_or(r);
                format!("<cyan><b>HEAD -></b> {}</cyan>", esc(branch))
            } else if r.starts_with("tag: ") {
                let tag = r.strip_prefix("tag: ").unwrap_or(r);
                format!("<yellow>{}</yellow>", esc(tag))
            } else if r.contains('/') {
                format!("<green>{}</green>", esc(r))
            } else {
                format!("<cyan>{}</cyan>", esc(r))
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
