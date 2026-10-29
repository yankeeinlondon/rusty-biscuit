//! Process-wide counts of named units of work, for measurements and
//! complexity regressions that read a before/after delta. Darkmatter's
//! counters (`darkmatter::work_counters`) are enabled by the same feature.
//!
//! Compiled only with the test-only `work-counters` feature; production
//! builds carry no counter state, and [`note_work!`](crate::note_work) expands
//! to nothing. Counts are process-wide atomics, so work spread over threads is
//! included; nextest's process-per-test model keeps them private to one test.
//!
//! ```ignore
//! use claudine::work_counters::{Work, snapshot};
//! let before = snapshot();
//! run_scenario();
//! assert_eq!(snapshot().since(&before).get(Work::RuntimeSnapshot), 2);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// One kind of counted work. Recorded with [`note_work!`](crate::note_work).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Work {
    /// A complete `RuntimeState` snapshot taken.
    RuntimeSnapshot,
    /// Committed output entries copied out of a `RuntimeState`, by a full
    /// snapshot or by a projection that reads outputs.
    RuntimeOutputsCopy,
    /// A private `RuntimeState` built for one group leg, when the leg is
    /// admitted; [`Gauge::RuntimeStateBranch`] counts the ones alive.
    RuntimeStateBranch,
    /// JSON nodes ([`value_nodes`]) deep-copied into group legs' live
    /// frontmatter cells: each cell copies the values the group-start view
    /// owns, never its shared base.
    LiveFrontmatterCopy,
    /// A private live-frontmatter cell made for one group leg, a view over
    /// the group-start state.
    LiveFrontmatterView,
    /// The whole reserved `hook` object built for one lookup.
    HookPayloadMaterialize,
    /// The whole `loop` global object built for one lookup.
    LoopAmbientMaterialize,
    /// An evaluation state built to resolve one pre-warm phrase.
    PrewarmStateBuild,
    /// A full frontmatter-sized state map built by provider prediction.
    PredictionStateBuild,
    /// An environment map copied (or a copy of an owner holding one) when a
    /// document is entered.
    EnvironmentMapCopy,
    /// JSON nodes ([`value_nodes`]) deep-copied into a preflight step's
    /// early-binding frontmatter: a preparation's seed once, then each step's
    /// own overlay.
    PreflightStepFrontmatterNodes,
    /// JSON nodes ([`value_nodes`]) a preflight walk scans for reads of a
    /// late root (`derived_deferred_keys`): a preparation's seed once, then
    /// each step's overlay, or the whole step view when the overlay changes
    /// what the seed derives.
    PreflightDeferredScanNodes,
    /// JSON nodes ([`value_nodes`]) an early-binding evaluation state copies
    /// from its frontmatter view: the values the view owns, not its shared
    /// base.
    EarlyBindingStateNodes,
    /// JSON nodes ([`value_nodes`]) copied to classify one provider hint that
    /// reads an expression: the hint and the keys it reads, directly or
    /// through other keys.
    PredictionWithStateNodes,
    /// JSON nodes ([`value_nodes`]) copied from the step frontmatter view to
    /// resolve one shell `set` value: the values the view owns, never its
    /// shared base; only a ternary's branches read it.
    ShellValueStateNodes,
    /// Recorded prompt references compared while deduplicating a new one.
    PromptReferenceCompare,
    /// JSON nodes ([`value_nodes`]) a looping container copies from its state
    /// view to resolve its templated cap and to stage each pass's action: the
    /// values the view owns (runtime writes, the action's own writes), never
    /// its shared base.
    ContainerStateNodes,
    /// JSON nodes ([`value_nodes`]) copied into pre-warm inputs: a whole
    /// captured document, or only the values a shared view owns.
    PrewarmStateNodes,
    /// A container's authoring document parsed by nested preparation: once
    /// per distinct text of the file in one invocation.
    NestedSourceParse,
    /// A referenced data document (group file or catalog, `kind: task` file,
    /// referenced sequence file) parsed by any preflight walk: once per
    /// distinct text of the file in one invocation.
    ReferencedDocumentParse,
    /// JSON nodes ([`nodes`]) of referenced data documents parsed, scanned in
    /// whole for retired reads, or copied out by a reference.
    ReferencedDocumentNodes,
    /// A file a group start reread to decide whether its sources changed
    /// since the audit walk captured them.
    FreshnessSourceReads,
}

impl Work {
    /// Every kind, in declaration order.
    pub const ALL: &[Work] = &[
        Work::RuntimeSnapshot,
        Work::RuntimeOutputsCopy,
        Work::RuntimeStateBranch,
        Work::LiveFrontmatterCopy,
        Work::LiveFrontmatterView,
        Work::HookPayloadMaterialize,
        Work::LoopAmbientMaterialize,
        Work::PrewarmStateBuild,
        Work::PredictionStateBuild,
        Work::EnvironmentMapCopy,
        Work::PreflightStepFrontmatterNodes,
        Work::PreflightDeferredScanNodes,
        Work::EarlyBindingStateNodes,
        Work::PredictionWithStateNodes,
        Work::ShellValueStateNodes,
        Work::PromptReferenceCompare,
        Work::ContainerStateNodes,
        Work::PrewarmStateNodes,
        Work::NestedSourceParse,
        Work::ReferencedDocumentParse,
        Work::ReferencedDocumentNodes,
        Work::FreshnessSourceReads,
    ];
}

static COUNTS: [AtomicU64; Work::ALL.len()] = [const { AtomicU64::new(0) }; Work::ALL.len()];

/// Adds `count` units of `work`. Use [`note_work!`](crate::note_work) at a
/// call site so it compiles away without the feature.
pub fn add(work: Work, count: u64) {
    COUNTS[work as usize].fetch_add(count, Ordering::Relaxed);
}

/// Units of work alive at once, each with the most that were alive at one
/// moment since the last [`reset_peak`]. Entered and left in pairs by the code
/// that owns the unit; a test reads [`live`] and [`peak`] directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Gauge {
    /// Group legs' private `RuntimeState` branches: built at admission and
    /// dropped when the leg settles, before a replacement leg is admitted.
    RuntimeStateBranch,
}

impl Gauge {
    /// Every gauge, in declaration order.
    pub const ALL: &[Gauge] = &[Gauge::RuntimeStateBranch];
}

static LIVE: [AtomicU64; Gauge::ALL.len()] = [const { AtomicU64::new(0) }; Gauge::ALL.len()];
static PEAK: [AtomicU64; Gauge::ALL.len()] = [const { AtomicU64::new(0) }; Gauge::ALL.len()];

/// One more `gauge` unit is alive.
pub fn enter(gauge: Gauge) {
    let live = LIVE[gauge as usize].fetch_add(1, Ordering::SeqCst) + 1;
    PEAK[gauge as usize].fetch_max(live, Ordering::SeqCst);
}

/// One `gauge` unit entered with [`enter`] is gone.
pub fn leave(gauge: Gauge) {
    LIVE[gauge as usize].fetch_sub(1, Ordering::SeqCst);
}

/// How many `gauge` units are alive now.
pub fn live(gauge: Gauge) -> u64 {
    LIVE[gauge as usize].load(Ordering::SeqCst)
}

/// The most `gauge` units alive at one moment since the last [`reset_peak`].
pub fn peak(gauge: Gauge) -> u64 {
    PEAK[gauge as usize].load(Ordering::SeqCst)
}

/// Start a new peak measurement from the units alive now.
pub fn reset_peak(gauge: Gauge) {
    PEAK[gauge as usize].store(live(gauge), Ordering::SeqCst);
}

/// Every counter's value at one moment, or the work between two
/// ([`Self::since`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkSnapshot {
    counts: Vec<u64>,
}

impl WorkSnapshot {
    /// The count for `work`.
    pub fn get(&self, work: Work) -> u64 {
        self.counts.get(work as usize).copied().unwrap_or(0)
    }

    /// The work done between `earlier` and this snapshot.
    pub fn since(&self, earlier: &WorkSnapshot) -> WorkSnapshot {
        WorkSnapshot {
            counts: self.counts.iter().enumerate().map(|(index, count)| count - earlier.counts.get(index).copied().unwrap_or(0)).collect(),
        }
    }

    /// `(kind, count)` for every nonzero counter.
    pub fn nonzero(&self) -> Vec<(Work, u64)> {
        Work::ALL.iter().map(|work| (*work, self.get(*work))).filter(|(_, count)| *count > 0).collect()
    }
}

/// The counters' current values.
pub fn snapshot() -> WorkSnapshot {
    WorkSnapshot { counts: COUNTS.iter().map(|count| count.load(Ordering::Relaxed)).collect() }
}

/// The JSON values in `entries`' values, counting every nested value once: a
/// size measure for a frontmatter-shaped map that grows with an embedded
/// `sequence:` list.
pub fn value_nodes<'a>(entries: impl IntoIterator<Item = (&'a String, &'a serde_json::Value)>) -> u64 {
    entries.into_iter().map(|(_, value)| nodes(value)).sum()
}

/// `value` and every value nested in it, each counted once.
pub fn nodes(value: &serde_json::Value) -> u64 {
    1 + match value {
        serde_json::Value::Array(items) => items.iter().map(nodes).sum(),
        serde_json::Value::Object(map) => map.values().map(nodes).sum(),
        _ => 0,
    }
}
