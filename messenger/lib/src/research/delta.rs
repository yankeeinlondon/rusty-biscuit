//! Deterministic fact-level comparison of accepted research and a candidate.
//!
//! The delta is mechanical: it compares validated structured metadata, never
//! arbitrary prose, and its flags are a fixed set computed from typed values.
//! Judging whether a changed claim is supported by its evidence belongs to the
//! independent evidence reviewer, whose conclusions are recorded separately;
//! nothing here approves anything.
//!
//! Each changed record keeps its full before/after JSON and splits its field
//! changes into typed values, evidence references (`knowledge.evidence`), and
//! record prose (`knowledge.explanation`, `note`). Evidence sources are compared
//! record by record, so a source whose locator, revision, or retrieval changed
//! at an unchanged URL is still reported. The document body is compared by its
//! Darkmatter hash, so a prose-only change with identical typed values is
//! visible.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use super::model::{Mappings, PlatformId, ReviewStatus};
use super::project::{AcceptedDocument, sorted};

/// What the candidate is compared against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Baseline {
    /// Initial research: there is no previous accepted baseline, and none is
    /// inferred from unverified legacy prose.
    None,
    Accepted { path: String, frontmatter_hash: String, body_hash: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

/// One differing leaf, addressed by JSON Pointer into the record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldChange {
    pub pointer: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// One added, removed, or changed fact record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactDelta {
    pub array: String,
    pub id: String,
    pub change: ChangeKind,
    pub typed_changes: Vec<FieldChange>,
    pub evidence_added: Vec<String>,
    pub evidence_removed: Vec<String>,
    pub prose_changes: Vec<FieldChange>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// One added, removed, or changed evidence source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceDelta {
    pub id: String,
    pub change: ChangeKind,
    /// A changed source whose URL (or repository location) is the same: its
    /// supporting evidence changed without a new link.
    pub same_location: bool,
    pub field_changes: Vec<FieldChange>,
    /// Facts in the candidate that cite this source.
    pub cited_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GapDelta {
    pub id: String,
    pub change: ChangeKind,
    pub field_changes: Vec<FieldChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProseDelta {
    pub changed: bool,
    pub before_body_hash: Option<String>,
    pub after_body_hash: String,
}

/// The fixed set of suspicious-change flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    RemovedConstraint,
    /// A maximum rose or a minimum fell: the bound got looser.
    RaisedLimit,
    /// A `support` value moved between `unsupported` and supported.
    SupportReversal,
    /// A constraint's unit or measurement stage changed.
    ChangedUnit,
    /// A record is (newly) `conflicting`.
    ConflictingEvidence,
    /// A typed value became `unknown`, `unmapped`, or `unspecified_characters`.
    NewUnmappableValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Flag {
    pub kind: FlagKind,
    pub fact: String,
    pub detail: String,
}

/// An accepted implementation assessment whose assessed facts changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedAssessment {
    pub assessment: String,
    pub adapter: super::model::AdapterId,
    pub category: super::model::Category,
    pub facts: Vec<String>,
}

/// The full comparison for one platform.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Delta {
    pub platform_id: PlatformId,
    /// Always `mechanical`: flags are computed, not reviewed.
    pub conclusion_kind: &'static str,
    pub baseline: Baseline,
    /// Sorted by `(array, id)`.
    pub facts: Vec<FactDelta>,
    /// Sorted by ID.
    pub sources: Vec<SourceDelta>,
    /// Sorted by ID.
    pub gaps: Vec<GapDelta>,
    pub prose: ProseDelta,
    /// Sorted by `(kind, fact)`.
    pub flags: Vec<Flag>,
    /// Sorted by assessment ID.
    pub affected_assessments: Vec<AffectedAssessment>,
}

impl Delta {
    /// No typed, evidence, source, gap, or prose difference.
    pub fn is_unchanged(&self) -> bool {
        matches!(self.baseline, Baseline::Accepted { .. })
            && self.facts.is_empty()
            && self.sources.is_empty()
            && self.gaps.is_empty()
            && !self.prose.changed
    }
}

/// Compares `candidate` with the `previous` accepted document of the same
/// platform (`None` for initial research).
///
/// ## Panics
///
/// When the two documents describe different platforms.
pub fn compare(previous: Option<&AcceptedDocument>, candidate: &AcceptedDocument, mappings: Option<&Mappings>) -> Delta {
    let platform_id = candidate.validated.platform_id();
    if let Some(previous) = previous {
        assert_eq!(previous.validated.platform_id(), platform_id, "delta across platforms");
    }
    let after_facts = facts_of(Some(candidate));
    let before_facts = facts_of(previous);

    let mut facts = Vec::new();
    let mut flags = BTreeSet::new();
    if previous.is_some() {
        let keys: BTreeSet<&(String, String)> = before_facts.keys().chain(after_facts.keys()).collect();
        for key in keys {
            let (array, id) = key;
            let before = before_facts.get(key);
            let after = after_facts.get(key);
            if before == after {
                continue;
            }
            let delta = fact_delta(array, id, before, after);
            flag(&delta, &mut flags);
            facts.push(delta);
        }
    } else {
        for ((array, id), record) in &after_facts {
            let delta = fact_delta(array, id, None, Some(record));
            flag(&delta, &mut flags);
            facts.push(delta);
        }
    }

    let sources = source_deltas(previous, candidate);
    let gaps = gap_deltas(previous, candidate);
    let prose = ProseDelta {
        changed: previous.is_none_or(|previous| previous.body_hash != candidate.body_hash),
        before_body_hash: previous.map(|previous| previous.body_hash.clone()),
        after_body_hash: candidate.body_hash.clone(),
    };

    let changed_ids: BTreeSet<&str> = facts.iter().map(|fact| fact.id.as_str()).collect();
    let mut affected_assessments: Vec<AffectedAssessment> = mappings
        .map(|mappings| {
            mappings
                .assessments
                .iter()
                .filter(|assessment| {
                    assessment.platform_id == platform_id && assessment.review.status == ReviewStatus::Accepted
                })
                .filter_map(|assessment| {
                    let facts: Vec<String> = assessment
                        .facts
                        .iter()
                        .filter(|fact| changed_ids.contains(fact.as_str()))
                        .cloned()
                        .collect();
                    (!facts.is_empty()).then(|| AffectedAssessment {
                        assessment: assessment.id.clone(),
                        adapter: assessment.adapter,
                        category: assessment.category,
                        facts,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    affected_assessments.sort_by(|a, b| a.assessment.cmp(&b.assessment));

    Delta {
        platform_id,
        conclusion_kind: "mechanical",
        baseline: match previous {
            None => Baseline::None,
            Some(previous) => Baseline::Accepted {
                path: previous.validated.path().to_string(),
                frontmatter_hash: previous.frontmatter_hash.clone(),
                body_hash: previous.body_hash.clone(),
            },
        },
        facts,
        sources,
        gaps,
        prose,
        flags: flags.into_iter().collect(),
        affected_assessments,
    }
}

fn facts_of(document: Option<&AcceptedDocument>) -> BTreeMap<(String, String), Value> {
    document
        .map(|document| {
            document
                .validated
                .fact_records()
                .into_iter()
                .map(|fact| ((fact.array, fact.id.to_string()), sorted(fact.record)))
                .collect()
        })
        .unwrap_or_default()
}

fn records_by_id(document: Option<&AcceptedDocument>, key: &str) -> BTreeMap<String, Value> {
    document
        .and_then(|document| document.validated.frontmatter().get(key).and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|record| Some((record.get("id")?.as_str()?.to_string(), sorted(record))))
        .collect()
}

/// `knowledge.explanation` and `note` are record prose; everything else is a
/// typed value or an evidence reference.
fn is_prose(pointer: &str) -> bool {
    pointer == "/knowledge/explanation" || pointer.ends_with("/note") || pointer == "/note"
}

fn fact_delta(array: &str, id: &str, before: Option<&Value>, after: Option<&Value>) -> FactDelta {
    let change = match (before, after) {
        (None, _) => ChangeKind::Added,
        (_, None) => ChangeKind::Removed,
        _ => ChangeKind::Changed,
    };
    let mut typed_changes = Vec::new();
    let mut prose_changes = Vec::new();
    if change == ChangeKind::Changed {
        for field in field_changes(before, after) {
            if field.pointer.starts_with("/knowledge/evidence") {
                continue;
            }
            if is_prose(&field.pointer) {
                prose_changes.push(field);
            } else {
                typed_changes.push(field);
            }
        }
    }
    let evidence = |record: Option<&Value>| -> BTreeSet<String> {
        record
            .and_then(|record| record.pointer("/knowledge/evidence"))
            .and_then(Value::as_array)
            .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default()
    };
    let (old, new) = (evidence(before), evidence(after));
    FactDelta {
        array: array.to_string(),
        id: id.to_string(),
        change,
        typed_changes,
        evidence_added: new.difference(&old).cloned().collect(),
        evidence_removed: old.difference(&new).cloned().collect(),
        prose_changes,
        before: before.cloned(),
        after: after.cloned(),
    }
}

/// Every differing leaf between two records (arrays compare by index).
fn field_changes(before: Option<&Value>, after: Option<&Value>) -> Vec<FieldChange> {
    fn flatten(value: &Value, pointer: String, out: &mut BTreeMap<String, Value>) {
        match value {
            Value::Object(map) if !map.is_empty() => {
                for (key, child) in map {
                    flatten(child, format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1")), out);
                }
            }
            Value::Array(items) if !items.is_empty() => {
                for (index, child) in items.iter().enumerate() {
                    flatten(child, format!("{pointer}/{index}"), out);
                }
            }
            leaf => {
                out.insert(pointer, leaf.clone());
            }
        }
    }
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    if let Some(before) = before {
        flatten(before, String::new(), &mut old);
    }
    if let Some(after) = after {
        flatten(after, String::new(), &mut new);
    }
    let pointers: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    pointers
        .into_iter()
        .filter(|pointer| old.get(*pointer) != new.get(*pointer))
        .map(|pointer| FieldChange {
            pointer: pointer.clone(),
            before: old.get(pointer).cloned(),
            after: new.get(pointer).cloned(),
        })
        .collect()
}

const MAX_KINDS: &[&str] = &["hard_max", "recommended_max", "aggregate_max", "item_count_max", "payload_bytes_max"];
const MIN_KINDS: &[&str] = &["hard_min", "item_count_min"];
const UNMAPPABLE: &[&str] = &["unknown", "unmapped", "unspecified_characters"];

fn flag(delta: &FactDelta, flags: &mut BTreeSet<Flag>) {
    let mut push = |kind: FlagKind, detail: String| {
        flags.insert(Flag { kind, fact: delta.id.clone(), detail });
    };
    let is_constraint = delta.array == "constraints";
    if is_constraint && delta.change == ChangeKind::Removed {
        push(FlagKind::RemovedConstraint, "constraint removed".to_string());
    }
    let state = |record: &Option<Value>| {
        record
            .as_ref()
            .and_then(|record| record.pointer("/knowledge/state"))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    if state(&delta.after).as_deref() == Some("conflicting") && state(&delta.before).as_deref() != Some("conflicting") {
        push(FlagKind::ConflictingEvidence, format!("now conflicting (was {})", state(&delta.before).unwrap_or_else(|| "absent".to_string())));
    }
    if is_constraint
        && delta.change == ChangeKind::Changed
        && let (Some(before), Some(after)) = (&delta.before, &delta.after)
    {
        let kind = after["kind"].as_str().unwrap_or_default();
        if let (Some(old), Some(new)) = (before["value"].as_u64(), after["value"].as_u64()) {
            let looser = (MAX_KINDS.contains(&kind) && new > old) || (MIN_KINDS.contains(&kind) && new < old);
            if looser && before["kind"] == after["kind"] {
                push(FlagKind::RaisedLimit, format!("{kind} {old} -> {new}"));
            }
        }
        for field in ["unit", "measurement_stage"] {
            if before[field] != after[field] {
                push(FlagKind::ChangedUnit, format!("{field} {} -> {}", before[field], after[field]));
            }
        }
    }
    for field in &delta.typed_changes {
        let text = |value: &Option<Value>| value.as_ref().and_then(Value::as_str).map(str::to_string);
        let (old, new) = (text(&field.before), text(&field.after));
        if field.pointer.ends_with("/support") {
            let supported = |value: &Option<String>| matches!(value.as_deref(), Some("supported" | "conditional"));
            let unsupported = |value: &Option<String>| value.as_deref() == Some("unsupported");
            if (supported(&old) && unsupported(&new)) || (unsupported(&old) && supported(&new)) {
                push(
                    FlagKind::SupportReversal,
                    format!("{} {} -> {}", field.pointer, old.clone().unwrap_or_default(), new.clone().unwrap_or_default()),
                );
            }
        }
        if !field.pointer.starts_with("/knowledge/")
            && new.as_deref().is_some_and(|value| UNMAPPABLE.contains(&value))
            && old != new
        {
            push(FlagKind::NewUnmappableValue, format!("{} is now {}", field.pointer, new.unwrap_or_default()));
        }
    }
    if delta.change == ChangeKind::Added
        && let Some(after) = &delta.after
    {
        let mut leaves = Vec::new();
        for field in field_changes(None, Some(after)) {
            if !field.pointer.starts_with("/knowledge/")
                && let Some(value) = field.after.as_ref().and_then(Value::as_str)
                && UNMAPPABLE.contains(&value)
            {
                leaves.push(format!("{} is {value}", field.pointer));
            }
        }
        if !leaves.is_empty() {
            push(FlagKind::NewUnmappableValue, leaves.join("; "));
        }
    }
}

fn source_deltas(previous: Option<&AcceptedDocument>, candidate: &AcceptedDocument) -> Vec<SourceDelta> {
    if previous.is_none() {
        return Vec::new();
    }
    let before = records_by_id(previous, "sources");
    let after = records_by_id(Some(candidate), "sources");
    let citations: BTreeMap<String, Vec<String>> = {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for fact in candidate.validated.fact_records() {
            let ids = fact
                .record
                .pointer("/knowledge/evidence")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str);
            for source in ids {
                map.entry(source.to_string()).or_default().push(fact.id.to_string());
            }
        }
        map
    };
    let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    ids.into_iter()
        .filter(|id| before.get(*id) != after.get(*id))
        .map(|id| {
            let (old, new) = (before.get(id), after.get(id));
            let change = match (old, new) {
                (None, _) => ChangeKind::Added,
                (_, None) => ChangeKind::Removed,
                _ => ChangeKind::Changed,
            };
            let location = |record: Option<&Value>| {
                record.map(|record| (record.get("url").cloned(), record.get("location").cloned()))
            };
            let mut cited_by = citations.get(id).cloned().unwrap_or_default();
            cited_by.sort();
            cited_by.dedup();
            SourceDelta {
                id: id.clone(),
                change,
                same_location: change == ChangeKind::Changed && location(old) == location(new),
                field_changes: if change == ChangeKind::Changed { field_changes(old, new) } else { Vec::new() },
                cited_by,
            }
        })
        .collect()
}

fn gap_deltas(previous: Option<&AcceptedDocument>, candidate: &AcceptedDocument) -> Vec<GapDelta> {
    if previous.is_none() {
        return Vec::new();
    }
    let before = records_by_id(previous, "gaps");
    let after = records_by_id(Some(candidate), "gaps");
    let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    ids.into_iter()
        .filter(|id| before.get(*id) != after.get(*id))
        .map(|id| {
            let (old, new) = (before.get(id), after.get(id));
            GapDelta {
                id: id.clone(),
                change: match (old, new) {
                    (None, _) => ChangeKind::Added,
                    (_, None) => ChangeKind::Removed,
                    _ => ChangeKind::Changed,
                },
                field_changes: field_changes(old, new),
            }
        })
        .collect()
}
