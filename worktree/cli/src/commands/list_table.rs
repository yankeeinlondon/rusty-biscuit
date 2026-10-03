//! The `wt list` output around the git work: the caption, the credentials
//! line, the table, the legend, the PR age line, the refresh hint, and the
//! closing notes, plus [`assemble`], which puts them and the graph and
//! verbose sections in their order.
//!
//! Rendering is pure over the library's listing facts and an explicit `now`,
//! so every variant can be tested without git. The table design is item 5
//! ("Table Design") of the worktree fix `2026-09-24-ux-improvements`; the
//! caption, the credentials line, the hint, and the notes are §4–§9 of the fix
//! `2026-09-27-list-freshness-ux`.
//!
//! The caption compares the local default branch with the local tracking ref
//! `origin/<default>` as gathered **after** this run's update attempt, and
//! says in a dim italic suffix what that attempt established
//! ([`RemoteStatus`]). A row that could not bring the tracking ref up to date
//! calls it "local origin/<default>".

use std::collections::HashMap;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{BasicColor, Color, RgbColor};
use biscuit_terminal::utils::wrap_policy::WordWrap;
use worktree::default_target::DefaultTarget;
use worktree::listing::{
    BranchComparisons, Caption, CaptionState, Comparison, MergeState, ParentComparison, TreeNode,
    TreeRow,
};
use worktree::pull_requests::{OpenPullRequest, PrListing, PrPlacement, placement};
use worktree::remote_head::{CheckFailure, FetchFailure, REMOTE_HEAD_REFRESH_DEADLINE};
use worktree::remote_update::FETCH_DEADLINE;
use worktree::worktree::{DirtyStatus, WorktreeList, WorktreeStatus};

/// The narrowest terminal that shows ahead/behind counts in the two target
/// columns; `--width` sizes only the graph and does not move this gate.
const METRICS_MIN_WIDTH: u32 = 100;

/// The §6 refresh hint.
pub const REFRESH_HINT: &str = "running this command again will provide updated metrics; alternatively use the --refresh / -r flags to force refresh immediately";

/// Everything the pure renderers show.
pub struct TableFacts<'a> {
    pub default_branch: &'a str,
    pub target: Option<&'a DefaultTarget>,
    pub caption: Option<&'a Caption>,
    pub tree: &'a [TreeRow],
    /// Indexed by [`TreeRow::worktree`].
    pub statuses: &'a [WorktreeStatus],
    pub comparisons: &'a HashMap<String, BranchComparisons>,
    pub prs: &'a PrListing,
    /// `None` without an `origin`, which also suppresses the caption.
    pub remote: Option<RemoteFacts<'a>>,
    /// The §5 line, only for a condition this run observed.
    pub credential_line: Option<CredentialLine>,
    /// The listing rendered while the worker was still working (§6).
    pub unfinished: bool,
    /// §9: set only after a completed check or fetch, with the default
    /// branch strictly behind.
    pub ff_suggestion: Option<FfSuggestion>,
    /// Why `--ff` did not move the default branch.
    pub ff_notice: Option<FfNotice>,
    /// §8: the variables that would let `wt` use the provider API.
    pub fallback_notice: Option<Vec<String>>,
}

/// What the caption can say about `origin`'s default branch.
#[derive(Debug, Clone, Copy)]
pub struct RemoteFacts<'a> {
    pub default_branch: &'a str,
    /// The local tracking ref `origin/<default>`'s tip in this listing's ref
    /// snapshot, if the ref exists.
    pub tracking_tip: Option<&'a str>,
    pub status: RemoteStatus,
}

/// What this run's update attempt established: one variant per §4 row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteStatus {
    /// No variance: the tracking ref matched `origin`.
    CheckedNow,
    /// Variance, and the fetch brought the tracking ref up to date.
    Fetched,
    /// Variance, and the fetch failed; the tracking ref is as it was.
    FetchFailed { reason: FetchFailure },
    /// The wait ran out before `origin` answered.
    StillChecking { last: LastKnown },
    /// The wait ran out while the fetch was running.
    StillPulling,
    /// No answer this run: the check failed, or the worker never started.
    CheckFailed { reason: CheckFailure, last: LastKnown },
    /// `origin` answered without the default branch.
    Absent,
}

/// The last time anything is known about `origin`'s default branch, for a
/// row without a current answer. Timestamps are Unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastKnown {
    /// The stored answer's check time.
    Answer { checked_at: u64 },
    /// No stored answer; the tracking ref's reflog dates its last change,
    /// which is not a check.
    TrackingRefChanged { at: u64 },
    Never,
}

/// A §5 condition, with the provider's display name and the variable
/// wording (the one used, or the accepted ones joined by "or").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialLine {
    pub provider: String,
    pub key: String,
    pub condition: CredentialCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialCondition {
    /// No key, the provider did not show the repository, and Git failed too.
    NotVisible,
    Rejected,
    Insufficient,
    RateLimited { authenticated: bool },
}

/// §9: the local default branch is strictly behind its tracking ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfSuggestion {
    pub behind: usize,
}

/// Why `--ff` left the default branch where it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfNotice {
    DirtyCheckout,
    Diverged,
    /// The ref that does not exist, as shown (`main`, `origin/main`).
    Missing(String),
    /// The branch or its checkout changed while `wt` worked.
    Changed,
    Failed,
}

impl<'a> TableFacts<'a> {
    /// Without `remote` (no `origin`) the caption is dropped too, so leftover
    /// `origin/*` refs never read as a comparison with a remote. The notes
    /// start empty.
    pub fn from_list(list: &'a WorktreeList, prs: &'a PrListing, remote: Option<RemoteFacts<'a>>) -> Self {
        Self {
            default_branch: &list.default_branch,
            target: list.target.as_ref(),
            caption: remote.and(list.caption.as_ref()),
            tree: &list.tree,
            statuses: &list.statuses,
            comparisons: &list.comparisons,
            prs,
            remote,
            credential_line: None,
            unfinished: false,
            ff_suggestion: None,
            ff_notice: None,
            fallback_notice: None,
        }
    }
}

/// The caption, the §5 line, the table, the legend, and the PR age line,
/// each separated as printed.
///
/// `now` is Unix seconds, for the PR and remote-observation ages.
pub fn render(facts: &TableFacts<'_>, terminal: &Terminal, now: u64) -> String {
    let prose = |markup: String| Prose::new(markup).render(terminal);
    let wrapped = |markup: String| {
        Prose::new(markup)
            .with_word_wrap(WordWrap::WrapProse(None, Some(1)))
            .render(terminal)
            .trim_end()
            .to_string()
    };
    let mut out = String::from("\n");
    let caption = facts.remote.as_ref().map(|remote| caption_markup(facts.caption, remote, now));
    let credentials = facts.credential_line.as_ref().map(credential_markup);
    if caption.is_some() || credentials.is_some() {
        for line in caption.into_iter().chain(credentials) {
            out.push_str(&format!(" {}\n", wrapped(line)));
        }
        out.push('\n');
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

/// The §6 hint, when the listing rendered with work unfinished.
pub fn render_hint(facts: &TableFacts<'_>, terminal: &Terminal) -> Option<String> {
    facts.unfinished.then(|| notes_list([format!("<dim>{REFRESH_HINT}</dim>")], terminal))
}

/// The closing notes: the `--ff` result or the §9 suggestion, then the §8
/// notice; `None` when there is nothing to say.
pub fn render_notes(facts: &TableFacts<'_>, terminal: &Terminal) -> Option<String> {
    let local = Prose::escape_text(facts.default_branch);
    let tracking = Prose::escape_text(&format!("origin/{}", facts.default_branch));
    let mut lines = Vec::new();
    if let Some(notice) = &facts.ff_notice {
        lines.push(match notice {
            FfNotice::DirtyCheckout => format!(
                "{local} wasn't fast-forwarded: the checkout has uncommitted changes to files the update touches."
            ),
            FfNotice::Diverged => format!("{local} has diverged from {tracking}, so it can't be fast-forwarded."),
            FfNotice::Missing(reference) => {
                format!("{local} wasn't fast-forwarded: {} doesn't exist.", Prose::escape_text(reference))
            }
            FfNotice::Changed => {
                format!("{local} wasn't fast-forwarded: it changed while wt was updating it.")
            }
            FfNotice::Failed => format!("{local} wasn't fast-forwarded: Git couldn't complete the update."),
        });
    }
    if let Some(suggestion) = &facts.ff_suggestion {
        lines.push(format!(
            "{local} is {} behind {tracking}; run {} to fast-forward it.",
            commits(suggestion.behind),
            command_badge("wt --ff")
        ));
    }
    if let Some(keys) = &facts.fallback_notice {
        lines.push("Git checked origin using `ls-remote`; this can take longer than the provider API.".to_string());
        lines.push(format!(
            "Set {} to let wt try the provider API, or use {} to use Git directly for this repository.",
            Prose::escape_text(&keys.join(" or ")),
            command_badge("--ignore-api")
        ));
    }
    (!lines.is_empty()).then(|| notes_list(lines, terminal))
}

/// The sections of one listing, in the order [`assemble`] prints them.
#[derive(Debug, Default, Clone, Copy)]
pub struct Sections<'s> {
    /// [`render`]'s output.
    pub table: &'s str,
    pub graph: Option<&'s str>,
    pub hint: Option<&'s str>,
    pub verbose: Option<&'s str>,
    pub notes: Option<&'s str>,
}

/// The whole listing: the table, the graph, the hint (so it follows the
/// graph, or the PR age line without one), the verbose section, then a blank
/// line and the notes.
pub fn assemble(sections: Sections<'_>) -> String {
    let mut out = sections.table.to_string();
    for part in [sections.graph, sections.hint, sections.verbose].into_iter().flatten() {
        out.push_str(part);
    }
    if let Some(notes) = sections.notes {
        out.push('\n');
        out.push_str(notes);
    }
    out
}

fn notes_list(lines: impl IntoIterator<Item = String>, terminal: &Terminal) -> String {
    let mut list = UnorderedList::empty();
    for line in lines {
        list.add(Prose::new(line));
    }
    let rendered = list.render(terminal);
    let mut out = String::new();
    for line in rendered.trim_end().lines() {
        out.push_str(&format!(" {line}\n"));
    }
    out
}

fn commits(n: usize) -> String {
    let noun = if n == 1 { "commit" } else { "commits" };
    format!("{n} {noun}")
}

/// The one-sentence caption: the comparison with the tracking ref, then what
/// this run established in dim italics, omitted when a check this run made
/// found the tracking ref current. Without a comparison (no local
/// default branch, no tracking ref, or git could not compare) the sentence
/// names what it can.
pub fn caption_markup(caption: Option<&Caption>, remote: &RemoteFacts<'_>, now: u64) -> String {
    let tracking = remote_badge(&format!("origin/{}", remote.default_branch));
    let suffix = match status_text(remote, now) {
        Some(text) => format!(" <dim><i>({})</i></dim>", Prose::escape_text(&text)),
        None => String::new(),
    };
    let Some(caption) = caption else {
        return match (remote.status, remote.tracking_tip) {
            // Pruned: only the remote-absence observation is left to show.
            (RemoteStatus::Absent, None) => format!(
                "{} <dim><i>was absent on origin when checked just now</i></dim>",
                local_badge(remote.default_branch)
            ),
            (_, None) => format!("No local tracking ref {tracking}{suffix}"),
            (_, Some(_)) => format!("{tracking}{suffix}"),
        };
    };
    let local = local_badge(&caption.local);
    let target = match remote.status {
        RemoteStatus::FetchFailed { .. } | RemoteStatus::StillPulling => format!("local {tracking}"),
        _ => tracking,
    };
    let count = |n: usize| format!("<yellow>{}</yellow>", commits(n));
    let comparison = match caption.state() {
        CaptionState::InSync => format!("{local} is in sync with {target}"),
        CaptionState::Behind(n) => format!("{local} is {} behind {target}", count(n)),
        CaptionState::Ahead(n) => format!("{local} is {} ahead of {target}", count(n)),
        CaptionState::Diverged { ahead, behind } => format!(
            "{local} has diverged from {target} ({} ahead, {} behind)",
            count(ahead),
            count(behind)
        ),
    };
    format!("{comparison}{suffix}")
}

/// The suffix's text, without parentheses or markup. `None` for a check this
/// run just made that found nothing to report: a fresh answer needs no date.
fn status_text(remote: &RemoteFacts<'_>, now: u64) -> Option<String> {
    let differed = "origin differed when checked just now";
    let text = match remote.status {
        RemoteStatus::CheckedNow => return None,
        RemoteStatus::Fetched => "updated from origin just now".to_string(),
        RemoteStatus::FetchFailed { reason } => format!("{differed}; {}", fetch_reason(reason)),
        RemoteStatus::StillChecking { last } => format!(
            "origin hasn't answered yet; still checking in the background; {}",
            last_known_text(last, now)
        ),
        RemoteStatus::StillPulling => format!("{differed}; pulling remote updates in the background"),
        RemoteStatus::CheckFailed { reason, last } => {
            format!("{}; {}", check_reason(reason), last_known_text(last, now))
        }
        // A verified absence leaves the tracking ref in place; the comparison
        // is still against it and must say so.
        RemoteStatus::Absent => format!(
            "{branch} was absent on origin when checked just now; origin/{branch} is a local tracking ref",
            branch = remote.default_branch
        ),
    };
    Some(text)
}

fn check_reason(reason: CheckFailure) -> String {
    match reason {
        CheckFailure::Timeout => {
            format!("origin didn't answer within {} s", REMOTE_HEAD_REFRESH_DEADLINE.as_secs())
        }
        CheckFailure::Credentials => "origin didn't accept Git's credentials".to_string(),
        CheckFailure::Other => "couldn't check origin".to_string(),
    }
}

fn fetch_reason(reason: FetchFailure) -> String {
    match reason {
        FetchFailure::Timeout => format!("fetch didn't finish within {} s", FETCH_DEADLINE.as_secs()),
        FetchFailure::Other => "fetch failed".to_string(),
    }
}

/// A timestamp after `now` is no evidence, as for stored answers.
fn last_known_text(last: LastKnown, now: u64) -> String {
    match last {
        LastKnown::Answer { checked_at } if checked_at <= now => {
            format!("last checked with origin {} ago", age_text(now - checked_at))
        }
        LastKnown::TrackingRefChanged { at } if at <= now => {
            format!("tracking ref last changed {} ago", age_text(now - at))
        }
        _ => "never checked with origin".to_string(),
    }
}

/// The dim §5 line.
pub fn credential_markup(line: &CredentialLine) -> String {
    let provider = &line.provider;
    let key = &line.key;
    let text = match line.condition {
        CredentialCondition::NotVisible => format!(
            "{provider} did not show this repository, and Git could not check it. If it is private, set {key} and try again."
        ),
        CredentialCondition::Rejected => {
            format!("{provider} didn't accept {key}; it may be invalid, expired, or revoked. Replace it and try again.")
        }
        CredentialCondition::Insufficient => {
            format!("The API key {key} doesn't have rights to view this repository on {provider}.")
        }
        CredentialCondition::RateLimited { authenticated: false } => format!(
            "{provider} rate limited the request for updated information. Add the {key} API key to get larger rate limits."
        ),
        CredentialCondition::RateLimited { authenticated: true } => {
            format!("{provider} rate limited the request for updated information. Try again in a few minutes.")
        }
    };
    format!("<dim>{}</dim>", Prose::escape_text(&text))
}

/// An age in the PR age line's units: `less than 1 min` below a minute, then
/// minutes below an hour, hours below two days, then days.
pub fn age_text(seconds: u64) -> String {
    let minutes = seconds / 60;
    match minutes {
        0 => "less than 1 min".to_string(),
        1..=59 => format!("{minutes} min"),
        60..=2879 => format!("{} h", minutes / 60),
        _ => format!("{} days", minutes / 1440),
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
            connector_markup("└─", Some(MergeState::Clean), false),
            connector_markup("└─", Some(MergeState::Conflicts), false),
            connector_markup("└┄", None, true),
        ),
    ]
}

/// The PR age line, shown whenever the badges are older than the freshness
/// window at `now`.
pub fn pr_age_markup(prs: &PrListing, now: u64) -> Option<String> {
    if !prs.is_stale_at(now) {
        return None;
    }
    let minutes = prs.age_minutes(now)?;
    Some(format!("<dim>PRs as of {} ago</dim>", age_text(minutes * 60)))
}

/// The table, one row per tree row, with the current worktree's row
/// highlighted. The target columns carry ahead/behind counts only when
/// `terminal` is at least [`METRICS_MIN_WIDTH`] columns wide.
pub fn table(facts: &TableFacts<'_>, terminal: &Terminal) -> Table {
    let prose_cell = |markup: String| -> TableCellContent { InlineProse::new(markup).render(terminal).into() };
    let target_header = match facts.target {
        Some(target) if target.reference.starts_with("origin/") => remote_badge(&target.reference),
        Some(target) => local_badge(&target.reference),
        None => local_badge(facts.default_branch),
    };
    let columns = vec![
        TableColumn::new("Worktree"),
        TableColumn::new("Branch"),
        TableColumn::new(InlineProse::new(format!("-> {target_header}")).render(terminal)),
        TableColumn::new("-> parent"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

    let show_metrics = terminal.width() >= METRICS_MIN_WIDTH;
    let mut current_row = None;
    for (index, row) in facts.tree.iter().enumerate() {
        let status = row.worktree.and_then(|worktree| facts.statuses.get(worktree));
        if status.is_some_and(|status| status.entry.is_current) {
            current_row = Some(index);
        }
        let cells = RowCells::new(facts, row, status, terminal.osc_link_support, show_metrics);
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
    show_metrics: bool,
}

impl<'f, 'a> RowCells<'f, 'a> {
    fn new(
        facts: &'f TableFacts<'a>,
        row: &'f TreeRow,
        status: Option<&'f WorktreeStatus>,
        links: bool,
        show_metrics: bool,
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
            show_metrics,
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
            (Some(_), Some(comparison)) => merge_markup(comparison, self.show_metrics),
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
            Some(ParentComparison::Compared(Some(comparison))) => merge_markup(comparison, self.show_metrics),
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

/// The merge state word, then `+ahead` and `-behind` when `show_metrics`;
/// a zero side is omitted.
fn merge_markup(comparison: Comparison, show_metrics: bool) -> String {
    let mut out = match comparison.merge_state() {
        MergeState::Clean => "<dim><i>clean</i></dim>".to_string(),
        MergeState::Conflicts => "<red>conflicts</red>".to_string(),
    };
    if show_metrics {
        if comparison.ahead > 0 {
            out.push_str(&format!(" <dim><green>+{}</green></dim>", comparison.ahead));
        }
        if comparison.behind > 0 {
            // ASCII hyphen-minus, so plain output stays greppable.
            out.push_str(&format!(" <dim><red>-{}</red></dim>", comparison.behind));
        }
    }
    out
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
        DirtyStatus::DirtySource => "<red>●</red>",
    }
}

/// A local branch badge.
fn local_badge(name: &str) -> String {
    format!("<bg-blue-800><white> {} </white></bg-blue-800>", Prose::escape_text(name))
}

/// A command the user can type, in reverse video.
fn command_badge(command: &str) -> String {
    format!("<inverse> {} </inverse>", Prose::escape_text(command))
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
