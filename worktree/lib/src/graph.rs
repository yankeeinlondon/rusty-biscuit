//! Graph and verbose data for `wt list`.
//!
//! The graph half gathers typed facts (lines of work with full commit SHAs,
//! fork and merge commits, ref tips) that the CLI turns into a graph; the
//! lane/tag rule, elision, sizing, and rendering are not decided here. The
//! verbose half gathers the commit details `wt list -v` prints.
//!
//! Every lane is its tip's first-parent history, so a merged branch's commits
//! never read as the default branch's. Each selected branch is classified
//! against the lanes that could contain it ([`topology::Integration`]), and
//! the fork, merge, and label commits other lanes need are kept on their lane
//! however old they are. A lane's boundary, the first commit below it, is
//! classified the same way, so a branch that continued after a candidate
//! merged it draws that earlier merge from its real source, back to the
//! branch's recorded creation commit. What Git cannot establish sets
//! [`GraphFacts::incomplete`] instead of being guessed. See
//! `worktree/docs/git-graph.md`.
//!
//! Every Git call runs in [`GatherInput::repo`], never in the process's
//! current directory.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::fork_origin::ForkOriginStore;
use crate::git::calls;
use crate::git::git_command_in;
use crate::listing::RefTips;
use crate::timing::{self, Span, SpanList, Stage};
use crate::worktree::WorktreeList;

mod topology;

use topology::{Boundary, Extent, GatherGap, History, Integration, LaneHistory, LaneWindow};

/// One entry on a line of work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneEntry {
    /// A commit, by full SHA.
    Commit(String),
    /// Commits the graph leaves out, drawn as one `+N` square.
    Elided(usize),
}

/// A merge of a line into another lane, by full SHAs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneMerge {
    /// The commit on the merged line that became the merge's second parent.
    pub source: String,
    /// The merge commit, on another lane.
    pub destination: String,
}

/// A branch and the commits it has that its parent lacks, oldest first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphLine {
    pub branch: String,
    /// The branch it forked from; `None` is the default branch.
    pub parent: Option<String>,
    /// Full SHA of the commit it forked at; `None` is an unknown connection.
    pub fork_sha: Option<String>,
    /// Full SHA of the branch's own tip, also for a line without commits.
    pub tip_sha: Option<String>,
    /// Merges of this line into other lanes, oldest first.
    pub merges: Vec<LaneMerge>,
    pub entries: Vec<LaneEntry>,
    /// Creation time (Unix seconds) from the branch's fork-origin record.
    pub created_at: Option<i64>,
    /// The tip commit's time (Unix seconds).
    pub last_active: Option<i64>,
}

#[allow(missing_docs)]
impl GraphLine {
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            ..Self::default()
        }
    }

    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub fn forked_at(mut self, sha: impl Into<String>) -> Self {
        self.fork_sha = Some(sha.into());
        self
    }

    pub fn with_tip(mut self, sha: impl Into<String>) -> Self {
        self.tip_sha = Some(sha.into());
        self
    }

    /// Appends a merge of `source`, a commit on this line, into the merge
    /// commit `destination` on another lane. Append oldest first.
    pub fn with_merge(mut self, source: impl Into<String>, destination: impl Into<String>) -> Self {
        self.merges.push(LaneMerge {
            source: source.into(),
            destination: destination.into(),
        });
        self
    }

    pub fn with_entries(mut self, entries: Vec<LaneEntry>) -> Self {
        self.entries = entries;
        self
    }

    pub fn with_created_at(mut self, unix_seconds: i64) -> Self {
        self.created_at = Some(unix_seconds);
        self
    }

    pub fn with_last_active(mut self, unix_seconds: i64) -> Self {
        self.last_active = Some(unix_seconds);
        self
    }
}

/// Newest commits drawn per line; older ones fold into one `+N` square.
pub const LINE_WINDOW: usize = 5;
/// Newest default-branch commits in the base view.
pub const BASE_DEFAULT_WINDOW: usize = 10;

/// A parsed commit for verbose display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetail {
    pub short_sha: String,
    pub message: String,
    /// Commit time, Unix seconds.
    pub committed_at: i64,
    pub refs: String,
}

/// The parsed state graph and verbose gathering start from.
#[derive(Debug, Clone)]
pub struct GatherInput {
    /// Where every Git call of the gather runs.
    pub repo: PathBuf,
    pub default_branch: String,
    /// `None` when the current worktree is detached (no graph, no verbose).
    pub current_branch: Option<String>,
    /// Every worktree's branch.
    pub branch_names: Vec<String>,
    pub refs: RefTips,
    pub forks: ForkOriginStore,
}

impl GatherInput {
    /// The listing's entries, default branch, and fork records, with `refs`
    /// as the tips: the initial snapshot for a speculative gather, or the
    /// final one for a regather.
    pub fn from_list(list: &WorktreeList, refs: &RefTips) -> Self {
        let entries = list.entries();
        Self {
            repo: list.repo().to_path_buf(),
            default_branch: list.default_branch.clone(),
            current_branch: entries
                .iter()
                .find(|entry| entry.is_current)
                .and_then(|entry| entry.branch.clone()),
            branch_names: entries.iter().filter_map(|entry| entry.branch.clone()).collect(),
            refs: refs.clone(),
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

/// The typed facts the CLI builds its graph from. PRs are added at build time,
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
    /// The repository is verifiably a shallow clone, which is what usually
    /// leaves a connection unestablished.
    pub shallow: bool,
    /// Branches whose tip reached another lane only through some other
    /// branch's merge. Their lanes are drawn without a merge, and that is
    /// complete history, not a gap.
    pub merged_elsewhere: Vec<MergedElsewhere>,
    /// Branches whose fork commit is not on any drawn lane, so the rendered
    /// graph leaves their lane unconnected, with the merge that brought the fork
    /// into the lane it was expected on.
    pub forked_off_line: Vec<ForkedOffLine>,
}

/// A branch that forked from a commit its expected lane holds only through
/// a merge: `fork` is not on `into`'s own line of commits, `merge` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkedOffLine {
    pub branch: String,
    pub fork: String,
    /// The lane's name, as in [`MergedElsewhere::into`].
    pub into: String,
    pub merge: String,
}

/// A branch whose commits are all on `into`, brought there by another
/// branch's merge rather than one of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedElsewhere {
    pub branch: String,
    pub tip: String,
    /// The lane's name: a branch, the default branch, or `origin/<default>`.
    pub into: String,
    /// The drawn branch whose merge brought it in; `None` when no drawn
    /// branch was merged at that commit.
    pub through: Option<String>,
}



/// The current branch's verbose details.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    gather_steps(input, needs_graph, needs_verbose, None)
}

/// [`gather`], measured: the span is [`Stage::GraphHistory`] when the graph
/// is needed and [`Stage::VerboseHistory`] otherwise, with one sequential
/// child per step that ran and `git` counts throughout. Lane placement, its
/// repeats, and the fork-holder searches are one step,
/// [`Stage::LaneAssembly`], since they interleave.
pub fn gather_timed(input: &GatherInput, needs_graph: bool, needs_verbose: bool) -> ((Option<GraphFacts>, Option<VerboseData>), Span) {
    let stage = if needs_graph { Stage::GraphHistory } else { Stage::VerboseHistory };
    let mut steps = SpanList::sequential();
    let (data, span) = timing::measure(stage, || gather_steps(input, needs_graph, needs_verbose, Some(&mut steps)));
    (data, span.with_children(steps))
}

fn gather_steps(
    input: &GatherInput,
    needs_graph: bool,
    needs_verbose: bool,
    mut steps: Option<&mut SpanList>,
) -> (Option<GraphFacts>, Option<VerboseData>) {
    let Some(current) = input.current_branch.as_deref() else {
        return (None, None);
    };
    if !needs_graph && !(needs_verbose && input.has_verbose()) {
        return (None, None);
    }
    // Verbose details alone never tell a shallow "no" from a real one, so
    // only the graph pays for the check.
    let (history, history_gap) = if needs_graph {
        let (history, read) = timing::record(steps.as_deref_mut(), Stage::ShallowCheck, || History::read(&input.repo));
        (history, read.is_err())
    } else {
        (History::complete(&input.repo), false)
    };
    let Some((tips, tips_gap)) =
        timing::record(steps.as_deref_mut(), Stage::DefaultTips, || DefaultTips::read(input, &history))
    else {
        return (None, None);
    };
    let gap = history_gap || tips_gap;
    let shallow = history.is_shallow() && !history_gap;

    if input.is_base_view() {
        let graph = needs_graph.then(|| {
            timing::record(steps.as_deref_mut(), Stage::LaneAssembly, || GraphFacts {
                shallow,
                ..base_view(input, &history, &tips, gap)
            })
        });
        return (graph, None);
    }

    let Some(current_tip) = input.refs.local(current) else {
        return (None, None);
    };
    let base = timing::record(steps.as_deref_mut(), Stage::FocusedMergeBase, || {
        history.merge_base(&tips.lane_tip, current_tip)
    });
    let verbose = match (&base, needs_verbose) {
        (Ok(Some(fork)), true) => Some(timing::record(steps.as_deref_mut(), Stage::VerboseDetails, || VerboseData {
            default_branch: input.default_branch.clone(),
            branch: current.to_string(),
            merge_base: commit_details(input, fork, 1).into_iter().next(),
            branch_commits: commit_details_since(input, current_tip, &tips.exclusions()),
        })),
        _ => None,
    };
    let graph = needs_graph.then(|| {
        timing::record(steps, Stage::LaneAssembly, || GraphFacts {
            shallow,
            ..focused_view(input, &history, &tips, gap, current, current_tip, base)
        })
    });
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
) -> GraphFacts {
    let parent = recorded_parent(input, current, |_| true);
    // Branches drawn beyond the current one and its parent, because a drawn
    // lane forked from a commit only their line holds.
    let mut holders: Vec<&str> = Vec::new();
    loop {
        let mut drawn: HashSet<&str> = holders.iter().copied().collect();
        drawn.extend(parent.map(|(parent, _)| parent));
        drawn.insert(current);
        let mut selected = Vec::new();
        let mut refs = tips.refs(&input.default_branch);
        for branch in holders.iter().copied().chain(parent.map(|(parent, _)| parent)) {
            let Some(tip) = input.refs.local(branch) else {
                continue;
            };
            selected.push(Selected {
                branch,
                tip,
                parent: recorded_parent(input, branch, |recorded| drawn.contains(recorded)),
                default_base: None,
            });
            refs.push((branch.to_string(), tip.to_string()));
        }
        selected.push(Selected {
            branch: current,
            tip: current_tip,
            parent,
            default_base: parent.is_none().then_some(default_base.clone()),
        });
        let facts = assemble(
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
        );
        let added: Vec<&str> = facts
            .forked_off_line
            .iter()
            .filter_map(|off| fork_holder(input, history, off, &drawn))
            .collect();
        if added.is_empty() {
            return facts;
        }
        for branch in added {
            if !holders.contains(&branch) {
                holders.insert(0, branch);
            }
        }
    }
}

/// An undrawn worktree branch whose own line (first-parent chain) holds
/// `off`'s fork commit: the branch's recorded parent when it does, else the
/// first such branch in listing order.
fn fork_holder<'a>(input: &'a GatherInput, history: &History, off: &ForkedOffLine, drawn: &HashSet<&str>) -> Option<&'a str> {
    let holds = |branch: &str| {
        input.refs.local(branch).is_some_and(|tip| {
            matches!(history.classify(&off.fork, &[tip]), Ok(Integration::NoSeparateHistory { .. }))
        })
    };
    let recorded = input.forks.get(&off.branch).map(|origin| origin.base_branch.as_str());
    let candidates = recorded.into_iter().chain(input.branch_names.iter().map(String::as_str));
    let undrawn = |branch: &&str| *branch != input.default_branch && !drawn.contains(branch);
    candidates
        .filter(undrawn)
        .find(|branch| holds(branch))
        .and_then(|branch| input.branch_names.iter().find(|name| name.as_str() == branch).map(String::as_str))
}

/// The default lane's newest commits and one line per worktree branch, each
/// under its fork parent when that parent is also drawn.
fn base_view(input: &GatherInput, history: &History, tips: &DefaultTips, gap: bool) -> GraphFacts {
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

/// A branch lane's window, or the gap that kept it from being read.
type Window = Result<LaneWindow, GatherGap>;

/// How a selected branch is drawn.
#[derive(Debug, Clone)]
enum Shape {
    /// A lane of the tip's first-parent history until its stop history.
    Lane {
        /// The lane's window, read against its final stop history.
        window: Window,
        /// `None`: the connection is unknown or the histories are unrelated.
        fork: Option<(String, LaneId)>,
        /// Earlier merges of the lane, oldest first: a source `B` on this
        /// lane, and its merge `C` on another.
        earlier: Vec<EarlierMerge>,
        /// The merge of the tip, which follows every earlier one.
        merge: Option<(String, LaneId)>,
    },
    /// No history of its own: a label at its tip on that lane.
    Label(LaneId),
}

#[derive(Debug, Clone)]
struct EarlierMerge {
    source: String,
    merge: String,
    lane: LaneId,
}

#[derive(Debug, Clone)]
struct Placement {
    branch: String,
    tip: String,
    parent: Option<String>,
    shape: Shape,
    /// Classification or the fork was unknown.
    gap: bool,
    /// The other branch's merge that brought the tip into this lane, when
    /// the tip was [`Integration::IntegratedOtherwise`].
    merged_through: Option<(String, LaneId)>,
}

impl Placement {
    /// Whether `merge` is one of this branch's own drawn merges.
    fn merges_at(&self, merge: &str) -> bool {
        match &self.shape {
            Shape::Lane { earlier, merge: own, .. } => {
                own.as_ref().is_some_and(|(sha, _)| sha == merge) || earlier.iter().any(|edge| edge.merge == merge)
            }
            Shape::Label(_) => false,
        }
    }

    /// The commits this branch needs drawn on other lanes.
    fn anchors(&self) -> Vec<(LaneId, String)> {
        match &self.shape {
            Shape::Lane { fork, earlier, merge, .. } => {
                let own = LaneId::Branch(self.branch.clone());
                let mut anchors: Vec<(LaneId, String)> = fork.iter().chain(merge).map(|(sha, lane)| (lane.clone(), sha.clone())).collect();
                for edge in earlier {
                    anchors.push((own.clone(), edge.source.clone()));
                    anchors.push((edge.lane.clone(), edge.merge.clone()));
                }
                anchors
            }
            Shape::Label(lane) => vec![(lane.clone(), self.tip.clone())],
        }
    }
}

/// Classifications shared by every placement of one gathering pass, so a
/// commit asked about against the same candidate tips, such as a boundary
/// two sibling lanes share, is classified once even when both ask at once.
type Classified = Result<Integration, GatherGap>;
/// A commit and the candidate tips it was classified against.
type Question = (String, Vec<String>);

#[derive(Default)]
struct Classifications {
    answers: Mutex<HashMap<Question, Arc<OnceLock<Classified>>>>,
}

impl Classifications {
    fn classify(&self, history: &History, commit: &str, candidates: &[&str]) -> Classified {
        let key = (commit.to_string(), candidates.iter().map(|tip| tip.to_string()).collect());
        let answer = Arc::clone(self.answers.lock().unwrap_or_else(PoisonError::into_inner).entry(key).or_default());
        answer.get_or_init(|| history.classify(commit, candidates)).clone()
    }
}

/// The fork a lane gets when no earlier merge is drawn.
enum Fork<'a> {
    /// Already known, with whether finding it was a gap.
    Known(Option<(String, LaneId)>, bool),
    /// `merge-base(against, tip)`, expected on `lane`.
    Against(&'a str, LaneId),
}

/// A lane after its earlier merges were walked.
struct Extension {
    window: Window,
    earlier: Vec<EarlierMerge>,
    /// `B` and `C^1` of the oldest earlier merge.
    oldest: Option<(String, String, LaneId)>,
    gap: bool,
}

/// Classifies one selected branch against its recorded parent's lane,
/// the default lane, and a diverged `origin/<default>` line, in that order.
/// `others` is every selected branch and its tip, for merges into a lane
/// other than the ones the branch is classified against.
fn place(
    history: &History,
    classifications: &Classifications,
    tips: &DefaultTips,
    selected: &Selected,
    record: Option<&str>,
    others: &[(&str, &str)],
) -> Placement {
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
    let fork_of = |against: &str, from: &str, lane: &LaneId| match history.merge_base(against, from) {
        Ok(fork) => (fork.map(|sha| (sha, parent_lane.clone().unwrap_or_else(|| lane.clone()))), false),
        Err(GatherGap) => (None, true),
    };

    let classified = classifications.classify(history, tip, &candidate_tips);
    let (stop, fork, merge, classified_gap) = match &classified {
        Ok(Integration::NoSeparateHistory { candidate }) => {
            return Placement {
                branch: selected.branch.to_string(),
                tip: tip.to_string(),
                parent: selected.parent.map(|(parent, _)| parent.to_string()),
                shape: Shape::Label(candidates[*candidate].0.clone()),
                gap: false,
                merged_through: None,
            };
        }
        Ok(
            integration @ (Integration::MergedDirectly { candidate, first_parent, .. }
            | Integration::IntegratedOtherwise { candidate, first_parent, .. }),
        ) => {
            let into = &candidates[*candidate].0;
            // A tip that contains the branch is its own merge base with it,
            // so the fork is taken against the parent's tip only when the
            // merge went elsewhere and the parent does not contain the branch
            // (after an indirect match it does), and otherwise against `C^1`.
            let after_indirect = matches!(integration, Integration::MergedDirectly { after_indirect: true, .. });
            let parent_elsewhere = parent_tip.filter(|_| !after_indirect && !matches!(into, LaneId::Branch(_)));
            let mut stop = vec![first_parent.clone()];
            stop.extend(parent_elsewhere.map(str::to_string));
            let merge = match integration {
                Integration::MergedDirectly { merge, .. } => Some((merge.clone(), into.clone())),
                _ => None,
            };
            // An indirect integration is a verified answer with no merge of
            // its own to draw, not a gap; `merged_elsewhere` reports it.
            (stop, Fork::Against(parent_elsewhere.unwrap_or(first_parent), into.clone()), merge, false)
        }
        classified @ (Ok(Integration::Unmerged) | Err(GatherGap)) => {
            let fork = match (&selected.default_base, parent_tip) {
                (Some(Ok(base)), None) => Fork::Known(base.clone().map(|sha| (sha, LaneId::Default)), false),
                (Some(Err(GatherGap)), None) => Fork::Known(None, true),
                (_, against) => Fork::Against(against.unwrap_or(&tips.lane_tip), LaneId::Default),
            };
            let mut stop = tips.exclusions();
            stop.extend(parent_tip.map(str::to_string));
            (stop, fork, None, classified.is_err())
        }
    };

    let extension = extend(history, classifications, tip, &candidates, stop, record);
    let (fork, fork_gap) = match (&extension.oldest, fork) {
        // Measured against `B`, not the tip: a tip that merged the default
        // branch back in contains `C^1`, which would be its merge base.
        (Some((source, first_parent, lane)), _) => fork_of(first_parent, source, lane),
        (None, Fork::Known(fork, gap)) => (fork, gap),
        (None, Fork::Against(against, lane)) => fork_of(against, tip, &lane),
    };
    // Only the parent, default, and origin lanes are classified against. A
    // tip merged directly into another drawn lane (a child or a sibling)
    // still gets that merge, keeping the fork and line found above: measuring
    // the fork from that lane would hang this one from it.
    let merge = merge.or_else(|| match &classified {
        Ok(Integration::Unmerged | Integration::IntegratedOtherwise { .. }) => {
            direct_merge_elsewhere(history, classifications, selected, others)
        }
        _ => None,
    });
    let shape = Shape::Lane {
        window: extension.window,
        fork,
        earlier: extension.earlier,
        merge,
    };
    let gap = classified_gap || fork_gap || extension.gap;
    let merged_through = match &classified {
        Ok(Integration::IntegratedOtherwise { candidate, merge, .. }) => Some((merge.clone(), candidates[*candidate].0.clone())),
        _ => None,
    };
    Placement {
        branch: selected.branch.to_string(),
        tip: tip.to_string(),
        parent: selected.parent.map(|(parent, _)| parent.to_string()),
        shape,
        gap,
        merged_through,
    }
}

/// The first other selected branch (not this one, not its recorded parent)
/// whose line merged `selected`'s tip directly, as its merge's second parent.
/// A tip merely on that lane's line (`NoSeparateHistory`, as when a child
/// forked at it) is no merge.
fn direct_merge_elsewhere(
    history: &History,
    classifications: &Classifications,
    selected: &Selected,
    others: &[(&str, &str)],
) -> Option<(String, LaneId)> {
    let parent = selected.parent.map(|(parent, _)| parent);
    others
        .iter()
        .filter(|(branch, _)| *branch != selected.branch && Some(*branch) != parent)
        .find_map(|(branch, other_tip)| match classifications.classify(history, selected.tip, &[other_tip]) {
            Ok(Integration::MergedDirectly { merge, .. }) => Some((merge, LaneId::Branch(branch.to_string()))),
            _ => None,
        })
}

/// Walks a lane's boundaries backward and extends it past each one that a
/// candidate lane merged directly, until a boundary is an ordinary fork, was
/// integrated otherwise, is at or below the branch's recorded creation
/// commit, or cannot be established (a gap). `window` is `Err` only when the
/// lane's first read failed, so an accepted edge always keeps its source.
fn extend(history: &History, classifications: &Classifications, tip: &str, candidates: &[(LaneId, &str)], mut stop: Vec<String>, record: Option<&str>) -> Extension {
    let candidate_tips: Vec<&str> = candidates.iter().map(|(_, sha)| *sha).collect();
    let mut earlier: Vec<EarlierMerge> = Vec::new();
    let mut known: Vec<Boundary> = Vec::new();
    let mut oldest = None;
    let mut record_distance = None;
    let mut gap = false;
    let read = |stop: &[String]| {
        let stop: Vec<&str> = stop.iter().map(String::as_str).collect();
        history.lane_window(tip, &stop, LINE_WINDOW)
    };
    let mut window = read(&stop);
    while let Ok(lane) = &window {
        let boundary = match history.boundary(tip, lane) {
            Ok(Some(boundary)) => boundary,
            Ok(None) => break,
            Err(GatherGap) => {
                gap = true;
                break;
            }
        };
        if known.last().is_some_and(|newer| boundary.distance <= newer.distance) || known.iter().any(|seen| seen.sha == boundary.sha) {
            gap = true;
            break;
        }
        let (candidate, merge, first_parent, after_indirect) = match classifications.classify(history, &boundary.sha, &candidate_tips) {
            Ok(Integration::MergedDirectly {
                candidate,
                merge,
                first_parent,
                after_indirect,
            }) => (candidate, merge, first_parent, after_indirect),
            Ok(Integration::NoSeparateHistory { .. } | Integration::IntegratedOtherwise { .. }) => break,
            // `B` is in the stop history, so a lane contains it: "unmerged"
            // is an answer Git should not give.
            Ok(Integration::Unmerged) | Err(GatherGap) => {
                gap = true;
                break;
            }
        };
        let cutoff = record_distance.get_or_insert_with(|| match record {
            Some(recorded) => history.chain_distance(tip, recorded),
            None => Ok(None),
        });
        match cutoff {
            // The branch was created at or after `B`, so the merge is an
            // older branch's.
            Ok(Some(created)) if *created <= boundary.distance => break,
            Ok(_) => {}
            Err(GatherGap) => {
                gap = true;
                break;
            }
        }

        // Classification already answered for the candidate tips it asked.
        let answered = |tip: &str| {
            if tip == candidate_tips[candidate] {
                Some(true)
            } else if !after_indirect && candidate_tips[..candidate].contains(&tip) {
                Some(false)
            } else {
                None
            }
        };
        let mut next = vec![first_parent.clone()];
        for kept in &stop {
            let contains = answered(kept).map_or_else(|| history.is_ancestor(&boundary.sha, kept), Ok);
            match contains {
                Ok(true) => {}
                Ok(false) => next.push(kept.clone()),
                // Keeping it can only make the lane shorter.
                Err(GatherGap) => {
                    gap = true;
                    next.push(kept.clone());
                }
            }
        }
        let mut seen = HashSet::new();
        next.retain(|sha| seen.insert(sha.clone()));

        let lane_id = candidates[candidate].0.clone();
        oldest = Some((boundary.sha.clone(), first_parent, lane_id.clone()));
        earlier.push(EarlierMerge {
            source: boundary.sha.clone(),
            merge,
            lane: lane_id,
        });
        known.push(boundary);
        // A failed reread keeps the window already read: the accepted edge's
        // source is its boundary, placed from `known` without asking Git.
        match read(&next) {
            Ok(wider) => window = Ok(wider),
            Err(GatherGap) => {
                gap = true;
                break;
            }
        }
        stop = next;
    }
    earlier.reverse();
    Extension {
        window: window.map(|lane| lane.with_known(known)),
        earlier,
        oldest,
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
) -> GraphFacts {
    let classifications = Classifications::default();
    let record = |branch: &str| input.forks.get(branch).map(|origin| origin.base_sha.as_str());
    let others: Vec<(&str, &str)> = selected.iter().map(|selected| (selected.branch, selected.tip)).collect();
    let placements: Vec<Placement> =
        parallel(selected, |selected| place(history, &classifications, tips, selected, record(selected.branch), &others))
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

    // A branch lane's window was read while placing it; `None` is read here.
    let mut jobs: Vec<(LaneId, &str, Option<&Window>)> = placements
        .iter()
        .filter_map(|placement| match &placement.shape {
            Shape::Lane { window, .. } => Some((LaneId::Branch(placement.branch.clone()), placement.tip.as_str(), Some(window))),
            Shape::Label(_) => None,
        })
        .collect();
    let diverged = tips.diverged();
    let local = tips.local.as_deref();
    if let (Some((_, origin_tip)), Some(_)) = (diverged, local) {
        jobs.push((LaneId::Origin, origin_tip, None));
    }
    let lanes: HashMap<LaneId, LaneHistory> = parallel(&jobs, |(lane, tip, window)| {
        let read;
        let window = match window {
            Some(window) => window.as_ref().ok()?,
            None => {
                read = history.lane_window(tip, &local.into_iter().collect::<Vec<_>>(), LINE_WINDOW).ok()?;
                &read
            }
        };
        Some(history.first_parent_entries(tip, Extent::Until(window), &anchors_for(lane)))
    })
    .into_iter()
    .zip(&jobs)
    .map(|(built, (lane, _, _))| {
        // No window was ever read, so the lane verified no commit to keep.
        let built = built.flatten().unwrap_or_else(|| LaneHistory {
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
    let default_built = history.first_parent_entries(
        &tips.lane_tip,
        Extent::Open {
            window: default_lane.window,
            cap_window: default_lane.cap_window,
        },
        &default_anchors,
    );
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
        if let Shape::Lane { fork, earlier, merge, .. } = &placement.shape {
            if let Some((fork, _)) = fork {
                line = line.forked_at(fork.clone());
            }
            for edge in earlier {
                line = line.with_merge(edge.source.clone(), edge.merge.clone());
            }
            if let Some((merge, _)) = merge {
                line = line.with_merge(placement.tip.clone(), merge.clone());
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

    // A fork no lane drew leaves the rendered graph an unconnected lane. When the
    // expected lane contains the fork, the commit that brought it in says why.
    let on = |built: &LaneHistory, sha: &str| {
        built.placed.contains(sha) || built.entries.iter().any(|entry| matches!(entry, LaneEntry::Commit(commit) if commit == sha))
    };
    let drawn = |sha: &str| on(&default_built, sha) || lanes.values().any(|built| on(built, sha));
    let tip_of = |lane: &LaneId| -> Option<&str> {
        match lane {
            LaneId::Default => Some(tips.lane_tip.as_str()),
            LaneId::Origin => diverged.map(|(_, tip)| tip),
            LaneId::Branch(branch) => placements.iter().find(|placement| &placement.branch == branch).map(|placement| placement.tip.as_str()),
        }
    };
    let forked_off_line = placements
        .iter()
        .filter_map(|placement| {
            let Shape::Lane { fork: Some((fork, lane)), .. } = &placement.shape else {
                return None;
            };
            if drawn(fork) {
                return None;
            }
            let merge = match classifications.classify(history, fork, &[tip_of(lane)?]) {
                Ok(Integration::MergedDirectly { merge, .. } | Integration::IntegratedOtherwise { merge, .. }) => merge,
                _ => return None,
            };
            Some(ForkedOffLine {
                branch: placement.branch.clone(),
                fork: fork.clone(),
                into: lane_name(input, tips, lane),
                merge,
            })
        })
        .collect();

    let merged_elsewhere = placements
        .iter()
        .filter_map(|placement| {
            let (merge, into) = placement.merged_through.as_ref()?;
            Some(MergedElsewhere {
                branch: placement.branch.clone(),
                tip: placement.tip.clone(),
                into: lane_name(input, tips, into),
                through: placements.iter().find(|other| other.merges_at(merge)).map(|other| other.branch.clone()),
            })
        })
        .collect();

    GraphFacts {
        default_branch: input.default_branch.clone(),
        default_entries: default_built.entries,
        lines,
        refs,
        current_branch: current_branch.to_string(),
        incomplete,
        shallow: false,
        merged_elsewhere,
        forked_off_line,
    }
}

/// The name a reader knows `lane` by: the default lane is the local default
/// branch unless `origin/<default>` is ahead of it and so is its tip.
fn lane_name(input: &GatherInput, tips: &DefaultTips, lane: &LaneId) -> String {
    let origin = || format!("origin/{}", input.default_branch);
    match lane {
        LaneId::Branch(branch) => branch.clone(),
        LaneId::Origin => origin(),
        LaneId::Default if tips.local.as_deref() == Some(tips.lane_tip.as_str()) => input.default_branch.clone(),
        LaneId::Default => origin(),
    }
}

fn has_commits(entries: &[LaneEntry]) -> bool {
    entries.iter().any(|entry| matches!(entry, LaneEntry::Commit(_)))
}

/// `work` over `items` on scoped threads, in order; an item whose thread
/// panicked is `None`. Each thread's Git calls count toward the caller's
/// counting scope.
fn parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<Option<R>> {
    let task = calls::TaskHandle::current();
    std::thread::scope(|scope| {
        let work = &work;
        let handles: Vec<_> = items.iter().map(|item| scope.spawn(move || task.run(|| work(item)))).collect();
        handles.into_iter().map(|handle| handle.join().ok().map(calls::joined)).collect()
    })
}

/// Git format string using %x1f (Unit Separator) as field delimiter.
/// Using git's own escape avoids embedding raw control chars in args.
const DETAIL_FMT: &str = "%H%x1f%h%x1f%s%x1f%at%x1f%D";

/// Leaves `%D` only the decorations [`GatherInput::refs`] does not capture
/// (tags, and any other non-branch refs Git decorates): branch, remote-tracking,
/// and `HEAD` labels are rebuilt from the snapshot by [`snapshot_labels`],
/// because Git's live refs may have moved since it was read.
const LIVE_DECORATION_EXCLUDES: [&str; 3] =
    ["--decorate-refs-exclude=HEAD", "--decorate-refs-exclude=refs/heads/", "--decorate-refs-exclude=refs/remotes/"];

/// Query git log for detailed commit info, returning oldest first.
fn commit_details(input: &GatherInput, rev: &str, max: usize) -> Vec<CommitDetail> {
    let max_str = max.to_string();
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let mut args = vec!["log", fmt_arg.as_str()];
    args.extend(LIVE_DECORATION_EXCLUDES);
    args.extend(["--max-count", &max_str, "--reverse", rev, "--"]);
    let Ok(output) = git_command_in(&input.repo, &args) else {
        return vec![];
    };
    parse_commit_lines(input, &output)
}

/// Query git log for commits reachable from `target` but not `excludes`, oldest first.
fn commit_details_since(input: &GatherInput, target: &str, excludes: &[String]) -> Vec<CommitDetail> {
    let fmt_arg = format!("--format={DETAIL_FMT}");
    let mut args = vec!["log", fmt_arg.as_str()];
    args.extend(LIVE_DECORATION_EXCLUDES);
    args.extend(["--reverse", target]);
    if !excludes.is_empty() {
        args.push("--not");
        args.extend(excludes.iter().map(String::as_str));
    }
    args.push("--");
    let Ok(output) = git_command_in(&input.repo, &args) else {
        return vec![];
    };
    parse_commit_lines(input, &output)
}

fn parse_commit_lines(input: &GatherInput, output: &str) -> Vec<CommitDetail> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(5, '\x1f').collect();
            if parts.len() < 4 {
                return None;
            }
            let committed_at: i64 = parts[3].parse().ok()?;
            Some(CommitDetail {
                short_sha: parts[1].to_string(),
                message: parts[2].to_string(),
                committed_at,
                refs: snapshot_labels(input, parts[0], parts.get(4).copied().unwrap_or("")),
            })
        })
        .collect()
}

/// `sha`'s decorations in Git's `%D` spelling, as of the snapshot: `HEAD ->
/// <current>` at the current branch's tip, then `live` (the decorations the
/// snapshot does not capture, such as tags), then the remote-tracking names
/// and the other local branches at `sha`, each group in Git's reverse refname
/// order.
fn snapshot_labels(input: &GatherInput, sha: &str, live: &str) -> String {
    let refs = &input.refs;
    let current = input.current_branch.as_deref().filter(|current| refs.local(current) == Some(sha));
    let mut labels: Vec<String> = current.map(|current| format!("HEAD -> {current}")).into_iter().collect();
    labels.extend(live.split(", ").filter(|label| !label.is_empty()).map(str::to_string));
    let mut remotes: Vec<&str> = refs
        .remote
        .iter()
        .filter(|(_, tip)| tip.as_str() == sha)
        .map(|(name, _)| name.as_str())
        .chain(
            refs.remote_heads
                .iter()
                .filter(|(_, target)| refs.remote(target) == Some(sha))
                .map(|(name, _)| name.as_str()),
        )
        .collect();
    remotes.sort_unstable_by(|a, b| b.cmp(a));
    labels.extend(remotes.into_iter().map(str::to_string));
    labels.extend(
        refs.local
            .iter()
            .rev()
            .filter(|(name, tip)| tip.as_str() == sha && Some(name.as_str()) != current)
            .map(|(name, _)| name.clone()),
    );
    labels.join(", ")
}

#[cfg(test)]
mod tests;
