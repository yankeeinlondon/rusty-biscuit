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
    /// A private `RuntimeState` built for one group leg.
    RuntimeStateBranch,
    /// A copy of the live frontmatter cell made for one group leg.
    LiveFrontmatterCopy,
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
    /// early-binding frontmatter.
    PreflightStepFrontmatterNodes,
    /// JSON nodes ([`value_nodes`]) a preflight walk scans for reads of a
    /// late root (`derived_deferred_keys`) over a step's frontmatter.
    PreflightDeferredScanNodes,
    /// JSON nodes ([`value_nodes`]) of the frontmatter an early-binding
    /// evaluation state is built from.
    EarlyBindingStateNodes,
    /// JSON nodes ([`value_nodes`]) of the step frontmatter copied to classify
    /// one `with` provider hint.
    PredictionWithStateNodes,
    /// JSON nodes ([`value_nodes`]) of the step frontmatter copied to resolve
    /// one shell `set` value.
    ShellValueStateNodes,
    /// Recorded prompt references compared while deduplicating a new one.
    PromptReferenceCompare,
}

impl Work {
    /// Every kind, in declaration order.
    pub const ALL: &[Work] = &[
        Work::RuntimeSnapshot,
        Work::RuntimeStateBranch,
        Work::LiveFrontmatterCopy,
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
    ];
}

static COUNTS: [AtomicU64; Work::ALL.len()] = [const { AtomicU64::new(0) }; Work::ALL.len()];

/// Adds `count` units of `work`. Use [`note_work!`](crate::note_work) at a
/// call site so it compiles away without the feature.
pub fn add(work: Work, count: u64) {
    COUNTS[work as usize].fetch_add(count, Ordering::Relaxed);
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
    fn nodes(value: &serde_json::Value) -> u64 {
        1 + match value {
            serde_json::Value::Array(items) => items.iter().map(nodes).sum(),
            serde_json::Value::Object(map) => map.values().map(nodes).sum(),
            _ => 0,
        }
    }
    entries.into_iter().map(|(_, value)| nodes(value)).sum()
}
