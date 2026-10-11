//! The staged per-file analysis behind [`TreeFile::query`] and
//! [`TreeFile::symbol_index_with_passes`]: one plan, built from the
//! requested capabilities, runs each needed pass once, after its
//! prerequisites.

use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use crate::analysis::counters::{count_occurrence_resolution, count_shared_context_steps};
use crate::analysis::occurrences::OccurrenceSyntax;
use crate::error::TreeHuggerError;
use crate::file::query::{Capability, SymbolQuery, SymbolQueryResult};
use crate::file::tree_file::{Definitions, MetadataSelection, RecordFacts, TreeFile};
use crate::shared::{
    AnalysisPass, CommentAttachment, Confidence, CoverageState, DocsFacet, ExportRecord,
    FileSymbolIndex, ImportRecord, ImportSymbol, Occurrence, OccurrenceKind, ParsedDocs, ProgrammingLanguage,
    ReferenceCoverage, SemanticFacet, SymbolId, SymbolKindV2, SymbolRecord, SymbolRef, TriState,
};

pub mod counters;
pub(crate) mod coverage;
pub(crate) mod declaration_names;
pub(crate) mod occurrences;
pub(crate) mod reexports;
pub(crate) mod name_identity;
pub(crate) mod scope;
pub(crate) mod syntax_markers;

/// Runs the requested passes, after their prerequisites, over every kind, as
/// [`TreeFile::query`] does.
pub fn analyze_file(
    tree_file: &TreeFile,
    requested_passes: &[AnalysisPass],
) -> Result<FileSymbolIndex, TreeHuggerError> {
    Ok(tree_file.query(&SymbolQuery::for_passes(requested_passes))?.index)
}

/// The work a query needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PassPlan {
    /// Parent and member links only, without the reference query.
    containment: bool,
    /// The full bind pass (containment, references, export targets), which
    /// resolves the file's occurrences to derive the reference edges.
    bind: bool,
    /// The occurrences and their coverage stay in the result.
    occurrences: bool,
    semantic: bool,
    docs: bool,
    diagnostics: bool,
    /// The re-export report and the imports' `reexported` markers.
    reexports: bool,
}

impl PassPlan {
    fn for_query(query: &SymbolQuery) -> Self {
        Self {
            containment: query.requires(Capability::Containment),
            bind: query.requires(Capability::References),
            occurrences: query.requires(Capability::Occurrences),
            semantic: query.requires(Capability::Semantics),
            docs: query.requires(Capability::StructuredDocs),
            diagnostics: query.requires(Capability::Diagnostics),
            reexports: query.requires(Capability::Reexports),
        }
    }

    /// The passes this plan runs, in canonical order.
    fn passes(self) -> Vec<AnalysisPass> {
        [
            (true, AnalysisPass::Parse),
            (self.bind, AnalysisPass::Bind),
            (self.semantic, AnalysisPass::Semantic),
            (self.docs, AnalysisPass::Docs),
            (self.diagnostics, AnalysisPass::Diagnostics),
        ]
        .into_iter()
        .filter_map(|(runs, pass)| runs.then_some(pass))
        .collect()
    }
}

/// Runs `query` over `tree_file`: the definition query runs once and its
/// result feeds the records, the occurrences and the diagnostics, and the
/// references query runs at most once for the occurrences and the
/// diagnostics. The result is unfinished (in-file qualified names, no
/// predicate applied), as the cache stores it.
pub(crate) fn run_query(
    tree_file: &TreeFile,
    query: &SymbolQuery,
) -> Result<SymbolQueryResult, TreeHuggerError> {
    tree_file.with_parent_index(|| run_indexed(tree_file, query))
}

/// [`run_query`] with the file's parent index installed.
fn run_indexed(
    tree_file: &TreeFile,
    query: &SymbolQuery,
) -> Result<SymbolQueryResult, TreeHuggerError> {
    let plan = PassPlan::for_query(query);
    let selection = MetadataSelection::for_kinds(query.selected_kinds());

    let Definitions { symbols: definitions, import_nodes, scopes: scope_nodes, rust_walk, rust_patterns } = if plan.bind {
        tree_file.definitions_with_scopes(&selection)?
    } else {
        tree_file.definitions(&selection)?
    };
    // The definition run captured the imports too: `imported_symbols` would
    // run the same query text again.
    let mut imports = tree_file.imports_from_nodes(&import_nodes);
    // Forwarding is proven before diagnostics, which read each import's
    // `is_reexport`; `Diagnostics` requires `Reexports`.
    let reexports = plan.reexports.then(|| reexports::flag_forwarded_imports(tree_file, &mut imports));
    let captures = if plan.bind || plan.diagnostics {
        tree_file.reference_captures()?
    } else {
        None
    };
    let diagnostics = if plan.diagnostics {
        let references = captures.as_deref().map_or_else(Vec::new, |captures| {
            tree_file.references_from_captures(captures, &definitions, &imports)
        });
        tree_file.diagnostics_from(&definitions, &imports, &references)?
    } else {
        Vec::new()
    };
    // Occurrence syntax is read from the definitions' nodes, which the
    // records below consume; records keep the definitions' order.
    let occurrence_syntax = plan.bind.then(|| {
        OccurrenceSyntax::new(
            tree_file.language,
            tree_file.source(),
            tree_file.root_node(),
            &definitions,
            scope_nodes,
            &imports,
            captures,
        )
        .with_rust_patterns(rust_patterns)
    });
    let exports = tree_file
        .exports_from(&definitions)
        .map(export_record)
        .collect();
    let facts = RecordFacts {
        flags: query.requires(Capability::Flags),
        metrics: query.requires(Capability::Metrics),
        containment: plan.containment,
    };
    let symbols = tree_file.records_from(definitions, &selection, facts, rust_walk)?;

    let import_records: Vec<ImportRecord> = imports.iter().map(import_record).collect();

    let mut index = FileSymbolIndex {
        schema_version: crate::shared::SchemaVersion::CURRENT,
        file: tree_file.file.clone(),
        language: tree_file.language,
        file_hash: tree_file.hash.clone(),
        completed_passes: plan.passes(),
        symbols,
        imports: import_records,
        exports,
        diagnostics,
        occurrences: Vec::new(),
        reference_coverage: Vec::new(),
        reexports,
    };

    let names = NameIndex::new(&index.symbols);
    if plan.containment {
        link_containers(&mut index.symbols, &names);
    }
    if let Some(syntax) = occurrence_syntax {
        // Resolved against every record, before unselected ones are dropped.
        count_occurrence_resolution();
        let resolved = syntax.resolve(&index.symbols);
        index.occurrences = resolved.evidence.occurrences;
        index.reference_coverage = resolved.evidence.reference_coverage;
        bind_references(&mut index, &resolved.bound_imports);
    }

    // Unselected records were needed only as owners, containers and
    // reference targets.
    index.symbols.retain(|symbol| query.selects(symbol.kind));

    if plan.semantic {
        semantic_pass(&mut index);
    }
    if !plan.occurrences {
        index.occurrences.clear();
        index.reference_coverage.clear();
    }
    if plan.docs {
        docs_pass(&mut index);
    }

    Ok(SymbolQueryResult::unfinished(
        index,
        query.required_capabilities(),
        query.selected_kinds(),
    ))
}

fn text_span(range: &crate::shared::CodeRange) -> crate::shared::TextSpan {
    crate::shared::TextSpan {
        start: crate::shared::TextPoint {
            line: range.start_line as u32,
            column: range.start_column as u32,
        },
        end: crate::shared::TextPoint {
            line: range.end_line as u32,
            column: range.end_column as u32,
        },
        start_byte: range.start_byte as u32,
        end_byte: range.end_byte as u32,
    }
}

pub(crate) fn import_record(import: &ImportSymbol) -> ImportRecord {
    ImportRecord {
        name: import.name.clone(),
        alias: import.alias.clone(),
        source: import.source.clone(),
        span: text_span(&import.range),
        statement_span: import.statement_range.as_ref().map(text_span),
        symbol: SymbolRef {
            source_span: None,
            id: None,
            name: import.name.clone(),
            qualified_name: import.original_name.clone(),
        },
        reexported: import.is_reexport,
    }
}

fn export_record(export: &crate::shared::SymbolInfo) -> ExportRecord {
    ExportRecord {
        name: export.name.clone(),
        span: text_span(&export.range),
        symbol: SymbolRef {
            source_span: None,
            id: None,
            name: export.name.clone(),
            qualified_name: None,
        },
    }
}

/// Record indexes by name, each bucket in record order, built once per file
/// so a name lookup is not a scan of every record.
struct NameIndex {
    by_name: HashMap<String, Vec<usize>>,
}

impl NameIndex {
    fn new(symbols: &[SymbolRecord]) -> Self {
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, symbol) in symbols.iter().enumerate() {
            by_name
                .entry(symbol.identity.name.clone())
                .or_default()
                .push(idx);
        }
        Self { by_name }
    }

    /// The indexes of the records called `name`, in record order.
    fn named(&self, name: &str) -> &[usize] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }
}

/// Containment: each record's parent and container, and each container's
/// members.
///
/// A record that already has a container (a Rust impl member or a Swift
/// extension member, linked by `TreeFile::records_from` from its impl's
/// resolved self type or its extension's resolved owner) keeps it and its
/// parent, and joins its parent's members in record order.
fn link_containers(symbols: &mut [SymbolRecord], names: &NameIndex) {
    let mut parents: Vec<Option<usize>> = vec![None; symbols.len()];
    for (child_idx, parent_idx) in resolve_containers(symbols, names) {
        parents[child_idx] = Some(parent_idx);
    }
    if symbols.iter().any(|symbol| symbol.relations.container.is_some()) {
        let by_id: HashMap<&SymbolId, usize> =
            symbols.iter().enumerate().map(|(idx, symbol)| (&symbol.id, idx)).collect();
        for (idx, symbol) in symbols.iter().enumerate() {
            if let Some(parent) = &symbol.relations.parent
                && let Some(id) = &parent.id
            {
                parents[idx] = by_id.get(id).copied();
            }
        }
    }

    let mut members_by_parent: HashMap<SymbolId, Vec<SymbolRef>> = HashMap::new();
    for (child_idx, parent_idx) in parents.iter().enumerate() {
        if let Some(parent_idx) = parent_idx {
            members_by_parent
                .entry(symbols[*parent_idx].id.clone())
                .or_default()
                .push(symbol_ref(&symbols[child_idx]));
        }
    }
    let parent_links: Vec<Option<SymbolRef>> =
        parents.iter().map(|parent| parent.map(|idx| symbol_ref(&symbols[idx]))).collect();

    for (symbol, parent) in symbols.iter_mut().zip(parent_links) {
        if symbol.relations.container.is_none() {
            symbol.relations.container = parent.clone();
            symbol.relations.parent = parent;
        }
        symbol.relations.members = members_by_parent.remove(&symbol.id).unwrap_or_default();
    }
}

/// Each record that names a container, paired with the record its container
/// names, in record order. The rules are those of [`smallest_enclosing`],
/// then [`find_container_by_identity`].
fn resolve_containers(symbols: &[SymbolRecord], names: &NameIndex) -> Vec<(usize, usize)> {
    let children: Vec<ContainerRequest<'_>> = symbols
        .iter()
        .enumerate()
        .filter(|(_, symbol)| symbol.relations.container.is_none())
        .filter_map(|(child_idx, symbol)| {
            let container_name = symbol.identity.container_name.as_deref()?;
            // A record may itself carry a dotted name (C# `namespace A.B`).
            let bare = if names.named(container_name).is_empty() {
                last_name_segment(container_name)
            } else {
                container_name
            };
            Some(ContainerRequest { child_idx, container_name, bare })
        })
        .collect();
    let enclosing = smallest_enclosing(symbols, names, &children);
    let owners = OnceCell::new();
    children
        .iter()
        .zip(enclosing)
        .filter_map(|(request, enclosing)| {
            let parent_idx = enclosing.or_else(|| {
                let owners = owners.get_or_init(|| OwnerIndex::new(symbols));
                find_container_by_identity(symbols, owners, request)
            })?;
            Some((request.child_idx, parent_idx))
        })
        .collect()
}

/// The record `container_name` names for the one child `child_idx`, by the
/// rules [`resolve_containers`] applies to a whole file.
#[cfg(test)]
fn find_container_symbol(
    symbols: &[SymbolRecord],
    names: &NameIndex,
    child_idx: usize,
    container_name: &str,
) -> Option<usize> {
    let bare = if names.named(container_name).is_empty() {
        last_name_segment(container_name)
    } else {
        container_name
    };
    let request = ContainerRequest { child_idx, container_name, bare };
    smallest_enclosing(symbols, names, std::slice::from_ref(&request))[0].or_else(|| {
        find_container_by_identity(symbols, &OwnerIndex::new(symbols), &request)
    })
}

/// The rest of the bind pass, after [`link_containers`]: reference,
/// dependency and referenced-by edges, and export targets.
///
/// The edges come from the occurrences alone, in first-occurrence order:
/// an owner's `references` are its `Scoped` targets with an in-file ID and
/// its occurrences bound to an import (`bound_imports`, aligned with
/// `index.occurrences`), `dependencies` only the latter, and a target's
/// `referenced_by` the owners of its `Scoped` occurrences. `NameOnly` and
/// `Unresolved` occurrences make no edge, and an occurrence without an owner
/// (file-level code, an initializer, an anonymous callable) makes none
/// either.
fn bind_references(index: &mut FileSymbolIndex, bound_imports: &[Option<usize>]) {
    let by_id: HashMap<&SymbolId, usize> = index
        .symbols
        .iter()
        .enumerate()
        .map(|(idx, symbol)| (&symbol.id, idx))
        .collect();
    let mut references_by_owner: Vec<RefSet> = vec![RefSet::default(); index.symbols.len()];
    let mut dependencies_by_owner: Vec<RefSet> = vec![RefSet::default(); index.symbols.len()];
    let mut referenced_by_target: Vec<RefSet> = vec![RefSet::default(); index.symbols.len()];

    for (occurrence, import) in index.occurrences.iter().zip(bound_imports) {
        if occurrence.confidence != Confidence::Scoped {
            continue;
        }
        let Some(&owner_idx) = occurrence
            .owner
            .as_ref()
            .and_then(|owner| owner.id.as_ref())
            .and_then(|id| by_id.get(id))
        else {
            continue;
        };
        if let Some(&target_idx) = occurrence.target.id.as_ref().and_then(|id| by_id.get(id)) {
            references_by_owner[owner_idx].push(symbol_ref(&index.symbols[target_idx]));
            referenced_by_target[target_idx].push(symbol_ref(&index.symbols[owner_idx]));
        } else if let Some(import) = import.and_then(|idx| index.imports.get(idx)) {
            let import_ref = SymbolRef {
                source_span: None,
                id: import.symbol.id.clone(),
                name: import.name.clone(),
                qualified_name: import.symbol.qualified_name.clone(),
            };
            references_by_owner[owner_idx].push(import_ref.clone());
            dependencies_by_owner[owner_idx].push(import_ref);
        }
    }

    let mut exported_ids = HashSet::new();
    let export_targets = (!index.exports.is_empty()).then(|| ExportTargets::new(&index.symbols));
    for export in &mut index.exports {
        if let Some(idx) = export_targets.as_ref().and_then(|targets| targets.find(export)) {
            let symbol = &index.symbols[idx];
            export.symbol.id = Some(symbol.id.clone());
            export.symbol.qualified_name = Some(symbol.identity.qualified_name.clone());
            exported_ids.insert(symbol.id.clone());
        }
    }

    for (((symbol, references), dependencies), referenced_by) in index
        .symbols
        .iter_mut()
        .zip(references_by_owner)
        .zip(dependencies_by_owner)
        .zip(referenced_by_target)
    {
        symbol.relations.references = references.refs;
        symbol.relations.dependencies = dependencies.refs;
        symbol.relations.referenced_by = referenced_by.refs;
        symbol.visibility.is_exported = exported_ids.contains(&symbol.id);
        symbol.provenance.parse_pass = AnalysisPass::Bind.as_str().to_string();
    }
}

/// The kinds the semantic pass evaluates.
pub(crate) fn has_semantics(kind: SymbolKindV2) -> bool {
    matches!(kind, SymbolKindV2::Function | SymbolKindV2::Method | SymbolKindV2::Constructor)
}

/// Semantic pass: fills initial low-risk semantic flags, `is_recursive`
/// from the index's occurrences and coverage.
fn semantic_pass(index: &mut FileSymbolIndex) {
    let facets = semantic_facets(&index.symbols, &index.occurrences, &index.reference_coverage, &index.imports);
    for (symbol, facet) in index.symbols.iter_mut().zip(facets) {
        if let Some(facet) = facet {
            symbol.semantics = facet;
            symbol.provenance.parse_pass = AnalysisPass::Semantic.as_str().to_string();
        }
    }
}

/// The semantic facet the semantic pass gives each of `symbols` (`None` for
/// a kind it does not evaluate), read only from the arguments, so the cache
/// reader can check a stored facet against the stored evidence.
pub(crate) fn semantic_facets(
    symbols: &[SymbolRecord],
    occurrences: &[Occurrence],
    coverage: &[ReferenceCoverage],
    imports: &[ImportRecord],
) -> Vec<Option<SemanticFacet>> {
    let mut calls_by_owner: HashMap<&SymbolId, Vec<&Occurrence>> = HashMap::new();
    for occurrence in occurrences {
        if occurrence.kind == OccurrenceKind::Call
            && let Some(owner) = occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref())
        {
            calls_by_owner.entry(owner).or_default().push(occurrence);
        }
    }
    let regions: HashMap<&SymbolId, &ReferenceCoverage> = coverage
        .iter()
        .filter_map(|region| Some((region.owner.as_ref()?.id.as_ref()?, region)))
        .collect();

    let imports: HashMap<_, _> = imports.iter().map(|import| ((import.span.start_byte, import.span.end_byte), import)).collect();
    symbols
        .iter()
        .map(|symbol| {
            has_semantics(symbol.kind).then(|| {
                let is_recursive = direct_recursion(
                    symbol,
                    regions.get(&symbol.id).copied(),
                    calls_by_owner.get(&symbol.id).map_or(&[][..], Vec::as_slice),
                    &imports,
                );
                let panics = symbol
                    .relations
                    .references
                    .iter()
                    .any(|reference| reference.name == "panic" || reference.name == "panic!");
                SemanticFacet {
                    is_recursive: Some(is_recursive),
                    may_panic: Some(if panics { TriState::Yes } else { TriState::Unknown }),
                    may_throw: Some(TriState::Unknown),
                    mutates_self: Some(TriState::Unknown),
                    mutates_arguments: Some(TriState::Unknown),
                    ..SemanticFacet::default()
                }
            })
        })
        .collect()
}

/// Whether `symbol` calls itself directly, from the calls its own region
/// owns (a nested callable's calls belong to that callable):
///
/// - `Yes`: a `Scoped` call whose target ID is `symbol`'s, whatever else is
///   uncertain;
/// - `Unknown`: no body region (a bodyless declaration), a `Limited`
///   region, or a call that might be to `symbol` (see
///   [`may_call_itself`]);
/// - `No`: a `Complete` region and none of that.
fn direct_recursion(
    symbol: &SymbolRecord,
    region: Option<&ReferenceCoverage>,
    calls: &[&Occurrence],
    imports: &HashMap<(u32, u32), &ImportRecord>,
) -> TriState {
    if calls.iter().any(|call| {
        call.confidence == Confidence::Scoped && call.target.id.as_ref() == Some(&symbol.id)
    }) {
        return TriState::Yes;
    }
    let complete = region.is_some_and(|region| region.state == CoverageState::Complete);
    if !complete || calls.iter().any(|call| may_call_itself(symbol, call, imports)) {
        TriState::Unknown
    } else {
        TriState::No
    }
}

/// Whether `call` (not a proven self-call) might still invoke `symbol`: a
/// `NameOnly` call whose candidates include it, an `Unresolved` call of its
/// name, a `Scoped` call through an import whose original name is its name
/// (`use crate::f as g;` inside `f`), which no file-local rule can tell
/// apart from another module's `f`, or, for a constructor, a call of its own
/// class (`new C()` inside `C`'s constructor), which targets the class.
fn may_call_itself(symbol: &SymbolRecord, call: &Occurrence, imports: &HashMap<(u32, u32), &ImportRecord>) -> bool {
    let name = name_identity::key(symbol.language, &symbol.identity.name);
    let class = (symbol.kind == SymbolKindV2::Constructor)
        .then(|| symbol.relations.parent.as_ref().and_then(|parent| parent.id.as_ref()))
        .flatten();
    let is_own = |id: Option<&SymbolId>| id.is_some_and(|id| *id == symbol.id || Some(id) == class);
    match call.confidence {
        Confidence::NameOnly => call.candidates.iter().any(|candidate| is_own(candidate.id.as_ref())),
        Confidence::Unresolved => name_identity::key(symbol.language, &call.target.name) == name,
        Confidence::Scoped => match &call.target.id {
            Some(id) => Some(id) == class,
            None => call.import_span.as_ref().and_then(|span| imports.get(&(span.start_byte, span.end_byte)))
                .is_some_and(|import| name_identity::original_key(symbol.language, import.symbol.qualified_name.as_deref().unwrap_or(&import.name)) == name),
        },
    }
}

/// Docs pass: parses the extractor's cleaned `raw_doc` into structured
/// `ParsedDocs`.
fn docs_pass(index: &mut FileSymbolIndex) {
    for symbol in &mut index.symbols {
        symbol.docs = parse_docs_facet(std::mem::take(&mut symbol.docs));
        symbol.provenance.parse_pass = AnalysisPass::Docs.as_str().to_string();
    }
}

pub(crate) fn symbol_ref(symbol: &SymbolRecord) -> SymbolRef {
    SymbolRef {
        source_span: None,
        id: Some(symbol.id.clone()),
        name: symbol.identity.name.clone(),
        qualified_name: Some(symbol.identity.qualified_name.clone()),
    }
}

/// An owner's references in first-seen order, without repeats: a resolved
/// reference repeats one with the same ID, an unresolved one (no ID) repeats
/// one with the same name and qualified name. Two missing IDs never make two
/// references equal.
#[derive(Debug, Clone, Default)]
struct RefSet {
    refs: Vec<SymbolRef>,
    seen: HashSet<RefKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RefKey {
    Resolved(SymbolId),
    Unresolved(String, Option<String>),
}

impl RefSet {
    fn push(&mut self, candidate: SymbolRef) {
        let key = match &candidate.id {
            Some(id) => RefKey::Resolved(id.clone()),
            None => RefKey::Unresolved(candidate.name.clone(), candidate.qualified_name.clone()),
        };
        if self.seen.insert(key) {
            self.refs.push(candidate);
        }
    }
}

/// One record whose container is named: the name as recorded and the name
/// its candidates are called (the last segment of a qualified container
/// name that no record is called).
struct ContainerRequest<'a> {
    child_idx: usize,
    container_name: &'a str,
    bare: &'a str,
}

/// The first rule of a child's container: the smallest record called its
/// `bare` container name whose declaration span encloses the child's (the
/// first in record order among equal spans), never the child itself.
///
/// Each name's records and children are swept once in span order, keeping
/// the records still open at the sweep position as a stack of nested spans,
/// so a child never scans the records of its name. Declaration spans come
/// from one syntax tree, so two of them are nested or disjoint.
fn smallest_enclosing(
    symbols: &[SymbolRecord],
    names: &NameIndex,
    children: &[ContainerRequest<'_>],
) -> Vec<Option<usize>> {
    let span = |idx: usize| {
        let span = &symbols[idx].source.declaration_span;
        (span.start_byte, span.end_byte)
    };
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (position, request) in children.iter().enumerate() {
        by_name.entry(request.bare).or_default().push(position);
    }

    let mut enclosing = vec![None; children.len()];
    let mut steps = 0;
    for (bare, mut positions) in by_name {
        let mut owners = names.named(bare).to_vec();
        // Outer spans first at one start; among equal spans the first record
        // ends on top of the stack.
        owners.sort_by_key(|&idx| {
            let (start, end) = span(idx);
            (start, Reverse(end), Reverse(idx))
        });
        positions.sort_by_key(|&position| span(children[position].child_idx).0);

        let mut open: Vec<usize> = Vec::new();
        let mut owners = owners.into_iter().peekable();
        for position in positions {
            let child_idx = children[position].child_idx;
            let (child_start, child_end) = span(child_idx);
            while let Some(owner) = owners.next_if(|&owner| span(owner).0 <= child_start) {
                steps += 1;
                while open.last().is_some_and(|&top| span(top).1 < span(owner).1) {
                    open.pop();
                    steps += 1;
                }
                open.push(owner);
            }
            while open.last().is_some_and(|&top| span(top).1 < child_start) {
                open.pop();
                steps += 1;
            }
            enclosing[position] = open
                .iter()
                .rev()
                .inspect(|_| steps += 1)
                .find(|&&owner| owner != child_idx && span(owner).1 >= child_end)
                .copied();
        }
    }
    count_shared_context_steps(steps + children.len());
    enclosing
}

/// A record's name, kind and one spelling of its owner identity (its in-file
/// qualified name, or a whole-segment tail of it).
type OwnerKey<'a> = (&'a str, SymbolKindV2, &'a str);

/// A file's records indexed by the identity a member declared outside its
/// type names its owner by, built once per file on the first such member.
struct OwnerIndex<'a> {
    symbols: &'a [SymbolRecord],
    /// Name, kind and in-file qualified name → `(start byte, index)`, in
    /// that order.
    by_qualified: HashMap<OwnerKey<'a>, Vec<(u32, usize)>>,
    /// Built on the first member no owner qualifies.
    tails: OnceCell<OwnerTails<'a>>,
}

struct OwnerTails<'a> {
    /// Name and kind → record indexes, in record order.
    by_kind: HashMap<(&'a str, SymbolKindV2), Vec<usize>>,
    /// Name, kind and every whole-segment tail of the qualified name
    /// ([`segment_suffixes`]) → record indexes, in record order.
    by_suffix: HashMap<OwnerKey<'a>, Vec<usize>>,
}

impl<'a> OwnerIndex<'a> {
    fn new(symbols: &'a [SymbolRecord]) -> Self {
        let mut by_qualified: HashMap<OwnerKey<'a>, Vec<(u32, usize)>> = HashMap::new();
        for (idx, symbol) in symbols.iter().enumerate() {
            let identity = &symbol.identity;
            by_qualified
                .entry((identity.name.as_str(), symbol.kind, identity.qualified_name.as_str()))
                .or_default()
                .push((symbol.source.declaration_span.start_byte, idx));
        }
        for bucket in by_qualified.values_mut() {
            bucket.sort_unstable();
        }
        count_shared_context_steps(symbols.len());
        Self { symbols, by_qualified, tails: OnceCell::new() }
    }

    fn tails(&self) -> &OwnerTails<'a> {
        self.tails.get_or_init(|| {
            let mut tails = OwnerTails { by_kind: HashMap::new(), by_suffix: HashMap::new() };
            let mut steps = 0;
            for (idx, symbol) in self.symbols.iter().enumerate() {
                let name = symbol.identity.name.as_str();
                tails.by_kind.entry((name, symbol.kind)).or_default().push(idx);
                for tail in segment_suffixes(&symbol.identity.qualified_name) {
                    tails.by_suffix.entry((name, symbol.kind, tail)).or_default().push(idx);
                    steps += 1;
                }
                steps += 1;
            }
            count_shared_context_steps(steps);
            tails
        })
    }
}

/// The remaining rules of a child's container, for a member declared
/// outside its type (a Go receiver method, a Lua table function), which is
/// matched by identity (Rust `impl` and Swift extension members arrive
/// linked from the relation resolver's owner decision, which no by-name
/// rule may replace): among the records of the recorded container kind,
/// the one whose in-file qualified name qualifies the child's (longest
/// first, see [`nearest_qualifying_owner`]), else the only one whose
/// qualified name agrees with the child's owner path
/// ([`agrees_with_owner_path`]). Same-named owners in different modules or
/// types are never confused: when several candidates remain and none
/// qualifies the child, there is no container, and a sole candidate never
/// overrides an explicit qualification that names another owner
/// (`impl other::W` beside a local `W` has no same-file container).
///
/// A qualified `container_name` (Lua `function Left.Repeated.f()`) that no
/// record is called is looked up by its last segment and, without a
/// qualifying match, must be the tail of the candidate's qualified name.
fn find_container_by_identity(
    symbols: &[SymbolRecord],
    owners: &OwnerIndex<'_>,
    request: &ContainerRequest<'_>,
) -> Option<usize> {
    let &ContainerRequest { child_idx, container_name, bare } = request;
    let child = &symbols[child_idx];
    let container_kind = child.identity.container_kind?;
    let child_name = child.identity.qualified_name.as_str();
    nearest_qualifying_owner(owners, bare, container_kind, child).or_else(|| {
        // No candidate qualifies the child, so a candidate agrees with its
        // owner path only by ending with it.
        count_shared_context_steps(1);
        let tails = owners.tails();
        let pool = match owner_path(child_name) {
            Some(path) => tails.by_suffix.get(&(bare, container_kind, path)),
            None if bare == container_name => tails.by_kind.get(&(bare, container_kind)),
            None => tails.by_suffix.get(&(bare, container_kind, container_name)),
        };
        let mut named = pool.map_or(&[][..], Vec::as_slice).iter().copied().filter(|&idx| {
            count_shared_context_steps(1);
            let owner = &symbols[idx].identity.qualified_name;
            idx != child_idx
                && (bare == container_name || ends_with_segments(owner, container_name))
                && agrees_with_owner_path(owner, child_name)
        });
        let only = named.next()?;
        named.next().is_none().then_some(only)
    })
}

/// Whether an owner called `owner` (in-file qualified name) is consistent
/// with the explicit qualification of a member called `member`: it qualifies
/// the member, or it ends with the member's owner path, or the member's name
/// records no owner path. `W` agrees with `W::probe`, `a::W` with `W::probe`
/// (a `use a::W; impl W`), and `right::W` with `right::W::m::self`; `W` never
/// agrees with `other::W::probe`.
pub(crate) fn agrees_with_owner_path(owner: &str, member: &str) -> bool {
    qualifies(owner, member) || owner_path(member).is_none_or(|path| ends_with_segments(owner, path))
}

/// The owner path a qualified name records: the name minus its last segment
/// (`other::W` for `other::W::probe`, `M` for Lua `M:m`), `None` when the
/// name is a single segment.
pub(crate) fn owner_path(qualified_name: &str) -> Option<&str> {
    let last = qualified_name.rfind(['.', ':', '\\'])?;
    let head = &qualified_name[..last];
    let head = if qualified_name[last..].starts_with(':') {
        head.strip_suffix(':').unwrap_or(head)
    } else {
        head
    };
    Some(head).filter(|path| !path.is_empty())
}

/// Of the records called `bare` of kind `kind`, the owner whose qualified
/// name qualifies the child's: the longest such name, and among equal names
/// the nearest declaration before the child, else the first after it (the
/// last in record order among equal starts), where equal names may be one
/// owner declared again ([`repeats_one_owner`]). Elsewhere equal names are
/// competing owners (two Go types of one name): proximity proves none of
/// them, and no shorter name is tried.
fn nearest_qualifying_owner(
    owners: &OwnerIndex<'_>,
    bare: &str,
    kind: SymbolKindV2,
    child: &SymbolRecord,
) -> Option<usize> {
    let child_start = child.source.declaration_span.start_byte;
    let bucket = qualifying_prefixes(&child.identity.qualified_name).find_map(|prefix| {
        count_shared_context_steps(1);
        owners.by_qualified.get(&(bare, kind, prefix))
    })?;
    if bucket.len() > 1 && !repeats_one_owner(child.language) {
        return None;
    }
    let before = bucket.partition_point(|&(start, _)| start <= child_start);
    let nearest = if before > 0 {
        before - 1
    } else {
        let first_start = bucket.first()?.0;
        bucket.partition_point(|&(start, _)| start <= first_start) - 1
    };
    Some(bucket[nearest].1)
}

/// Whether same-named owners of one qualified name may be one owner
/// declared again, so the nearest one before a member is its owner: a Lua
/// `local M = {}` in force until the next, a C++ class's forward
/// declarations and definition.
pub(crate) fn repeats_one_owner(language: ProgrammingLanguage) -> bool {
    matches!(language, ProgrammingLanguage::Lua | ProgrammingLanguage::Cpp)
}

/// Every whole-segment prefix of `member` that [`qualifies`] it, longest
/// first.
pub(crate) fn qualifying_prefixes(member: &str) -> impl Iterator<Item = &str> {
    (0..member.len())
        .rev()
        .filter(move |&end| member.is_char_boundary(end) && member[end..].starts_with(['.', ':', '\\']))
        .map(move |end| &member[..end])
        .filter(move |prefix| qualifies(prefix, member))
}

/// Every tail of `name` that [`ends_with_segments`] accepts, `name` itself
/// included.
pub(crate) fn segment_suffixes(name: &str) -> impl Iterator<Item = &str> {
    (0..=name.len())
        .filter(move |&start| {
            start == 0 || (name.is_char_boundary(start) && name[..start].ends_with(['.', ':', '\\']))
        })
        .map(move |start| &name[start..])
        .filter(move |tail| ends_with_segments(name, tail))
}

/// Separators between qualified-name segments.
const NAME_SEPARATORS: [&str; 4] = ["::", ".", "\\", ":"];

/// The last segment of a possibly qualified name (`Left.Repeated` →
/// `Repeated`).
fn last_name_segment(name: &str) -> &str {
    name.rsplit(['.', ':', '\\'])
        .next()
        .unwrap_or(name)
}

/// `owner` is a whole-segment prefix of `child` (`a::B` qualifies
/// `a::B::f`, not `a::Bc::f`).
fn qualifies(owner: &str, child: &str) -> bool {
    child.strip_prefix(owner).is_some_and(|rest| {
        NAME_SEPARATORS.iter().any(|separator| {
            rest.strip_prefix(separator)
                .is_some_and(|name| !name.is_empty() && !name.starts_with(':'))
        })
    })
}

/// `tail` is a whole-segment suffix of `name` (`Left.Repeated` ends
/// `Outer.Left.Repeated`).
fn ends_with_segments(name: &str, tail: &str) -> bool {
    name == tail
        || name.strip_suffix(tail).is_some_and(|head| {
            NAME_SEPARATORS.iter().any(|separator| {
                head.strip_suffix(separator)
                    .is_some_and(|qualifier| !qualifier.is_empty() && !qualifier.ends_with(':'))
            })
        })
}

/// Records by name and name span, built once per file with exports, so an
/// export finds its target without scanning every record of its name.
struct ExportTargets<'a> {
    /// The first record, in record order, with this name and name span.
    by_name_span: HashMap<(&'a str, u32, u32), usize>,
}

impl<'a> ExportTargets<'a> {
    fn new(symbols: &'a [SymbolRecord]) -> Self {
        let mut by_name_span = HashMap::new();
        for (idx, symbol) in symbols.iter().enumerate() {
            if let Some(span) = &symbol.source.name_span {
                by_name_span
                    .entry((symbol.identity.name.as_str(), span.start_byte, span.end_byte))
                    .or_insert(idx);
            }
        }
        count_shared_context_steps(symbols.len());
        Self { by_name_span }
    }

    /// The record an export names: the first one with the export's name and
    /// span.
    fn find(&self, export: &ExportRecord) -> Option<usize> {
        count_shared_context_steps(1);
        self.by_name_span
            .get(&(export.name.as_str(), export.span.start_byte, export.span.end_byte))
            .copied()
    }
}

fn parse_docs_facet(mut docs: DocsFacet) -> DocsFacet {
    let Some(raw) = docs.raw_doc.clone() else {
        return docs;
    };

    let mut parsed = ParsedDocs::default();
    let mut summary_lines = Vec::new();

    // `raw_doc` is already cleaned by the extractor, which removed exactly
    // the delimiters and decoration its language's doc tool removes; a `*`
    // or `//` still at the start of a line is documentation text.
    for line in raw.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("@param") {
            let value = rest.trim();
            if let Some((name, description)) = value.split_once(' ') {
                parsed.params.push(crate::shared::DocParam {
                    name: name.trim().to_string(),
                    description: description.trim().to_string(),
                });
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@returns") {
            parsed.returns = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('@')
            && let Some((name, value)) = rest.split_once(' ')
        {
            parsed.tags.push(crate::shared::DocTag {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            });
            continue;
        }

        if !trimmed.is_empty() {
            summary_lines.push(trimmed.to_string());
        }
    }

    if !summary_lines.is_empty() {
        parsed.summary = Some(summary_lines.join(" "));
    }

    if !parsed.params.is_empty() || parsed.returns.is_some() || parsed.summary.is_some() {
        docs.parsed = Some(parsed);
    }

    if docs.comments.is_empty() {
        // Fallback comment attachment when parse pass only populated raw_doc.
        if let Some(raw_doc) = docs.raw_doc.clone() {
            docs.comments.push(crate::shared::AttachedComment {
                kind: crate::shared::CommentKind::Doc,
                attachment: CommentAttachment::Leading,
                span: crate::shared::TextSpan {
                    start: crate::shared::TextPoint { line: 0, column: 0 },
                    end: crate::shared::TextPoint { line: 0, column: 0 },
                    start_byte: 0,
                    end_byte: 0,
                },
                raw_text: raw_doc.clone(),
                cleaned_text: raw_doc.trim().to_string(),
                line_distance: None,
            });
        }
    }

    docs
}

#[cfg(test)]
mod tests;
