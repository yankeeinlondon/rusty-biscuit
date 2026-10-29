//! Process-wide counts of named units of work, for measurements and
//! complexity regressions that read a before/after delta.
//!
//! Compiled only with the test-only `work-counters` feature; production
//! builds carry no counter state, and [`note_work!`](crate::note_work) expands
//! to nothing. Counts are process-wide atomics, so work spread over threads is
//! included; nextest's process-per-test model keeps them private to one test.
//!
//! ```ignore
//! use darkmatter::work_counters::{Work, snapshot};
//! let before = snapshot();
//! run_scenario();
//! assert_eq!(snapshot().since(&before).get(Work::ValidatorCompile), 2);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// One kind of counted work. Recorded with [`note_work!`](crate::note_work).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Work {
    /// An expression lexed and parsed by the expression parser.
    ExpressionParse,
    /// A trigger-registry scan of a document's schema roots.
    TriggerDiscovery,
    /// An effective schema prepared for one document.
    EffectivePreparation,
    /// A `jsonschema` validator compiled with Darkmatter's formats and
    /// keywords, whoever asked for it.
    ValidatorCompile,
    /// An eager-file union arm compiled by the rewrite (also counted as a
    /// [`Work::ValidatorCompile`]).
    RewriteArmCompile,
    /// An `alphanumeric` default/suggest check compiled during lowering (also
    /// counted as a [`Work::ValidatorCompile`]).
    AlphanumericCheckCompile,
    /// A source [`PositionMap`](crate::markdown::schemas::PositionMap)
    /// prepared for one validation.
    PositionMapBuild,
    /// A `LifecycleSourceMap` built from frontmatter.
    LifecycleSourceMapBuild,
    /// A link visited while a scope join collects ancestor addresses or
    /// walks a branch back to the shared history.
    ScopeJoinAncestorVisit,
    /// An earlier operand site compared by path while recording a new one.
    OperandSiteComparison,
    /// A schema node's operand markers collected.
    OperandMarksComputation,
    /// A schema node visited while collecting operand markers.
    OperandMarksNodeVisit,
    /// A named-type table parsed from an imported file's text (a memo miss).
    NamedTypeTableParse,
    /// A rejected overload's `CandidateFailure` built while binding a call.
    CandidateFailureBuild,
    /// An owned conversion `ValueFailure` built by the coercion engine.
    CoercionFailureBuild,
    /// Bytes of prefix scanned to turn one byte offset into a line and
    /// column.
    PositionPrefixScanBytes,
    /// A one-alternative schema prepared (definitions collected, lowered,
    /// hashed) to test one value in editor assistance.
    AlternativeMatcherPrepare,
    /// A recursive `$ref` target body translated into a conversion shape.
    RecursiveTargetTranslate,
    /// A mapping entry key compared while locating a problem's position.
    PositionKeyComparison,
    /// A document's lifecycle configuration parsed from frontmatter.
    LifecycleConfigParse,
    /// An expression parsed while scanning frontmatter values for reads of a
    /// late root (`derived_deferred_keys`); also counted as a
    /// [`Work::ExpressionParse`].
    DeferredRootScanParse,
    /// JSON nodes ([`value_nodes`]) copied into the state a lifecycle `set`
    /// action's shell values execute against: the values the frontmatter view
    /// owns, never its shared base.
    ShellValueStateNodes,
}

impl Work {
    /// Every kind, in declaration order.
    pub const ALL: &[Work] = &[
        Work::ExpressionParse,
        Work::TriggerDiscovery,
        Work::EffectivePreparation,
        Work::ValidatorCompile,
        Work::RewriteArmCompile,
        Work::AlphanumericCheckCompile,
        Work::PositionMapBuild,
        Work::LifecycleSourceMapBuild,
        Work::ScopeJoinAncestorVisit,
        Work::OperandSiteComparison,
        Work::OperandMarksComputation,
        Work::OperandMarksNodeVisit,
        Work::NamedTypeTableParse,
        Work::CandidateFailureBuild,
        Work::CoercionFailureBuild,
        Work::PositionPrefixScanBytes,
        Work::AlternativeMatcherPrepare,
        Work::RecursiveTargetTranslate,
        Work::PositionKeyComparison,
        Work::LifecycleConfigParse,
        Work::DeferredRootScanParse,
        Work::ShellValueStateNodes,
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

    /// The work of this snapshot and `other` together, for a total over
    /// separately measured phases.
    pub fn plus(&self, other: &WorkSnapshot) -> WorkSnapshot {
        let count = |snapshot: &WorkSnapshot, index: usize| snapshot.counts.get(index).copied().unwrap_or(0);
        WorkSnapshot { counts: (0..Work::ALL.len()).map(|index| count(self, index) + count(other, index)).collect() }
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
