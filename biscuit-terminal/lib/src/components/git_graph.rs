//! Git branch topology drawn as a Mermaid `gitGraph` through [`MermaidDiagram`].
//!
//! Callers gather the git facts (lines of work with full commit SHAs, ref
//! tips, open PRs) and hand them over typed; the component runs no git
//! commands. It owns the lane/tag rule, lane ordering, elision markers, the
//! `mermaid-rs-renderer` workarounds, and fitting the graph to the terminal by
//! trimming. The design is item 5 ("Graph View") of the worktree fix
//! `2026-09-24-ux-improvements`.
//!
//! ## Lanes and tags
//!
//! A **lane** is a line of work whose commits are on no other drawn lane: the
//! default branch, the current branch, its fork parent when that is not the
//! default branch, and `origin/<default>` when it has commits of its own
//! (it has diverged). With no current branch, or the default branch checked
//! out, every line with commits of its own gets a lane. Every other ref whose
//! tip is a drawn commit is a **tag** on that commit, and an open PR is a tag
//! on its source branch's tip; the precise form is documented in
//! `worktree/docs/cli/list.md`.
//!
//! ## Renderer workarounds
//!
//! - `mermaid-rs-renderer` reads everything after `branch` or `merge` as the
//!   branch name, attributes included, so no attributes are emitted on those
//!   statements. Lane order is the order of the `branch` statements.
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
    /// Full SHA of the commit it forked at. A line with no commits of its own
    /// has its tip here.
    pub fork_sha: Option<String>,
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

    /// The branch tip: its newest commit, or the fork commit when it has none.
    fn tip(&self) -> Option<&str> {
        last_commit(&self.entries).or(self.fork_sha.as_deref())
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
    children: HashMap<(LaneKey, usize), Vec<usize>>,
    tags: HashMap<String, Vec<String>>,
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

    /// The Mermaid instructions for the whole graph, with nothing trimmed.
    ///
    /// `None` when the default lane has no commit to hang the graph from.
    pub fn mermaid(&self) -> Option<String> {
        self.emit(&self.full_draft())
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
    /// Then, while the graph is wider than the viewport, commits that are not a
    /// lane tip, a fork point, or tagged move into `+N` squares one at a time:
    /// first a commit beside an existing square, else the oldest on the lane
    /// showing the most commits. When measuring fails, nothing is trimmed.
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
        let full_text = self.emit(&draft)?;
        let Some(full_size) = measure(&full_text) else {
            return Some(GitGraphPlan {
                mermaid: full_text,
                columns: viewport.columns,
                rows: 0,
                hidden_lanes: 0,
                trimmed_commits: 0,
            });
        };

        let eligible = draft.lanes.len();
        let max_rows = (viewport.rows / 2).max(1);
        if self.is_base_view() && rows_of(full_size) > max_rows {
            draft = self.fit_lanes(draft, max_rows, &|text| measure(text).map(rows_of));
        }
        let hidden_lanes = eligible - draft.lanes.len();

        let mut text = self.emit(&draft)?;
        let mut size = measure(&text).unwrap_or(full_size);
        let mut trimmed_commits = 0;
        while trims_width && columns_of(size) > viewport.columns {
            let Some(next) = self.trim_one_commit(&draft) else {
                break;
            };
            let Some(next_text) = self.emit(&next) else {
                break;
            };
            let Some(next_size) = measure(&next_text) else {
                break;
            };
            draft = next;
            text = next_text;
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
            mermaid: text,
            columns,
            rows: rows_of(size),
            hidden_lanes,
            trimmed_commits,
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
        Ok(self.with_hidden_lanes_note(result.output, plan.hidden_lanes, term))
    }

    fn with_hidden_lanes_note(&self, mut output: String, hidden_lanes: usize, term: &Terminal) -> String {
        if hidden_lanes > 0 {
            let noun = if hidden_lanes == 1 { "worktree" } else { "worktrees" };
            let note = Prose::new(format!("<dim>{hidden_lanes} more {noun} not shown</dim>"));
            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(note.render(term).trim_end());
            output.push('\n');
        }
        output
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

    /// Indices of the lines that get a lane under the lane rule.
    fn eligible_lanes(&self) -> Vec<usize> {
        let base_view = self.is_base_view();
        let current = self.current_branch.as_deref();
        let current_parent = current.and_then(|name| {
            self.lines
                .iter()
                .find(|line| line.branch == name)
                .and_then(|line| line.parent.as_deref())
                .filter(|parent| *parent != self.default_branch)
        });
        let origin_peer = format!("origin/{}", self.default_branch);

        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| seen.insert(line.branch.as_str()))
            .filter(|(_, line)| line.branch != self.default_branch && line.has_commits())
            .filter(|(_, line)| {
                base_view
                    || Some(line.branch.as_str()) == current
                    || Some(line.branch.as_str()) == current_parent
                    || line.branch == origin_peer
            })
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

    /// The lane `index` hangs from: its fork commit wherever that is drawn,
    /// else the start of its drawn parent lane, else the start of the default lane.
    fn attach_point(&self, draft: &Draft, index: usize, positions: &HashMap<&str, (LaneKey, usize)>) -> (LaneKey, usize) {
        let line = &self.lines[index];
        if let Some(&(key, position)) = line.fork_sha.as_deref().and_then(|sha| positions.get(sha))
            && key != Some(index)
        {
            return (key, position);
        }
        let parent_lane = line.parent.as_deref().and_then(|parent| {
            draft
                .lanes
                .iter()
                .find(|(other, _)| self.lines[*other].branch == parent)
                .map(|(other, _)| Some(*other))
        });
        let key = parent_lane.filter(|key| *key != Some(index)).unwrap_or(None);
        let first = first_commit_index(draft.entries(key)).unwrap_or(0);
        (key, first)
    }

    fn arrange(&self, draft: &Draft) -> Option<Layouted> {
        first_commit_index(&draft.default_entries)?;

        let mut positions: HashMap<&str, (LaneKey, usize)> = HashMap::new();
        for key in draft.keys() {
            for (position, entry) in draft.entries(key).iter().enumerate() {
                if let LaneEntry::Commit(sha) = entry {
                    positions.entry(sha.as_str()).or_insert((key, position));
                }
            }
        }

        // Attach every lane, breaking any cycle (inconsistent input) by moving
        // the lane to the start of the default lane.
        let mut attach: HashMap<usize, (LaneKey, usize)> = draft
            .lanes
            .iter()
            .map(|(index, _)| (*index, self.attach_point(draft, *index, &positions)))
            .collect();
        let default_start = (None, first_commit_index(&draft.default_entries).unwrap_or(0));
        for (index, _) in &draft.lanes {
            let mut seen = HashSet::from([*index]);
            let mut cursor = attach[index].0;
            while let Some(parent) = cursor {
                if !seen.insert(parent) {
                    attach.insert(*index, default_start);
                    break;
                }
                cursor = attach.get(&parent).and_then(|point| point.0);
            }
        }

        let mut ordered: Vec<usize> = draft.lanes.iter().map(|(index, _)| *index).collect();
        ordered.sort_by_key(|index| (self.lines[*index].created_at.is_none(), self.lines[*index].created_at, *index));
        let mut children: HashMap<(LaneKey, usize), Vec<usize>> = HashMap::new();
        for index in ordered {
            children.entry(attach[&index]).or_default().push(index);
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

        Some(Layouted {
            lane_names,
            children,
            tags: self.tags(draft, &positions),
        })
    }

    fn tags(&self, draft: &Draft, positions: &HashMap<&str, (LaneKey, usize)>) -> HashMap<String, Vec<String>> {
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

        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        let mut add = |sha: &str, tag: String| {
            if positions.contains_key(sha) {
                let list = tags.entry(sha.to_string()).or_default();
                if !list.contains(&tag) {
                    list.push(tag);
                }
            }
        };

        for (name, sha) in &self.refs {
            // A lane's label already names its own tip.
            let labels_it = lane_of(name).is_some_and(|key| drawn_tip(key) == Some(sha.as_str()));
            if !labels_it {
                add(sha, name.clone());
            }
        }
        let mut seen = HashSet::new();
        for line in &self.lines {
            if !seen.insert(line.branch.as_str())
                || line.branch == self.default_branch
                || lane_of(&line.branch).is_some()
                || line.has_commits()
            {
                continue;
            }
            if let Some(tip) = line.tip() {
                add(tip, line.branch.clone());
            }
        }
        for pull_request in &self.pull_requests {
            if let Some(tip) = tip_of(&pull_request.source_branch) {
                add(
                    &tip,
                    format!("PR #{} → {}", pull_request.number, pull_request.target_branch),
                );
            }
        }
        tags
    }

    fn emit(&self, draft: &Draft) -> Option<String> {
        let arranged = self.arrange(draft)?;
        let ids = commit_ids(draft);
        let mut elided_ids = HashSet::new();
        let mut lines = vec!["gitGraph".to_string()];
        self.emit_lane(draft, None, &arranged, &ids, &mut elided_ids, &mut lines);
        Some(lines.join("\n"))
    }

    fn emit_lane(
        &self,
        draft: &Draft,
        key: LaneKey,
        arranged: &Layouted,
        ids: &HashMap<String, String>,
        elided_ids: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        for (position, entry) in draft.entries(key).iter().enumerate() {
            match entry {
                LaneEntry::Commit(sha) => {
                    let mut line = format!("    commit id: \"{}\"", ids[sha]);
                    for tag in arranged.tags.get(sha).into_iter().flatten() {
                        line.push_str(&format!(" tag: \"{}\"", tag.replace('"', "'")));
                    }
                    out.push(line);
                }
                LaneEntry::Elided(count) => {
                    // Parents are found by ID, so a repeated `+N` gets
                    // trailing spaces, which do not show.
                    let mut id = format!("+{count}");
                    while !elided_ids.insert(id.clone()) {
                        id.push(' ');
                    }
                    out.push(format!("    commit id: \"{id}\" type: HIGHLIGHT"));
                }
            }
            for child in arranged.children.get(&(key, position)).into_iter().flatten() {
                let name = &arranged.lane_names[&Some(*child)];
                out.push(format!("    branch {name}"));
                out.push(format!("    checkout {name}"));
                self.emit_lane(draft, Some(*child), arranged, ids, elided_ids, out);
                out.push(format!("    checkout {}", arranged.lane_names[&key]));
            }
        }
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

        for candidate in candidates {
            if keep.contains(&candidate) {
                continue;
            }
            let mut tentative = keep.clone();
            let mut cursor = Some(candidate);
            while let Some(index) = cursor {
                if !tentative.insert(index) {
                    break;
                }
                cursor = self.lines[index].parent.as_deref().and_then(|parent| {
                    full.lanes
                        .iter()
                        .map(|(other, _)| *other)
                        .find(|other| self.lines[*other].branch == parent)
                });
            }
            let fits = self
                .emit(&draft_of(&tentative))
                .and_then(|text| rows_of(&text))
                .is_some_and(|rows| rows <= max_rows);
            if !fits {
                break;
            }
            keep = tentative;
        }
        draft_of(&keep)
    }

    /// Moves one commit into a `+N` square, or `None` when every drawn commit
    /// is a lane tip, a fork point, or tagged.
    fn trim_one_commit(&self, draft: &Draft) -> Option<Draft> {
        let arranged = self.arrange(draft)?;
        let mut pinned: HashSet<(LaneKey, usize)> = arranged.children.keys().copied().collect();
        for key in draft.keys() {
            let entries = draft.entries(key);
            if let Some(tip) = entries.iter().rposition(|entry| matches!(entry, LaneEntry::Commit(_))) {
                pinned.insert((key, tip));
            }
            for (position, entry) in entries.iter().enumerate() {
                if let LaneEntry::Commit(sha) = entry
                    && arranged.tags.contains_key(sha)
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
    /// The fitted graph as an image, followed by a dim note when lanes were
    /// left out; the diagram's code block on terminals without images.
    fn render(&self, term: &Terminal) -> String {
        let Some(plan) = self.plan(GraphViewport::for_terminal(term, &self.layout)) else {
            return String::new();
        };
        let output = self.diagram(&plan.mermaid).render(term);
        self.with_hidden_lanes_note(output, plan.hidden_lanes, term)
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
