//! Git branch topology drawn as a Mermaid `gitGraph` through [`MermaidDiagram`].
//!
//! Callers gather the git facts (lines of work with full commit SHAs, ref
//! tips, merge destinations, open PRs) and hand them over typed; the component
//! runs no git commands. It owns the lane/tag rule, lane ordering, merges,
//! elision markers, the `mermaid-rs-renderer` workarounds, and fitting the
//! graph to the terminal by trimming.
//!
//! ## Lanes and tags
//!
//! A **lane** is a line of work whose commits are on no other drawn lane: the
//! default branch, the current branch, its fork parent when that is not the
//! default branch, and `origin/<default>` when it has commits of its own
//! (it has diverged). With no current branch, or the default branch checked
//! out, every line with commits of its own gets a lane. A lane hangs from its
//! fork commit, and a line [merged into](GraphLine::merged_into) a drawn commit
//! is drawn merging there. Every other ref whose tip is a drawn commit is a
//! **tag** on that commit, a line without commits is a tag on its own
//! [tip](GraphLine::with_tip), and an open PR is a tag on its source branch's
//! tip; the precise form is documented in `worktree/docs/cli/list.md`.
//!
//! ## Nothing undrawn is substituted
//!
//! A lane whose fork commit is not drawn (or unknown) is drawn unconnected,
//! never from another commit. A tag whose commit is not drawn, and a merge whose
//! destination is not drawn, are left out. Each of these, and a caller's own
//! [`GitGraph::with_incomplete_history`], sets [`GitGraphPlan::incomplete`],
//! which renders the dim notice "Some history is not shown". Lanes and tags the
//! base view's height cap leaves out are counted by its own note instead.
//!
//! ## Renderer workarounds
//!
//! - `mermaid-rs-renderer` reads everything after `branch` or `merge` as the
//!   branch name, attributes included, so no attributes are emitted on
//!   `branch`. A `merge` carries its ID and tags, and `biscuit-visualized`
//!   restores the merge's second parent that this costs. The parser drops a
//!   labeled merge into a lane with no commit yet, so a merge is emitted only
//!   where its lane already has one. Lane order is the order of the `branch`
//!   statements.
//! - Its parser always names the first lane `main`, whatever the default
//!   branch is called. The default branch is that lane, and its real name
//!   appears as a tag when the caller passes its ref.
//! - Commit IDs double as labels, and a parent is found by ID, so every ID in
//!   one diagram is unique.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::git_graph::{GitGraph, GraphLine, LaneEntry};
//! use biscuit_terminal::components::renderable::TerminalRenderable;
//! use biscuit_terminal::terminal::Terminal;
//!
//! let base = "4c2e4d5a".repeat(5);
//! let tip = "7a1b2c3d".repeat(5);
//! let graph = GitGraph::new("main", vec![LaneEntry::Commit(base.clone())])
//!     .with_ref("main", base.clone())
//!     .with_line(
//!         GraphLine::new("feat/theme")
//!             .forked_at(base)
//!             .with_entries(vec![LaneEntry::Commit(tip)]),
//!     )
//!     .with_current_branch("feat/theme");
//! print!("{}", graph.render(&Terminal::new()));
//! ```

use std::any::Any;
use std::collections::{HashMap, HashSet};

use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::tree::{RenderNode, TreeRenderable};

use crate::components::mermaid::{MermaidDiagram, MermaidRenderError, MermaidTheme, terminal_theme};
use crate::components::prose::Prose;
use crate::components::renderable::{BrowserRenderable, TerminalRenderable};
use crate::components::terminal_image::{
    ImageWidth, SCALE_REFERENCE_TEXT_UNITS, TerminalImage,
};
use crate::discovery::fonts::CellSize;
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

pub use biscuit_visualized::mermaid::NaturalSize;

/// The scale `GitGraph` draws at: gitGraph's 10-unit commit IDs and 14–16-unit
/// branch labels need more than the 100% body-text reference to read well.
pub const DEFAULT_GIT_GRAPH_SCALE: f32 = 1.25;

/// The renderer's name for the first lane (see the module docs).
const ROOT_LANE: &str = "main";

/// One entry on a line of work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneEntry {
    /// A commit, by full SHA.
    Commit(String),
    /// Commits the graph leaves out, drawn as one `+N` square.
    Elided(usize),
}

/// A branch and the commits it has that its parent lacks, oldest first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphLine {
    pub branch: String,
    /// The branch it forked from; `None` is the default branch.
    pub parent: Option<String>,
    /// Full SHA of the commit it forked at; `None` is an unknown connection,
    /// and the lane is drawn unconnected.
    pub fork_sha: Option<String>,
    /// Full SHA of the branch's own tip. A line without commits is labeled
    /// here; a line with commits defaults to its newest drawn commit.
    pub tip_sha: Option<String>,
    /// Full SHA of the merge commit that brought the line into another lane.
    pub merged_into: Option<String>,
    pub entries: Vec<LaneEntry>,
    /// Creation time (Unix seconds). Lanes forking at one commit are ordered
    /// by it, oldest first, and lines without one come last.
    pub created_at: Option<i64>,
    /// The tip commit's time (Unix seconds). The base view's height cap keeps
    /// the most recently active lanes.
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

    /// The merge commit on another lane that merged this line; its lane is
    /// found by where that commit is drawn.
    pub fn merged_into(mut self, sha: impl Into<String>) -> Self {
        self.merged_into = Some(sha.into());
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

    fn has_commits(&self) -> bool {
        self.entries.iter().any(|entry| matches!(entry, LaneEntry::Commit(_)))
    }

    /// The branch tip: [`tip_sha`](Self::tip_sha), else its newest commit;
    /// never the fork commit.
    fn tip(&self) -> Option<&str> {
        self.tip_sha.as_deref().or_else(|| last_commit(&self.entries))
    }
}

/// An open pull request, drawn as a tag on its source branch's tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPullRequest {
    pub number: u64,
    pub source_branch: String,
    pub target_branch: String,
}

/// The space a graph must fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphViewport {
    /// Columns available after margins.
    pub columns: u32,
    /// Terminal rows; the base view is capped at half of them.
    pub rows: u32,
    pub cell: CellSize,
}

impl GraphViewport {
    /// The viewport for `term`, after `layout`'s margins. An unknown cell size
    /// uses [`CellSize::FALLBACK`].
    pub fn for_terminal(term: &Terminal, layout: &Layout) -> Self {
        let dims = TerminalImage::resolve_dimensions_for(&ImageWidth::Fill, layout, term.width());
        Self {
            columns: dims.available_width,
            rows: term.height(),
            cell: term.cell_size().unwrap_or(CellSize::FALLBACK),
        }
    }
}

/// The graph chosen for a viewport.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphPlan {
    /// Mermaid `gitGraph` instructions.
    pub mermaid: String,
    /// Image columns at the graph's scale, before the clamp to the viewport.
    /// Greater than the viewport only when even the fully trimmed graph is too
    /// wide, and then the image shrinks.
    pub columns: u32,
    /// Image rows at the graph's scale.
    pub rows: u32,
    /// Lanes the base view's height cap left out.
    pub hidden_lanes: usize,
    /// Commits the width cap moved into `+N` squares.
    pub trimmed_commits: usize,
    /// Some history is not shown: an undrawn fork, merge, or tagged commit,
    /// or the caller's [`GitGraph::with_incomplete_history`].
    pub incomplete: bool,
}

/// A git graph for the terminal and the browser. See the module docs.
#[derive(Debug, Clone)]
pub struct GitGraph {
    default_branch: String,
    default_entries: Vec<LaneEntry>,
    lines: Vec<GraphLine>,
    refs: Vec<(String, String)>,
    pull_requests: Vec<GraphPullRequest>,
    current_branch: Option<String>,
    scale: f32,
    width: Option<ImageWidth>,
    theme: Option<MermaidTheme>,
    incomplete_history: bool,
    layout: Layout,
}

/// Which lane an entry sits on: `None` is the default lane, `Some(i)` is `lines[i]`.
type LaneKey = Option<usize>;

/// The lanes and entries one candidate graph draws.
#[derive(Debug, Clone)]
struct Draft {
    default_entries: Vec<LaneEntry>,
    /// Drawn lanes as `(index into lines, entries)`, in line order.
    lanes: Vec<(usize, Vec<LaneEntry>)>,
}

impl Draft {
    fn entries(&self, key: LaneKey) -> &[LaneEntry] {
        match key {
            None => &self.default_entries,
            Some(index) => self
                .lanes
                .iter()
                .find(|(line, _)| *line == index)
                .map(|(_, entries)| entries.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn entries_mut(&mut self, key: LaneKey) -> Option<&mut Vec<LaneEntry>> {
        match key {
            None => Some(&mut self.default_entries),
            Some(index) => self
                .lanes
                .iter_mut()
                .find(|(line, _)| *line == index)
                .map(|(_, entries)| entries),
        }
    }

    fn keys(&self) -> impl Iterator<Item = LaneKey> + '_ {
        std::iter::once(None).chain(self.lanes.iter().map(|(line, _)| Some(*line)))
    }
}

/// Emission facts derived from a draft.
struct Layouted {
    lane_names: HashMap<LaneKey, String>,
    /// Children attached after each `(lane, entry index)`, in lane order.
    /// Unconnected lanes are emitted after the default lane's first entry.
    children: HashMap<(LaneKey, usize), Vec<usize>>,
    /// Lanes with no drawn fork, declared before the default lane's first
    /// commit so their first commit has no parent.
    unconnected: Vec<usize>,
    /// Lanes merged at each drawn commit, in lane order.
    merges: HashMap<String, Vec<usize>>,
    tags: HashMap<String, Vec<String>>,
    /// Something the draft should show is not drawn.
    incomplete: bool,
}

/// Mermaid text and whether it leaves history out.
struct Emitted {
    text: String,
    incomplete: bool,
}

/// Emission progress: which lanes have been emitted in full, and `+N` IDs used.
#[derive(Default)]
struct EmitState {
    finished: HashSet<LaneKey>,
    elided_ids: HashSet<String>,
    incomplete: bool,
}

impl GitGraph {
    /// A graph of `default_branch` whose lane shows `default_entries`, oldest first.
    pub fn new(default_branch: impl Into<String>, default_entries: Vec<LaneEntry>) -> Self {
        Self {
            default_branch: default_branch.into(),
            default_entries,
            lines: Vec::new(),
            refs: Vec::new(),
            pull_requests: Vec::new(),
            current_branch: None,
            scale: DEFAULT_GIT_GRAPH_SCALE,
            width: None,
            theme: None,
            incomplete_history: false,
            layout: Layout::default(),
        }
    }

    /// Adds a line of work. Only the first line per branch name is used.
    pub fn with_line(mut self, line: GraphLine) -> Self {
        self.lines.push(line);
        self
    }

    /// Adds a ref tip (a branch, remote branch, or tag name and its full SHA).
    pub fn with_ref(mut self, name: impl Into<String>, sha: impl Into<String>) -> Self {
        self.refs.push((name.into(), sha.into()));
        self
    }

    pub fn with_pull_request(mut self, pull_request: GraphPullRequest) -> Self {
        self.pull_requests.push(pull_request);
        self
    }

    /// The checked-out branch. Unset, or the default branch, is the base view.
    pub fn with_current_branch(mut self, branch: impl Into<String>) -> Self {
        self.current_branch = Some(branch.into());
        self
    }

    /// The scale relative to terminal text (default [`DEFAULT_GIT_GRAPH_SCALE`]).
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Replaces the scale-derived width. A non-scale width is never trimmed
    /// to; `ImageWidth::Scale` replaces the scale.
    pub fn with_width(mut self, width: ImageWidth) -> Self {
        self.width = Some(width);
        self
    }

    /// The Mermaid theme; unset follows the terminal's color mode.
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// The caller could not establish some of the history it passed (an
    /// unknown connection, for instance); the rendered graph carries the
    /// incomplete-history notice.
    pub fn with_incomplete_history(mut self) -> Self {
        self.incomplete_history = true;
        self
    }

    /// The Mermaid instructions for the whole graph, with nothing trimmed.
    ///
    /// `None` when the default lane has no commit to hang the graph from.
    pub fn mermaid(&self) -> Option<String> {
        self.emit(&self.full_draft()).map(|emitted| emitted.text)
    }

    /// Fits the graph to `viewport`, measuring each candidate with the theme
    /// it will render with.
    ///
    /// `None` when there is nothing to draw (see [`mermaid`](Self::mermaid)).
    pub fn plan(&self, viewport: GraphViewport) -> Option<GitGraphPlan> {
        let theme = self.theme.unwrap_or_else(terminal_theme);
        self.plan_with(viewport, &|text: &str| {
            biscuit_visualized::mermaid::MermaidDiagram::new(text)
                .with_theme(theme)
                .natural_size()
                .ok()
        })
    }

    /// [`plan`](Self::plan) with an injected measurement of Mermaid text.
    ///
    /// Lanes are trimmed first (base view only): past half the viewport's rows,
    /// the default lane is kept and other lanes are added most recently active
    /// first, each with its drawn ancestors, until the next would not fit.
    /// A lane's drawn ancestors are the lanes holding its fork commit and its
    /// merge destination, and its parent's lane. Then, while the graph is wider
    /// than the viewport, commits that are not a lane tip, a fork point, a merge
    /// destination, or tagged move into `+N` squares one at a time: first a
    /// commit beside an existing square, else the oldest on the lane showing
    /// the most commits. When measuring fails, nothing is trimmed.
    pub fn plan_with(
        &self,
        viewport: GraphViewport,
        measure: &dyn Fn(&str) -> Option<NaturalSize>,
    ) -> Option<GitGraphPlan> {
        let scale = match self.width {
            Some(ImageWidth::Scale(scale)) => scale,
            _ => self.scale,
        };
        let trims_width = matches!(self.width, None | Some(ImageWidth::Scale(_)));
        let columns_of = |size: NaturalSize| ImageWidth::scaled_columns(scale, size.width, viewport.cell);
        let rows_of = |size: NaturalSize| rows_for(scale, size.height);

        let mut draft = self.full_draft();
        let full = self.emit(&draft)?;
        let Some(full_size) = measure(&full.text) else {
            return Some(GitGraphPlan {
                mermaid: full.text,
                columns: viewport.columns,
                rows: 0,
                hidden_lanes: 0,
                trimmed_commits: 0,
                incomplete: full.incomplete || self.incomplete_history,
            });
        };

        let eligible = draft.lanes.len();
        let max_rows = (viewport.rows / 2).max(1);
        if self.is_base_view() && rows_of(full_size) > max_rows {
            draft = self.fit_lanes(draft, max_rows, &|text| measure(text).map(rows_of));
        }
        let hidden_lanes = eligible - draft.lanes.len();

        let mut emitted = self.emit(&draft)?;
        let mut size = measure(&emitted.text).unwrap_or(full_size);
        let mut trimmed_commits = 0;
        while trims_width && columns_of(size) > viewport.columns {
            let Some(next) = self.trim_one_commit(&draft) else {
                break;
            };
            let Some(next_emitted) = self.emit(&next) else {
                break;
            };
            let Some(next_size) = measure(&next_emitted.text) else {
                break;
            };
            draft = next;
            emitted = next_emitted;
            size = next_size;
            trimmed_commits += 1;
        }

        let columns = match &self.width {
            Some(width @ (ImageWidth::Fill | ImageWidth::Percent(_) | ImageWidth::Characters(_))) => {
                TerminalImage::resolve_dimensions_for(width, &Layout::default(), viewport.columns)
                    .image_width
            }
            _ => columns_of(size),
        };
        Some(GitGraphPlan {
            mermaid: emitted.text,
            columns,
            rows: rows_of(size),
            hidden_lanes,
            trimmed_commits,
            incomplete: emitted.incomplete || self.incomplete_history,
        })
    }

    /// Renders the graph, or explains why it cannot.
    ///
    /// ## Errors
    ///
    /// Returns the [`MermaidRenderError`] of the underlying diagram, and
    /// `MermaidRenderError::DisplayError` when there is nothing to draw.
    pub fn try_render(&self, term: &Terminal) -> Result<String, MermaidRenderError> {
        let plan = self
            .plan(GraphViewport::for_terminal(term, &self.layout))
            .ok_or_else(|| MermaidRenderError::DisplayError("the graph has no commits".into()))?;
        let result = self.diagram(&plan.mermaid).try_render(term)?;
        Ok(with_notes(result.output, &plan, term))
    }

    fn diagram(&self, mermaid: &str) -> MermaidDiagram {
        let mut diagram = MermaidDiagram::new(mermaid)
            .with_width(self.width.clone().unwrap_or(ImageWidth::Scale(self.scale)));
        if let Some(theme) = self.theme {
            diagram = diagram.with_theme(theme);
        }
        *diagram.layout_mut() = self.layout.clone();
        diagram
    }

    fn is_base_view(&self) -> bool {
        self.current_branch
            .as_deref()
            .is_none_or(|current| current == self.default_branch)
    }

    /// Whether the view draws `branch`: every branch in the base view; the
    /// current branch, its non-default parent, and `origin/<default>` otherwise.
    fn in_view(&self, branch: &str) -> bool {
        if self.is_base_view() {
            return true;
        }
        let current = self.current_branch.as_deref();
        let current_parent = current.and_then(|name| {
            self.lines
                .iter()
                .find(|line| line.branch == name)
                .and_then(|line| line.parent.as_deref())
                .filter(|parent| *parent != self.default_branch)
        });
        Some(branch) == current || Some(branch) == current_parent || branch == format!("origin/{}", self.default_branch)
    }

    /// Indices of the first line per branch name, the default branch excluded.
    fn distinct_lines(&self) -> impl Iterator<Item = (usize, &GraphLine)> + '_ {
        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter(move |(_, line)| seen.insert(line.branch.as_str()))
            .filter(|(_, line)| line.branch != self.default_branch)
    }

    /// Indices of the lines that get a lane under the lane rule.
    fn eligible_lanes(&self) -> Vec<usize> {
        self.distinct_lines()
            .filter(|(_, line)| line.has_commits() && self.in_view(&line.branch))
            .map(|(index, _)| index)
            .collect()
    }

    fn full_draft(&self) -> Draft {
        Draft {
            default_entries: self.default_entries.clone(),
            lanes: self
                .eligible_lanes()
                .into_iter()
                .map(|index| (index, self.lines[index].entries.clone()))
                .collect(),
        }
    }

    /// Where lane `index` hangs from: its fork commit wherever that is drawn
    /// on another lane; `None` (unconnected) when it is not.
    fn attach_point(&self, index: usize, positions: &HashMap<&str, (LaneKey, usize)>) -> Option<(LaneKey, usize)> {
        let (key, position) = *positions.get(self.lines[index].fork_sha.as_deref()?)?;
        (key != Some(index)).then_some((key, position))
    }

    fn arrange(&self, draft: &Draft) -> Option<Layouted> {
        first_commit_index(&draft.default_entries)?;
        let positions = drawn_positions(draft);

        // Attach every lane. A cycle (inconsistent input) is broken by
        // leaving the lane that closes it unconnected.
        let mut attach: HashMap<usize, Option<(LaneKey, usize)>> = draft
            .lanes
            .iter()
            .map(|(index, _)| (*index, self.attach_point(*index, &positions)))
            .collect();
        for (index, _) in &draft.lanes {
            let mut seen = HashSet::from([*index]);
            let mut cursor = attach[index].and_then(|point| point.0);
            while let Some(parent) = cursor {
                if !seen.insert(parent) {
                    attach.insert(*index, None);
                    break;
                }
                cursor = attach.get(&parent).copied().flatten().and_then(|point| point.0);
            }
        }

        let mut ordered: Vec<usize> = draft.lanes.iter().map(|(index, _)| *index).collect();
        ordered.sort_by_key(|index| (self.lines[*index].created_at.is_none(), self.lines[*index].created_at, *index));
        let mut children: HashMap<(LaneKey, usize), Vec<usize>> = HashMap::new();
        let mut unconnected = Vec::new();
        for index in ordered {
            let point = attach[&index].unwrap_or_else(|| {
                unconnected.push(index);
                (None, 0)
            });
            children.entry(point).or_default().push(index);
        }
        let merge_lane = |index: usize| {
            let destination = self.lines[index].merged_into.as_deref()?;
            positions.get(destination).and_then(|(key, _)| *key)
        };
        for siblings in children.values_mut() {
            emit_merged_lanes_first(siblings, &merge_lane, &|lane| attach.get(&lane).copied().flatten().and_then(|point| point.0));
        }

        let mut merges: HashMap<String, Vec<usize>> = HashMap::new();
        let mut incomplete = !unconnected.is_empty();
        for (index, _) in &draft.lanes {
            let Some(destination) = self.lines[*index].merged_into.as_deref() else {
                continue;
            };
            match positions.get(destination) {
                Some((key, _)) if *key != Some(*index) => merges.entry(destination.to_string()).or_default().push(*index),
                _ => incomplete = true,
            }
        }

        let mut lane_names = HashMap::from([(None, ROOT_LANE.to_string())]);
        let mut used = HashSet::from([ROOT_LANE.to_string()]);
        for (index, _) in &draft.lanes {
            // `~` cannot appear in a git branch name, so the suffix never
            // collides with a real branch.
            let mut name = self.lines[*index].branch.clone();
            while !used.insert(name.clone()) {
                name.push('~');
            }
            lane_names.insert(Some(*index), name);
        }

        let (tags, unplaced) = self.tags(draft, &positions);
        Some(Layouted {
            lane_names,
            children,
            unconnected,
            merges,
            tags,
            incomplete: incomplete || unplaced,
        })
    }

    /// Commits of lanes the draft leaves out under the height cap, whose
    /// labels its own note accounts for.
    fn hidden_commits(&self, draft: &Draft) -> HashSet<&str> {
        self.eligible_lanes()
            .into_iter()
            .filter(|index| !draft.lanes.iter().any(|(drawn, _)| drawn == index))
            .flat_map(|index| {
                let line = &self.lines[index];
                line.entries
                    .iter()
                    .filter_map(|entry| match entry {
                        LaneEntry::Commit(sha) => Some(sha.as_str()),
                        LaneEntry::Elided(_) => None,
                    })
                    .chain(line.tip_sha.as_deref())
            })
            .collect()
    }

    /// Tags by commit, and whether any tag the view should show has no drawn
    /// commit to sit on.
    fn tags(&self, draft: &Draft, positions: &HashMap<&str, (LaneKey, usize)>) -> (HashMap<String, Vec<String>>, bool) {
        let drawn_tip = |key: LaneKey| last_commit(draft.entries(key));
        let lane_of = |branch: &str| {
            draft
                .lanes
                .iter()
                .find(|(index, _)| self.lines[*index].branch == branch)
                .map(|(index, _)| Some(*index))
        };
        let tip_of = |branch: &str| -> Option<String> {
            if let Some(key) = lane_of(branch) {
                return drawn_tip(key).map(str::to_string);
            }
            if let Some(line) = self.lines.iter().find(|line| line.branch == branch) {
                return line.tip().map(str::to_string);
            }
            self.refs
                .iter()
                .find(|(name, _)| name == branch)
                .map(|(_, sha)| sha.clone())
        };

        let hidden = self.hidden_commits(draft);
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        let mut unplaced = false;
        // `accountable`: the view should show this tag, so an undrawn commit
        // is reported rather than silently skipped.
        let mut add = |sha: Option<&str>, tag: String, accountable: bool| match sha {
            Some(sha) if positions.contains_key(sha) => {
                let list = tags.entry(sha.to_string()).or_default();
                if !list.contains(&tag) {
                    list.push(tag);
                }
            }
            Some(sha) if hidden.contains(sha) => {}
            _ => unplaced |= accountable,
        };

        for (name, sha) in &self.refs {
            // A lane's label already names its own tip.
            let labels_it = lane_of(name).is_some_and(|key| drawn_tip(key) == Some(sha.as_str()));
            if !labels_it {
                add(Some(sha), name.clone(), true);
            }
        }
        for (_, line) in self.distinct_lines() {
            if lane_of(&line.branch).is_some() || line.has_commits() {
                continue;
            }
            add(line.tip(), line.branch.clone(), self.in_view(&line.branch));
        }
        for pull_request in &self.pull_requests {
            let source = &pull_request.source_branch;
            let tip = tip_of(source);
            let accountable = tip.is_some() && self.in_view(source);
            add(
                tip.as_deref(),
                format!("PR #{} → {}", pull_request.number, pull_request.target_branch),
                accountable,
            );
        }
        (tags, unplaced)
    }

    fn emit(&self, draft: &Draft) -> Option<Emitted> {
        let arranged = self.arrange(draft)?;
        let ids = commit_ids(draft);
        let mut state = EmitState::default();
        let mut lines = vec!["gitGraph".to_string()];
        if !arranged.unconnected.is_empty() {
            for index in &arranged.unconnected {
                lines.push(format!("    branch {}", arranged.lane_names[&Some(*index)]));
            }
            lines.push(format!("    checkout {ROOT_LANE}"));
        }
        self.emit_lane(draft, None, false, &arranged, &ids, &mut state, &mut lines);
        Some(Emitted {
            text: lines.join("\n"),
            incomplete: arranged.incomplete || state.incomplete,
        })
    }

    /// Emits lane `key`'s entries, each followed by the lanes that hang from
    /// it. `has_head`: the lane already has a commit to merge into (a
    /// connected lane starts at its fork commit).
    #[allow(clippy::too_many_arguments)]
    fn emit_lane(
        &self,
        draft: &Draft,
        key: LaneKey,
        mut has_head: bool,
        arranged: &Layouted,
        ids: &HashMap<String, String>,
        state: &mut EmitState,
        out: &mut Vec<String>,
    ) {
        for (position, entry) in draft.entries(key).iter().enumerate() {
            match entry {
                LaneEntry::Commit(sha) => {
                    let mut line = match self.merged_lane(sha, has_head, arranged, state) {
                        Some(merged) => format!("    merge {} id: \"{}\"", arranged.lane_names[&Some(merged)], ids[sha]),
                        None => format!("    commit id: \"{}\"", ids[sha]),
                    };
                    for tag in arranged.tags.get(sha).into_iter().flatten() {
                        line.push_str(&format!(" tag: \"{}\"", tag.replace('"', "'")));
                    }
                    out.push(line);
                }
                LaneEntry::Elided(count) => {
                    // Parents are found by ID, so a repeated `+N` gets
                    // trailing spaces, which do not show.
                    let mut id = format!("+{count}");
                    while !state.elided_ids.insert(id.clone()) {
                        id.push(' ');
                    }
                    out.push(format!("    commit id: \"{id}\" type: HIGHLIGHT"));
                }
            }
            has_head = true;
            for child in arranged.children.get(&(key, position)).into_iter().flatten() {
                let name = &arranged.lane_names[&Some(*child)];
                let connected = !arranged.unconnected.contains(child);
                if connected {
                    out.push(format!("    branch {name}"));
                }
                out.push(format!("    checkout {name}"));
                self.emit_lane(draft, Some(*child), connected, arranged, ids, state, out);
                out.push(format!("    checkout {}", arranged.lane_names[&key]));
            }
        }
        state.finished.insert(key);
    }

    /// The lane merged at `sha`, when a merge can be drawn there: the merged
    /// lane is already emitted in full and this lane has a commit to merge
    /// into. A merge that cannot be drawn marks the graph incomplete; a second
    /// lane merged at one commit is never drawn.
    fn merged_lane(&self, sha: &str, has_head: bool, arranged: &Layouted, state: &mut EmitState) -> Option<usize> {
        let merged = arranged.merges.get(sha)?;
        let drawn = merged
            .first()
            .copied()
            .filter(|lane| has_head && state.finished.contains(&Some(*lane)));
        if drawn.is_none() || merged.len() > 1 {
            state.incomplete = true;
        }
        drawn
    }

    /// The lanes lane `index` needs drawn: those holding its fork commit and
    /// its merge destination, and its parent's lane.
    fn lane_ancestors(&self, full: &Draft, index: usize, positions: &HashMap<&str, (LaneKey, usize)>) -> Vec<usize> {
        let line = &self.lines[index];
        let on_lane = |sha: Option<&str>| sha.and_then(|sha| positions.get(sha)).and_then(|(key, _)| *key);
        let parent = line.parent.as_deref().and_then(|parent| {
            full.lanes
                .iter()
                .map(|(other, _)| *other)
                .find(|other| self.lines[*other].branch == parent)
        });
        [on_lane(line.fork_sha.as_deref()), on_lane(line.merged_into.as_deref()), parent]
            .into_iter()
            .flatten()
            .filter(|other| *other != index)
            .collect()
    }

    /// Keeps the default lane plus, most recently active first, each lane and
    /// its drawn ancestors while the graph stays within `max_rows`.
    fn fit_lanes(&self, full: Draft, max_rows: u32, rows_of: &dyn Fn(&str) -> Option<u32>) -> Draft {
        let mut candidates: Vec<usize> = full.lanes.iter().map(|(index, _)| *index).collect();
        candidates.sort_by_key(|index| {
            let line = &self.lines[*index];
            (line.last_active.is_none(), std::cmp::Reverse(line.last_active), *index)
        });

        let origin_peer = format!("origin/{}", self.default_branch);
        let mut keep: HashSet<usize> = candidates
            .iter()
            .copied()
            .filter(|index| self.lines[*index].branch == origin_peer)
            .collect();
        let draft_of = |keep: &HashSet<usize>| Draft {
            default_entries: full.default_entries.clone(),
            lanes: full
                .lanes
                .iter()
                .filter(|(index, _)| keep.contains(index))
                .cloned()
                .collect(),
        };

        let positions = drawn_positions(&full);
        for candidate in candidates {
            if keep.contains(&candidate) {
                continue;
            }
            let mut tentative = keep.clone();
            let mut pending = vec![candidate];
            while let Some(index) = pending.pop() {
                if tentative.insert(index) {
                    pending.extend(self.lane_ancestors(&full, index, &positions));
                }
            }
            let fits = self
                .emit(&draft_of(&tentative))
                .and_then(|emitted| rows_of(&emitted.text))
                .is_some_and(|rows| rows <= max_rows);
            if !fits {
                break;
            }
            keep = tentative;
        }
        draft_of(&keep)
    }

    /// Moves one commit into a `+N` square, or `None` when every drawn commit
    /// is a lane tip, a fork point, a merge destination, or tagged.
    fn trim_one_commit(&self, draft: &Draft) -> Option<Draft> {
        let arranged = self.arrange(draft)?;
        let mut pinned: HashSet<(LaneKey, usize)> = arranged
            .children
            .iter()
            .filter(|(_, lanes)| lanes.iter().any(|lane| !arranged.unconnected.contains(lane)))
            .map(|(point, _)| *point)
            .collect();
        for key in draft.keys() {
            let entries = draft.entries(key);
            if let Some(tip) = entries.iter().rposition(|entry| matches!(entry, LaneEntry::Commit(_))) {
                pinned.insert((key, tip));
            }
            for (position, entry) in entries.iter().enumerate() {
                if let LaneEntry::Commit(sha) = entry
                    && (arranged.tags.contains_key(sha) || arranged.merges.contains_key(sha))
                {
                    pinned.insert((key, position));
                }
            }
        }

        // A commit beside a `+N` square folds into it and removes a node; a
        // lone commit becomes a `+1` square and saves only its label's width.
        // So merges come first, then the lane showing the most commits, and
        // ties go to the earlier lane.
        let (_, _, _, key, position) = draft
            .keys()
            .enumerate()
            .filter_map(|(order, key)| {
                let entries = draft.entries(key);
                let unpinned: Vec<usize> = entries
                    .iter()
                    .enumerate()
                    .filter(|(position, entry)| {
                        matches!(entry, LaneEntry::Commit(_)) && !pinned.contains(&(key, *position))
                    })
                    .map(|(position, _)| position)
                    .collect();
                let beside_elision = unpinned.iter().copied().find(|position| {
                    let elided = |at: Option<usize>| {
                        at.and_then(|at| entries.get(at))
                            .is_some_and(|entry| matches!(entry, LaneEntry::Elided(_)))
                    };
                    elided(position.checked_sub(1)) || elided(Some(position + 1))
                });
                let position = beside_elision.or_else(|| unpinned.first().copied())?;
                let shown = entries.iter().filter(|entry| matches!(entry, LaneEntry::Commit(_))).count();
                Some((beside_elision.is_some(), shown, std::cmp::Reverse(order), key, position))
            })
            .max_by_key(|(merges, shown, order, _, _)| (*merges, *shown, *order))?;

        let mut next = draft.clone();
        let entries = next.entries_mut(key)?;
        entries[position] = LaneEntry::Elided(1);
        merge_elisions(entries);
        Some(next)
    }
}

/// Image rows for `natural_height` units at `scale`: a cell is
/// [`SCALE_REFERENCE_TEXT_UNITS`] units tall at scale 1.0, whatever its pixel
/// size.
fn rows_for(scale: f32, natural_height: f32) -> u32 {
    ((natural_height * scale / SCALE_REFERENCE_TEXT_UNITS).ceil() as u32).max(1)
}

/// Reorders lanes that hang from one commit so a lane merged into a sibling,
/// or into a lane hanging from that sibling, comes before it: a merge is drawn
/// only after its lane's commits. Otherwise the order is kept. `hangs_from`
/// must be acyclic (`arrange` breaks cycles first).
fn emit_merged_lanes_first(
    siblings: &mut Vec<usize>,
    merge_lane: &dyn Fn(usize) -> Option<usize>,
    hangs_from: &dyn Fn(usize) -> Option<usize>,
) {
    let target = |lane: usize, siblings: &[usize]| {
        let mut cursor = merge_lane(lane);
        while let Some(current) = cursor {
            if siblings.contains(&current) {
                return (current != lane).then_some(current);
            }
            cursor = hangs_from(current);
        }
        None
    };
    // Bounded: each move places a lane before its target; a cycle of targets
    // (inconsistent input) stops after `len²` moves.
    for _ in 0..siblings.len() * siblings.len() {
        let misplaced = siblings.iter().enumerate().find_map(|(at, &lane)| {
            let target_at = siblings.iter().position(|&other| Some(other) == target(lane, siblings))?;
            (target_at < at).then_some((at, target_at))
        });
        let Some((at, target_at)) = misplaced else {
            return;
        };
        let lane = siblings.remove(at);
        siblings.insert(target_at, lane);
    }
}

/// Where each drawn commit sits: its first lane and entry index.
fn drawn_positions(draft: &Draft) -> HashMap<&str, (LaneKey, usize)> {
    let mut positions = HashMap::new();
    for key in draft.keys() {
        for (position, entry) in draft.entries(key).iter().enumerate() {
            if let LaneEntry::Commit(sha) = entry {
                positions.entry(sha.as_str()).or_insert((key, position));
            }
        }
    }
    positions
}

/// Appends the dim notes below a rendered graph: lanes the height cap left
/// out, then history not shown.
fn with_notes(mut output: String, plan: &GitGraphPlan, term: &Terminal) -> String {
    let mut notes = Vec::new();
    if plan.hidden_lanes > 0 {
        let noun = if plan.hidden_lanes == 1 { "worktree" } else { "worktrees" };
        notes.push(format!("{} more {noun} not shown", plan.hidden_lanes));
    }
    if plan.incomplete {
        notes.push(INCOMPLETE_HISTORY_NOTE.to_string());
    }
    for note in notes {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(Prose::new(format!("<dim>{note}</dim>")).render(term).trim_end());
        output.push('\n');
    }
    output
}

/// The notice under a graph whose [`GitGraphPlan::incomplete`] is set.
pub const INCOMPLETE_HISTORY_NOTE: &str = "Some history is not shown";

fn last_commit(entries: &[LaneEntry]) -> Option<&str> {
    entries.iter().rev().find_map(|entry| match entry {
        LaneEntry::Commit(sha) => Some(sha.as_str()),
        LaneEntry::Elided(_) => None,
    })
}

fn first_commit_index(entries: &[LaneEntry]) -> Option<usize> {
    entries.iter().position(|entry| matches!(entry, LaneEntry::Commit(_)))
}

/// Merges adjacent `+N` squares into one.
fn merge_elisions(entries: &mut Vec<LaneEntry>) {
    let mut merged: Vec<LaneEntry> = Vec::with_capacity(entries.len());
    for entry in entries.drain(..) {
        match (merged.last_mut(), entry) {
            (Some(LaneEntry::Elided(total)), LaneEntry::Elided(count)) => *total += count,
            (_, entry) => merged.push(entry),
        }
    }
    *entries = merged;
}

/// Display IDs for every drawn commit: the first 7 characters of the SHA,
/// lengthened only where two SHAs would share an ID.
fn commit_ids(draft: &Draft) -> HashMap<String, String> {
    let shas: Vec<&str> = draft
        .keys()
        .flat_map(|key| draft.entries(key))
        .filter_map(|entry| match entry {
            LaneEntry::Commit(sha) => Some(sha.as_str()),
            LaneEntry::Elided(_) => None,
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    shas.iter()
        .map(|sha| {
            let mut length = 7.min(sha.len());
            while length < sha.len()
                && shas
                    .iter()
                    .any(|other| other != sha && other.len() >= length && other[..length] == sha[..length])
            {
                length += 1;
            }
            (sha.to_string(), sha[..length].to_string())
        })
        .collect()
}

impl TerminalRenderable for GitGraph {
    /// The fitted graph as an image, followed by dim notes when lanes or
    /// history were left out; the diagram's code block on terminals without
    /// images.
    fn render(&self, term: &Terminal) -> String {
        let Some(plan) = self.plan(GraphViewport::for_terminal(term, &self.layout)) else {
            return String::new();
        };
        let output = self.diagram(&plan.mermaid).render(term);
        with_notes(output, &plan, term)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl TreeRenderable for GitGraph {
    /// The untrimmed graph as [`MermaidDiagram`] projects it: an image node
    /// whose alt text is the Mermaid source.
    fn render_tree(&self) -> RenderNode {
        match self.mermaid() {
            Some(mermaid) => self.diagram(&mermaid).render_tree(),
            None => RenderNode::paragraph(Vec::new()),
        }
    }
}

impl BrowserRenderable for GitGraph {
    /// The untrimmed graph's SVG as a raw-HTML island (the default theme
    /// unless one is set); the Mermaid source in a code block if rendering fails.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let html = match self.mermaid() {
            None => String::new(),
            Some(mermaid) => {
                let diagram = MermaidDiagram::new(mermaid.as_str())
                    .with_theme(self.theme.unwrap_or(MermaidTheme::Default));
                match diagram.render_to_svg() {
                    Ok(svg) => svg,
                    Err(error) => format!(
                        "<!-- biscuit-terminal git graph render failed: {} -->\n<pre><code class=\"language-mermaid\">{}</code></pre>",
                        html_escape(&error.to_string()),
                        html_escape(&mermaid),
                    ),
                }
            }
        };
        BrowserFragment::new().define_as_raw_html(html).finalize()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests;
