//! Process-wide counts of expensive analysis work, so tests can prove work
//! was skipped (for example, that a symbol listing ran no diagnostics)
//! without measuring time.
//!
//! The counts are atomics, not thread-locals, so work done on any thread
//! (including rayon workers) is counted. They only ever grow: compare two
//! [`work_counters`] snapshots with [`WorkCounters::since`].

use std::sync::atomic::{AtomicU64, Ordering};

static DIAGNOSTICS_RUNS: AtomicU64 = AtomicU64::new(0);
static PARSER_CONSTRUCTIONS: AtomicU64 = AtomicU64::new(0);
static DEFINITION_CAPTURES: AtomicU64 = AtomicU64::new(0);

/// A snapshot of the process-wide work counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkCounters {
    /// Lint diagnostic runs (pattern and semantic rules over one file), by
    /// the diagnostics pass or a direct `lint_diagnostics` /
    /// `try_lint_diagnostics` / `try_diagnostics` call.
    pub diagnostics_runs: u64,
    /// Tree-sitter parsers constructed to parse a source file.
    pub parser_constructions: u64,
    /// Executions of a file's definition (locals) query.
    pub definition_captures: u64,
}

impl WorkCounters {
    /// The work done between `earlier` and this snapshot.
    pub fn since(self, earlier: WorkCounters) -> WorkCounters {
        WorkCounters {
            diagnostics_runs: self.diagnostics_runs - earlier.diagnostics_runs,
            parser_constructions: self.parser_constructions - earlier.parser_constructions,
            definition_captures: self.definition_captures - earlier.definition_captures,
        }
    }
}

/// The current process-wide work counts.
///
/// ## Notes
///
/// The counts are shared by every caller in the process; a test reading a
/// delta must not run other analyses concurrently in the same process
/// (nextest runs each test in its own process).
pub fn work_counters() -> WorkCounters {
    WorkCounters {
        diagnostics_runs: DIAGNOSTICS_RUNS.load(Ordering::Relaxed),
        parser_constructions: PARSER_CONSTRUCTIONS.load(Ordering::Relaxed),
        definition_captures: DEFINITION_CAPTURES.load(Ordering::Relaxed),
    }
}

pub(crate) fn count_diagnostics_run() {
    DIAGNOSTICS_RUNS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_parser_construction() {
    PARSER_CONSTRUCTIONS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_definition_capture() {
    DEFINITION_CAPTURES.fetch_add(1, Ordering::Relaxed);
}
