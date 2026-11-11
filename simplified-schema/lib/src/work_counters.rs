//! Process-wide counts of the library's named units of work, for
//! measurements and complexity regressions that read a before/after delta.
//!
//! Compiled only with the test-only `work-counters` feature; production
//! builds carry no counter state, and the crate-internal `note_work!` expands
//! to nothing. Darkmatter's `work-counters` feature enables this one and folds
//! these counts into its own `darkmatter::work_counters` snapshot, so a host
//! reads one set of counters.
//!
//! ```
//! use simplified_schema::work_counters::{Work, snapshot};
//!
//! let before = snapshot();
//! simplified_schema::expression::parse("a || b").unwrap();
//! assert_eq!(snapshot().since(&before).get(Work::ExpressionParse), 1);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// One kind of counted work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Work {
    /// An expression lexed and parsed by the expression parser.
    ExpressionParse,
    /// A `jsonschema` validator compiled with the library's formats and
    /// keywords, whoever asked for it.
    ValidatorCompile,
    /// An `alphanumeric` default/suggest check compiled during lowering: a
    /// miss in the shared structural cache (also counted as a
    /// [`Work::ValidatorCompile`]).
    AlphanumericCheckCompile,
    /// A named-type table parsed from an imported file's text (a memo miss).
    NamedTypeTableParse,
    /// An owned conversion `ValueFailure` built by the coercion engine.
    CoercionFailureBuild,
    /// A one-alternative schema prepared (definitions collected, lowered,
    /// hashed) to test one value in editor assistance.
    AlternativeMatcherPrepare,
    /// A recursive `$ref` target body translated into a conversion shape.
    RecursiveTargetTranslate,
    /// A mapping entry key compared while locating a problem's position.
    PositionKeyComparison,
    /// A shipped artifact's bytes decoded ([`crate::shipped::accept`]).
    ArtifactDecode,
    /// A shipped artifact accepted: its entries replace runtime preparation.
    ArtifactAccept,
    /// A fresh, uncached preparation of the library's shipped sources for
    /// artifact generation ([`crate::shipped::FreshLower::prepare`]).
    ArtifactGeneration,
    /// A shipped source prepared from its text at run time: the envelope
    /// definitions after their artifact was refused, or a shipped partial's
    /// named types parsed without an accepted prepared table.
    ShippedRuntimePreparation,
    /// An artifact refused because none was supplied.
    ArtifactFallbackMissing,
    /// An artifact refused as malformed.
    ArtifactFallbackMalformed,
    /// An artifact refused for its wire version.
    ArtifactFallbackWireVersion,
    /// An artifact refused for its owner's preparation revision.
    ArtifactFallbackRevision,
    /// An artifact refused because a source text changed.
    ArtifactFallbackSource,
    /// An artifact refused because a lower artifact changed or was refused.
    ArtifactFallbackDependency,
    /// An artifact refused because preparation configuration changed.
    ArtifactFallbackConfiguration,
    /// An artifact refused for an unusable payload record or another owner.
    ArtifactFallbackRecord,
}

impl Work {
    /// Every kind, in declaration order.
    pub const ALL: &[Work] = &[
        Work::ExpressionParse,
        Work::ValidatorCompile,
        Work::AlphanumericCheckCompile,
        Work::NamedTypeTableParse,
        Work::CoercionFailureBuild,
        Work::AlternativeMatcherPrepare,
        Work::RecursiveTargetTranslate,
        Work::PositionKeyComparison,
        Work::ArtifactDecode,
        Work::ArtifactAccept,
        Work::ArtifactGeneration,
        Work::ShippedRuntimePreparation,
        Work::ArtifactFallbackMissing,
        Work::ArtifactFallbackMalformed,
        Work::ArtifactFallbackWireVersion,
        Work::ArtifactFallbackRevision,
        Work::ArtifactFallbackSource,
        Work::ArtifactFallbackDependency,
        Work::ArtifactFallbackConfiguration,
        Work::ArtifactFallbackRecord,
    ];
}

static COUNTS: [AtomicU64; Work::ALL.len()] = [const { AtomicU64::new(0) }; Work::ALL.len()];

/// Adds `count` units of `work`.
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
}

/// The counters' current values.
pub fn snapshot() -> WorkSnapshot {
    WorkSnapshot { counts: COUNTS.iter().map(|count| count.load(Ordering::Relaxed)).collect() }
}
