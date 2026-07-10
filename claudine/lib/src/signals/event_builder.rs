//! Fired detection record + payload → typed [`SignalEvent`].
//!
//! The exhaustive [`build_from_fields`] match is the CANONICAL mapping from
//! corpus extraction `field` names to `SignalEvent` variant fields. Corpus
//! fields the match does not consume are supplementary research evidence
//! (e.g. `cache_read`, `cost`, `provider`, `session_id`) — they resolve but
//! never reach the frozen variants, and are surfaced at debug level.

use chrono::{DateTime, Local, TimeZone, Utc};
use claudine_catalog_types::{
    CapScope, DetectionRecord, DriftObservation, ExtractStrategy, ExtractionSpec, Quantity,
    SignalEvent, SignalKind, Unit, Zone,
};
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};

use super::path::walk;
use super::sink::SignalContext;

/// One extraction site's value, converted per the declared unit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Timestamp(DateTime<Utc>),
    Measure(Quantity),
}

impl ResolvedValue {
    /// Flatten to a display string for the supplementary context map. The
    /// context is provenance, not a typed axis, so a `Measure`'s unit is
    /// dropped (the value alone is what a report or rule reads).
    fn into_display_string(self) -> String {
        match self {
            ResolvedValue::Text(s) => s,
            ResolvedValue::Number(n) => display_number(n),
            ResolvedValue::Bool(b) => b.to_string(),
            ResolvedValue::Timestamp(at) => at.to_rfc3339(),
            ResolvedValue::Measure(q) => display_number(q.value),
        }
    }
}

/// Field-name-keyed extraction results for one fired record.
///
/// Accessors REMOVE entries so the builder can report leftover
/// (supplementary / unmapped) fields after the variant is assembled.
#[derive(Debug, Default)]
pub(crate) struct ResolvedFields(Vec<(&'static str, ResolvedValue)>);

impl ResolvedFields {
    fn take(&mut self, name: &str) -> Option<ResolvedValue> {
        let index = self.0.iter().position(|(field, _)| *field == name)?;
        Some(self.0.remove(index).1)
    }

    /// Test-only inspection of resolved-but-unconsumed extraction values.
    #[cfg(test)]
    pub(crate) fn get(&self, name: &str) -> Option<&ResolvedValue> {
        self.0
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| value)
    }

    fn take_string(&mut self, name: &str) -> Option<String> {
        match self.take(name)? {
            ResolvedValue::Text(s) => Some(s),
            ResolvedValue::Number(n) => Some(display_number(n)),
            ResolvedValue::Bool(b) => Some(b.to_string()),
            other => {
                debug!(field = name, ?other, "extraction is not string-shaped");
                None
            }
        }
    }

    fn take_quantity(&mut self, name: &str) -> Option<Quantity> {
        match self.take(name)? {
            ResolvedValue::Measure(q) => Some(q),
            other => {
                debug!(field = name, ?other, "extraction has no measure unit");
                None
            }
        }
    }

    fn take_datetime(&mut self, name: &str) -> Option<DateTime<Utc>> {
        match self.take(name)? {
            ResolvedValue::Timestamp(at) => Some(at),
            other => {
                debug!(field = name, ?other, "extraction has no time unit");
                None
            }
        }
    }

    fn take_bool(&mut self, name: &str) -> Option<bool> {
        match self.take(name)? {
            ResolvedValue::Bool(b) => Some(b),
            ResolvedValue::Text(s) => s.parse().ok(),
            other => {
                debug!(field = name, ?other, "extraction is not bool-shaped");
                None
            }
        }
    }

    /// Integer accessor shared by the `u16`/`u32`/`u64` variant fields.
    ///
    /// Accepts a bare number, a `Measure` (some corpus counters carry a unit,
    /// e.g. kimi's `attempt` in `requests`), or a numeric string.
    fn take_integer(&mut self, name: &str) -> Option<u64> {
        let value = match self.take(name)? {
            ResolvedValue::Number(n) => n,
            ResolvedValue::Measure(q) => q.value,
            ResolvedValue::Text(s) => s.trim().parse::<f64>().ok()?,
            other => {
                debug!(field = name, ?other, "extraction is not integer-shaped");
                return None;
            }
        };
        (value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64)
            .then_some(value as u64)
    }

    fn take_u16(&mut self, name: &str) -> Option<u16> {
        self.take_integer(name).and_then(|n| u16::try_from(n).ok())
    }

    fn take_u32(&mut self, name: &str) -> Option<u32> {
        self.take_integer(name).and_then(|n| u32::try_from(n).ok())
    }

    /// A `model` extraction naming a specific model tier narrows the cap
    /// scope; its absence means the cap is account-wide ([`CapScope::All`]).
    /// Claude's per-token cap records pin the model via `ExtractStrategy::
    /// Literal`; providers whose cap payload names no model resolve to `All`.
    fn take_cap_scope(&mut self, name: &str) -> CapScope {
        match self.take_string(name) {
            Some(model) => CapScope::Specific(model.into()),
            None => CapScope::All,
        }
    }

    /// Resolved field names, in extraction declaration order.
    /// `pub(crate)` for [`SignalEngine::observe_detailed`]'s replay report.
    ///
    /// [`SignalEngine::observe_detailed`]: super::SignalEngine::observe_detailed
    pub(crate) fn field_names(&self) -> Vec<&'static str> {
        self.0.iter().map(|(field, _)| *field).collect()
    }

    /// Consume the still-unconsumed fields into a supplementary context map.
    ///
    /// Call after [`build_from_fields`] has taken every field with a variant
    /// slot; whatever remains had no home in the frozen taxonomy and is held
    /// as observation context instead of being dropped.
    fn into_context(self) -> SignalContext {
        self.0
            .into_iter()
            .map(|(field, value)| (field, value.into_display_string()))
            .collect()
    }
}

/// `f64` display without a trailing `.0` for whole numbers, matching the
/// serde_json integer serialization used by match coercion.
fn display_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

/// Resolve every extraction site of `record` against `payload`.
///
/// Absent or `null` paths resolve to nothing (debug-logged); values that
/// cannot convert to the declared unit are likewise dropped.
pub(crate) fn resolve_extractions(record: &DetectionRecord, payload: &Value) -> ResolvedFields {
    let mut fields = ResolvedFields::default();
    for spec in record.extractions {
        let Some(raw) = resolve_source(payload, &spec.source) else {
            debug!(
                record = record.id,
                field = spec.field,
                source = ?spec.source,
                "extraction source produced nothing"
            );
            continue;
        };
        if raw.is_null() {
            debug!(record = record.id, field = spec.field, "extraction value is null");
            continue;
        }
        match convert(&raw, spec) {
            Some(value) => fields.0.push((spec.field, value)),
            None => debug!(
                record = record.id,
                field = spec.field,
                unit = ?spec.unit,
                "extraction value does not convert to declared unit"
            ),
        }
    }
    fields
}

/// Read the raw value for one extraction strategy. `Path` walks the payload;
/// `Literal` yields a constant; `Regex` / `StartStopTokens` carve a substring
/// out of the string at their `path` and yield it as a JSON string.
fn resolve_source(payload: &Value, source: &ExtractStrategy) -> Option<Value> {
    match source {
        ExtractStrategy::Path(path) => walk(payload, path).cloned(),
        ExtractStrategy::Literal(value) => Some(Value::String((*value).to_string())),
        ExtractStrategy::Regex { path, pattern } => {
            let haystack = walk(payload, path)?.as_str()?;
            let re = Regex::new(pattern).ok()?;
            let caps = re.captures(haystack)?;
            // Capture group 1 when present, else the whole match.
            let hit = caps.get(1).or_else(|| caps.get(0))?;
            Some(Value::String(hit.as_str().to_string()))
        }
        ExtractStrategy::StartStopTokens { path, start, stop } => {
            let haystack = walk(payload, path)?.as_str()?;
            let after = haystack.split_once(start)?.1;
            let inner = after.split_once(stop)?.0;
            Some(Value::String(inner.to_string()))
        }
    }
}

fn convert(raw: &Value, spec: &ExtractionSpec) -> Option<ResolvedValue> {
    match spec.unit {
        Some(Unit::UnixSeconds) => epoch_to_datetime(raw, 1.0),
        Some(Unit::UnixMillis) => epoch_to_datetime(raw, 1e3),
        Some(Unit::UnixNanos) => epoch_to_datetime(raw, 1e9),
        Some(Unit::Iso8601) => parse_iso8601(raw.as_str()?, spec.zone).map(ResolvedValue::Timestamp),
        Some(
            unit @ (Unit::DurationSecs
            | Unit::DurationMillis
            | Unit::Percent
            | Unit::Tokens
            | Unit::Requests
            | Unit::Usd),
        ) => Some(ResolvedValue::Measure(Quantity {
            value: lenient_f64(raw)?,
            unit,
        })),
        None => match raw {
            Value::String(s) => Some(ResolvedValue::Text(s.clone())),
            Value::Number(n) => Some(ResolvedValue::Number(n.as_f64()?)),
            Value::Bool(b) => Some(ResolvedValue::Bool(*b)),
            _ => None,
        },
    }
}

/// Numeric value from a JSON number or a numeric string (`"429"`, `"1.25"`).
fn lenient_f64(raw: &Value) -> Option<f64> {
    match raw {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Epoch timestamps are absolute instants, so the declared `zone` never
/// changes the result — it is corpus documentation of the emitting side.
fn epoch_to_datetime(raw: &Value, per_second: f64) -> Option<ResolvedValue> {
    let seconds = lenient_f64(raw)? / per_second;
    let secs = seconds.floor();
    let nanos = ((seconds - secs) * 1e9).round() as u32;
    DateTime::<Utc>::from_timestamp(secs as i64, nanos).map(ResolvedValue::Timestamp)
}

/// RFC 3339 first (its embedded offset always wins). A naive timestamp is
/// interpreted per the declared `zone`: `Zone::Local` converts host-local →
/// UTC at ingest (storage is always UTC); every other zone — `Utc`,
/// `EmbeddedOffset` (with no offset actually present), `Unspecified`, or an
/// absent zone — assumes UTC. A `local` conversion that is ambiguous or
/// nonexistent (DST boundaries) falls back to the earliest candidate, then to
/// a UTC reading.
fn parse_iso8601(text: &str, zone: Option<Zone>) -> Option<DateTime<Utc>> {
    if let Ok(at) = DateTime::parse_from_rfc3339(text) {
        return Some(at.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(text, format) {
            return Some(match zone {
                Some(Zone::Local) => Local
                    .from_local_datetime(&naive)
                    .earliest()
                    .map(|at| at.with_timezone(&Utc))
                    .unwrap_or_else(|| naive.and_utc()),
                _ => naive.and_utc(),
            });
        }
    }
    None
}

/// Build the typed event for a fired record, plus a supplementary context
/// map of the extraction fields that have no variant slot.
///
/// `None` when a required (non-`Option`) variant field has no usable
/// extraction — logged at `warn!` because in practice those kinds are
/// bespoke-only and a declarative record reaching this state is a corpus bug.
///
/// Fields the variant does not consume (e.g. `provider`, `model`,
/// `session_id`) were previously debug-dropped; they now ride on the
/// observation as supplementary context (more-struture Cat 1B) so no resolved
/// research is silently lost.
pub(crate) fn build_event(
    record: &DetectionRecord,
    payload: &Value,
) -> Option<(SignalEvent, SignalContext)> {
    let mut fields = resolve_extractions(record, payload);
    let Some(event) = build_from_fields(record.kind, &mut fields) else {
        warn!(
            record = record.id,
            kind = ?record.kind,
            "record fired but a required payload field has no extraction; not emitting"
        );
        return None;
    };
    Some((event, fields.into_context()))
}

/// The canonical `SignalKind` → variant assembly. Exhaustive: adding a
/// taxonomy member must force a compile error here.
fn build_from_fields(kind: SignalKind, f: &mut ResolvedFields) -> Option<SignalEvent> {
    let event = match kind {
        SignalKind::UsageCapApproaching => SignalEvent::UsageCapApproaching {
            model: f.take_cap_scope("model"),
            timeframe: f.take_quantity("timeframe"),
            // Approaching with no counted remaining is known-low, exact %
            // unknown → `None` (design ruling).
            remaining: f.take_quantity("remaining"),
            resets_at: f.take_datetime("resets_at"),
            message: f.take_string("message"),
        },
        SignalKind::UsageCapped => SignalEvent::UsageCapped {
            model: f.take_cap_scope("model"),
            timeframe: f.take_quantity("timeframe"),
            // A fired cap means zero capacity left: default to `Some(0%)`
            // when the payload carries no explicit remaining (design ruling).
            remaining: f.take_quantity("remaining").or(Some(Quantity {
                value: 0.0,
                unit: Unit::Percent,
            })),
            lifts_at: f.take_datetime("lifts_at"),
            message: f.take_string("message"),
        },
        SignalKind::RateLimited => SignalEvent::RateLimited {
            status_code: f.take_u16("status_code"),
            reset_at: f.take_datetime("reset_at"),
            retry_after: f.take_quantity("retry_after"),
            message: f.take_string("message"),
        },
        SignalKind::ProviderOverloaded => SignalEvent::ProviderOverloaded {
            status_code: f.take_u16("status_code"),
            message: f.take_string("message"),
        },
        SignalKind::RetriesExhausted => SignalEvent::RetriesExhausted {
            status_code: f.take_u16("status_code"),
            attempts: f.take_u32("attempts"),
            message: f.take_string("message"),
        },
        SignalKind::NoFunds => SignalEvent::NoFunds {
            message: f.take_string("message"),
            top_up_url: f.take_string("top_up_url"),
        },
        SignalKind::AuthInvalid => SignalEvent::AuthInvalid {
            auth_kind: f.take_string("auth_kind"),
            expired: f.take_bool("expired"),
            message: f.take_string("message"),
        },
        SignalKind::AuthKindDetected => SignalEvent::AuthKindDetected {
            auth_kind: f.take_string("auth_kind")?,
        },
        SignalKind::PermissionDeniedRead => SignalEvent::PermissionDeniedRead {
            path: f.take_string("path"),
            message: f.take_string("message"),
        },
        SignalKind::PermissionDeniedWrite => SignalEvent::PermissionDeniedWrite {
            path: f.take_string("path"),
            message: f.take_string("message"),
        },
        SignalKind::TokensConsumed => SignalEvent::TokensConsumed {
            input: f.take_quantity("input"),
            output: f.take_quantity("output"),
            total: f.take_quantity("total"),
            cache_read: f.take_quantity("cache_read"),
            cache_write: f.take_quantity("cache_write"),
            cost: f.take_quantity("cost"),
            reasoning: f.take_quantity("reasoning"),
        },
        SignalKind::ModelResolved => SignalEvent::ModelResolved {
            requested: f.take_string("requested"),
            resolved: f.take_string("resolved")?,
        },
        SignalKind::ModelFallback => SignalEvent::ModelFallback {
            from: f.take_string("from"),
            to: f.take_string("to")?,
            message: f.take_string("message"),
        },
        // Bespoke-only kind (wrapper catalog comparison) — no detection
        // records exist, so this arm only keeps the match exhaustive. A
        // single extraction path carries at most one id per list.
        SignalKind::ModelCatalogDrift => SignalEvent::ModelCatalogDrift {
            unexpected: f.take_string("unexpected").map(|id| vec![id]).unwrap_or_default(),
            missing: f.take_string("missing").map(|id| vec![id]).unwrap_or_default(),
            observed_via: match f.take_string("observed_via").as_deref() {
                Some("resolved_model") => DriftObservation::ResolvedModel,
                _ => DriftObservation::Listing,
            },
        },
        SignalKind::ProviderVersion => SignalEvent::ProviderVersion {
            version: f.take_string("version")?,
        },
        SignalKind::GenerationRetried => SignalEvent::GenerationRetried {
            attempt: f.take_u32("attempt"),
            max_attempts: f.take_u32("max_attempts"),
            wait: f.take_quantity("wait"),
            error_type: f.take_string("error_type"),
            status_code: f.take_u16("status_code"),
            message: f.take_string("message"),
        },
        SignalKind::StalledGeneration => SignalEvent::StalledGeneration {
            message: f.take_string("message"),
        },
        SignalKind::RepeatedStreamError => SignalEvent::RepeatedStreamError {
            count: f.take_u32("count")?,
        },
        SignalKind::Timeout => SignalEvent::Timeout {
            message: f.take_string("message"),
        },
        SignalKind::StepTimeout => SignalEvent::StepTimeout {
            message: f.take_string("message"),
        },
        SignalKind::ExitExpression => SignalEvent::ExitExpression {
            pattern: f.take_string("pattern")?,
            scope: f.take_string("scope"),
        },
        SignalKind::RunawayRepetition => SignalEvent::RunawayRepetition {
            cycle_len: f.take_u32("cycle_len")?,
            repeats: f.take_u32("repeats")?,
        },
        SignalKind::RunawayVolume => SignalEvent::RunawayVolume {
            lines: f.take_integer("lines")?,
            bytes: f.take_integer("bytes")?,
        },
        SignalKind::UnsupportedProtocolVersion => SignalEvent::UnsupportedProtocolVersion {
            version: f.take_string("version")?,
            // A single extraction path cannot carry the supported-set list;
            // this kind is wrapper-bespoke in practice (kimi gap entry).
            supported: vec![f.take_string("supported")?],
        },
        SignalKind::TurnLimitReached => SignalEvent::TurnLimitReached {
            limit: f.take_u32("limit"),
            message: f.take_string("message"),
        },
        SignalKind::SessionTimeLimitReached => SignalEvent::SessionTimeLimitReached {
            limit: f.take_quantity("limit"),
        },
        SignalKind::Interrupted => SignalEvent::Interrupted {
            message: f.take_string("message"),
        },
        SignalKind::SessionTainted => SignalEvent::SessionTainted {
            cause: f.take_string("cause")?,
        },
        SignalKind::HumanInputRequested => SignalEvent::HumanInputRequested {
            prompt: f.take_string("prompt"),
        },
        SignalKind::SessionResumable => SignalEvent::SessionResumable {
            session_id: f.take_string("session_id"),
        },
    };
    Some(event)
}

#[cfg(test)]
mod strategy_tests {
    use super::resolve_source;
    use claudine_catalog_types::ExtractStrategy;
    use serde_json::{Value, json};

    #[test]
    fn literal_yields_the_constant_ignoring_payload() {
        let got = resolve_source(&json!({}), &ExtractStrategy::Literal("opus"));
        assert_eq!(got, Some(Value::String("opus".into())));
    }

    #[test]
    fn regex_captures_group_one_from_the_string_at_path() {
        let payload = json!({ "msg": "reset in 42s, retry later" });
        let source = ExtractStrategy::Regex {
            path: "msg",
            pattern: r"reset in (\d+)s",
        };
        assert_eq!(
            resolve_source(&payload, &source),
            Some(Value::String("42".into()))
        );
    }

    #[test]
    fn start_stop_tokens_carve_the_inner_substring() {
        let payload = json!({ "msg": "window=[seven_day_opus] done" });
        let source = ExtractStrategy::StartStopTokens {
            path: "msg",
            start: "window=[",
            stop: "]",
        };
        assert_eq!(
            resolve_source(&payload, &source),
            Some(Value::String("seven_day_opus".into()))
        );
    }

    #[test]
    fn strategies_over_a_missing_or_non_string_path_yield_none() {
        let payload = json!({ "n": 7 });
        let re = ExtractStrategy::Regex { path: "n", pattern: r"(\d+)" };
        assert_eq!(resolve_source(&payload, &re), None);
        let missing = ExtractStrategy::Path("absent");
        assert_eq!(resolve_source(&payload, &missing), None);
    }
}

#[cfg(test)]
mod timezone_tests {
    use super::parse_iso8601;
    use chrono::{Local, TimeZone};
    use claudine_catalog_types::Zone;

    /// An embedded offset always wins, regardless of the declared zone.
    #[test]
    fn rfc3339_offset_overrides_declared_zone() {
        let parsed = parse_iso8601("2026-04-16T04:18:56+02:00", Some(Zone::Local)).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-04-16T02:18:56+00:00");
    }

    /// A naive `zone: utc` timestamp is read as UTC (the default behavior for
    /// every zone except `local`).
    #[test]
    fn naive_utc_zone_reads_as_utc() {
        let parsed = parse_iso8601("2026-04-16T04:18:56", Some(Zone::Utc)).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-04-16T04:18:56+00:00");
    }

    /// A naive `zone: local` timestamp is converted host-local → UTC at
    /// ingest, matching an explicit `Local` interpretation of the same wall
    /// clock (equals the UTC reading only on a UTC host).
    #[test]
    fn naive_local_zone_converts_host_local_to_utc() {
        let parsed = parse_iso8601("2026-04-16T04:18:56", Some(Zone::Local)).unwrap();
        let expected = Local
            .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
            .earliest()
            .expect("wall clock is representable in the host zone")
            .to_utc();
        assert_eq!(parsed, expected);
    }
}
