//! The staged per-file analysis behind [`TreeFile::query`] and
//! [`TreeFile::symbol_index_with_passes`]: one plan, built from the
//! requested capabilities, runs each needed pass once, after its
//! prerequisites.

use std::collections::{HashMap, HashSet};

use crate::error::TreeHuggerError;
use crate::file::query::{Capability, SymbolQuery, SymbolQueryResult};
use crate::file::tree_file::{MetadataSelection, RecordFacts, TreeFile};
use crate::shared::{
    AnalysisPass, CommentAttachment, DocsFacet, ExportRecord, FileSymbolIndex, ImportRecord,
    ImportSymbol, ParsedDocs, ReferencedSymbol, SemanticFacet, SymbolId, SymbolKindV2,
    SymbolRecord, SymbolRef, TriState,
};

pub mod counters;
pub(crate) mod coverage;

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
    /// The full bind pass (containment, references, export targets).
    bind: bool,
    semantic: bool,
    docs: bool,
    diagnostics: bool,
}

impl PassPlan {
    fn for_query(query: &SymbolQuery) -> Self {
        Self {
            containment: query.requires(Capability::Containment),
            bind: query.requires(Capability::References),
            semantic: query.requires(Capability::Semantics),
            docs: query.requires(Capability::StructuredDocs),
            diagnostics: query.requires(Capability::Diagnostics),
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
/// result feeds the records, the references and the diagnostics. The result
/// is unfinished (in-file qualified names, no predicate applied), as the
/// cache stores it.
pub(crate) fn run_query(
    tree_file: &TreeFile,
    query: &SymbolQuery,
) -> Result<SymbolQueryResult, TreeHuggerError> {
    let plan = PassPlan::for_query(query);
    let selection = MetadataSelection::for_kinds(query.selected_kinds());

    let definitions = tree_file.definitions(&selection)?;
    let imports = tree_file.imported_symbols()?;
    let references = if plan.bind || plan.diagnostics {
        Some(tree_file.references_from(&definitions, &imports)?)
    } else {
        None
    };
    let diagnostics = match &references {
        Some(references) if plan.diagnostics => {
            tree_file.diagnostics_from(&definitions, &imports, references)?
        }
        _ => Vec::new(),
    };
    let exports = tree_file
        .exports_from(&definitions)
        .map(export_record)
        .collect();
    let facts = RecordFacts {
        flags: query.requires(Capability::Flags),
        metrics: query.requires(Capability::Metrics),
    };
    let symbols = tree_file.records_from(definitions, &selection, facts);

    let mut index = FileSymbolIndex {
        schema_version: crate::shared::SchemaVersion::V4_0,
        file: tree_file.file.clone(),
        language: tree_file.language,
        file_hash: tree_file.hash.clone(),
        completed_passes: plan.passes(),
        symbols,
        imports: imports.into_iter().map(import_record).collect(),
        exports,
        diagnostics,
    };

    let names = NameIndex::new(&index.symbols);
    if plan.containment {
        link_containers(&mut index.symbols, &names);
    }
    if plan.bind {
        bind_references(&mut index, &names, references.as_deref().unwrap_or_default());
    }

    // Unselected records were needed only as owners, containers and
    // reference targets.
    index.symbols.retain(|symbol| query.selects(symbol.kind));

    if plan.semantic {
        semantic_pass(&mut index);
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

fn import_record(import: ImportSymbol) -> ImportRecord {
    ImportRecord {
        name: import.name.clone(),
        alias: import.alias,
        source: import.source,
        span: text_span(&import.range),
        statement_span: import.statement_range.as_ref().map(text_span),
        symbol: SymbolRef {
            id: None,
            name: import.name,
            qualified_name: import.original_name,
        },
    }
}

fn export_record(export: &crate::shared::SymbolInfo) -> ExportRecord {
    ExportRecord {
        name: export.name.clone(),
        span: text_span(&export.range),
        symbol: SymbolRef {
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
fn link_containers(symbols: &mut [SymbolRecord], names: &NameIndex) {
    let mut parent_links: Vec<Option<SymbolRef>> = vec![None; symbols.len()];
    let mut members_by_parent: HashMap<SymbolId, Vec<SymbolRef>> = HashMap::new();

    for (child_idx, symbol) in symbols.iter().enumerate() {
        let Some(container_name) = symbol.identity.container_name.as_deref() else {
            continue;
        };

        let Some(parent_idx) = find_container_symbol(symbols, names, child_idx, container_name)
        else {
            continue;
        };

        parent_links[child_idx] = Some(symbol_ref(&symbols[parent_idx]));
        members_by_parent
            .entry(symbols[parent_idx].id.clone())
            .or_default()
            .push(symbol_ref(symbol));
    }

    for (symbol, parent) in symbols.iter_mut().zip(parent_links) {
        symbol.relations.container = parent.clone();
        symbol.relations.parent = parent;
        symbol.relations.members = members_by_parent.remove(&symbol.id).unwrap_or_default();
    }
}

/// The rest of the bind pass, after [`link_containers`]: reference,
/// dependency and referenced-by edges by name, and export targets.
fn bind_references(index: &mut FileSymbolIndex, names: &NameIndex, references: &[ReferencedSymbol]) {
    let import_refs_by_name: HashMap<_, _> = index
        .imports
        .iter()
        .map(|import| {
            (
                import.name.clone(),
                SymbolRef {
                    id: import.symbol.id.clone(),
                    name: import.name.clone(),
                    qualified_name: import.symbol.qualified_name.clone(),
                },
            )
        })
        .collect();

    let mut references_by_owner: Vec<RefSet> = vec![RefSet::default(); index.symbols.len()];
    let mut dependencies_by_owner: Vec<RefSet> = vec![RefSet::default(); index.symbols.len()];
    let mut referenced_by_target: HashMap<SymbolId, RefSet> = HashMap::new();
    let owner_index = OwnerSymbolIndex::new(&index.symbols);

    for reference in references {
        let Some(owner_idx) = owner_index.find(
            reference.range.start_byte as u32,
            reference.range.end_byte as u32,
        ) else {
            continue;
        };

        let owner_ref = symbol_ref(&index.symbols[owner_idx]);

        if let Some(&target_idx) = names.named(&reference.name).first() {
            let target = &index.symbols[target_idx];
            references_by_owner[owner_idx].push(symbol_ref(target));
            referenced_by_target
                .entry(target.id.clone())
                .or_default()
                .push(owner_ref);
            continue;
        }

        if let Some(import_ref) = import_refs_by_name.get(&reference.name) {
            references_by_owner[owner_idx].push(import_ref.clone());
            dependencies_by_owner[owner_idx].push(import_ref.clone());
        }
    }

    let mut exported_ids = HashSet::new();
    for export in &mut index.exports {
        if let Some(idx) = find_export_target(&index.symbols, names, export) {
            let symbol = &index.symbols[idx];
            export.symbol.id = Some(symbol.id.clone());
            export.symbol.qualified_name = Some(symbol.identity.qualified_name.clone());
            exported_ids.insert(symbol.id.clone());
        }
    }

    for ((symbol, references), dependencies) in index
        .symbols
        .iter_mut()
        .zip(references_by_owner)
        .zip(dependencies_by_owner)
    {
        symbol.relations.references = references.refs;
        symbol.relations.dependencies = dependencies.refs;
        symbol.relations.referenced_by = referenced_by_target
            .remove(&symbol.id)
            .map(|set| set.refs)
            .unwrap_or_default();
        symbol.visibility.is_exported = exported_ids.contains(&symbol.id);
        symbol.provenance.parse_pass = AnalysisPass::Bind.as_str().to_string();
    }
}

/// Semantic pass: fills initial low-risk semantic flags.
fn semantic_pass(index: &mut FileSymbolIndex) {
    for symbol in &mut index.symbols {
        if !matches!(symbol.kind, SymbolKindV2::Function | SymbolKindV2::Method) {
            continue;
        }

        let panics = symbol
            .relations
            .references
            .iter()
            .any(|reference| reference.name == "panic" || reference.name == "panic!");

        symbol.semantics = SemanticFacet {
            // Name-only binding can neither prove that a same-named call
            // targets this function nor that none does.
            is_recursive: Some(TriState::Unknown),
            may_panic: Some(if panics {
                TriState::Yes
            } else {
                TriState::Unknown
            }),
            may_throw: Some(TriState::Unknown),
            mutates_self: Some(TriState::Unknown),
            mutates_arguments: Some(TriState::Unknown),
            ..SemanticFacet::default()
        };
        symbol.provenance.parse_pass = AnalysisPass::Semantic.as_str().to_string();
    }
}

/// Docs pass: parses the extractor's cleaned `raw_doc` into structured
/// `ParsedDocs`.
fn docs_pass(index: &mut FileSymbolIndex) {
    for symbol in &mut index.symbols {
        symbol.docs = parse_docs_facet(symbol.docs.clone());
        symbol.provenance.parse_pass = AnalysisPass::Docs.as_str().to_string();
    }
}

fn symbol_ref(symbol: &SymbolRecord) -> SymbolRef {
    SymbolRef {
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

#[derive(Debug, Clone, Copy)]
struct OwnerSymbolEntry {
    idx: usize,
    start_byte: u32,
    end_byte: u32,
}

impl OwnerSymbolEntry {
    fn contains(self, start_byte: u32, end_byte: u32) -> bool {
        self.start_byte <= start_byte && self.end_byte >= end_byte
    }

    fn len(self) -> u32 {
        self.end_byte.saturating_sub(self.start_byte)
    }
}

#[derive(Debug)]
struct OwnerSymbolIndex {
    root: Option<Box<OwnerSymbolNode>>,
}

impl OwnerSymbolIndex {
    fn new(symbols: &[SymbolRecord]) -> Self {
        let entries = symbols
            .iter()
            .enumerate()
            .map(|(idx, symbol)| {
                let span = &symbol.source.declaration_span;
                OwnerSymbolEntry {
                    idx,
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                }
            })
            .collect();

        Self {
            root: OwnerSymbolNode::build(entries),
        }
    }

    fn find(&self, start_byte: u32, end_byte: u32) -> Option<usize> {
        self.root
            .as_ref()
            .and_then(|root| root.find(start_byte, end_byte).map(|entry| entry.idx))
    }
}

#[derive(Debug)]
struct OwnerSymbolNode {
    center: u32,
    spanning: Vec<OwnerSymbolEntry>,
    left: Option<Box<OwnerSymbolNode>>,
    right: Option<Box<OwnerSymbolNode>>,
}

impl OwnerSymbolNode {
    fn build(mut entries: Vec<OwnerSymbolEntry>) -> Option<Box<Self>> {
        if entries.is_empty() {
            return None;
        }

        entries.sort_by_key(|entry| (entry.start_byte, entry.end_byte, entry.idx));
        let center = entries[entries.len() / 2].start_byte;

        let mut left = Vec::new();
        let mut spanning = Vec::new();
        let mut right = Vec::new();

        for entry in entries {
            if entry.end_byte < center {
                left.push(entry);
            } else if entry.start_byte > center {
                right.push(entry);
            } else {
                spanning.push(entry);
            }
        }

        spanning.sort_by_key(|entry| (entry.len(), entry.start_byte, entry.end_byte, entry.idx));

        Some(Box::new(Self {
            center,
            spanning,
            left: Self::build(left),
            right: Self::build(right),
        }))
    }

    fn find(&self, start_byte: u32, end_byte: u32) -> Option<OwnerSymbolEntry> {
        let mut best = self
            .spanning
            .iter()
            .copied()
            .find(|entry| entry.contains(start_byte, end_byte));

        let child = if end_byte < self.center {
            self.left.as_ref()
        } else if start_byte > self.center {
            self.right.as_ref()
        } else {
            None
        };

        if let Some(candidate) = child.and_then(|node| node.find(start_byte, end_byte))
            && best.is_none_or(|current| {
                (
                    candidate.len(),
                    candidate.start_byte,
                    candidate.end_byte,
                    candidate.idx,
                ) < (
                    current.len(),
                    current.start_byte,
                    current.end_byte,
                    current.idx,
                )
            })
        {
            best = Some(candidate);
        }

        best
    }
}

/// Finds the record a child's container names.
///
/// The container is the smallest declaration called `container_name` that
/// encloses the child (the first of equal size). A member declared outside
/// its type (a Rust `impl` member, a Swift extension member) is matched by
/// identity instead: among the records of the recorded container kind, the
/// one whose in-file qualified name qualifies the child's (longest first),
/// else the only one whose qualified name ends with the child's owner path
/// (the child's qualified name minus its last segment). Same-named owners in
/// different modules or types are never confused: when several candidates
/// remain and none qualifies the child, there is no container, and a sole
/// candidate never overrides an explicit qualification that names another
/// owner (`impl other::W` beside a local `W` has no same-file container).
///
/// A qualified `container_name` (Swift `extension Left.Repeated`) that no
/// record is called is looked up by its last segment and, without a
/// qualifying match, must be the tail of the candidate's qualified name.
fn find_container_symbol(
    symbols: &[SymbolRecord],
    names: &NameIndex,
    child_idx: usize,
    container_name: &str,
) -> Option<usize> {
    let child = &symbols[child_idx];
    // A record may itself carry a dotted name (C# `namespace A.B`).
    let bare = if names.named(container_name).is_empty() {
        last_name_segment(container_name)
    } else {
        container_name
    };
    let candidates = names
        .named(bare)
        .iter()
        .copied()
        .filter(|&idx| idx != child_idx);
    let enclosing = candidates
        .clone()
        .filter(|&idx| {
            let span = &symbols[idx].source.declaration_span;
            span.start_byte <= child.source.declaration_span.start_byte
                && span.end_byte >= child.source.declaration_span.end_byte
        })
        .min_by_key(|&idx| {
            let span = &symbols[idx].source.declaration_span;
            span.end_byte.saturating_sub(span.start_byte)
        });

    enclosing.or_else(|| {
        let container_kind = child.identity.container_kind?;
        let of_kind: Vec<usize> = candidates
            .filter(|&idx| symbols[idx].kind == container_kind)
            .collect();
        nearest_qualifying_owner(symbols, child, &of_kind).or_else(|| {
            let named: Vec<usize> = of_kind
                .into_iter()
                .filter(|&idx| {
                    let owner = &symbols[idx].identity.qualified_name;
                    (bare == container_name || ends_with_segments(owner, container_name))
                        && agrees_with_owner_path(owner, &child.identity.qualified_name)
                })
                .collect();
            (named.len() == 1).then(|| named[0])
        })
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
fn owner_path(qualified_name: &str) -> Option<&str> {
    let last = qualified_name.rfind(['.', ':', '\\'])?;
    let head = &qualified_name[..last];
    let head = if qualified_name[last..].starts_with(':') {
        head.strip_suffix(':').unwrap_or(head)
    } else {
        head
    };
    Some(head).filter(|path| !path.is_empty())
}

/// Of `candidates`, the owner whose qualified name qualifies the child's:
/// the longest such name, and among equal names (two Lua `local M = {}`
/// tables in different blocks) the nearest declaration before the child,
/// else the first after it.
fn nearest_qualifying_owner(
    symbols: &[SymbolRecord],
    child: &SymbolRecord,
    candidates: &[usize],
) -> Option<usize> {
    let child_start = i64::from(child.source.declaration_span.start_byte);
    candidates
        .iter()
        .copied()
        .filter(|&idx| qualifies(&symbols[idx].identity.qualified_name, &child.identity.qualified_name))
        .max_by_key(|&idx| {
            let start = i64::from(symbols[idx].source.declaration_span.start_byte);
            let nearness = if start <= child_start { start } else { -start - 1 };
            (symbols[idx].identity.qualified_name.len(), start <= child_start, nearness)
        })
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

fn find_export_target(
    symbols: &[SymbolRecord],
    names: &NameIndex,
    export: &ExportRecord,
) -> Option<usize> {
    names.named(&export.name).iter().copied().find(|&idx| {
        symbols[idx].source.name_span.as_ref().is_some_and(|span| {
            span.start_byte == export.span.start_byte && span.end_byte == export.span.end_byte
        })
    })
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
