//! The `wt list` table: the caption, the Worktree and Branch columns, the two
//! target columns with their PR badges, the legend, and the PR age line.
//!
//! Rendering is pure over the library's listing facts, so every variant can be
//! tested without git. The design is item 5 ("Table Design") of the worktree
//! fix `2026-09-24-ux-improvements`.

use std::collections::HashMap;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{BasicColor, Color, RgbColor};
use worktree::default_target::DefaultTarget;
use worktree::listing::{
    BranchComparisons, Caption, CaptionState, Comparison, MergeState, ParentComparison, TreeNode,
    TreeRow,
};
use worktree::pull_requests::{OpenPullRequest, PrListing, PrPlacement, placement};
use worktree::worktree::{DirtyStatus, WorktreeList, WorktreeStatus};

/// Everything the table shows.
pub struct TableFacts<'a> {
    pub default_branch: &'a str,
    pub target: Option<&'a DefaultTarget>,
    pub caption: Option<&'a Caption>,
    pub tree: &'a [TreeRow],
    /// Indexed by [`TreeRow::worktree`].
    pub statuses: &'a [WorktreeStatus],
    pub comparisons: &'a HashMap<String, BranchComparisons>,
    pub prs: &'a PrListing,
}

impl<'a> TableFacts<'a> {
    pub fn from_list(list: &'a WorktreeList, prs: &'a PrListing) -> Self {
        Self {
            default_branch: &list.default_branch,
            target: list.target.as_ref(),
            caption: list.caption.as_ref(),
            tree: &list.tree,
            statuses: &list.statuses,
            comparisons: &list.comparisons,
            prs,
        }
    }
}

/// The caption, table, legend, and PR age line, each separated as printed.
///
/// `now` is Unix seconds, for the PR age.
pub fn render(facts: &TableFacts<'_>, terminal: &Terminal, now: u64) -> String {
    let prose = |markup: String| Prose::new(markup).render(terminal);
    let mut out = String::from("\n");
    if let Some(caption) = facts.caption {
        out.push_str(&format!(" {}\n\n", prose(caption_markup(caption)).trim_end()));
    }
    out.push_str(table(facts, terminal).render(terminal).trim_end());
    out.push_str("\n\n");
    for line in legend_markup() {
        out.push_str(&format!(" {}\n", prose(line).trim_end()));
    }
    if let Some(age) = pr_age_markup(facts.prs, now) {
        out.push_str(&format!(" {}\n", prose(age).trim_end()));
    }
    out
}

/// The caption: the local default branch against its origin peer.
pub fn caption_markup(caption: &Caption) -> String {
    let local = local_badge(&caption.local);
    let remote = remote_badge(&caption.remote);
    let count = |n: usize| {
        let noun = if n == 1 { "commit" } else { "commits" };
        format!("<yellow>{n} {noun}</yellow>")
    };
    match caption.state() {
        CaptionState::InSync => format!("{local} is in sync with {remote}"),
        CaptionState::Behind(n) => format!("{local} is {} behind {remote}", count(n)),
        CaptionState::Ahead(n) => format!("{local} is {} ahead of {remote}", count(n)),
        CaptionState::Diverged { ahead, behind } => format!(
            "{local} has diverged from {remote}: {} ahead, {} behind",
            count(ahead),
            count(behind)
        ),
    }
}

/// The two legend lines, one each for the Worktree and Branch column glyphs.
pub fn legend_markup() -> [String; 2] {
    [
        format!(
            "Worktree   {} <dim>clean</dim>    {} <dim>uncommitted files</dim>    {} <dim>uncommitted source files</dim>",
            dirty_dot(DirtyStatus::Clean),
            dirty_dot(DirtyStatus::DirtyNonSource),
            dirty_dot(DirtyStatus::DirtySource),
        ),
        format!(
            "Branch     {} <dim>merges cleanly into parent</dim>    {} <dim>conflicts with parent</dim>    {} <dim>parent deleted</dim>",
            connector_markup("├─", Some(MergeState::Clean), false),
            connector_markup("├─", Some(MergeState::Conflicts), false),
            connector_markup("└┄", None, true),
        ),
    ]
}

/// The PR age line, shown only when the request failed and stored results
/// stand in.
pub fn pr_age_markup(prs: &PrListing, now: u64) -> Option<String> {
    if !prs.stale {
        return None;
    }
    let minutes = prs.age_minutes(now)?;
    let age = match minutes {
        0 => "less than a minute".to_string(),
        1..=59 => format!("{minutes} min"),
        60..=2879 => format!("{} h", minutes / 60),
        _ => format!("{} days", minutes / 1440),
    };
    Some(format!("<dim>PRs as of {age} ago</dim>"))
}

/// The table, one row per tree row, with the current worktree's row
/// highlighted.
pub fn table(facts: &TableFacts<'_>, terminal: &Terminal) -> Table {
    let prose_cell = |markup: String| -> TableCellContent { Prose::new(markup).render(terminal).into() };
    let target_header = match facts.target {
        Some(target) if target.reference.starts_with("origin/") => remote_badge(&target.reference),
        Some(target) => local_badge(&target.reference),
        None => local_badge(facts.default_branch),
    };
    let columns = vec![
        TableColumn::new("Worktree"),
        TableColumn::new("Branch"),
        TableColumn::new(Prose::new(format!("-> {target_header}")).render(terminal)),
        TableColumn::new("-> parent"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

    let mut current_row = None;
    for (index, row) in facts.tree.iter().enumerate() {
        let status = row.worktree.and_then(|worktree| facts.statuses.get(worktree));
        if status.is_some_and(|status| status.entry.is_current) {
            current_row = Some(index);
        }
        let cells = RowCells::new(facts, row, status, terminal.osc_link_support);
        table.add_row(vec![
            prose_cell(cells.worktree()),
            prose_cell(cells.branch()),
            prose_cell(cells.target()),
            prose_cell(cells.parent()),
        ]);
    }

    match current_row {
        Some(row) => table.highlight_row(row, row_emphasis(terminal)),
        None => table,
    }
}

/// A very subtle background for the current worktree's row.
fn row_emphasis(terminal: &Terminal) -> Color {
    match terminal.color_mode {
        ColorMode::Light => Color::Rgb(RgbColor::new(234, 238, 246, BasicColor::White)),
        _ => Color::Rgb(RgbColor::new(38, 42, 54, BasicColor::Black)),
    }
}

struct RowCells<'f, 'a> {
    facts: &'f TableFacts<'a>,
    row: &'f TreeRow,
    status: Option<&'f WorktreeStatus>,
    comparisons: Option<&'f BranchComparisons>,
    prs: Vec<&'f OpenPullRequest>,
    links: bool,
}

impl<'f, 'a> RowCells<'f, 'a> {
    fn new(
        facts: &'f TableFacts<'a>,
        row: &'f TreeRow,
        status: Option<&'f WorktreeStatus>,
        links: bool,
    ) -> Self {
        let branch = row.branch();
        let prs = match (branch, status) {
            // Badges follow a worktree's branch; a parent row without a
            // worktree has empty target cells.
            (Some(branch), Some(_)) => facts.prs.for_branch(branch).collect(),
            _ => Vec::new(),
        };
        Self {
            facts,
            row,
            status,
            comparisons: branch.and_then(|branch| facts.comparisons.get(branch)),
            prs,
            links,
        }
    }

    fn is_default(&self) -> bool {
        matches!(self.row.node, TreeNode::Branch { is_default: true, .. })
    }

    fn badges(&self, wanted: PrPlacement) -> String {
        self.prs
            .iter()
            .filter(|pr| placement(&pr.target_branch, self.row.parent.as_deref(), self.facts.default_branch) == wanted)
            .map(|pr| {
                let label = match wanted {
                    PrPlacement::BesideBranch => format!("PR #{} → {}", pr.number, pr.target_branch),
                    _ => format!("PR #{}", pr.number),
                };
                pr_badge(&label, pr.url.as_deref(), self.links)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn worktree(&self) -> String {
        let Some(status) = self.status else {
            return String::new();
        };
        let entry = &status.entry;
        let name = if entry.is_main {
            if entry.is_current {
                "<b><i>base repo</i></b>".to_string()
            } else {
                "<dim><i>base repo</i></dim>".to_string()
            }
        } else {
            let basename = entry
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let basename = Prose::escape_text(&basename);
            if entry.is_current {
                format!("<b>{basename}</b>")
            } else {
                basename
            }
        };
        format!("{} {name}", dirty_dot(status.dirty))
    }

    fn branch(&self) -> String {
        let mut out = String::new();
        for guide in &self.row.guides {
            if *guide {
                out.push_str("<dim>│</dim>  ");
            } else {
                out.push_str("   ");
            }
        }
        if self.row.depth > 0 {
            let glyph = match (self.row.last, self.row.parent_deleted) {
                (true, false) => "└─",
                (false, false) => "├─",
                (true, true) => "└┄",
                (false, true) => "├┄",
            };
            out.push_str(&connector_markup(glyph, self.connector_state(), self.row.parent_deleted));
            out.push(' ');
        }
        match &self.row.node {
            TreeNode::Branch { name, is_default: true } => out.push_str(&local_badge(name)),
            TreeNode::Branch { name, .. } => out.push_str(&Prose::escape_text(name)),
            TreeNode::DeletedBranch(name) => out.push_str(&format!(
                "<dim><i><strikethrough>{}</strikethrough></i> (deleted)</dim>",
                Prose::escape_text(name)
            )),
            TreeNode::Detached(sha) => out.push_str(&format!("<dim><i>detached @ {sha}</i></dim>")),
        }
        let beside = self.badges(PrPlacement::BesideBranch);
        if !beside.is_empty() {
            out.push(' ');
            out.push_str(&beside);
        }
        out
    }

    /// Whether this row merges cleanly into its tree parent: the parent
    /// comparison, or the target comparison under the default branch.
    fn connector_state(&self) -> Option<MergeState> {
        let comparisons = self.comparisons?;
        match comparisons.parent {
            ParentComparison::Compared(comparison) => comparison.map(|c| c.merge_state()),
            ParentComparison::NotApplicable => comparisons.target.map(|c| c.merge_state()),
            ParentComparison::Deleted => None,
        }
    }

    fn target(&self) -> String {
        if matches!(self.row.node, TreeNode::Detached(_)) || (self.is_default() && self.status.is_some()) {
            return "<dim>—</dim>".to_string();
        }
        if self.status.is_none() {
            return String::new();
        }
        let cell = match (self.facts.target, self.comparisons.and_then(|c| c.target)) {
            (None, _) => "<dim>—</dim>".to_string(),
            (Some(_), Some(comparison)) => merge_markup(comparison),
            (Some(_), None) => "<dim>?</dim>".to_string(),
        };
        with_badges(cell, self.badges(PrPlacement::Default))
    }

    fn parent(&self) -> String {
        if matches!(self.row.node, TreeNode::Detached(_)) || (self.is_default() && self.status.is_some()) {
            return "<dim>—</dim>".to_string();
        }
        if self.status.is_none() {
            return String::new();
        }
        let cell = match self.comparisons.map(|c| c.parent) {
            Some(ParentComparison::Compared(Some(comparison))) => merge_markup(comparison),
            Some(ParentComparison::Compared(None)) => "<dim>?</dim>".to_string(),
            Some(ParentComparison::Deleted) => "<gray-400>parent deleted</gray-400>".to_string(),
            Some(ParentComparison::NotApplicable) | None => "<dim>—</dim>".to_string(),
        };
        with_badges(cell, self.badges(PrPlacement::Parent))
    }
}

fn with_badges(cell: String, badges: String) -> String {
    if badges.is_empty() { cell } else { format!("{cell} {badges}") }
}

fn merge_markup(comparison: Comparison) -> String {
    match comparison.merge_state() {
        MergeState::AlreadyIn => "<dim><i>already in</i></dim>".to_string(),
        MergeState::Clean => "<dim><i>clean</i></dim>".to_string(),
        MergeState::Conflicts => "<red>conflicts</red>".to_string(),
    }
}

/// Colors a tree connector by the row's merge state; a deleted parent
/// overrides it.
fn connector_markup(glyph: &str, state: Option<MergeState>, parent_deleted: bool) -> String {
    if parent_deleted {
        return format!("<dim>{glyph}</dim>");
    }
    match state {
        Some(MergeState::Conflicts) => format!("<red>{glyph}</red>"),
        _ => format!("<gray-500>{glyph}</gray-500>"),
    }
}

/// Single-column text glyphs.
fn dirty_dot(dirty: DirtyStatus) -> &'static str {
    match dirty {
        DirtyStatus::Clean => "<dim>○</dim>",
        DirtyStatus::DirtyNonSource => "<yellow>●</yellow>",
        DirtyStatus::DirtySource => "<orange>●</orange>",
    }
}

/// A local branch badge.
fn local_badge(name: &str) -> String {
    format!("<bg-blue-800><white> {} </white></bg-blue-800>", Prose::escape_text(name))
}

/// A remote-tracking branch badge.
fn remote_badge(name: &str) -> String {
    format!("<bg-violet-800><white> {} </white></bg-violet-800>", Prose::escape_text(name))
}

/// A PR badge, linked when the provider sent a URL and the terminal shows
/// OSC 8 links.
///
/// Without OSC 8, Prose would print the URL beside the badge, and `Table`
/// never breaks inside a word, so one URL could leave the table no width to
/// render in at all.
fn pr_badge(label: &str, url: Option<&str>, links: bool) -> String {
    let badge = format!("<bg-emerald-800><white> {} </white></bg-emerald-800>", Prose::escape_text(label));
    match url {
        Some(url) if links => format!("<a href=\"{}\">{badge}</a>", url.replace('"', "%22")),
        _ => badge,
    }
}
