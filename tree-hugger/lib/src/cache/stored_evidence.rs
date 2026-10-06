//! The cache entry's form of a file's reference evidence, and the checks a
//! read entry's evidence must pass before it is served.
//!
//! The public `Occurrence` and `ReferenceCoverage` omit empty lists and
//! absent owners and IDs when serialized, and read a missing key as empty.
//! The entry writes every key instead (`null`, `[]`), and the reader requires
//! each one, so an absent key, `null` and an empty list stay distinct and a
//! damaged entry is a miss, never evidence read as empty.

use std::collections::{HashMap, HashSet};

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::analysis::occurrences::{occurrence_order, whole_file_span};
use crate::shared::{
    Confidence, CoverageState, LimitReason, Occurrence, OccurrenceKind, ReferenceCoverage,
    SymbolId, SymbolRecord, SymbolRef, TextPoint, TextSpan, TriState,
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

/// Checks a read entry's evidence against its source, its capability claim
/// and the records' summaries derived from it. The error names the first
/// inconsistency found.
///
/// Evidence a fresh analysis could have produced passes; what cannot be told
/// apart without a parse (a list emptied while nothing derived from it
/// remains) is beyond these checks.
pub(super) fn validate_evidence(
    source: &str,
    has_occurrences: bool,
    occurrences: &[Occurrence],
    coverage: &[ReferenceCoverage],
    records: &[SymbolRecord],
) -> Result<(), &'static str> {
    if !has_occurrences {
        if occurrences.is_empty() && coverage.is_empty() {
            return Ok(());
        }
        return Err("occurrences stored without the occurrences capability");
    }
    let lines = LineStarts::new(source);
    // A reference to a stored record repeats its name and qualified name.
    let by_id: HashMap<&SymbolId, &SymbolRecord> = records.iter().map(|record| (&record.id, record)).collect();
    let names_its_record = |reference: &SymbolRef| {
        reference.id.as_ref().and_then(|id| by_id.get(id)).is_none_or(|record| {
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
    let mut previous: Option<(u32, std::cmp::Reverse<u32>)> = None;
    for region in coverage {
        if !lines.is_region(&region.span) {
            return Err("a coverage span outside the source or off a character boundary");
        }
        let complete = region.state == CoverageState::Complete;
        if complete != region.reasons.is_empty() || !region.reasons.is_sorted_by(|left, right| left < right) {
            return Err("coverage reasons that do not match its state");
        }
        let order = (region.span.start_byte, std::cmp::Reverse(region.span.end_byte));
        if previous.is_some_and(|previous| previous > order) {
            return Err("coverage out of order");
        }
        previous = Some(order);
        if !region.owner.as_ref().is_none_or(names_its_record) {
            return Err("a region owner that does not name its record");
        }
        regions.insert(region_key(region.owner.as_ref(), &region.span)?, region.owner.as_ref());
    }

    let mut previous: Option<&Occurrence> = None;
    let mut scoped = HashSet::new();
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
        if source.get(start..end) != Some(occurrence.target.name.as_str()) {
            return Err("an occurrence whose target name is not the text at its span");
        }
        if regions.get(&region_key(occurrence.owner.as_ref(), &occurrence.owner_span)?)
            != Some(&occurrence.owner.as_ref())
        {
            return Err("an occurrence whose owner is no coverage region");
        }
        let consistent = match occurrence.confidence {
            Confidence::Scoped => occurrence.candidates.is_empty(),
            Confidence::NameOnly => {
                let mut ids = HashSet::new();
                occurrence.target.id.is_none()
                    && !occurrence.candidates.is_empty()
                    && occurrence.candidates.iter().all(|candidate| {
                        candidate.id.as_ref().is_some_and(|id| ids.insert(id)) && names_its_record(candidate)
                    })
            }
            Confidence::Unresolved => occurrence.target.id.is_none() && occurrence.candidates.is_empty(),
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
        if occurrence.confidence == Confidence::Scoped
            && let (Some(owner), Some(target)) = (
                occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref()),
                occurrence.target.id.as_ref(),
            )
        {
            scoped.insert((owner, target, occurrence.kind));
        }
    }

    // The records' summaries and recursion are derived from the occurrences,
    // so each established edge needs the occurrence that made it.
    let made = |owner: &SymbolId, target: &SymbolId| {
        [OccurrenceKind::Call, OccurrenceKind::Usage]
            .iter()
            .any(|&kind| scoped.contains(&(owner, target, kind)))
    };
    for record in records {
        let relations = &record.relations;
        let local_targets = relations
            .references
            .iter()
            .filter(|reference| !relations.dependencies.contains(reference))
            .filter_map(|reference| reference.id.as_ref());
        for target in local_targets {
            if !made(&record.id, target) {
                return Err("a reference with no scoped occurrence");
            }
        }
        for owner in relations.referenced_by.iter().filter_map(|owner| owner.id.as_ref()) {
            if !made(owner, &record.id) {
                return Err("a referenced-by edge with no scoped occurrence");
            }
        }
        if record.semantics.is_recursive == Some(TriState::Yes)
            && !scoped.contains(&(&record.id, &record.id, OccurrenceKind::Call))
        {
            return Err("direct recursion with no scoped call to itself");
        }
    }
    Ok(())
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
}
