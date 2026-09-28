//! Graph and verbose data for `wt list`.
//!
//! The graph half gathers typed facts (lines of work with full commit SHAs,
//! fork and merge commits, ref tips) and hands them to biscuit-terminal's
//! [`GitGraph`], which owns the lane/tag rule, elision, sizing, and the
//! Mermaid text. The verbose half gathers the commit details `wt list -v`
//! prints.
//!
//! Every lane is its tip's first-parent history, so a merged branch's commits
//! never read as the default branch's. Each selected branch is classified
//! against the lanes that could contain it ([`topology::Integration`]), and
//! the fork, merge, and label commits other lanes need are kept on their lane
//! however old they are. What Git cannot establish sets
//! [`GraphFacts::incomplete`] instead of being guessed. See
//! `worktree/docs/git-graph.md`.

use std::collections::{HashMap, HashSet};

use biscuit_terminal::components::git_graph::{GitGraph, GraphLine, GraphPullRequest, LaneEntry};
use biscuit_terminal::components::terminal_image::ImageWidth;
use chrono::{DateTime, Local, TimeZone, Timelike};
use worktree::fork_origin::ForkOriginStore;
use worktree::git::git_command;
use worktree::listing::RefTips;
use worktree::pull_requests::PrListing;
use worktree::worktree::WorktreeList;

mod topology;

use topology::{Extent, GatherGap, History, Integration, LaneHistory};

/// Newest commits drawn per line; older ones fold into one `+N` square.
const LINE_WINDOW: usize = 5;
/// Newest default-branch commits in the base view.
const BASE_DEFAULT_WINDOW: usize = 10;

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
    /// Ref tips drawn as tags: the local default branch and `origin/<default>`,
    /// plus the recorded parent's tip in a focused view.
    pub refs: Vec<(String, String)>,
    pub current_branch: String,
    /// Local history could not establish something the graph would show (a
    /// shallow clone, a failed git command): the graph carries the notice.
    pub incomplete: bool,
}

impl GraphFacts {
    /// The graph, with each open PR from origin's own repository whose head
    /// is a drawn branch, and `width` replacing the scale-derived width.
    pub fn to_git_graph(&self, prs: &PrListing, width: Option<ImageWidth>) -> GitGraph {
        let mut graph = GitGraph::new(self.default_branch.clone(), self.default_entries.clone())
            .with_current_branch(self.current_branch.clone());
        if self.incomplete {
            graph = graph.with_incomplete_history();
        }
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
    /// One `merge-base` when both tips exist and differ. Also returns whether
    /// that `merge-base` was a gap, in which case `origin/<default>` gets no
    /// line and its tag is accounted for by the graph.
    fn read(input: &GatherInput, history: &History) -> Option<(Self, bool)> {
        let local = input.refs.local(&input.default_branch).map(str::to_string);
        let origin_name = format!("origin/{}", input.default_branch);
        let origin = input
            .refs
            .remote(&origin_name)
            .map(|sha| (origin_name, sha.to_string()));
        let (lane_tip, diverged_at, gap) = match (&local, &origin) {
            (None, None) => return None,
            (Some(local), None) => (local.clone(), None, false),
            (None, Some((_, origin))) => (origin.clone(), None, false),
            (Some(local), Some((_, origin))) if local == origin => (local.clone(), None, false),
            (Some(local), Some((_, origin))) => match history.merge_base(local, origin) {
                Ok(Some(base)) if base == *local => (origin.clone(), None, false),
                Ok(Some(base)) if base == *origin => (local.clone(), None, false),
                Ok(base) => (local.clone(), base, false),
                Err(GatherGap) => (local.clone(), None, true),
            },
        };
        Some((
            Self {
                local,
                origin,
                lane_tip,
                diverged_at,
            },
            gap,
        ))
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

    /// `origin/<default>`'s name and tip, only when it has diverged and so
    /// gets a line of its own.
    fn diverged(&self) -> Option<(&str, &str)> {
        self.diverged_at.as_ref()?;
        self.local.as_ref()?;
        self.origin.as_ref().map(|(name, sha)| (name.as_str(), sha.as_str()))
    }

    /// Ref tips kept on the default lane: both, unless `origin/<default>` has
    /// a line of its own.
    fn lane_refs(&self) -> Vec<String> {
        let mut tips: Vec<String> = self.local.iter().cloned().collect();
        if self.diverged().is_none()
            && let Some((_, origin)) = &self.origin
        {
            tips.push(origin.clone());
        }
        tips
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
    // Verbose details alone never tell a shallow "no" from a real one, so
    // only the graph pays for the check.
    let (history, history_gap) = if needs_graph {
        let (history, read) = History::read();
        (history, read.is_err())
    } else {
        (History::complete(), false)
    };
    let Some((tips, tips_gap)) = DefaultTips::read(input, &history) else {
        return (None, None);
    };
    let gap = history_gap || tips_gap;

    if input.is_base_view() {
        let graph = needs_graph.then(|| base_view(input, &history, &tips, gap)).flatten();
        return (graph, None);
    }

    let Some(current_tip) = input.refs.local(current) else {
        return (None, None);
    };
    let base = history.merge_base(&tips.lane_tip, current_tip);
    let verbose = match (&base, needs_verbose) {
        (Ok(Some(fork)), true) => Some(VerboseData {
            default_branch: input.default_branch.clone(),
            branch: current.to_string(),
            merge_base: commit_details(fork, 1).into_iter().next(),
            branch_commits: commit_details_since(current_tip, &tips.exclusions()),
        }),
        _ => None,
    };
    let graph = if needs_graph {
        focused_view(input, &history, &tips, gap, current, current_tip, base)
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
/// down to the oldest commit they connect at.
fn focused_view(
    input: &GatherInput,
    history: &History,
    tips: &DefaultTips,
    gap: bool,
    current: &str,
    current_tip: &str,
    default_base: Result<Option<String>, GatherGap>,
) -> Option<GraphFacts> {
    let parent = recorded_parent(input, current, |_| true);
    let mut selected = Vec::new();
    let mut refs = tips.refs(&input.default_branch);
    if let Some((parent, parent_tip)) = parent {
        selected.push(Selected {
            branch: parent,
            tip: parent_tip,
            parent: None,
            default_base: None,
        });
        refs.push((parent.to_string(), parent_tip.to_string()));
    }
    selected.push(Selected {
        branch: current,
        tip: current_tip,
        parent,
        default_base: parent.is_none().then_some(default_base),
    });
    assemble(
        input,
        history,
        tips,
        &selected,
        DefaultLane {
            window: LINE_WINDOW,
            cap_window: true,
        },
        refs,
        current,
        gap,
    )
}

/// The default lane's newest commits and one line per worktree branch, each
/// under its fork parent when that parent is also drawn.
fn base_view(input: &GatherInput, history: &History, tips: &DefaultTips, gap: bool) -> Option<GraphFacts> {
    let mut seen = HashSet::new();
    let branches: Vec<&str> = input
        .branch_names
        .iter()
        .map(String::as_str)
        .filter(|branch| *branch != input.default_branch && seen.insert(*branch))
        .collect();
    let drawn: HashSet<&str> = branches.iter().copied().collect();
    let selected: Vec<Selected> = branches
        .iter()
        .filter_map(|branch| {
            Some(Selected {
                branch,
                tip: input.refs.local(branch)?,
                parent: recorded_parent(input, branch, |parent| drawn.contains(parent)),
                default_base: None,
            })
        })
        .collect();
    assemble(
        input,
        history,
        tips,
        &selected,
        DefaultLane {
            window: BASE_DEFAULT_WINDOW,
            cap_window: false,
        },
        tips.refs(&input.default_branch),
        &input.default_branch,
        gap,
    )
}

/// A branch the view draws.
struct Selected<'a> {
    branch: &'a str,
    tip: &'a str,
    /// Its recorded parent and that parent's tip, when the parent is drawn.
    parent: Option<(&'a str, &'a str)>,
    /// `merge-base(default lane tip, tip)` when the caller already has it.
    default_base: Option<Result<Option<String>, GatherGap>>,
}

/// The lane a fork, merge, or label commit is expected on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LaneId {
    Default,
    Branch(String),
    Origin,
}

/// How a selected branch is drawn.
#[derive(Debug, Clone)]
enum Shape {
    /// A lane of the tip's first-parent history until `stop`'s history.
    Lane {
        stop: Vec<String>,
        /// `None`: the connection is unknown or the histories are unrelated.
        fork: Option<(String, LaneId)>,
        merge: Option<(String, LaneId)>,
    },
    /// No history of its own: a label at its tip on that lane.
    Label(LaneId),
}

#[derive(Debug, Clone)]
struct Placement {
    branch: String,
    tip: String,
    parent: Option<String>,
    shape: Shape,
    /// Classification or the fork was unknown.
    gap: bool,
}

impl Placement {
    /// The commits this branch needs drawn on other lanes.
    fn anchors(&self) -> Vec<(LaneId, String)> {
        match &self.shape {
            Shape::Lane { fork, merge, .. } => fork
                .iter()
                .chain(merge)
                .map(|(sha, lane)| (lane.clone(), sha.clone()))
                .collect(),
            Shape::Label(lane) => vec![(lane.clone(), self.tip.clone())],
        }
    }
}

/// Classifies one selected branch against its recorded parent's lane,
/// the default lane, and a diverged `origin/<default>` line, in that order.
fn place(history: &History, tips: &DefaultTips, selected: &Selected) -> Placement {
    let tip = selected.tip;
    let mut candidates: Vec<(LaneId, &str)> = Vec::new();
    if let Some((parent, parent_tip)) = selected.parent {
        candidates.push((LaneId::Branch(parent.to_string()), parent_tip));
    }
    candidates.push((LaneId::Default, &tips.lane_tip));
    if let Some((_, origin_tip)) = tips.diverged() {
        candidates.push((LaneId::Origin, origin_tip));
    }
    let candidate_tips: Vec<&str> = candidates.iter().map(|(_, sha)| *sha).collect();
    let parent_tip = selected.parent.map(|(_, sha)| sha);
    let parent_lane = selected.parent.map(|(parent, _)| LaneId::Branch(parent.to_string()));
    // A fork is expected on the parent's lane, else on the lane it is
    // measured against.
    let fork = |against: &str, lane: &LaneId| match history.merge_base(against, tip) {
        Ok(fork) => (fork.map(|sha| (sha, parent_lane.clone().unwrap_or_else(|| lane.clone()))), false),
        Err(GatherGap) => (None, true),
    };

    let (shape, gap) = match history.classify(tip, &candidate_tips) {
        Ok(Integration::NoSeparateHistory { candidate }) => (Shape::Label(candidates[candidate].0.clone()), false),
        Ok(
            ref integration @ (Integration::MergedDirectly { candidate, ref first_parent, .. }
            | Integration::IntegratedOtherwise { candidate, ref first_parent }),
        ) => {
            let into = &candidates[candidate].0;
            // A tip that contains the branch is its own merge base with it,
            // so the fork is taken against the parent's tip only when the
            // merge went elsewhere, and otherwise against `C^1`.
            let parent_elsewhere = parent_tip.filter(|_| !matches!(into, LaneId::Branch(_)));
            let (fork, fork_gap) = fork(parent_elsewhere.unwrap_or(first_parent), into);
            let mut stop = vec![first_parent.clone()];
            stop.extend(parent_elsewhere.map(str::to_string));
            let merge = match integration {
                Integration::MergedDirectly { merge, .. } => Some((merge.clone(), into.clone())),
                _ => None,
            };
            let indirect = merge.is_none();
            (Shape::Lane { stop, fork, merge }, fork_gap || indirect)
        }
        classified @ (Ok(Integration::Unmerged) | Err(GatherGap)) => {
            let (fork, fork_gap) = match (&selected.default_base, parent_tip) {
                (Some(Ok(base)), None) => (base.clone().map(|sha| (sha, LaneId::Default)), false),
                (Some(Err(GatherGap)), None) => (None, true),
                (_, against) => fork(against.unwrap_or(&tips.lane_tip), &LaneId::Default),
            };
            let mut stop = tips.exclusions();
            stop.extend(parent_tip.map(str::to_string));
            (Shape::Lane { stop, fork, merge: None }, fork_gap || classified.is_err())
        }
    };
    Placement {
        branch: selected.branch.to_string(),
        tip: tip.to_string(),
        parent: selected.parent.map(|(parent, _)| parent.to_string()),
        shape,
        gap,
    }
}

/// How the default lane is drawn: its window, and whether the window, too,
/// stops just below the oldest connection (the focused view's context).
struct DefaultLane {
    window: usize,
    cap_window: bool,
}

/// Classifies every selected branch in parallel, then builds every branch
/// lane (and a diverged `origin/<default>` line) in parallel, then the default
/// lane. A fork, merge, or label commit is kept on the lane it was expected
/// on, and otherwise tried on the default lane, which is built last so it
/// holds only what no branch lane placed.
#[allow(clippy::too_many_arguments)]
fn assemble(
    input: &GatherInput,
    history: &History,
    tips: &DefaultTips,
    selected: &[Selected],
    default_lane: DefaultLane,
    refs: Vec<(String, String)>,
    current_branch: &str,
    mut incomplete: bool,
) -> Option<GraphFacts> {
    let placements: Vec<Placement> = parallel(selected, |selected| place(history, tips, selected))
        .into_iter()
        .filter_map(|placement| {
            incomplete |= placement.is_none();
            placement
        })
        .collect();
    let requests: Vec<(LaneId, String)> = placements.iter().flat_map(Placement::anchors).collect();
    let anchors_for = |lane: &LaneId| -> Vec<&str> {
        requests
            .iter()
            .filter(|(on, _)| on == lane)
            .map(|(_, sha)| sha.as_str())
            .collect()
    };

    let mut jobs: Vec<(LaneId, &str, Vec<&str>)> = placements
        .iter()
        .filter_map(|placement| match &placement.shape {
            Shape::Lane { stop, .. } => Some((
                LaneId::Branch(placement.branch.clone()),
                placement.tip.as_str(),
                stop.iter().map(String::as_str).collect(),
            )),
            Shape::Label(_) => None,
        })
        .collect();
    let diverged = tips.diverged();
    if let (Some((_, origin_tip)), Some(local)) = (diverged, tips.local.as_deref()) {
        jobs.push((LaneId::Origin, origin_tip, vec![local]));
    }
    let lanes: HashMap<LaneId, LaneHistory> = parallel(&jobs, |(lane, tip, stop)| {
        history.first_parent_entries(tip, Extent::Until(stop), LINE_WINDOW, &anchors_for(lane))
    })
    .into_iter()
    .zip(&jobs)
    .map(|(built, (lane, _, _))| {
        let built = built.and_then(Result::ok).unwrap_or_else(|| LaneHistory {
            gap: true,
            ..LaneHistory::default()
        });
        (lane.clone(), built)
    })
    .collect();

    let mut default_anchors = tips.lane_refs();
    for (lane, sha) in &requests {
        let placed = *lane != LaneId::Default && lanes.get(lane).is_some_and(|built| built.placed.contains(sha));
        if !placed {
            default_anchors.push(sha.clone());
        }
    }
    // A lane that turned out to have no commits is labeled at its tip.
    for placement in &placements {
        let lane = LaneId::Branch(placement.branch.clone());
        if lanes.get(&lane).is_some_and(|built| !has_commits(&built.entries)) {
            default_anchors.push(placement.tip.clone());
        }
    }
    let default_anchors: Vec<&str> = default_anchors.iter().map(String::as_str).collect();
    let default_built = history
        .first_parent_entries(
            &tips.lane_tip,
            Extent::Open {
                cap_window: default_lane.cap_window,
            },
            default_lane.window,
            &default_anchors,
        )
        .ok()?;
    incomplete |= default_built.gap || lanes.values().any(|built| built.gap);

    let mut lines = Vec::with_capacity(placements.len() + 1);
    for placement in &placements {
        incomplete |= placement.gap;
        let mut line = GraphLine::new(placement.branch.clone()).with_tip(placement.tip.clone());
        if let Some(parent) = &placement.parent {
            line = line.with_parent(parent.clone());
        }
        if let Some(time) = created_at(input, &placement.branch) {
            line = line.with_created_at(time);
        }
        if let Shape::Lane { fork, merge, .. } = &placement.shape {
            if let Some((fork, _)) = fork {
                line = line.forked_at(fork.clone());
            }
            if let Some((merge, _)) = merge {
                line = line.merged_into(merge.clone());
            }
            if let Some(built) = lanes.get(&LaneId::Branch(placement.branch.clone())) {
                line = line.with_entries(built.entries.clone());
                if let Some(time) = built.last_active {
                    line = line.with_last_active(time);
                }
            }
        }
        lines.push(line);
    }
    if let (Some((name, origin_tip)), Some(fork), Some(built)) = (diverged, &tips.diverged_at, lanes.get(&LaneId::Origin)) {
        let mut line = GraphLine::new(name)
            .forked_at(fork.clone())
            .with_tip(origin_tip)
            .with_entries(built.entries.clone());
        if let Some(time) = built.last_active {
            line = line.with_last_active(time);
        }
        lines.push(line);
    }

    Some(GraphFacts {
        default_branch: input.default_branch.clone(),
        default_entries: default_built.entries,
        lines,
        refs,
        current_branch: current_branch.to_string(),
        incomplete,
    })
}

fn has_commits(entries: &[LaneEntry]) -> bool {
    entries.iter().any(|entry| matches!(entry, LaneEntry::Commit(_)))
}

/// `work` over `items` on scoped threads, in order; an item whose thread
/// panicked is `None`.
fn parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<Option<R>> {
    std::thread::scope(|scope| {
        let work = &work;
        let handles: Vec<_> = items.iter().map(|item| scope.spawn(move || work(item))).collect();
        handles.into_iter().map(|handle| handle.join().ok()).collect()
    })
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
