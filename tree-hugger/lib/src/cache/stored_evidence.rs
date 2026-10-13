//! The cache entry's form of a file's reference evidence, and the checks a
//! read entry's evidence must pass before it is served.
//!
//! The public `Occurrence` and `ReferenceCoverage` omit empty lists and
//! absent owners and IDs when serialized, and read a missing key as empty.
//! The entry writes every key instead (`null`, `[]`), and the reader requires
//! each one, so an absent key, `null` and an empty list stay distinct and a
//! damaged entry is a miss, never evidence read as empty.
//!
//! Each stored occurrence also holds `path_start_byte`, a cache-only copy of
//! where its written path (`target.qualified_name`) starts in the source, or
//! `null` exactly when it has none. Nothing derived from the source can tell
//! a `null` path that was never written from one that was deleted (Rust macro
//! token trees, a leading `::` and JS spread all put punctuation before
//! unqualified names), so this redundant fact is what makes a single edit of
//! either field detectable.
//!
//! The selected `import_span` similarly repeats its start as
//! `import_start_byte`: null exactly for non-import bindings. The reader
//! verifies the span against imports and reconstructs dependency summaries
//! from these particular selections, without repeating lexical analysis.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::analysis::occurrences::{occurrence_order, whole_file_span};
use crate::analysis::scope::LanguageRules;
use crate::analysis::{has_semantics, semantic_facets};
use crate::file::query::Capability;
use crate::shared::{
    Confidence, CoverageState, ImportRecord, LimitReason, Occurrence, OccurrenceKind, ProgrammingLanguage,
    ReferenceCoverage, SemanticFacet, SymbolId, SymbolKindV2, SymbolRecord, SymbolRef, TextPoint,
    TextSpan,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredOccurrence {
    #[serde(deserialize_with = "nullable")]
    owner: Option<StoredRef>,
    owner_span: StoredSpan,
    target: StoredRef,
    /// Where `target.qualified_name` starts; `null` iff it is `null`.
    #[serde(deserialize_with = "nullable")]
    path_start_byte: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    import_span: Option<StoredSpan>,
    #[serde(deserialize_with = "nullable")]
    import_start_byte: Option<u32>,
    candidates: Vec<StoredRef>,
    span: StoredSpan,
    kind: OccurrenceKind,
    confidence: Confidence,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCoverage {
    #[serde(deserialize_with = "nullable")]
    owner: Option<StoredRef>,
    span: StoredSpan,
    state: CoverageState,
    reasons: Vec<LimitReason>,
}

/// An occurrence's `SymbolRef`, which never carries a `source_span`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRef {
    #[serde(deserialize_with = "nullable")]
    id: Option<SymbolId>,
    name: String,
    #[serde(deserialize_with = "nullable")]
    qualified_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSpan {
    start: StoredPoint,
    end: StoredPoint,
    start_byte: u32,
    end_byte: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPoint {
    line: u32,
    column: u32,
}

/// Import facts use the same strict geometry reader as occurrences. Nullable
/// metadata accepts genuinely absent/null controls; producer and source
/// inventory checks reject loss of meaningful facts before any capability is served.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredImport {
    name: String,
    alias: Option<String>,
    source: Option<String>,
    span: StoredSpan,
    statement_span: Option<StoredSpan>,
    symbol: StoredImportRef,
    reexported: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredImportRef {
    id: Option<SymbolId>,
    name: String,
    qualified_name: Option<String>,
    source_span: Option<StoredSpan>,
}

/// The written form of a [`StoredImport`], borrowed from its record. Its
/// fields and their order are the stored form's, so the entry's bytes and
/// the import inventory's digest do not depend on which one serialized it.
#[derive(Serialize)]
struct StoredImportView<'a> {
    name: &'a str,
    alias: Option<&'a str>,
    source: Option<&'a str>,
    span: StoredSpan,
    statement_span: Option<StoredSpan>,
    symbol: StoredImportRefView<'a>,
    reexported: bool,
}

#[derive(Serialize)]
struct StoredImportRefView<'a> {
    id: Option<&'a SymbolId>,
    name: &'a str,
    qualified_name: Option<&'a str>,
    source_span: Option<StoredSpan>,
}

pub(super) fn write_imports<S: Serializer>(imports: &&[ImportRecord], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(imports.iter().map(StoredImportView::from))
}

/// A compact witness of the complete grammar-produced import inventory. All
/// nullable metadata and geometry are included, before query projection.
/// This detects edits to stored facts without lexing source a second time;
/// it is a consistency witness, not authentication of an untrusted writer.
pub(super) fn import_inventory(imports: &[ImportRecord]) -> String {
    let canonical: Vec<_> = imports.iter().map(StoredImportView::from).collect();
    biscuit_hash::blake3_hash(&serde_json::to_string(&canonical).expect("import records serialize"))
}

pub(super) fn read_imports<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<ImportRecord>, D::Error> {
    Ok(Vec::<StoredImport>::deserialize(deserializer)?.into_iter().map(ImportRecord::from).collect())
}

impl<'a> From<&'a ImportRecord> for StoredImportView<'a> {
    fn from(import: &'a ImportRecord) -> Self {
        Self {
            name: &import.name, alias: import.alias.as_deref(), source: import.source.as_deref(),
            span: StoredSpan::from(&import.span),
            statement_span: import.statement_span.as_ref().map(StoredSpan::from),
            symbol: StoredImportRefView {
                id: import.symbol.id.as_ref(), name: &import.symbol.name,
                qualified_name: import.symbol.qualified_name.as_deref(),
                source_span: import.symbol.source_span.as_ref().map(StoredSpan::from),
            },
            reexported: import.reexported,
        }
    }
}

impl From<StoredImport> for ImportRecord {
    fn from(import: StoredImport) -> Self {
        Self {
            name: import.name, alias: import.alias, source: import.source,
            span: TextSpan::from(import.span), statement_span: import.statement_span.map(TextSpan::from),
            symbol: SymbolRef {
                id: import.symbol.id, name: import.symbol.name, qualified_name: import.symbol.qualified_name,
                source_span: import.symbol.source_span.map(TextSpan::from),
            },
            reexported: import.reexported,
        }
    }
}

/// A value or `null`; unlike a plain `Option` field, an absent key is an
/// error rather than `None`.
pub(super) fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

pub(super) fn write_occurrences<S: Serializer>(
    occurrences: &&[Occurrence],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(occurrences.iter().map(StoredOccurrence::from))
}

/// A read entry's occurrences and, aligned with them, each one's stored
/// `path_start_byte`, which [`validate_evidence`] compares with its path.
pub(super) struct ReadOccurrences {
    pub(super) occurrences: Vec<Occurrence>,
    pub(super) path_starts: Vec<Option<u32>>,
    pub(super) import_starts: Vec<Option<u32>>,
}

pub(super) fn read_occurrences<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ReadOccurrences, D::Error> {
    let stored = Vec::<StoredOccurrence>::deserialize(deserializer)?;
    let path_starts = stored.iter().map(|occurrence| occurrence.path_start_byte).collect();
    let import_starts = stored.iter().map(|occurrence| occurrence.import_start_byte).collect();
    Ok(ReadOccurrences { occurrences: stored.into_iter().map(Occurrence::from).collect(), path_starts, import_starts })
}

pub(super) fn write_coverage<S: Serializer>(
    coverage: &&[ReferenceCoverage],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(coverage.iter().map(StoredCoverage::from))
}

pub(super) fn read_coverage<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ReferenceCoverage>, D::Error> {
    let stored = Vec::<StoredCoverage>::deserialize(deserializer)?;
    Ok(stored.into_iter().map(ReferenceCoverage::from).collect())
}

impl From<&Occurrence> for StoredOccurrence {
    fn from(occurrence: &Occurrence) -> Self {
        Self {
            owner: occurrence.owner.as_ref().map(StoredRef::from),
            owner_span: StoredSpan::from(&occurrence.owner_span),
            target: StoredRef::from(&occurrence.target),
            // A written path is the source text ending at the name, so it
            // starts its length before the occurrence's end.
            path_start_byte: occurrence
                .target
                .qualified_name
                .as_ref()
                .map(|path| occurrence.span.end_byte.saturating_sub(u32::try_from(path.len()).unwrap_or(u32::MAX))),
            import_span: occurrence.import_span.as_ref().map(StoredSpan::from),
            import_start_byte: occurrence.import_span.as_ref().map(|span| span.start_byte),
            candidates: occurrence.candidates.iter().map(StoredRef::from).collect(),
            span: StoredSpan::from(&occurrence.span),
            kind: occurrence.kind,
            confidence: occurrence.confidence,
        }
    }
}

impl From<StoredOccurrence> for Occurrence {
    fn from(stored: StoredOccurrence) -> Self {
        Self {
            owner: stored.owner.map(SymbolRef::from),
            owner_span: TextSpan::from(stored.owner_span),
            target: SymbolRef::from(stored.target),
            import_span: stored.import_span.map(TextSpan::from),
            candidates: stored.candidates.into_iter().map(SymbolRef::from).collect(),
            span: TextSpan::from(stored.span),
            kind: stored.kind,
            confidence: stored.confidence,
        }
    }
}

impl From<&ReferenceCoverage> for StoredCoverage {
    fn from(coverage: &ReferenceCoverage) -> Self {
        Self {
            owner: coverage.owner.as_ref().map(StoredRef::from),
            span: StoredSpan::from(&coverage.span),
            state: coverage.state,
            reasons: coverage.reasons.clone(),
        }
    }
}

impl From<StoredCoverage> for ReferenceCoverage {
    fn from(stored: StoredCoverage) -> Self {
        Self {
            owner: stored.owner.map(SymbolRef::from),
            span: TextSpan::from(stored.span),
            state: stored.state,
            reasons: stored.reasons,
        }
    }
}

impl From<&SymbolRef> for StoredRef {
    fn from(symbol: &SymbolRef) -> Self {
        debug_assert!(symbol.source_span.is_none(), "occurrence references carry no source span");
        Self {
            id: symbol.id.clone(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
        }
    }
}

impl From<StoredRef> for SymbolRef {
    fn from(stored: StoredRef) -> Self {
        Self {
            id: stored.id,
            name: stored.name,
            qualified_name: stored.qualified_name,
            source_span: None,
        }
    }
}

impl From<&TextSpan> for StoredSpan {
    fn from(span: &TextSpan) -> Self {
        Self {
            start: StoredPoint { line: span.start.line, column: span.start.column },
            end: StoredPoint { line: span.end.line, column: span.end.column },
            start_byte: span.start_byte,
            end_byte: span.end_byte,
        }
    }
}

impl From<StoredSpan> for TextSpan {
    fn from(stored: StoredSpan) -> Self {
        Self {
            start: TextPoint { line: stored.start.line, column: stored.start.column },
            end: TextPoint { line: stored.end.line, column: stored.end.column },
            start_byte: stored.start_byte,
            end_byte: stored.end_byte,
        }
    }
}

/// All import producers use external, source-less symbol references and
/// unique name captures in source order. These invariants do not depend on
/// retaining occurrences or the kind of any selected declaration.
pub(super) fn validate_imports(
    lines: &LineStarts<'_>,
    language: ProgrammingLanguage,
    imports: &[ImportRecord],
    inventory: &str,
) -> Result<(), &'static str> {
    let source = lines.source;
    use ProgrammingLanguage as L;
    if matches!(language, L::C | L::Cpp | L::Lua | L::Perl | L::Bash | L::Zsh) && !imports.is_empty() {
        return Err("imports in a language whose import query has no captures");
    }
    if inventory != import_inventory(imports) {
        return Err("imports disagree with their complete producer inventory");
    }
    let mut previous = None;
    for import in imports {
        let position = (import.span.start_byte, import.span.end_byte);
        if previous.is_some_and(|last| last >= position) {
            return Err("imports are duplicated or out of capture order");
        }
        previous = Some(position);
        if !lines.is_region(&import.span) || import.span.start_byte == import.span.end_byte {
            return Err("an import name span disagrees with the source");
        }
        let written = &source[import.span.start_byte as usize..import.span.end_byte as usize];
        let expected_name = if language == L::Go && (written.starts_with('"') || written.starts_with('`')) {
            crate::file::tree_file::go_import_path(written).rsplit('/').next().unwrap_or(written)
        } else { written };
        if import.name != expected_name || import.symbol.name != import.name
            || import.symbol.id.is_some() || import.symbol.source_span.is_some()
            || import.alias.as_ref().is_some_and(|alias| alias != &import.name)
        {
            return Err("an import identity disagrees with its producer or name token");
        }
        if let Some(statement) = &import.statement_span
            && (!lines.is_region(statement) || statement.start_byte > import.span.start_byte
                || statement.end_byte < import.span.end_byte || statement.start_byte == statement.end_byte
            )
        {
            return Err("an import statement span disagrees with the source");
        }
    }
    Ok(())
}

/// What a read entry stores that its reference evidence must agree with.
pub(super) struct StoredEvidence<'a> {
    /// The entry's source, with its line starts.
    pub(super) lines: &'a LineStarts<'a>,
    pub(super) language: ProgrammingLanguage,
    pub(super) capabilities: &'a BTreeSet<Capability>,
    /// The kinds the entry covers (`None`: every kind). Its records are
    /// every record of these kinds the analysis resolved against; evidence
    /// may still name a declaration of any other kind.
    pub(super) covered_kinds: Option<&'a BTreeSet<SymbolKindV2>>,
    pub(super) occurrences: &'a [Occurrence],
    /// Each occurrence's stored `path_start_byte`, in the same order.
    pub(super) path_starts: &'a [Option<u32>],
    pub(super) import_starts: &'a [Option<u32>],
    pub(super) coverage: &'a [ReferenceCoverage],
    pub(super) records: &'a [SymbolRecord],
    pub(super) imports: &'a [ImportRecord],
}

/// Checks a read entry's evidence against its source, its capability claim
/// and the records' summaries and semantics derived from it. The error names
/// the first inconsistency found.
///
/// Evidence a fresh analysis could have produced passes. Every ID names a
/// retained record of its name, except where the entry omits a kind the ID
/// could have: a kind-narrowed entry keeps evidence that names records it
/// does not retain (an owner is always a callable, so an owner ID is exempt
/// only when a callable kind is omitted). What cannot be told apart without
/// a parse (an occurrence/region list emptied while nothing derived from it
/// remains, an anonymous region added or removed, a written path and its stored start
/// both set to `null` together, the order of omitted candidates) is beyond
/// these checks. Imports are checked separately against retained provenance
/// and the complete producer-inventory witness, including source roles,
/// nullable metadata, and exact statement boundaries.
pub(super) fn validate_evidence(stored: &StoredEvidence<'_>) -> Result<(), &'static str> {
    let StoredEvidence {
        lines,
        language,
        capabilities,
        covered_kinds,
        occurrences,
        path_starts,
        import_starts,
        coverage,
        records,
        imports,
    } = *stored;
    let source = lines.source;
    if occurrences.len() != path_starts.len() || occurrences.len() != import_starts.len() {
        return Err("occurrence provenance lists are not aligned");
    }
    let imports_by_span: HashMap<_, _> = imports.iter().map(|import| ((import.span.start_byte, import.span.end_byte), import)).collect();
    let all_kinds = covered_kinds.is_none();
    // Whether a record of one of `kinds` may be absent from the records.
    let omits = |kinds: &[SymbolKindV2]| covered_kinds.is_some_and(|covered| kinds.iter().any(|kind| !covered.contains(kind)));
    // Any kind: a target, candidate or reference summary.
    let omits_any = !all_kinds;
    let omits_callables = omits(&[SymbolKindV2::Function, SymbolKindV2::Method, SymbolKindV2::Constructor]);
    let has_occurrences = capabilities.contains(&Capability::Occurrences);
    if !capabilities.contains(&Capability::Semantics) {
        if records.iter().any(|record| record.semantics != SemanticFacet::default()) {
            return Err("semantics stored without the semantics capability");
        }
    } else if !has_occurrences {
        return Err("semantics stored without the occurrences they are read from");
    }
    if !has_occurrences {
        if occurrences.is_empty() && coverage.is_empty() {
            return Ok(());
        }
        return Err("occurrences stored without the occurrences capability");
    }
    if !evidence_ids(occurrences, coverage, records).all(is_generated_id) {
        return Err("an ID that is not a generated declaration ID");
    }
    // Only Rust and TS/JS resolve names; every other language's evidence is
    // name-only and all of its regions are limited by that.
    let resolves = LanguageRules::for_language(language).is_some();
    // A reference to a stored record repeats its name and qualified name.
    let by_id: HashMap<&SymbolId, &SymbolRecord> = records.iter().map(|record| (&record.id, record)).collect();
    let retained = |reference: &SymbolRef| reference.id.as_ref().and_then(|id| by_id.get(id)).copied();
    // An ID that names no retained record passes only when its record may
    // have been omitted.
    let names_its_record = |reference: &SymbolRef, may_be_omitted: bool| match retained(reference) {
        Some(record) => {
            reference.name == record.identity.name
                && reference.qualified_name.as_deref() == Some(record.identity.qualified_name.as_str())
        }
        None => reference.id.is_none() || may_be_omitted,
    };

    // Every analysis has the file-level region, first.
    let Some(file_region) = coverage.first() else {
        return Err("no reference coverage under the occurrences capability");
    };
    if file_region.owner.is_some() || file_region.span != whole_file_span(source) {
        return Err("the first coverage region is not the whole file");
    }
    let mut regions = HashMap::new();
    let mut owned_regions: HashMap<&SymbolId, &TextSpan> = HashMap::new();
    let mut previous: Option<(u32, std::cmp::Reverse<u32>)> = None;
    for region in coverage {
        if !lines.is_region(&region.span) {
            return Err("a coverage span outside the source or off a character boundary");
        }
        let complete = region.state == CoverageState::Complete;
        if complete != region.reasons.is_empty() || !region.reasons.is_sorted_by(|left, right| left < right) {
            return Err("coverage reasons that do not match its state");
        }
        if resolves == region.reasons.contains(&LimitReason::UnsupportedLanguage) {
            return Err("an unsupported-language limit that does not match the language");
        }
        let order = (region.span.start_byte, std::cmp::Reverse(region.span.end_byte));
        if previous.is_some_and(|previous| previous > order) {
            return Err("coverage out of order");
        }
        previous = Some(order);
        if let Some(owner) = &region.owner {
            if !names_its_record(owner, omits_callables) {
                return Err("a region owner that does not name its record");
            }
            if retained(owner).is_some_and(|record| !has_semantics(record.kind)) {
                return Err("a region owner that is not a callable");
            }
            // A region's owner is a key here; it is checked for an ID below.
            if owner.id.as_ref().is_some_and(|id| owned_regions.insert(id, &region.span).is_some()) {
                return Err("a repeated coverage region");
            }
        }
        if regions.insert(region_key(region.owner.as_ref(), &region.span)?, region.owner.as_ref()).is_some() {
            return Err("a repeated coverage region");
        }
    }
    // A named callable has its own region exactly when it has a body
    // (`body_span`, empty bodies included), in every language, and that
    // region holds the body. It need not equal the declaration span: a C/C++
    // record keeps only its declarator, while the region is the definition.
    if records.iter().any(|record| {
        has_semantics(record.kind)
            && match (&record.source.body_span, owned_regions.get(&record.id)) {
                (Some(body), Some(region)) => {
                    body.start_byte < region.start_byte || region.end_byte < body.end_byte
                }
                (body, region) => body.is_some() != region.is_some(),
            }
    }) {
        return Err("a callable whose coverage region does not match its body");
    }

    // Retained record IDs by name, to check what an unresolving language's
    // name-only lookup over every record must have found.
    let mut ids_by_name: HashMap<&str, HashSet<&SymbolId>> = HashMap::new();
    for record in records {
        ids_by_name.entry(record.identity.name.as_str()).or_default().insert(&record.id);
    }
    let mut previous: Option<&Occurrence> = None;
    let mut scoped = HashSet::new();
    // What each owner's summaries were derived from.
    let mut scoped_by_owner: ScopedByOwner<'_> = HashMap::new();
    let mut unbound = Vec::new();
    let mut expected_dependencies: HashMap<&SymbolId, Vec<SymbolRef>> = HashMap::new();
    let mut seen_dependencies: HashMap<&SymbolId, HashSet<(&str, Option<&str>)>> = HashMap::new();
    for (position, occurrence) in occurrences.iter().enumerate() {
        let (start, end) = (occurrence.span.start_byte as usize, occurrence.span.end_byte as usize);
        if start >= end
            || !lines.is_region(&occurrence.span)
            || !lines.is_region(&occurrence.owner_span)
            || occurrence.span.start_byte < occurrence.owner_span.start_byte
            || occurrence.span.end_byte > occurrence.owner_span.end_byte
        {
            return Err("an occurrence span outside the source, its owner or a character boundary");
        }
        let name = occurrence.target.name.as_str();
        if source.get(start..end) != Some(name) {
            return Err("an occurrence whose target name is not the text at its span");
        }
        // A target's qualified name is its written path: the source text from
        // the qualifier through the name, so strictly longer than the name.
        let path = occurrence.target.qualified_name.as_deref();
        if let Some(path) = path
            && (path.len() <= name.len() || end < path.len() || source.get(end - path.len()..end) != Some(path))
        {
            return Err("an occurrence whose written path is not the source text ending at its name");
        }
        // Its stored start must agree, in every language and whether or not
        // the target's record is retained: a path set to `null` alone, or a
        // start set to `null` alone, is refused.
        match (path, path_starts.get(position).copied().flatten()) {
            (None, None) => {}
            (Some(path), Some(path_start)) if path_start as usize == end - path.len() => {}
            _ => return Err("an occurrence whose written path disagrees with its stored start"),
        }
        if occurrence.import_span.as_ref().map(|span| span.start_byte) != import_starts.get(position).copied().flatten() {
            return Err("an occurrence whose selected import disagrees with its stored start");
        }
        if let Some(span) = &occurrence.import_span
            && (occurrence.confidence != Confidence::Scoped || occurrence.target.id.is_some()
                || !lines.is_region(span)
                || !imports_by_span.get(&(span.start_byte, span.end_byte)).is_some_and(|import| &import.span == span
                    && crate::analysis::name_identity::equal(language, import.alias.as_deref().unwrap_or(&import.name), name)))
        {
                return Err("an occurrence whose selected import contradicts its binding");
        }
        // Candidates are in declaration order (name position, then ID). An
        // omitted record's position is unknown, so only the retained ones
        // are compared.
        if !occurrence
            .candidates
            .iter()
            .filter_map(retained)
            .map(|record| (record.source.name_span.as_ref().map_or(0, |span| span.start_byte), &record.id.0))
            .is_sorted()
        {
            return Err("candidates out of declaration order");
        }
        if regions.get(&region_key(occurrence.owner.as_ref(), &occurrence.owner_span)?)
            != Some(&occurrence.owner.as_ref())
        {
            return Err("an occurrence whose owner is no coverage region");
        }
        let consistent = match occurrence.confidence {
            Confidence::Scoped => {
                resolves
                    && occurrence.candidates.is_empty()
                    // A target repeats only its record's name.
                    && match retained(&occurrence.target) {
                        Some(record) => crate::analysis::name_identity::equal(language, &record.identity.name, name),
                        None => occurrence.target.id.is_none() || omits_any,
                    }
            }
            Confidence::NameOnly => {
                let mut ids = HashSet::new();
                occurrence.target.id.is_none()
                    && !occurrence.candidates.is_empty()
                    && occurrence.candidates.iter().all(|candidate| {
                        crate::analysis::name_identity::equal(language, &candidate.name, name)
                            && candidate.id.as_ref().is_some_and(|id| ids.insert(id))
                            && names_its_record(candidate, omits_any)
                    })
                    && (resolves || {
                        // The lookup took every record of the name: each
                        // retained one, and nothing else when all are.
                        let named = ids_by_name.get(name);
                        named.is_none_or(|named| named.is_subset(&ids))
                            && (!all_kinds || named.map_or(0, HashSet::len) == ids.len())
                    })
            }
            Confidence::Unresolved => {
                occurrence.target.id.is_none()
                    && occurrence.candidates.is_empty()
                    && (resolves || !ids_by_name.contains_key(name))
            }
        };
        if !consistent {
            return Err("an occurrence whose target ID and candidates contradict its confidence");
        }
        if let Some(previous) = previous {
            let spans = |occurrence: &Occurrence| (occurrence.span.start_byte, occurrence.span.end_byte);
            if spans(previous) >= spans(occurrence) || occurrence_order(previous) > occurrence_order(occurrence) {
                return Err("occurrences out of order or repeated");
            }
        }
        previous = Some(occurrence);
        if occurrence.confidence == Confidence::Scoped {
            let owner = occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref());
            if let Some(owner) = owner {
                let own = scoped_by_owner.entry(owner).or_default();
                match &occurrence.target.id {
                    Some(target) => {
                        own.first_by_target.entry(target).or_insert(position);
                    }
                    None => own
                        .unbound_by_name
                        .entry(crate::analysis::name_identity::key(language, name))
                        .or_default()
                        .push(position),
                }
                if let Some(import) = occurrence.import_span.as_ref().and_then(|span| imports_by_span.get(&(span.start_byte, span.end_byte)))
                    && seen_dependencies.entry(owner).or_default().insert((&import.name, import.symbol.qualified_name.as_deref()))
                {
                        expected_dependencies.entry(owner).or_default().push(SymbolRef {
                            source_span: None,
                            id: None,
                            name: import.name.clone(),
                            qualified_name: import.symbol.qualified_name.clone(),
                        });
                }
            }
            match &occurrence.target.id {
                Some(target) => {
                    if let Some(owner) = owner {
                        scoped.insert((owner, Some(target), crate::analysis::name_identity::key(language, name), occurrence.kind));
                    }
                }
                None => {
                    if let Some(owner) = owner {
                        scoped.insert((owner, None, crate::analysis::name_identity::key(language, name), occurrence.kind));
                    }
                    unbound.push(occurrence);
                }
            }
        }
    }
    innermost_owners(occurrences, coverage)?;
    if !unbound.is_empty() {
        unbound_names_have_a_binding(source, &unbound, occurrences, records, language)?;
    }

    // The records' summaries are derived from the occurrences, so each edge
    // needs the occurrence that made it, and each occurrence the edges it
    // makes between retained records.
    // `scoped` is keyed by name key, which is what `name_identity::equal`
    // compares.
    let made = |owner: &SymbolId, target: Option<&SymbolId>, name: &str| {
        let name = crate::analysis::name_identity::key(language, name);
        [OccurrenceKind::Call, OccurrenceKind::Usage]
            .into_iter()
            .any(|kind| scoped.contains(&(owner, target, name.clone(), kind)))
    };
    let made_to = |owner: &SymbolId, target: &SymbolRecord| made(owner, Some(&target.id), &target.identity.name);
    for record in records {
        let relations = &record.relations;
        let summary = relations.references.iter().chain(&relations.referenced_by).chain(&relations.dependencies);
        // A referencing record is an owner, so a callable.
        if !summary.clone().filter_map(|reference| reference.id.as_ref()).all(is_generated_id)
            || !relations.references.iter().chain(&relations.dependencies).all(|reference| names_its_record(reference, omits_any))
            || !relations.referenced_by.iter().all(|reference| names_its_record(reference, omits_callables))
        {
            return Err("a reference summary that does not name its record");
        }
        let (dependencies, references) = (RefList::new(&relations.dependencies), RefList::new(&relations.references));
        for reference in &relations.references {
            let is_dependency = dependencies.contains(reference);
            let found = match &reference.id {
                Some(target) if !is_dependency => made(&record.id, Some(target), &reference.name),
                None if is_dependency => true,
                _ => false,
            };
            if !found {
                return Err("a reference with no scoped occurrence");
            }
        }
        for dependency in &relations.dependencies {
            if dependency.id.is_some()
                || !references.contains(dependency)
                || !made(&record.id, None, &dependency.name)
            {
                return Err("a dependency with no scoped occurrence of an import");
            }
        }
        for owner in &relations.referenced_by {
            let Some(owner) = owner.id.as_ref() else {
                return Err("a referenced-by edge with no scoped occurrence");
            };
            if !made_to(owner, record) {
                return Err("a referenced-by edge with no scoped occurrence");
            }
        }
        summaries_in_first_occurrence_order(record, &scoped_by_owner, language)?;
        if relations.dependencies.as_slice() != expected_dependencies.get(&record.id).map_or(&[][..], Vec::as_slice) {
            return Err("dependency summaries disagree with the selected imports");
        }

    }
    for &(owner, target, _, _) in &scoped {
        let (Some(owner), Some(target)) = (by_id.get(owner), target) else { continue };
        let refers = owner.relations.references.iter().any(|reference| reference.id.as_ref() == Some(target));
        let referred = by_id.get(target).is_none_or(|target| {
            target.relations.referenced_by.iter().any(|reference| reference.id.as_ref() == Some(&owner.id))
        });
        if !refers || !referred {
            return Err("a scoped occurrence whose reference edge is missing");
        }
    }

    if capabilities.contains(&Capability::Semantics) {
        let expected = semantic_facets(records, occurrences, coverage, imports);
        for (record, facet) in records.iter().zip(expected) {
            if record.semantics != facet.unwrap_or_default() {
                return Err("semantics that the stored evidence does not produce");
            }
        }
    }
    Ok(())
}

/// Each owner's `Scoped` occurrences, by their position in the
/// occurrences.
type ScopedByOwner<'a> = HashMap<&'a SymbolId, OwnScoped<'a>>;

#[derive(Default)]
struct OwnScoped<'a> {
    /// The first occurrence of each target ID.
    first_by_target: HashMap<&'a SymbolId, usize>,
    /// Every occurrence with no target ID, ascending, by name key.
    unbound_by_name: HashMap<Cow<'a, str>, Vec<usize>>,
}

/// The fields [`SymbolRef`] equality compares, without the unhashable
/// source span.
type RefIdentity<'a> = (Option<&'a SymbolId>, &'a str, Option<&'a str>);

/// A summary list, for membership tests by [`SymbolRef`] equality.
struct RefList<'a>(HashMap<RefIdentity<'a>, Vec<&'a SymbolRef>>);

impl<'a> RefList<'a> {
    fn new(list: &'a [SymbolRef]) -> Self {
        let mut by_identity: HashMap<_, Vec<_>> = HashMap::new();
        for reference in list {
            by_identity.entry(Self::identity(reference)).or_default().push(reference);
        }
        Self(by_identity)
    }

    fn contains(&self, reference: &SymbolRef) -> bool {
        self.0.get(&Self::identity(reference)).is_some_and(|found| found.contains(&reference))
    }

    fn identity(reference: &SymbolRef) -> RefIdentity<'_> {
        (reference.id.as_ref(), &reference.name, reference.qualified_name.as_deref())
    }
}

/// A record's summaries list each edge once (an ID once; an import, which
/// has none, once per name and qualified name) in the order of the first
/// occurrence that made it, as the bind pass derives them.
///
/// A resolved edge's first occurrence is known exactly: the owner's first
/// `Scoped` occurrence of that target ID. Which `Scoped` occurrence with no
/// ID was bound to the import, rather than to a binding no record reports,
/// is not stored, so an import edge may take any of the owner's no-ID
/// occurrences of its name; the list is accepted when some choice is
/// strictly increasing (chosen greedily, earliest first, which finds one
/// whenever one exists). This never rejects a derived list.
fn summaries_in_first_occurrence_order(
    record: &SymbolRecord,
    scoped_by_owner: &ScopedByOwner<'_>,
    language: ProgrammingLanguage,
) -> Result<(), &'static str> {
    let relations = &record.relations;
    for list in [&relations.references, &relations.dependencies, &relations.referenced_by] {
        let mut seen = HashSet::new();
        if !list.iter().all(|reference| {
            seen.insert(match &reference.id {
                Some(id) => (Some(id), None, None),
                None => (None, Some(reference.name.as_str()), reference.qualified_name.as_deref()),
            })
        }) {
            return Err("a reference summary that repeats an edge");
        }
    }
    let empty = OwnScoped::default();
    let own = scoped_by_owner.get(&record.id).unwrap_or(&empty);
    let in_order = |list: &[SymbolRef]| {
        let mut last: Option<usize> = None;
        list.iter().all(|reference| {
            let after = |position: &usize| last.is_none_or(|last| *position > last);
            let next = match &reference.id {
                // The first occurrence of the target, which must follow.
                Some(id) => own.first_by_target.get(id).copied().filter(after),
                None => own
                    .unbound_by_name
                    .get(&crate::analysis::name_identity::key(language, &reference.name))
                    .and_then(|positions| positions.get(positions.partition_point(|position| !after(position))).copied()),
            };
            next.map(|position| last = Some(position)).is_some()
        })
    };
    if !in_order(&relations.references) || !in_order(&relations.dependencies) {
        return Err("a reference summary out of first-occurrence order");
    }
    // A target's owners, by each one's first `Scoped` occurrence of it.
    let mut last: Option<usize> = None;
    for owner in &relations.referenced_by {
        let first = owner
            .id
            .as_ref()
            .and_then(|owner| scoped_by_owner.get(owner))
            .and_then(|own| own.first_by_target.get(&record.id).copied());
        match first {
            Some(position) if last.is_none_or(|last| position > last) => last = Some(position),
            _ => return Err("a reference summary out of first-occurrence order"),
        }
    }
    Ok(())
}

/// Every ID the evidence stores.
fn evidence_ids<'a>(
    occurrences: &'a [Occurrence],
    coverage: &'a [ReferenceCoverage],
    records: &'a [SymbolRecord],
) -> impl Iterator<Item = &'a SymbolId> {
    let occurrence_refs = occurrences.iter().flat_map(|occurrence| {
        occurrence.owner.iter().chain(std::iter::once(&occurrence.target)).chain(&occurrence.candidates)
    });
    occurrence_refs
        .chain(coverage.iter().filter_map(|region| region.owner.as_ref()))
        .filter_map(|reference| reference.id.as_ref())
        .chain(records.iter().map(|record| &record.id))
}

/// Whether `id` has the form every declaration ID has: a BLAKE3 digest in
/// lowercase hex.
fn is_generated_id(id: &SymbolId) -> bool {
    id.0.len() == 64 && id.0.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Each occurrence's owner is the innermost region around it: regions are
/// syntax nodes, so they nest, and the analysis owns a reference by the
/// nearest one. Regions of one span are interchangeable here; their owners
/// were matched by key already.
fn innermost_owners(occurrences: &[Occurrence], coverage: &[ReferenceCoverage]) -> Result<(), &'static str> {
    let contains = |outer: &TextSpan, inner: &TextSpan| {
        outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
    };
    let mut open: Vec<&TextSpan> = Vec::new();
    let mut next = coverage.iter().peekable();
    for occurrence in occurrences {
        while let Some(region) = next.next_if(|region| region.span.start_byte <= occurrence.span.start_byte) {
            while open.last().is_some_and(|last| !contains(last, &region.span)) {
                open.pop();
            }
            open.push(&region.span);
        }
        while open.last().is_some_and(|last| !contains(last, &occurrence.span)) {
            open.pop();
        }
        if open.last() != Some(&&occurrence.owner_span) {
            return Err("an occurrence owned by a region around the innermost one");
        }
    }
    Ok(())
}

/// A `Scoped` occurrence with no target ID is bound to an import or to a
/// binding no record reports, and either is written somewhere in the source
/// as the same identifier: never at a record's name (that binding has an
/// ID) and never at an occurrence (binding names are not occurrences). An
/// identifier with no such other spelling has lost its target.
///
/// Identifiers are read as maximal runs of ASCII letters, digits, `_` and
/// `$`, each run also as its `$`-free pieces (`$` is part of a TS/JS
/// identifier but starts a Rust macro metavariable); a name outside that
/// alphabet is not checked. Raw Rust and escaped ECMA tokens additionally
/// supply canonical witnesses, excluding the entire declaration or occurrence
/// token (including its ASCII tail). This check does not require a parse.
fn unbound_names_have_a_binding(
    source: &str,
    unbound: &[&Occurrence],
    occurrences: &[Occurrence],
    records: &[SymbolRecord],
    language: ProgrammingLanguage,
) -> Result<(), &'static str> {
    let is_identifier_byte = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$';
    let checked: HashSet<&str> = unbound
        .iter()
        .map(|occurrence| occurrence.target.name.as_str())
        .filter(|name| name.bytes().all(is_identifier_byte))
        .collect();
    if checked.is_empty() {
        return Ok(());
    }
    let mut taken: Vec<(u32, u32)> = occurrences
        .iter()
        .map(|occurrence| &occurrence.span)
        .chain(records.iter().filter_map(|record| record.source.name_span.as_ref()))
        .map(|span| (span.start_byte, span.end_byte))
        .collect();
    taken.sort_unstable();
    taken.dedup();
    let mut covering = Covering { spans: &taken, next: 0, furthest_end: None };
    let mut written: HashSet<&str> = HashSet::new();
    let bytes = source.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        if !is_identifier_byte(bytes[start]) {
            start += 1;
            continue;
        }
        let end = (start..bytes.len()).find(|&at| !is_identifier_byte(bytes[at])).unwrap_or(bytes.len());
        for (from, to) in identifier_readings(start, &bytes[start..end]) {
            if let Some(text) = source.get(from..to)
                && checked.contains(text)
                && !covering.covers(from as u32, to as u32)
            {
                written.insert(text);
            }
        }
        start = end;
    }
    // Encoded tokens must be excluded as a whole: the ASCII tail of a raw
    // identifier or Unicode escape is not a second binding witness. Only a
    // raw Rust (`r#`) or escaped TS/JS (`\u`) token can be one, so a source
    // with neither has none.
    let mut encoded = HashSet::new();
    let encoding = match language {
        ProgrammingLanguage::Rust => Some("r#"),
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => Some("\\u"),
        _ => None,
    };
    let mut at = if encoding.is_some_and(|marker| memchr::memmem::find(bytes, marker.as_bytes()).is_some()) {
        0
    } else {
        source.len()
    };
    while at < source.len() {
        let start = at;
        let raw = language == ProgrammingLanguage::Rust && source[at..].starts_with("r#");
        if raw { at += 2; }
        let mut escaped = false;
        while at < source.len() {
            let rest = &source[at..];
            if matches!(language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript) && rest.starts_with("\\u") {
                escaped = true;
                at += 2;
                if source[at..].starts_with('{') {
                    let Some(end) = source[at..].find('}') else { break };
                    at += end + 1;
                } else {
                    let mut count = 0;
                    while at < source.len() && count < 4 && source.as_bytes()[at].is_ascii_hexdigit() { at += 1; count += 1; }
                }
            } else {
                let ch = rest.chars().next().expect("character");
                if ch.is_alphanumeric() || matches!(ch, '_' | '$') { at += ch.len_utf8(); } else { break; }
            }
        }
        if at == start { at += source[at..].chars().next().expect("character").len_utf8(); continue; }
        if (raw || escaped) && taken.binary_search(&(start as u32, at as u32)).is_err() {
            let key = crate::analysis::name_identity::key(language, &source[start..at]).into_owned();
            if checked.contains(key.as_str()) { encoded.insert(key); }
        }
    }
    if checked.iter().all(|name| written.contains(name) || encoded.contains(*name)) {
        Ok(())
    } else {
        Err("a scoped occurrence with no target ID and no binding of its name")
    }
}

/// The byte ranges one run of identifier bytes starting at `start` can be
/// read as: the whole run and each `$`-free piece of it, in that order, so
/// their starts never decrease.
fn identifier_readings(start: usize, run: &[u8]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut offset = start;
    let pieces = run.split(|&byte| byte == b'$').filter_map(move |piece| {
        let reading = (offset, offset + piece.len());
        offset += piece.len() + 1;
        (!piece.is_empty() && piece.len() != run.len()).then_some(reading)
    });
    std::iter::once((start, start + run.len())).chain(pieces)
}

/// Whether some span of a start-sorted list contains a range, for ranges
/// asked in order of non-decreasing start.
///
/// A span contains `from..to` exactly when it starts at or before `from` and
/// ends at or after `to`, so the furthest end among the spans starting at or
/// before `from` decides. Spans are not merged: a range across two
/// overlapping spans is in neither.
struct Covering<'a> {
    spans: &'a [(u32, u32)],
    next: usize,
    furthest_end: Option<u32>,
}

impl Covering<'_> {
    fn covers(&mut self, from: u32, to: u32) -> bool {
        while let Some(&(start, end)) = self.spans.get(self.next)
            && start <= from
        {
            self.furthest_end = self.furthest_end.max(Some(end));
            self.next += 1;
        }
        self.furthest_end.is_some_and(|end| to <= end)
    }
}

/// A region's identity: its owner's ID (a named owner must have one) and
/// its byte range.
fn region_key<'a>(
    owner: Option<&'a SymbolRef>,
    span: &TextSpan,
) -> Result<(Option<&'a SymbolId>, u32, u32), &'static str> {
    let owner = match owner {
        Some(owner) => Some(owner.id.as_ref().ok_or("an owner without an ID")?),
        None => None,
    };
    Ok((owner, span.start_byte, span.end_byte))
}

/// Byte offsets of each line's first byte, to check that a span's points
/// are the one-based line and byte column of its offsets.
pub(super) struct LineStarts<'a> {
    source: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineStarts<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(newline, _)| newline + 1))
            .collect();
        Self { source, starts }
    }

    /// The point at `byte`, when it is a character boundary of the source.
    fn point(&self, byte: u32) -> Option<TextPoint> {
        let byte = byte as usize;
        if !self.source.is_char_boundary(byte) {
            return None;
        }
        let line = self.starts.partition_point(|&start| start <= byte);
        Some(TextPoint {
            line: u32::try_from(line).ok()?,
            column: u32::try_from(byte - self.starts[line - 1] + 1).ok()?,
        })
    }

    /// Whether `span` is a (possibly empty) range of the source whose points
    /// agree with its offsets.
    pub(super) fn is_region(&self, span: &TextSpan) -> bool {
        span.start_byte <= span.end_byte
            && self.point(span.start_byte) == Some(span.start)
            && self.point(span.end_byte) == Some(span.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_one_based_lines_and_byte_columns() {
        let lines = LineStarts::new("é\r\nab");
        assert_eq!(lines.point(0), Some(TextPoint { line: 1, column: 1 }));
        assert_eq!(lines.point(1), None, "inside `é`");
        assert_eq!(lines.point(2), Some(TextPoint { line: 1, column: 3 }));
        assert_eq!(lines.point(4), Some(TextPoint { line: 2, column: 1 }));
        assert_eq!(lines.point(6), Some(TextPoint { line: 2, column: 3 }), "end of source");
        assert_eq!(lines.point(7), None, "past the end");
    }

    #[test]
    fn a_dollar_run_is_also_read_as_its_pieces() {
        let readings = |start, run| identifier_readings(start, run).collect::<Vec<_>>();
        assert_eq!(readings(3, b"f"), [(3, 4)]);
        assert_eq!(readings(0, b"$f"), [(0, 2), (1, 2)]);
        assert_eq!(readings(10, b"a$$bc$"), [(10, 16), (10, 11), (13, 15)]);
    }

    /// The cursor answers exactly what a scan of every span answers,
    /// including for ranges across two overlapping or touching spans.
    #[test]
    fn covering_agrees_with_a_scan_of_every_span() {
        let mut taken = vec![(2, 5), (4, 9), (9, 12), (20, 20), (20, 26), (22, 24), (30, 31)];
        taken.sort_unstable();
        let mut covering = Covering { spans: &taken, next: 0, furthest_end: None };
        for from in 0..34 {
            for to in from..34 {
                let scanned = taken.iter().any(|&(start, end)| start <= from && to <= end);
                assert_eq!(covering.covers(from, to), scanned, "{from}..{to}");
            }
        }
        assert!(!Covering { spans: &[], next: 0, furthest_end: None }.covers(0, 0));
    }

    /// The inventory is persisted in every cache entry, so its digest must
    /// not move when the way it is computed does.
    #[test]
    fn import_inventory_digest_is_pinned() {
        let span = |start: u32, end: u32| TextSpan {
            start: TextPoint { line: 1, column: start + 1 },
            end: TextPoint { line: 1, column: end + 1 },
            start_byte: start,
            end_byte: end,
        };
        let imports = [
            ImportRecord {
                name: "HashMap".into(),
                alias: None,
                source: Some("std::collections".into()),
                span: span(22, 29),
                statement_span: Some(span(0, 30)),
                symbol: SymbolRef {
                    id: None,
                    name: "HashMap".into(),
                    qualified_name: Some("std::collections::HashMap".into()),
                    source_span: None,
                },
                reexported: false,
            },
            ImportRecord {
                name: "io".into(),
                alias: Some("io".into()),
                source: None,
                span: span(35, 37),
                statement_span: None,
                symbol: SymbolRef {
                    id: Some(SymbolId("ab".repeat(32))),
                    name: "io".into(),
                    qualified_name: None,
                    source_span: Some(span(35, 37)),
                },
                reexported: false,
            },
        ];
        assert_eq!(import_inventory(&imports), "67e537e1556367bab0e019d379a74bb473fae432d8857d49cc7ed6c1b99ae79f");
        assert_eq!(import_inventory(&[]), "d53d18c23212ea7b6300594bb89bce60218f6eff2b9d628b8cc42d3e79bbd5ab");
    }

    #[test]
    fn generated_ids_are_lowercase_blake3_hex() {
        let id = crate::shared::symbol_id_from_stable_key("rust::a.rs::Function::f::0");
        assert!(is_generated_id(&id));
        assert!(!is_generated_id(&SymbolId(id.0.to_uppercase())));
        assert!(!is_generated_id(&SymbolId(id.0[1..].to_string())));
        assert!(!is_generated_id(&SymbolId("bogus".to_string())));
    }
}
