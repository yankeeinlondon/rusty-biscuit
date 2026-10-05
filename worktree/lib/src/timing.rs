//! Where the time of one `wt list` run went: a tree of measured [`Span`]s,
//! each naming a typed [`Stage`], plus diagnostic reports measured inside the
//! refresh worker.
//!
//! A span's children are either **sequential** (consecutive parts of it, which
//! reconcile to it) or **concurrent** (overlapping tasks inside it, which are
//! never summed). Every sequential parent, and the root, reconciles exactly in
//! integer microseconds:
//!
//! ```text
//! sum(children) + unattributed − over_attributed = elapsed
//! ```
//!
//! where at most one of the two remainders is nonzero. A span with concurrent
//! children, or with none, carries zero remainders.
//!
//! Every duration and remainder is an unsigned 64-bit count of microseconds,
//! and the equation holds exactly, never against a clamped value:
//! [`Timings::new`] and [`WorkerTimings::new`] reject a tree that does not
//! fit ([`TimingsError::Unrepresentable`]), so every value of those types is
//! a document [`Timings::to_json`] writes exactly.
//!
//! [`Timings::to_json`] writes the version-1 document; [`Timings::from_json`]
//! is its only reader and rejects anything that does not reconcile. The
//! document is the contract of `wt list --perf=json`; see
//! `worktree/docs/performance-testing.md`.

use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::remote_head::is_attempt_id;
use crate::strict_json;

/// The only document version this module writes or reads.
pub const FORMAT_VERSION: u64 = 1;

macro_rules! stages {
    ($($(#[$doc:meta])* $variant:ident => $id:literal, $label:literal;)+) => {
        /// One measured step of `wt list` or of its refresh worker.
        ///
        /// The [`id`](Stage::id) is the stable contract read by JSON consumers
        /// and tests; the [`label`](Stage::label) is display text and may
        /// change freely.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Stage {
            $($(#[$doc])* $variant,)+
        }

        impl Stage {
            /// Every stage, in report order.
            pub const ALL: &'static [Stage] = &[$(Stage::$variant,)+];

            /// The stable snake_case id.
            pub const fn id(self) -> &'static str {
                match self {
                    $(Stage::$variant => $id,)+
                }
            }

            /// The display label.
            pub const fn label(self) -> &'static str {
                match self {
                    $(Stage::$variant => $label,)+
                }
            }

            /// The stage whose [`id`](Stage::id) is `id`.
            pub fn from_id(id: &str) -> Option<Stage> {
                match id {
                    $($id => Some(Stage::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

stages! {
    /// Process start to the listing: argument parsing and terminal detection.
    Startup => "startup", "startup";
    /// `git worktree list`, the default branch, refs, and fork-origin records.
    ReadWorktrees => "read_worktrees", "read worktrees and refs";
    /// The origin URL, the `--ignore-api` record, and the API preference.
    OriginLookup => "origin_lookup", "origin and API preferences";
    /// Comparison-cache load and graph input assembly.
    PrepareLocal => "prepare_local", "prepare local inputs";
    /// The refresh worker and the local reads it overlaps; used only when a
    /// wait ran.
    RemoteAndLocal => "remote_and_local", "refresh worker ‖ local reads";
    /// The local reads when no wait ran.
    LocalReads => "local_reads", "local reads";
    /// Launching the refresh worker and waiting for its results.
    RefreshWorker => "refresh_worker", "launch and wait for refresh";
    /// Spawning the worker, by the foreground's monotonic clock.
    WorkerLaunch => "worker_launch", "launch worker";
    /// Following the worker's published results until both halves end or the
    /// budget runs out.
    WorkerWait => "worker_wait", "follow results";
    /// The origin recheck and the PR store read.
    PrCacheRead => "pr_cache_read", "PR cache read";
    /// Worktree status and branch comparisons.
    LocalGather => "local_gather", "local listing facts";
    /// One `git status` per worktree.
    WorktreeStatus => "worktree_status", "worktree status";
    /// Branch comparisons, including caption and fork-tree work.
    BranchComparisons => "branch_comparisons", "branch comparisons";
    /// Commit-history reads for the graph.
    GraphHistory => "graph_history", "graph history";
    /// Commit-history reads when only `-v` needs history.
    VerboseHistory => "verbose_history", "verbose history";
    /// Whether the repository is shallow.
    ShallowCheck => "shallow_check", "shallow check";
    /// The default branch's tips.
    DefaultTips => "default_tips", "default-branch tips";
    /// The focused worktree's merge base.
    FocusedMergeBase => "focused_merge_base", "focused merge base";
    /// Commit details for `-v`.
    VerboseDetails => "verbose_details", "verbose commit details";
    /// The whole lane-assembly loop, including repeated assembly after a fork
    /// holder is found.
    LaneAssembly => "lane_assembly", "assemble lanes and fork holders";
    /// `--ff` fast-forwards.
    FastForward => "fast_forward", "fast-forward";
    /// Rereading refs after the wait.
    RefReread => "ref_reread", "ref reread";
    /// Regathering ref-dependent facts after refs changed or a read failed.
    Regather => "regather", "regather";
    /// Comparison-cache save and record pruning.
    Commit => "commit", "save caches and records";
    /// Another status read for a checkout `--ff` moved.
    CheckoutRefresh => "checkout_refresh", "checkout status refresh";
    /// The caption's status, including any reflog read.
    CaptionStatus => "caption_status", "caption status";
    /// The conversion to table facts.
    DisplayFacts => "display_facts", "prepare display facts";
    /// Rendering the table.
    TableRender => "table_render", "table render";
    /// Rendering `-v` details.
    VerboseRender => "verbose_render", "verbose render";
    /// Rendering the status and preliminary notes.
    NotesRender => "notes_render", "status and preliminary notes";
    /// Sizing the graph's row budget.
    GraphBudget => "graph_budget", "graph row budget";
    /// Rendering the graph image.
    GraphRender => "graph_render", "graph image render (biscuit-terminal)";
    /// Rendering notes the graph added.
    FinalNotesRender => "final_notes_render", "final notes render";
    /// Assembling the listing and writing it to stderr.
    WriteOutput => "write_output", "assemble and write output";
    /// Worker: everything before its two halves start.
    WorkerSetup => "worker_setup", "worker setup";
    /// Worker: the PR and head halves, which run concurrently.
    WorkerHalves => "worker_halves", "PR refresh ‖ head refresh";
    /// Worker: the PR lock, provider request, and publication.
    PrRefresh => "pr_refresh", "PR refresh";
    /// Worker: the default-branch check and optional fetch, with their store
    /// work.
    HeadRefresh => "head_refresh", "default-branch refresh";
    /// Worker: the provider request for open PRs.
    PrRequest => "pr_request", "PR request";
    /// Worker: the default-branch check, by provider API or Git fallback.
    HeadCheck => "head_check", "head check";
    /// Worker: the default-branch `git fetch`.
    HeadFetch => "head_fetch", "head fetch";
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A stage id no [`Stage`] has.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown stage id {0:?}")]
pub struct UnknownStage(pub String);

impl FromStr for Stage {
    type Err = UnknownStage;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        Stage::from_id(id).ok_or_else(|| UnknownStage(id.to_string()))
    }
}

impl Serialize for Stage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Stage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        id.parse().map_err(D::Error::custom)
    }
}

/// How a span's children relate to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildrenKind {
    /// Consecutive parts of the parent; they reconcile to it.
    Sequential,
    /// Tasks that overlapped inside the parent; never summed.
    Concurrent,
}

/// Which interval a [`Timings`] covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The library listing, from entry to checkout refresh.
    Library,
    /// The whole `wt list` command, from process start to the listing write.
    Command,
}

/// One measured step and the steps measured inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    stage: Stage,
    elapsed: Duration,
    git_calls: Option<u64>,
    children_kind: ChildrenKind,
    children: Vec<Span>,
}

#[allow(missing_docs)]
impl Span {
    /// A span with no children and no `git` count.
    pub fn new(stage: Stage, elapsed: Duration) -> Self {
        Self { stage, elapsed, git_calls: None, children_kind: ChildrenKind::Sequential, children: Vec::new() }
    }

    /// Records `git_calls` `git` processes started inside this span; give a
    /// count only when every call in the span was counted.
    pub fn with_git_calls(mut self, git_calls: u64) -> Self {
        self.git_calls = Some(git_calls);
        self
    }

    pub fn with_children(mut self, children: SpanList) -> Self {
        self.children_kind = children.kind;
        self.children = children.spans;
        self
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// `None` means unknown, not zero.
    pub fn git_calls(&self) -> Option<u64> {
        self.git_calls
    }

    pub fn children_kind(&self) -> ChildrenKind {
        self.children_kind
    }

    pub fn children(&self) -> &[Span] {
        &self.children
    }

    /// The descendant at `path` below this span; an empty path is this span.
    pub fn find(&self, path: &[Stage]) -> Option<&Span> {
        match path.split_first() {
            None => Some(self),
            Some((first, rest)) => self.children.iter().find(|child| child.stage == *first)?.find(rest),
        }
    }

    /// The part of a sequential parent's elapsed time no child covers.
    ///
    /// ## Panics
    ///
    /// When the remainder exceeds `u64::MAX` microseconds, which no span of a
    /// [`Timings`] or [`WorkerTimings`] does.
    pub fn unattributed(&self) -> Duration {
        Duration::from_micros(self.representable_remainders().0)
    }

    /// How far a sequential parent's children exceed its elapsed time.
    ///
    /// ## Panics
    ///
    /// As [`Span::unattributed`].
    pub fn over_attributed(&self) -> Duration {
        Duration::from_micros(self.representable_remainders().1)
    }

    /// `None` when either exact remainder exceeds the document's range.
    fn remainders_us(&self) -> Option<(u64, u64)> {
        span_remainders(self.children_kind, micros(self.elapsed), self.children.iter().map(|child| micros(child.elapsed)))
    }

    fn representable_remainders(&self) -> (u64, u64) {
        self.remainders_us().expect("the remainder exceeds u64::MAX microseconds")
    }

    /// Folds a later, non-overlapping invocation of the same stage into this
    /// one. A count stays known only when both are.
    fn absorb(&mut self, other: Span) {
        debug_assert_eq!(self.stage, other.stage);
        self.elapsed = self.elapsed.saturating_add(other.elapsed);
        self.git_calls = self.git_calls.zip(other.git_calls).map(|(a, b)| a.saturating_add(b));
        if self.children.is_empty() {
            self.children_kind = other.children_kind;
        } else {
            debug_assert!(
                other.children.is_empty() || other.children_kind == self.children_kind,
                "{} was recorded with both child kinds",
                self.stage
            );
        }
        for child in other.children {
            push_unique(&mut self.children, child);
        }
    }
}

/// The children of one parent while they are being recorded.
///
/// Sibling stages are unique: pushing a stage already present adds the new
/// span into it. That is only correct for invocations that did not overlap,
/// such as one step run twice in a row; overlapping invocations must be
/// measured as one span around both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanList {
    kind: ChildrenKind,
    spans: Vec<Span>,
}

impl SpanList {
    pub fn sequential() -> Self {
        Self { kind: ChildrenKind::Sequential, spans: Vec::new() }
    }

    pub fn concurrent() -> Self {
        Self { kind: ChildrenKind::Concurrent, spans: Vec::new() }
    }

    pub fn kind(&self) -> ChildrenKind {
        self.kind
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub fn push(&mut self, span: Span) {
        push_unique(&mut self.spans, span);
    }

    /// Runs `work` and records its elapsed time as a childless `stage`.
    pub fn time<T>(&mut self, stage: Stage, work: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let output = work();
        self.push(Span::new(stage, started.elapsed()));
        output
    }

    /// Runs `work` and records its elapsed time as `stage`, with the children
    /// `work` pushes into the list it is given.
    pub fn time_parent<T>(&mut self, stage: Stage, kind: ChildrenKind, work: impl FnOnce(&mut SpanList) -> T) -> T {
        let started = Instant::now();
        let mut children = SpanList { kind, spans: Vec::new() };
        let output = work(&mut children);
        self.push(Span::new(stage, started.elapsed()).with_children(children));
        output
    }
}

/// Runs `work` as `stage`: its elapsed time, and the `git` processes it
/// starts on this thread and in every task that carries the count scope
/// ([`crate::git::calls::TaskHandle`]).
pub(crate) fn measure<T>(stage: Stage, work: impl FnOnce() -> T) -> (T, Span) {
    let scope = crate::git::calls::CallScope::enter();
    let started = Instant::now();
    let output = work();
    let elapsed = started.elapsed();
    (output, Span::new(stage, elapsed).with_git_calls(scope.finish()))
}

/// [`measure`] into `steps`; with no list, only runs `work`, reading no
/// clock and opening no count scope.
pub(crate) fn record<T>(steps: Option<&mut SpanList>, stage: Stage, work: impl FnOnce() -> T) -> T {
    let Some(steps) = steps else {
        return work();
    };
    let (output, span) = measure(stage, work);
    steps.push(span);
    output
}

/// [`record`] for a parent: `work` gets the list its children go into (of
/// `kind`), or `None` when `steps` is `None`. Concurrent tasks push their
/// spans after joining.
pub(crate) fn record_parent<T>(
    steps: Option<&mut SpanList>,
    stage: Stage,
    kind: ChildrenKind,
    work: impl FnOnce(Option<&mut SpanList>) -> T,
) -> T {
    let Some(steps) = steps else {
        return work(None);
    };
    let mut children = SpanList { kind, spans: Vec::new() };
    let (output, span) = measure(stage, || work(Some(&mut children)));
    steps.push(span.with_children(children));
    output
}

fn push_unique(spans: &mut Vec<Span>, span: Span) {
    match spans.iter_mut().find(|existing| existing.stage == span.stage) {
        Some(existing) => existing.absorb(span),
        None => spans.push(span),
    }
}

/// The timings of one listing or command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timings {
    scope: Scope,
    total: Duration,
    spans: Vec<Span>,
    worker_reports: Vec<WorkerReport>,
    worker_report_status: Option<WorkerReportStatus>,
}

#[allow(missing_docs)]
impl Timings {
    /// Top-level spans are sequential parts of `total`; `spans` must be a
    /// [`SpanList::sequential`] list.
    ///
    /// ## Errors
    ///
    /// [`TimingsError::Unrepresentable`] when a duration, or the exact
    /// remainder of the root or of a sequential parent, exceeds `u64::MAX`
    /// microseconds: the document could not state it exactly.
    pub fn new(scope: Scope, total: Duration, spans: SpanList) -> Result<Self, TimingsError> {
        debug_assert_eq!(spans.kind, ChildrenKind::Sequential, "top-level spans are sequential");
        check_representable(total, &spans.spans)?;
        Ok(Self { scope, total, spans: spans.spans, worker_reports: Vec::new(), worker_report_status: None })
    }

    /// Attaches the worker's reports, one per owned launch, as diagnostics
    /// outside the reconciliation.
    pub fn with_worker_reports(mut self, reports: Vec<WorkerReport>, status: WorkerReportStatus) -> Self {
        self.worker_reports = reports;
        self.worker_report_status = Some(status);
        self
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// The span at `path` of stage ids from the top level, or `None` when
    /// that step did not run (or `path` is empty).
    pub fn span(&self, path: &[Stage]) -> Option<&Span> {
        let (first, rest) = path.split_first()?;
        self.spans.iter().find(|span| span.stage == *first)?.find(rest)
    }

    pub fn unattributed(&self) -> Duration {
        Duration::from_micros(self.root_remainders_us().0)
    }

    pub fn over_attributed(&self) -> Duration {
        Duration::from_micros(self.root_remainders_us().1)
    }

    pub fn worker_reports(&self) -> &[WorkerReport] {
        &self.worker_reports
    }

    /// `None` when no refresh worker was followed.
    pub fn worker_report_status(&self) -> Option<WorkerReportStatus> {
        self.worker_report_status
    }

    fn root_remainders_us(&self) -> (u64, u64) {
        root_remainders(self.total, &self.spans)
    }

    /// The compact version-1 document.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a timings document always serializes")
    }

    /// Reads a version-1 document.
    ///
    /// ## Errors
    ///
    /// Rejects a document that is not one JSON object, repeats a key, has
    /// another `format_version`, names an unknown stage, repeats a sibling
    /// stage, or states remainders that do not reconcile exactly. Children
    /// whose exact excess over their parent exceeds `u64::MAX` microseconds
    /// never reconcile, whatever remainder the document states.
    pub fn from_json(text: &str) -> Result<Self, TimingsError> {
        let value = strict_value(text.as_bytes())?;
        check_version(&value)?;
        let wire = WireTimings::deserialize(value).map_err(malformed)?;
        let spans = decode_root(wire.total_us, wire.spans, wire.unattributed_us, wire.over_attributed_us)?;
        let worker_reports = wire.worker_reports.into_iter().map(decode_report).collect::<Result<Vec<_>, _>>()?;
        check_reports(&worker_reports, wire.worker_report_status)?;
        Ok(Self {
            scope: wire.scope,
            total: Duration::from_micros(wire.total_us),
            spans,
            worker_reports,
            worker_report_status: wire.worker_report_status,
        })
    }
}

impl Serialize for Timings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (unattributed_us, over_attributed_us) = self.root_remainders_us();
        WireTimings {
            format_version: FORMAT_VERSION,
            scope: self.scope,
            total_us: wire_micros(self.total),
            spans: self.spans.iter().map(WireSpan::from).collect(),
            unattributed_us,
            over_attributed_us,
            worker_reports: self.worker_reports.iter().map(WireReport::from).collect(),
            worker_report_status: self.worker_report_status,
        }
        .serialize(serializer)
    }
}

/// The timings one refresh worker measured for its own attempt, from its
/// entry through both halves, as diagnostics for the foreground.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTimings {
    total: Duration,
    spans: Vec<Span>,
}

#[allow(missing_docs)]
impl WorkerTimings {
    /// `spans` must be a [`SpanList::sequential`] list.
    ///
    /// ## Errors
    ///
    /// As [`Timings::new`].
    pub fn new(total: Duration, spans: SpanList) -> Result<Self, TimingsError> {
        debug_assert_eq!(spans.kind, ChildrenKind::Sequential, "top-level spans are sequential");
        check_representable(total, &spans.spans)?;
        Ok(Self { total, spans: spans.spans })
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// See [`Timings::span`].
    pub fn span(&self, path: &[Stage]) -> Option<&Span> {
        let (first, rest) = path.split_first()?;
        self.spans.iter().find(|span| span.stage == *first)?.find(rest)
    }

    pub fn unattributed(&self) -> Duration {
        Duration::from_micros(self.root_remainders_us().0)
    }

    pub fn over_attributed(&self) -> Duration {
        Duration::from_micros(self.root_remainders_us().1)
    }

    fn root_remainders_us(&self) -> (u64, u64) {
        root_remainders(self.total, &self.spans)
    }

    /// The compact version-1 worker document (the receipt's `durations`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(&WireWorkerTimings::from(self)).expect("a timings document always serializes")
    }

    /// Reads a version-1 worker document, by the same rules as
    /// [`Timings::from_json`].
    ///
    /// ## Errors
    ///
    /// As [`Timings::from_json`].
    pub fn from_json(text: &str) -> Result<Self, TimingsError> {
        decode_worker(strict_value(text.as_bytes())?)
    }

    /// [`WorkerTimings::to_json`] as a value, to embed in a receipt.
    pub(crate) fn to_value(&self) -> Value {
        serde_json::to_value(WireWorkerTimings::from(self)).expect("a timings document always serializes")
    }

    /// [`WorkerTimings::from_json`] for a value already read with no
    /// repeated key (a receipt's `durations` member).
    pub(crate) fn from_value(value: Value) -> Result<Self, TimingsError> {
        decode_worker(value)
    }

    /// `complete` when both halves were measured, `partial` when one is
    /// absent (it panicked), and `None` without the halves group, which
    /// every worker document records.
    fn status(&self) -> Option<WorkerReportStatus> {
        let halves = self.span(&[Stage::WorkerHalves])?;
        let measured = |stage| halves.find(&[stage]).is_some();
        Some(if measured(Stage::PrRefresh) && measured(Stage::HeadRefresh) {
            WorkerReportStatus::Complete
        } else {
            WorkerReportStatus::Partial
        })
    }
}

/// Whether worker timings are available, for one launch or for the listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerReportStatus {
    /// Every owned launch has a full report.
    Complete,
    /// Some timings are usable and others are not.
    Partial,
    /// No timings reached the wait: the receipt carried none, or the wait
    /// ended before the receipt was written.
    Missing,
    /// The receipt's timings did not decode or lack the halves group, or the
    /// worker's measurements could not be represented.
    Invalid,
    /// An adopted attempt was followed and no owned report is available.
    Adopted,
    /// The origin changed during the wait, so reports were suppressed.
    OriginChanged,
}

/// One owned worker launch's timings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerReport {
    /// 0-based, in launch order; a forced retry is 1.
    pub launch_index: u32,
    pub attempt_id: String,
    pub report: LaunchReport,
}

impl WorkerReport {
    /// This launch's status: never `adopted` or `origin_changed`.
    pub fn status(&self) -> WorkerReportStatus {
        match self.report {
            LaunchReport::Complete(_) => WorkerReportStatus::Complete,
            LaunchReport::Partial(_) => WorkerReportStatus::Partial,
            LaunchReport::Missing => WorkerReportStatus::Missing,
            LaunchReport::Invalid => WorkerReportStatus::Invalid,
        }
    }

    /// The usable timings, full or partial.
    pub fn timings(&self) -> Option<&WorkerTimings> {
        self.report.timings()
    }
}

/// What one launch's receipt held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LaunchReport {
    Complete(WorkerTimings),
    /// One half's measurements are absent, for example after it panicked.
    Partial(WorkerTimings),
    #[default]
    Missing,
    Invalid,
}

impl LaunchReport {
    /// The report a worker document makes: [`LaunchReport::Complete`] or
    /// [`LaunchReport::Partial`] by which halves it measured, or
    /// [`LaunchReport::Invalid`] without the halves group.
    pub fn from_worker(timings: WorkerTimings) -> Self {
        match timings.status() {
            Some(WorkerReportStatus::Complete) => LaunchReport::Complete(timings),
            Some(_) => LaunchReport::Partial(timings),
            None => LaunchReport::Invalid,
        }
    }

    /// The usable timings, full or partial.
    pub fn timings(&self) -> Option<&WorkerTimings> {
        match self {
            LaunchReport::Complete(timings) | LaunchReport::Partial(timings) => Some(timings),
            LaunchReport::Missing | LaunchReport::Invalid => None,
        }
    }
}

/// The listing's summary of `reports`, its owned launches in order.
///
/// A changed origin wins. Otherwise, any usable report gives `complete` when
/// every launch is complete and `partial` when not. With none usable, an
/// adopted attempt gives `adopted`; otherwise the latest launch's status, or
/// `missing` when there was none.
pub fn summarize_worker_reports(reports: &[WorkerReport], adopted: bool, origin_changed: bool) -> WorkerReportStatus {
    if origin_changed {
        return WorkerReportStatus::OriginChanged;
    }
    if reports.iter().any(|report| report.timings().is_some()) {
        return if reports.iter().all(|report| report.status() == WorkerReportStatus::Complete) {
            WorkerReportStatus::Complete
        } else {
            WorkerReportStatus::Partial
        };
    }
    if adopted {
        return WorkerReportStatus::Adopted;
    }
    reports.last().map_or(WorkerReportStatus::Missing, WorkerReport::status)
}

/// Why a timings document was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimingsError {
    #[error("malformed timings document: {0}")]
    Malformed(String),
    #[error("unsupported timings format_version {0}")]
    UnsupportedVersion(u64),
    #[error(transparent)]
    UnknownStage(#[from] UnknownStage),
    #[error("stage {} repeats under one parent", display_path(.0))]
    DuplicateSibling(Vec<Stage>),
    #[error("remainders of {} do not reconcile", display_path(.0))]
    InconsistentReconciliation(Vec<Stage>),
    /// A duration, or a remainder that reconciles it exactly, exceeds
    /// `u64::MAX` microseconds.
    #[error("{} exceeds the document's microsecond range", display_path(.0))]
    Unrepresentable(Vec<Stage>),
    #[error("invalid worker reports: {0}")]
    InvalidWorkerReports(String),
}

/// The path of a span, or `(root)` for the top level.
fn display_path(path: &[Stage]) -> String {
    if path.is_empty() {
        return "(root)".to_string();
    }
    path.iter().map(|stage| stage.id()).collect::<Vec<_>>().join("/")
}

/// Whole microseconds, truncated but never clamped.
fn micros(duration: Duration) -> u128 {
    duration.as_micros()
}

/// [`micros`] as the document's field.
///
/// ## Panics
///
/// Beyond `u64::MAX` microseconds, which [`check_representable`] rejects
/// before any value reaches a serializer.
fn wire_micros(duration: Duration) -> u64 {
    u64::try_from(micros(duration)).expect("a constructed duration fits the document")
}

/// The exact `(unattributed, over_attributed)` remainders, in microseconds, of
/// a sequential parent of `total` whose children took `children`; `None` when
/// the sum or either remainder does not fit, since no stated pair of
/// unsigned 64-bit fields could then satisfy the equation.
fn reconcile(total: u128, mut children: impl Iterator<Item = u128>) -> Option<(u64, u64)> {
    let attributed = children.try_fold(0u128, u128::checked_add)?;
    let (unattributed, over_attributed) =
        if attributed <= total { (total - attributed, 0) } else { (0, attributed - total) };
    Some((u64::try_from(unattributed).ok()?, u64::try_from(over_attributed).ok()?))
}

/// [`reconcile`] for a span: only a span with sequential children is a
/// parent that reconciles.
fn span_remainders(kind: ChildrenKind, elapsed: u128, children: impl ExactSizeIterator<Item = u128>) -> Option<(u64, u64)> {
    if kind == ChildrenKind::Concurrent || children.len() == 0 {
        return Some((0, 0));
    }
    reconcile(elapsed, children)
}

/// The root's remainders, for a tree [`check_representable`] accepted.
fn root_remainders(total: Duration, spans: &[Span]) -> (u64, u64) {
    reconcile(micros(total), spans.iter().map(|span| micros(span.elapsed))).expect("checked at construction")
}

/// Every duration under `total`, and every remainder of the root and of each
/// sequential parent, fits the document's unsigned 64-bit field.
fn check_representable(total: Duration, spans: &[Span]) -> Result<(), TimingsError> {
    let fits = |duration: Duration| u64::try_from(micros(duration)).is_ok();
    if !fits(total) || reconcile(micros(total), spans.iter().map(|span| micros(span.elapsed))).is_none() {
        return Err(TimingsError::Unrepresentable(Vec::new()));
    }
    fn walk(spans: &[Span], path: &mut Vec<Stage>, fits: &impl Fn(Duration) -> bool) -> Result<(), TimingsError> {
        for span in spans {
            path.push(span.stage);
            if !fits(span.elapsed) || span.remainders_us().is_none() {
                return Err(TimingsError::Unrepresentable(path.clone()));
            }
            walk(&span.children, path, fits)?;
            path.pop();
        }
        Ok(())
    }
    walk(spans, &mut Vec::new(), &fits)
}

// --- The version-1 wire format ---------------------------------------------
//
// Every field is required except the three read through `present`, whose
// absence means "unknown" or "none" and whose `null` is rejected; unknown
// fields are ignored.

#[derive(Serialize, Deserialize)]
struct WireTimings {
    format_version: u64,
    scope: Scope,
    total_us: u64,
    spans: Vec<WireSpan>,
    unattributed_us: u64,
    over_attributed_us: u64,
    worker_reports: Vec<WireReport>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    worker_report_status: Option<WorkerReportStatus>,
}

#[derive(Serialize, Deserialize)]
struct WireWorkerTimings {
    format_version: u64,
    total_us: u64,
    spans: Vec<WireSpan>,
    unattributed_us: u64,
    over_attributed_us: u64,
}

#[derive(Serialize, Deserialize)]
struct WireSpan {
    // A string, so an unknown id is reported as such rather than as a shape
    // error.
    stage: String,
    elapsed_us: u64,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    git_calls: Option<u64>,
    children_kind: ChildrenKind,
    children: Vec<WireSpan>,
    unattributed_us: u64,
    over_attributed_us: u64,
}

#[derive(Serialize, Deserialize)]
struct WireReport {
    launch_index: u32,
    attempt_id: String,
    status: WorkerReportStatus,
    // Read as a value so a nested report's version is checked before its
    // shape, as at the top level.
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    report: Option<Value>,
}

/// A field that may be absent (`None` through `#[serde(default)]`) but is
/// never `null`: serde alone reads both as `None`.
fn present<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned>(deserializer: D) -> Result<Option<T>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if value.is_null() {
        return Err(D::Error::custom("null where a value or no field is required"));
    }
    T::deserialize(value).map(Some).map_err(D::Error::custom)
}

impl From<&Span> for WireSpan {
    fn from(span: &Span) -> Self {
        let (unattributed_us, over_attributed_us) = span.representable_remainders();
        Self {
            stage: span.stage.id().to_string(),
            elapsed_us: wire_micros(span.elapsed),
            git_calls: span.git_calls,
            children_kind: span.children_kind,
            children: span.children.iter().map(WireSpan::from).collect(),
            unattributed_us,
            over_attributed_us,
        }
    }
}

impl From<&WorkerTimings> for WireWorkerTimings {
    fn from(timings: &WorkerTimings) -> Self {
        let (unattributed_us, over_attributed_us) = timings.root_remainders_us();
        Self {
            format_version: FORMAT_VERSION,
            total_us: wire_micros(timings.total),
            spans: timings.spans.iter().map(WireSpan::from).collect(),
            unattributed_us,
            over_attributed_us,
        }
    }
}

impl From<&WorkerReport> for WireReport {
    fn from(report: &WorkerReport) -> Self {
        Self {
            launch_index: report.launch_index,
            attempt_id: report.attempt_id.clone(),
            status: report.status(),
            report: report.timings().map(WorkerTimings::to_value),
        }
    }
}

fn malformed(error: impl fmt::Display) -> TimingsError {
    TimingsError::Malformed(error.to_string())
}

/// `bytes` as one JSON object with no repeated key at any depth and nothing
/// after it.
fn strict_value(bytes: &[u8]) -> Result<Value, TimingsError> {
    strict_json::members(bytes)
        .and_then(strict_json::Members::into_value)
        .ok_or_else(|| TimingsError::Malformed("not one JSON object free of repeated keys".to_string()))
}

fn check_version(value: &Value) -> Result<(), TimingsError> {
    match value.get("format_version").map(Value::as_u64) {
        Some(Some(FORMAT_VERSION)) => Ok(()),
        Some(Some(other)) => Err(TimingsError::UnsupportedVersion(other)),
        _ => Err(TimingsError::Malformed("format_version must be an unsigned integer".to_string())),
    }
}

fn decode_worker(value: Value) -> Result<WorkerTimings, TimingsError> {
    check_version(&value)?;
    let wire = WireWorkerTimings::deserialize(value).map_err(malformed)?;
    let spans = decode_root(wire.total_us, wire.spans, wire.unattributed_us, wire.over_attributed_us)?;
    Ok(WorkerTimings { total: Duration::from_micros(wire.total_us), spans })
}

fn decode_root(total_us: u64, spans: Vec<WireSpan>, unattributed_us: u64, over_attributed_us: u64) -> Result<Vec<Span>, TimingsError> {
    let spans = decode_spans(spans, &mut Vec::new())?;
    let expected = reconcile(u128::from(total_us), spans.iter().map(|span| micros(span.elapsed)));
    if expected != Some((unattributed_us, over_attributed_us)) {
        return Err(TimingsError::InconsistentReconciliation(Vec::new()));
    }
    Ok(spans)
}

fn decode_spans(wires: Vec<WireSpan>, path: &mut Vec<Stage>) -> Result<Vec<Span>, TimingsError> {
    let mut spans: Vec<Span> = Vec::with_capacity(wires.len());
    for wire in wires {
        let stage: Stage = wire.stage.parse()?;
        path.push(stage);
        if spans.iter().any(|span| span.stage == stage) {
            return Err(TimingsError::DuplicateSibling(path.clone()));
        }
        let children = decode_spans(wire.children, path)?;
        let span = Span {
            stage,
            elapsed: Duration::from_micros(wire.elapsed_us),
            git_calls: wire.git_calls,
            children_kind: wire.children_kind,
            children,
        };
        if span.remainders_us() != Some((wire.unattributed_us, wire.over_attributed_us)) {
            return Err(TimingsError::InconsistentReconciliation(path.clone()));
        }
        path.pop();
        spans.push(span);
    }
    Ok(spans)
}

fn decode_report(wire: WireReport) -> Result<WorkerReport, TimingsError> {
    let invalid = |reason: &str| TimingsError::InvalidWorkerReports(format!("launch {}: {reason}", wire.launch_index));
    if !is_attempt_id(&wire.attempt_id) {
        return Err(invalid("attempt_id is not an attempt id"));
    }
    let report = match (wire.status, wire.report) {
        (status @ (WorkerReportStatus::Complete | WorkerReportStatus::Partial), Some(value)) => {
            let timings = decode_worker(value)?;
            if timings.status() != Some(status) {
                return Err(invalid("the status does not match the halves the report measured"));
            }
            LaunchReport::from_worker(timings)
        }
        (WorkerReportStatus::Missing, None) => LaunchReport::Missing,
        (WorkerReportStatus::Invalid, None) => LaunchReport::Invalid,
        (WorkerReportStatus::Complete | WorkerReportStatus::Partial, None) => return Err(invalid("a usable status needs a report")),
        (WorkerReportStatus::Missing | WorkerReportStatus::Invalid, Some(_)) => return Err(invalid("an unusable status has no report")),
        (WorkerReportStatus::Adopted | WorkerReportStatus::OriginChanged, _) => {
            return Err(invalid("adopted and origin_changed describe the listing, not a launch"));
        }
    };
    Ok(WorkerReport { launch_index: wire.launch_index, attempt_id: wire.attempt_id, report })
}

/// Launches are listed `0..n` in order, and the summary is one
/// [`summarize_worker_reports`] could give them.
fn check_reports(reports: &[WorkerReport], status: Option<WorkerReportStatus>) -> Result<(), TimingsError> {
    let in_order = reports.iter().enumerate().all(|(index, report)| u32::try_from(index) == Ok(report.launch_index));
    if !in_order {
        return Err(TimingsError::InvalidWorkerReports("launches are not listed 0..n in order".to_string()));
    }
    let consistent = match status {
        None => reports.is_empty(),
        Some(WorkerReportStatus::OriginChanged) => reports.is_empty(),
        Some(WorkerReportStatus::Adopted) => summarize_worker_reports(reports, true, false) == WorkerReportStatus::Adopted,
        Some(status) => summarize_worker_reports(reports, false, false) == status,
    };
    if !consistent {
        return Err(TimingsError::InvalidWorkerReports(format!("summary {status:?} does not describe the launches")));
    }
    Ok(())
}

/// A worker document as a timed worker writes it: setup, then the halves
/// group holding `halves` (each with one operation inside), all in whole
/// microseconds so it round-trips exactly.
#[cfg(test)]
pub(crate) fn worker_fixture(halves: &[Stage]) -> WorkerTimings {
    let us = Duration::from_micros;
    let mut group = SpanList::concurrent();
    for &half in halves {
        let inside = match half {
            Stage::PrRefresh => Stage::PrRequest,
            Stage::HeadRefresh => Stage::HeadCheck,
            other => panic!("{other} is not a worker half"),
        };
        let mut steps = SpanList::sequential();
        steps.push(Span::new(inside, us(200)));
        group.push(Span::new(half, us(300)).with_children(steps));
    }
    let mut top = SpanList::sequential();
    top.push(Span::new(Stage::WorkerSetup, us(40)));
    top.push(Span::new(Stage::WorkerHalves, us(350)).with_children(group));
    WorkerTimings::new(us(400), top).expect("the fixture is representable")
}

#[cfg(test)]
mod tests;
