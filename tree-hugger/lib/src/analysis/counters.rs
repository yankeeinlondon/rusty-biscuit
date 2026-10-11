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
static SHARED_CONTEXT_STEPS: AtomicU64 = AtomicU64::new(0);
static RELATION_LINKS: AtomicU64 = AtomicU64::new(0);
static COMPLEXITY_WALKS: AtomicU64 = AtomicU64::new(0);
static REFERENCE_CAPTURES: AtomicU64 = AtomicU64::new(0);
static OCCURRENCE_RESOLUTIONS: AtomicU64 = AtomicU64::new(0);
static REEXPORT_EXTRACTIONS: AtomicU64 = AtomicU64::new(0);

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
    /// Nodes and entries visited while deriving context that a file's or a
    /// module's declarations share (the file-level namespace in force, the
    /// preceding sibling-scoped container or access section, inner
    /// `#![cfg(test)]` attributes, same-file traits, `static` prototypes,
    /// package segments, same-file owners and members, the records a
    /// member's container or an export's target resolves to, and the records
    /// a god-file container call-out reads), counting both building that
    /// context and each declaration's lookup in it. Relation resolution
    /// also counts each lookup segment, candidate visited, inline-namespace
    /// step, member-range comparison and written-target insertion, plus
    /// generic parameter reads, name-set/index construction and bound-subject
    /// lookup. It grows with declarations, written paths and parameter lists,
    /// including those on unselected or record-less owners; a per-declaration
    /// rescan of the file or module shows up here as quadratic growth.
    pub shared_context_steps: u64,
    /// Runs of a file's relation linker, which builds the file's declaration
    /// index and, in Rust, walks its impls. One analysis runs it once,
    /// however many records it selects.
    pub relation_links: u64,
    /// Callable bodies walked to count `metrics.cyclomatic`: one per
    /// measured callable of a language with counting rules, and none for a
    /// query without metrics or a language without rules.
    pub complexity_walks: u64,
    /// Executions of a file's references query. One analysis runs it at
    /// most once, for the occurrences and the diagnostics together, and not
    /// at all without references or diagnostics.
    pub reference_captures: u64,
    /// Resolutions of a file's reference occurrences (scope tree, lookup,
    /// coverage): one per analysis that binds references, from which the
    /// reference edges and recursion are both read.
    pub occurrence_resolutions: u64,
    /// Extractions of a file's re-export report: one per analysis that
    /// requires `Capability::Reexports`, none otherwise.
    pub reexport_extractions: u64,
}

impl WorkCounters {
    /// The work done between `earlier` and this snapshot.
    pub fn since(self, earlier: WorkCounters) -> WorkCounters {
        WorkCounters {
            diagnostics_runs: self.diagnostics_runs - earlier.diagnostics_runs,
            parser_constructions: self.parser_constructions - earlier.parser_constructions,
            definition_captures: self.definition_captures - earlier.definition_captures,
            shared_context_steps: self.shared_context_steps - earlier.shared_context_steps,
            relation_links: self.relation_links - earlier.relation_links,
            complexity_walks: self.complexity_walks - earlier.complexity_walks,
            reference_captures: self.reference_captures - earlier.reference_captures,
            occurrence_resolutions: self.occurrence_resolutions - earlier.occurrence_resolutions,
            reexport_extractions: self.reexport_extractions - earlier.reexport_extractions,
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
        shared_context_steps: SHARED_CONTEXT_STEPS.load(Ordering::Relaxed),
        relation_links: RELATION_LINKS.load(Ordering::Relaxed),
        complexity_walks: COMPLEXITY_WALKS.load(Ordering::Relaxed),
        reference_captures: REFERENCE_CAPTURES.load(Ordering::Relaxed),
        occurrence_resolutions: OCCURRENCE_RESOLUTIONS.load(Ordering::Relaxed),
        reexport_extractions: REEXPORT_EXTRACTIONS.load(Ordering::Relaxed),
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

pub(crate) fn count_shared_context_steps(steps: usize) {
    SHARED_CONTEXT_STEPS.fetch_add(steps as u64, Ordering::Relaxed);
}

pub(crate) fn count_relation_link() {
    RELATION_LINKS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_complexity_walk() {
    COMPLEXITY_WALKS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_reference_capture() {
    REFERENCE_CAPTURES.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_occurrence_resolution() {
    OCCURRENCE_RESOLUTIONS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_reexport_extraction() {
    REEXPORT_EXTRACTIONS.fetch_add(1, Ordering::Relaxed);
}
