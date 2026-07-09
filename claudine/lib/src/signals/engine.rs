//! Generic declarative detection engine over the compiled provider tables.
//!
//! Ratified semantics (design §Record grammar / §Version-range selection):
//!
//! - **Per-SIGNAL-KIND first-match-wins.** Within one provider × source
//!   group, the highest-priority matching record fires PER KIND, so one
//!   payload can co-fire different kinds (claude `init` carries both
//!   `model_resolved` and `auth_kind_detected`) while same-kind fallbacks
//!   stay mutually exclusive (`total_cost_usd` beats legacy `cost_usd`).
//! - **Non-declarative records are skipped here.** Bespoke emitters construct
//!   [`SignalEvent`]s directly and feed the same sink; they carry no
//!   declarative match fields.
//! - **Version-range selection.** While the provider version is unknown the
//!   candidate set is the UNION of all records (version-scoped included);
//!   once [`SignalEngine::observe_provider_version`] sees a parsable
//!   version, candidates narrow to records whose inclusive `since`/`until`
//!   bounds admit it.

use claudine_catalog_types::{
    DetectionMode, DetectionRecord, MatchOp, ProviderSignalTable, SignalEvent, SignalKind,
    SignalSource,
};
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;
use tracing::{debug, warn};

use super::event_builder::{build_event, resolve_extractions};
use super::path::walk;
use super::sink::SignalContext;
use super::version::{self, ParsedVersion};

/// One declarative record with its regex compiled once at engine
/// construction.
struct CompiledRecord {
    record: &'static DetectionRecord,
    regex: Option<Regex>,
}

/// All candidate records for one (source, kind) pair, priority-sorted.
struct KindGroup {
    source: SignalSource,
    kind: SignalKind,
    records: Vec<CompiledRecord>,
}

/// One fired record with its extraction-resolution outcome, as reported by
/// [`SignalEngine::observe_detailed`].
pub struct ReplayObservation {
    pub record: &'static DetectionRecord,
    /// Extraction `field` names that resolved on the payload, in extraction
    /// declaration order; sites that were absent, `null`, or failed unit
    /// conversion are omitted.
    pub resolved_fields: Vec<&'static str>,
}

/// Per-run detection engine for one provider's compiled table.
pub struct SignalEngine {
    groups: Vec<KindGroup>,
    /// `None` → union mode: version-scoped records stay candidates.
    observed_version: Option<ParsedVersion>,
}

impl SignalEngine {
    /// Compile `table` into priority-sorted per-(source, kind) groups.
    ///
    /// Only declarative records enter the groups (bespoke records carry no
    /// match fields). Regex compilation cannot fail for a
    /// table the generator validated — the impossible-error branch skips the
    /// record with a `warn!` instead of panicking.
    pub fn new(table: &'static ProviderSignalTable) -> Self {
        let mut groups: Vec<KindGroup> = Vec::new();
        for record in table.records {
            if record.mode != DetectionMode::Declarative {
                continue;
            }
            let regex = match (record.op, record.value) {
                (Some(MatchOp::Regex), Some(pattern)) => match Regex::new(pattern) {
                    Ok(regex) => Some(regex),
                    Err(error) => {
                        warn!(
                            record = record.id,
                            pattern,
                            %error,
                            "regex failed to compile despite generate-time validation; skipping record"
                        );
                        continue;
                    }
                },
                _ => None,
            };
            let group = match groups
                .iter_mut()
                .find(|g| g.source == record.source && g.kind == record.kind)
            {
                Some(group) => group,
                None => {
                    groups.push(KindGroup {
                        source: record.source,
                        kind: record.kind,
                        records: Vec::new(),
                    });
                    groups.last_mut().expect("group was just pushed")
                }
            };
            group.records.push(CompiledRecord { record, regex });
        }
        for group in &mut groups {
            group.records.sort_by_key(|r| r.record.priority);
        }
        Self {
            groups,
            observed_version: None,
        }
    }

    /// Evaluate one payload from `source`, returning at most one event per
    /// signal kind (per-kind first-match-wins). Drops the supplementary
    /// context — use [`observe_with_context`](Self::observe_with_context) when
    /// the caller feeds a sink.
    pub fn observe(&mut self, source: SignalSource, payload: &Value) -> Vec<SignalEvent> {
        self.observe_with_context(source, payload)
            .into_iter()
            .map(|(event, _context)| event)
            .collect()
    }

    /// [`observe`](Self::observe) with each event's supplementary context map
    /// (extraction fields with no variant slot). The sink path uses this so no
    /// resolved research is dropped (more-struture Cat 1B).
    pub(crate) fn observe_with_context(
        &mut self,
        source: SignalSource,
        payload: &Value,
    ) -> Vec<(SignalEvent, SignalContext)> {
        self.fired_records(source, payload)
            .into_iter()
            .filter_map(|record| build_event(record, payload))
            .collect()
    }

    /// The record-selection half of [`observe`](Self::observe), split out so
    /// tests can assert WHICH record won when two records of the same kind
    /// would build identical events.
    pub(crate) fn fired_records(
        &self,
        source: SignalSource,
        payload: &Value,
    ) -> Vec<&'static DetectionRecord> {
        self.groups
            .iter()
            .filter(|group| group.source == source)
            .filter_map(|group| {
                group
                    .records
                    .iter()
                    .find(|r| self.version_admits(r.record) && matches(r, payload))
                    .map(|r| r.record)
            })
            .collect()
    }

    /// [`observe`](Self::observe)'s record selection with per-record
    /// extraction-resolution detail instead of built events.
    ///
    /// Exists for the `claudine signals check` fixture-replay harness (and
    /// E5 bespoke-emitter diagnostics): the harness must know WHICH records
    /// fired and which extraction sites resolved, not just the resulting
    /// events. Production streaming uses [`observe`](Self::observe).
    pub fn observe_detailed(
        &self,
        source: SignalSource,
        payload: &Value,
    ) -> Vec<ReplayObservation> {
        self.fired_records(source, payload)
            .into_iter()
            .map(|record| ReplayObservation {
                resolved_fields: resolve_extractions(record, payload).field_names(),
                record,
            })
            .collect()
    }

    /// Narrow record selection to `version`. Unparsable versions leave the
    /// engine in union mode.
    pub fn observe_provider_version(&mut self, version: &str) {
        match version::parse(version) {
            Some(parsed) => {
                debug!(version, "provider version observed; narrowing record selection");
                self.observed_version = Some(parsed);
            }
            None => {
                debug!(version, "provider version unparsable; staying in union mode");
            }
        }
    }

    fn version_admits(&self, record: &DetectionRecord) -> bool {
        match &self.observed_version {
            None => true,
            Some(observed) => version::admits(observed, record.since, record.until),
        }
    }
}

fn matches(compiled: &CompiledRecord, payload: &Value) -> bool {
    let record = compiled.record;
    let (Some(path), Some(op)) = (record.match_path, record.op) else {
        return false;
    };
    let walked = walk(payload, path);
    // `exists` = present AND non-null; unlike the value ops it DOES fire on
    // objects and arrays.
    if op == MatchOp::Exists {
        return walked.is_some_and(|value| !value.is_null());
    }
    let Some(candidate) = walked.and_then(coerce_scalar) else {
        return false;
    };
    match op {
        MatchOp::Eq => record.value == Some(candidate.as_ref()),
        MatchOp::In => record.values.contains(&candidate.as_ref()),
        MatchOp::SubstringCi => record
            .value
            .is_some_and(|needle| candidate.to_lowercase().contains(&needle.to_lowercase())),
        MatchOp::Regex => compiled
            .regex
            .as_ref()
            .is_some_and(|regex| regex.is_match(&candidate)),
        MatchOp::Exists => unreachable!("handled above"),
    }
}

/// Scalar→string coercion for match evaluation — the pinned rule:
///
/// - string → compared directly;
/// - number → its serde_json `Display` serialization (integers without a
///   decimal point, `-32004` keeps its sign);
/// - boolean → `true` / `false`;
/// - null, objects, and arrays → NO value: nothing matches for
///   `eq`/`in`/`substring_ci`/`regex` (`exists` is evaluated before
///   coercion and fires on non-null objects/arrays).
fn coerce_scalar(value: &Value) -> Option<Cow<'_, str>> {
    match value {
        Value::String(s) => Some(Cow::Borrowed(s)),
        Value::Number(n) => Some(Cow::Owned(n.to_string())),
        Value::Bool(b) => Some(Cow::Borrowed(if *b { "true" } else { "false" })),
        Value::Null | Value::Object(_) | Value::Array(_) => None,
    }
}
