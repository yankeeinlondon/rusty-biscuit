//! The cache entry's form of a file's reference evidence, and the checks a
//! read entry's evidence must pass before it is served.
//!
//! The public `Occurrence` and `ReferenceCoverage` omit empty lists and
//! absent owners and IDs when serialized, and read a missing key as empty.
//! The entry writes every key instead (`null`, `[]`), and the reader requires
//! each one, so an absent key, `null` and an empty list stay distinct and a
//! damaged entry is a miss, never evidence read as empty.

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
    ReferenceCoverage, SemanticFacet, SymbolId, SymbolRecord, SymbolRef, TextPoint, TextSpan,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredOccurrence {
    #[serde(deserialize_with = "nullable")]
    owner: Option<StoredRef>,
    owner_span: StoredSpan,
    target: StoredRef,
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

pub(super) fn read_occurrences<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Occurrence>, D::Error> {
    let stored = Vec::<StoredOccurrence>::deserialize(deserializer)?;
    Ok(stored.into_iter().map(Occurrence::from).collect())
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

/// What a read entry stores that its reference evidence must agree with.
pub(super) struct StoredEvidence<'a> {
    pub(super) source: &'a str,
    pub(super) language: ProgrammingLanguage,
    pub(super) capabilities: &'a BTreeSet<Capability>,
    /// The entry covers every kind, so its records are every record the
    /// analysis resolved against.
    pub(super) all_kinds: bool,
    pub(super) occurrences: &'a [Occurrence],
    pub(super) coverage: &'a [ReferenceCoverage],
    pub(super) records: &'a [SymbolRecord],
    pub(super) imports: &'a [ImportRecord],
}

/// Checks a read entry's evidence against its source, its capability claim
/// and the records' summaries and semantics derived from it. The error names
/// the first inconsistency found.
///
/// Evidence a fresh analysis could have produced passes. A kind-narrowed
/// entry keeps evidence that names records it does not retain, so an ID is
/// checked against a record only when that record is retained; what cannot
/// be told apart without a parse (a list emptied while nothing derived from
/// it remains, an anonymous region added or removed) is beyond these checks.
pub(super) fn validate_evidence(stored: &StoredEvidence<'_>) -> Result<(), &'static str> {
    let StoredEvidence { source, language, capabilities, all_kinds, occurrences, coverage, records, imports } = *stored;
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
    let lines = LineStarts::new(source);
    // A reference to a stored record repeats its name and qualified name.
    let by_id: HashMap<&SymbolId, &SymbolRecord> = records.iter().map(|record| (&record.id, record)).collect();
    let retained = |reference: &SymbolRef| reference.id.as_ref().and_then(|id| by_id.get(id)).copied();
    let names_its_record = |reference: &SymbolRef| {
        retained(reference).is_none_or(|record| {
            reference.name == record.identity.name
                && reference.qualified_name.as_deref() == Some(record.identity.qualified_name.as_str())
        })
    };

    // Every analysis has the file-level region, first.
    let Some(file_region) = coverage.first() else {
        return Err("no reference coverage under the occurrences capability");
    };
    if file_region.owner.is_some() || file_region.span != whole_file_span(source) {
        return Err("the first coverage region is not the whole file");
    }
    let mut regions = HashMap::new();
    let mut owned_regions = HashSet::new();
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
            if !names_its_record(owner) {
                return Err("a region owner that does not name its record");
            }
            if retained(owner).is_some_and(|record| !has_semantics(record.kind)) {
                return Err("a region owner that is not a callable");
            }
            // A region's owner is a key here; it is checked for an ID below.
            if owner.id.as_ref().is_some_and(|id| !owned_regions.insert(id)) {
                return Err("a repeated coverage region");
            }
        }
        if regions.insert(region_key(region.owner.as_ref(), &region.span)?, region.owner.as_ref()).is_some() {
            return Err("a repeated coverage region");
        }
    }
    // Every named callable with a body gets its own region (Python and Lua
    // read a body that need not be a `body` field, so a region is not
    // provable for them).
    if !matches!(language, ProgrammingLanguage::Python | ProgrammingLanguage::Lua)
        && records.iter().any(|record| {
            has_semantics(record.kind) && record.source.body_span.is_some() && !owned_regions.contains(&record.id)
        })
    {
        return Err("a callable with a body and no coverage region");
    }

    // Retained record IDs by name, to check what an unresolving language's
    // name-only lookup over every record must have found.
    let mut ids_by_name: HashMap<&str, HashSet<&SymbolId>> = HashMap::new();
    for record in records {
        ids_by_name.entry(record.identity.name.as_str()).or_default().insert(&record.id);
    }
    let mut previous: Option<&Occurrence> = None;
    let mut scoped = HashSet::new();
    let mut unbound = Vec::new();
    for occurrence in occurrences {
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
        if regions.get(&region_key(occurrence.owner.as_ref(), &occurrence.owner_span)?)
            != Some(&occurrence.owner.as_ref())
        {
            return Err("an occurrence whose owner is no coverage region");
        }
        let consistent = match occurrence.confidence {
            Confidence::Scoped => {
                resolves
                    && occurrence.candidates.is_empty()
                    && retained(&occurrence.target).is_none_or(|record| record.identity.name == name)
            }
            Confidence::NameOnly => {
                let mut ids = HashSet::new();
                occurrence.target.id.is_none()
                    && !occurrence.candidates.is_empty()
                    && occurrence.candidates.iter().all(|candidate| {
                        candidate.name == name
                            && candidate.id.as_ref().is_some_and(|id| ids.insert(id))
                            && names_its_record(candidate)
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
            match &occurrence.target.id {
                Some(target) => {
                    if let Some(owner) = owner {
                        scoped.insert((owner, Some(target), name, occurrence.kind));
                    }
                }
                None => {
                    if let Some(owner) = owner {
                        scoped.insert((owner, None, name, occurrence.kind));
                    }
                    unbound.push(occurrence);
                }
            }
        }
    }
    innermost_owners(occurrences, coverage)?;
    if !unbound.is_empty() {
        unbound_names_have_a_binding(source, &unbound, occurrences, records)?;
    }

    // The records' summaries are derived from the occurrences, so each edge
    // needs the occurrence that made it, and each occurrence the edges it
    // makes between retained records.
    let made = |owner: &SymbolId, target: Option<&SymbolId>, name: &str| {
        [OccurrenceKind::Call, OccurrenceKind::Usage]
            .iter()
            .any(|&kind| scoped.contains(&(owner, target, name, kind)))
    };
    let made_to = |owner: &SymbolId, target: &SymbolRecord| made(owner, Some(&target.id), &target.identity.name);
    for record in records {
        let relations = &record.relations;
        let summary = relations.references.iter().chain(&relations.referenced_by).chain(&relations.dependencies);
        if !summary.clone().filter_map(|reference| reference.id.as_ref()).all(is_generated_id)
            || !summary.clone().all(&names_its_record)
        {
            return Err("a reference summary that does not name its record");
        }
        for reference in &relations.references {
            let is_dependency = relations.dependencies.contains(reference);
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
                || !relations.references.contains(dependency)
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
/// alphabet is not checked. Any other character ends a run, so an
/// identifier is only ever read as more spellings, never fewer, and the
/// rule never rejects evidence a fresh analysis could produce.
fn unbound_names_have_a_binding(
    source: &str,
    unbound: &[&Occurrence],
    occurrences: &[Occurrence],
    records: &[SymbolRecord],
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
    let taken: HashSet<(u32, u32)> = occurrences
        .iter()
        .map(|occurrence| &occurrence.span)
        .chain(records.iter().filter_map(|record| record.source.name_span.as_ref()))
        .map(|span| (span.start_byte, span.end_byte))
        .collect();
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
            if !taken.contains(&(from as u32, to as u32))
                && let Some(text) = source.get(from..to)
                && checked.contains(text)
            {
                written.insert(text);
            }
        }
        start = end;
    }
    if checked.iter().all(|name| written.contains(name)) {
        Ok(())
    } else {
        Err("a scoped occurrence with no target ID and no binding of its name")
    }
}

/// The byte ranges one run of identifier bytes starting at `start` can be
/// read as: the whole run and each `$`-free piece of it.
fn identifier_readings(start: usize, run: &[u8]) -> Vec<(usize, usize)> {
    let mut readings = vec![(start, start + run.len())];
    let mut offset = start;
    for piece in run.split(|&byte| byte == b'$') {
        if !piece.is_empty() && piece.len() != run.len() {
            readings.push((offset, offset + piece.len()));
        }
        offset += piece.len() + 1;
    }
    readings
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
struct LineStarts<'a> {
    source: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineStarts<'a> {
    fn new(source: &'a str) -> Self {
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
    fn is_region(&self, span: &TextSpan) -> bool {
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
        assert_eq!(identifier_readings(3, b"f"), [(3, 4)]);
        assert_eq!(identifier_readings(0, b"$f"), [(0, 2), (1, 2)]);
        assert_eq!(identifier_readings(10, b"a$$bc$"), [(10, 16), (10, 11), (13, 15)]);
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
