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
}

impl Work {
    /// Every kind, in declaration order.
    pub const ALL: &[Work] = &[Work::ExpressionParse];
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
