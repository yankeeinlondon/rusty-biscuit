//! Reusing a stored analysis for a different query: narrowing a wider result
//! to exactly what a query computes, and widening a result by the passes that
//! need only its records.
//!
//! Both rely on how each pass changes a record, as [`run_query`] runs them:
//! containment sets `relations.parent`/`container`/`members` (a Rust impl
//! member's from its impl, while records are built); binding resolves the
//! file's occurrences and sets `relations.references`/`dependencies`/
//! `referenced_by` from them, export targets and `is_exported` (to the
//! value the declaration already had, since both come from the same export
//! spans); the occurrences and their coverage stay in the index only under
//! `Occurrences` and describe the whole file whatever kinds are selected;
//! the semantic pass sets `semantics` on functions, methods and
//! constructors from those occurrences; the docs pass sets `docs.parsed`. The
//! re-export report and the imports' `reexported` markers describe the whole
//! file and exist only under `Reexports`, so projection drops both together
//! and adding the capability needs a parse. Flags and
//! metrics are read from the syntax tree while records are built, so adding
//! either needs a parse; without them a record's `is_test` and
//! `is_deprecated` are `false` and it has no `metrics` facet. Generic
//! parameters, modifiers, attributes and `extends`/`implements` come with
//! the declarations and do not depend on the selected kinds (targets resolve
//! against every declaration of the file), so projection keeps them as they
//! are. The file-wide
//! test rules are not part of any stored result: `SymbolQueryResult::finish`
//! applies them after projection. Each pass also
//! stamps `provenance.parse_pass`, which [`stamp_parse_pass`] recomputes.
//! `symbols_cache::projected_results_equal_uncached_queries` (and its `multi/` twin) pin the
//! equivalence for every fixture.
//!
//! [`run_query`]: super::run_query

use std::collections::BTreeSet;

use super::{NameIndex, PassPlan, docs_pass, has_semantics, link_containers, semantic_pass};
use crate::file::query::{Capability, SymbolQuery, SymbolQueryResult};
use crate::shared::{AnalysisPass, FileSymbolIndex, ProgrammingLanguage, SemanticFacet, SymbolKindV2};

/// Whether `missing` can be added to a result covering `kinds` (`None` =
/// every kind) and `completed` from its records alone, without parsing.
///
/// Containment needs every record as a candidate owner, so it needs a result
/// of every kind, and in Rust and Swift the syntax tree: an impl member's
/// container is its impl's self type as written and resolved, and a Swift
/// extension member's the extension's owner as resolved, which no record
/// retains;
/// semantics needs the stored occurrences and coverage (the reference edges
/// alone cannot tell a proven self-call from a same-named one); occurrences,
/// flags and metrics need the syntax tree.
pub(crate) fn record_passes_suffice(
    missing: &BTreeSet<Capability>,
    completed: &BTreeSet<Capability>,
    kinds: Option<&BTreeSet<SymbolKindV2>>,
    language: ProgrammingLanguage,
) -> bool {
    missing.iter().all(|capability| match capability {
        Capability::StructuredDocs => true,
        Capability::Semantics => completed.contains(&Capability::Occurrences),
        Capability::Containment => {
            kinds.is_none() && !matches!(language, ProgrammingLanguage::Rust | ProgrammingLanguage::Swift)
        }
        Capability::Declarations
        | Capability::References
        | Capability::Occurrences
        | Capability::Diagnostics
        | Capability::Flags
        | Capability::Metrics
        | Capability::Reexports => false,
    })
}

/// Runs the record-only passes for `missing` over `index`, which holds the
/// result of `completed`. The caller checks [`record_passes_suffice`] first.
pub(crate) fn add_record_passes(
    index: &mut FileSymbolIndex,
    completed: &BTreeSet<Capability>,
    missing: &BTreeSet<Capability>,
) {
    if missing.contains(&Capability::Containment) {
        let names = NameIndex::new(&index.symbols);
        link_containers(&mut index.symbols, &names);
    }
    if missing.contains(&Capability::Semantics) {
        semantic_pass(index);
    }
    if missing.contains(&Capability::StructuredDocs) {
        docs_pass(index);
    }
    let widened = SymbolQuery::new().capabilities(completed.union(missing).copied());
    index.completed_passes = PassPlan::for_query(&widened).passes();
    stamp_parse_pass(index, &widened);
}

/// Narrows `index`, a result of at least `query`'s kinds and capabilities, to
/// what [`run_query`](super::run_query) returns for `query`: in-file
/// qualified names, no predicate applied.
pub(crate) fn project(mut index: FileSymbolIndex, query: &SymbolQuery) -> SymbolQueryResult {
    index.symbols.retain(|symbol| query.selects(symbol.kind));
    let containment = query.requires(Capability::Containment);
    let references = query.requires(Capability::References);
    for symbol in &mut index.symbols {
        let relations = &mut symbol.relations;
        if !containment {
            relations.parent = None;
            relations.container = None;
            relations.members.clear();
        }
        if !references {
            relations.references.clear();
            relations.dependencies.clear();
            relations.referenced_by.clear();
        }
        if !query.requires(Capability::Semantics) {
            symbol.semantics = SemanticFacet::default();
        }
        if !query.requires(Capability::StructuredDocs) {
            symbol.docs.parsed = None;
        }
        if !query.requires(Capability::Flags) {
            symbol.visibility.is_test = false;
            symbol.visibility.is_deprecated = false;
        }
        if !query.requires(Capability::Metrics) {
            symbol.metrics = None;
        }
    }
    if !references {
        for export in &mut index.exports {
            export.symbol.id = None;
            export.symbol.qualified_name = None;
        }
    }
    if !query.requires(Capability::Occurrences) {
        index.occurrences.clear();
        index.reference_coverage.clear();
    }
    if !query.requires(Capability::Diagnostics) {
        index.diagnostics.clear();
    }
    if !query.requires(Capability::Reexports) {
        index.reexports = None;
        for import in &mut index.imports {
            import.reexported = false;
        }
    }
    index.completed_passes = PassPlan::for_query(query).passes();
    stamp_parse_pass(&mut index, query);
    SymbolQueryResult::unfinished(index, query.required_capabilities(), query.selected_kinds())
}

/// Sets each record's `provenance.parse_pass` to the last pass of `query`
/// that changes it.
fn stamp_parse_pass(index: &mut FileSymbolIndex, query: &SymbolQuery) {
    for symbol in &mut index.symbols {
        let pass = if query.requires(Capability::StructuredDocs) {
            AnalysisPass::Docs
        } else if query.requires(Capability::Semantics) && has_semantics(symbol.kind) {
            AnalysisPass::Semantic
        } else if query.requires(Capability::References) {
            AnalysisPass::Bind
        } else {
            AnalysisPass::Parse
        };
        symbol.provenance.parse_pass = pass.as_str().to_string();
    }
}
