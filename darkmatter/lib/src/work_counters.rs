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
    /// An expression lexed and parsed by the expression parser. Counted by
    /// simplified-schema, which owns the parser, as are the other kinds
    /// whose code moved there (see `library_kind`).
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
    /// An `alphanumeric` default/suggest check compiled during lowering: a
    /// miss in the shared structural cache (also counted as a
    /// [`Work::ValidatorCompile`]).
    AlphanumericCheckCompile,
    /// A source [`PositionMap`](simplified_schema::PositionMap) built
    /// from a document, at most once per validation report and only for a
    /// report that holds a problem.
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
    /// A whole YAML document's selected schema prepared: its `$schema`
    /// resolved (on the fallback route) and its validators built. The
    /// threads it ran on are recorded ([`threads`]).
    YamlContractPreparation,
    /// The file-reading kind checks of a `schema` or `yaml-schema` document
    /// run (its own imports resolved). The threads are recorded.
    YamlKindCheck,
    /// A standalone schema document's named-type imports checked through
    /// [`check_standalone_named_types`](crate::markdown::schemas::resolve::check_standalone_named_types)
    /// (an editor's schema-authoring state). The threads are recorded.
    StandaloneImportCheck,
    /// A schema slot evaluated by Darkmatter's slot host (a resolution-cache
    /// hit evaluates none).
    SlotResolution,
    /// A cached slot resolution's input file read again to confirm its bytes
    /// are unchanged.
    SlotInputRehash,
    /// A shipped artifact's bytes decoded (any owner; counted by
    /// simplified-schema's shared loader).
    ArtifactDecode,
    /// A shipped artifact accepted: its entries replace runtime preparation.
    ArtifactAccept,
    /// A fresh, uncached preparation for artifact generation.
    ArtifactGeneration,
    /// A shipped source prepared from its compiled text at run time because
    /// no accepted artifact entry served it (library and Darkmatter sources
    /// together).
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
    /// The baseline simple-object check (`validate_baseline_schema`) run on
    /// a baseline being attached; an accepted shipped baseline skips it.
    BaselineCheck,
    /// A trigger payload's merge-compatibility (simple-object) check run;
    /// an accepted prepared trigger skips it.
    PayloadMergeCheck,
    /// A trigger payload's compile check run; an accepted prepared trigger
    /// skips it.
    PayloadCompileCheck,
    /// A simplified-schema `PositionMap` built from its source text (an
    /// eager map, or a deferred one forced by its first query).
    SourcePositionMapConstruction,
    /// A schema serialized to canonical JSON and hashed for its
    /// validator-cache identity (counted by simplified-schema). A baseline or
    /// trigger payload served from a shipped artifact carries its digest,
    /// and a merged or phase-projected schema is identified by its layers, so
    /// neither is serialized for its key.
    CanonicalSchemaSerialization,
    /// An effective schema's merged JSON Schema projected to the declared
    /// layer (`RawDeclarations`), done the first time an expression site
    /// reads a declaration; a document whose expressions read none (or that
    /// has none) never pays it.
    RawDeclarationProjection,
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
        Work::YamlContractPreparation,
        Work::YamlKindCheck,
        Work::StandaloneImportCheck,
        Work::SlotResolution,
        Work::SlotInputRehash,
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
        Work::BaselineCheck,
        Work::PayloadMergeCheck,
        Work::PayloadCompileCheck,
        Work::SourcePositionMapConstruction,
        Work::CanonicalSchemaSerialization,
        Work::RawDeclarationProjection,
    ];

    /// Whether [`add`] records the thread this work ran on.
    fn records_thread(self) -> bool {
        matches!(self, Work::YamlContractPreparation | Work::YamlKindCheck | Work::StandaloneImportCheck)
    }
}

static COUNTS: [AtomicU64; Work::ALL.len()] = [const { AtomicU64::new(0) }; Work::ALL.len()];

/// Adds `count` units of `work`. Use [`note_work!`](crate::note_work) at a
/// call site so it compiles away without the feature.
pub fn add(work: Work, count: u64) {
    COUNTS[work as usize].fetch_add(count, Ordering::Relaxed);
    if work.records_thread() {
        let thread = std::thread::current().id();
        let mut threads = THREADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !threads.contains(&(work, thread)) {
            threads.push((work, thread));
        }
    }
}

/// Distinct `(work, thread)` pairs, for the kinds that record their thread.
static THREADS: std::sync::Mutex<Vec<(Work, std::thread::ThreadId)>> = std::sync::Mutex::new(Vec::new());

/// Every thread `work` has run on, in first-run order; empty for a kind
/// that does not record its thread.
pub fn threads(work: Work) -> Vec<std::thread::ThreadId> {
    let threads = THREADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    threads.iter().filter(|(kind, _)| *kind == work).map(|(_, thread)| *thread).collect()
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

/// The counters' current values, with simplified-schema's counts folded into
/// the kinds they share (the work of code that moved into the library).
pub fn snapshot() -> WorkSnapshot {
    let library = simplified_schema::work_counters::snapshot();
    WorkSnapshot {
        counts: Work::ALL
            .iter()
            .map(|work| COUNTS[*work as usize].load(Ordering::Relaxed) + library_kind(*work).map_or(0, |kind| library.get(kind)))
            .collect(),
    }
}

/// The simplified-schema counter that records `work`, when that code moved
/// into the library.
fn library_kind(work: Work) -> Option<simplified_schema::work_counters::Work> {
    use simplified_schema::work_counters::Work as Library;
    match work {
        Work::ExpressionParse => Some(Library::ExpressionParse),
        Work::ValidatorCompile => Some(Library::ValidatorCompile),
        Work::AlphanumericCheckCompile => Some(Library::AlphanumericCheckCompile),
        Work::NamedTypeTableParse => Some(Library::NamedTypeTableParse),
        Work::CoercionFailureBuild => Some(Library::CoercionFailureBuild),
        Work::AlternativeMatcherPrepare => Some(Library::AlternativeMatcherPrepare),
        Work::RecursiveTargetTranslate => Some(Library::RecursiveTargetTranslate),
        Work::PositionKeyComparison => Some(Library::PositionKeyComparison),
        Work::ArtifactDecode => Some(Library::ArtifactDecode),
        Work::ArtifactAccept => Some(Library::ArtifactAccept),
        Work::ArtifactGeneration => Some(Library::ArtifactGeneration),
        Work::ShippedRuntimePreparation => Some(Library::ShippedRuntimePreparation),
        Work::ArtifactFallbackMissing => Some(Library::ArtifactFallbackMissing),
        Work::ArtifactFallbackMalformed => Some(Library::ArtifactFallbackMalformed),
        Work::ArtifactFallbackWireVersion => Some(Library::ArtifactFallbackWireVersion),
        Work::ArtifactFallbackRevision => Some(Library::ArtifactFallbackRevision),
        Work::ArtifactFallbackSource => Some(Library::ArtifactFallbackSource),
        Work::ArtifactFallbackDependency => Some(Library::ArtifactFallbackDependency),
        Work::ArtifactFallbackConfiguration => Some(Library::ArtifactFallbackConfiguration),
        Work::ArtifactFallbackRecord => Some(Library::ArtifactFallbackRecord),
        Work::SourcePositionMapConstruction => Some(Library::PositionMapConstruction),
        Work::CanonicalSchemaSerialization => Some(Library::CanonicalSchemaSerialization),
        _ => None,
    }
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
