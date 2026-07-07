//! Fired detection record + payload → typed [`SignalEvent`].
//!
//! The exhaustive [`build_from_fields`] match is the CANONICAL mapping from
//! corpus extraction `field` names to `SignalEvent` variant fields. Corpus
//! fields the match does not consume are supplementary research evidence
//! (e.g. `cache_read`, `cost`, `provider`, `session_id`) — they resolve but
//! never reach the frozen variants, and are surfaced at debug level.

use chrono::{DateTime, Utc};
use claudine_catalog_types::{
    DetectionRecord, DriftObservation, ExtractionSpec, Quantity, SignalEvent, SignalKind, Unit,
    UsageWindow,
};
use serde_json::Value;
use tracing::{debug, warn};

use super::path::walk;

/// One extraction site's value, converted per the declared unit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Timestamp(DateTime<Utc>),
    Measure(Quantity),
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

    /// Missing or unrecognized window spellings collapse to
    /// [`UsageWindow::Unknown`] instead of suppressing the event —
    /// `Unknown` exists precisely for cap signals whose payload does not
    /// name a window (codex/kilo/opencode/pi caps carry none).
    fn take_window(&mut self, name: &str) -> UsageWindow {
        match self.take_string(name).as_deref() {
            Some("five_hour") => UsageWindow::FiveHour,
            Some("seven_day") => UsageWindow::SevenDay,
            Some("seven_day_opus") => UsageWindow::SevenDayOpus,
            Some("monthly") => UsageWindow::Monthly,
            // Observed claude spelling "usage" does not name a window;
            // SDK-declared members without a taxonomy slot (seven_day_sonnet,
            // overage, ...) land here too.
            _ => UsageWindow::Unknown,
        }
    }

    /// Resolved field names, in extraction declaration order.
    /// `pub(crate)` for [`SignalEngine::observe_detailed`]'s replay report.
    ///
    /// [`SignalEngine::observe_detailed`]: super::SignalEngine::observe_detailed
    pub(crate) fn field_names(&self) -> Vec<&'static str> {
        self.0.iter().map(|(field, _)| *field).collect()
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
        let Some(raw) = walk(payload, spec.path) else {
            debug!(
                record = record.id,
                field = spec.field,
                path = spec.path,
                "extraction path absent"
            );
            continue;
        };
        if raw.is_null() {
            debug!(record = record.id, field = spec.field, "extraction value is null");
            continue;
        }
        match convert(raw, spec) {
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

fn convert(raw: &Value, spec: &ExtractionSpec) -> Option<ResolvedValue> {
    match spec.unit {
        Some(Unit::UnixSeconds) => epoch_to_datetime(raw, 1.0),
        Some(Unit::UnixMillis) => epoch_to_datetime(raw, 1e3),
        Some(Unit::UnixNanos) => epoch_to_datetime(raw, 1e9),
        Some(Unit::Iso8601) => parse_iso8601(raw.as_str()?).map(ResolvedValue::Timestamp),
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

/// RFC 3339 first; naive fallbacks are interpreted as UTC. For sites whose
/// zone is `local`/`unspecified` that is an approximation — the corpus
/// flags those through `known_gaps` rather than the engine guessing a zone.
fn parse_iso8601(text: &str) -> Option<DateTime<Utc>> {
    if let Ok(at) = DateTime::parse_from_rfc3339(text) {
        return Some(at.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(text, format) {
            return Some(naive.and_utc());
        }
    }
    None
}

/// Build the typed event for a fired record. `None` when a required
/// (non-`Option`) variant field has no usable extraction — logged at `warn!`
/// because in practice those kinds are bespoke-only and a declarative record
/// reaching this state is a corpus bug.
pub(crate) fn build_event(record: &DetectionRecord, payload: &Value) -> Option<SignalEvent> {
    let mut fields = resolve_extractions(record, payload);
    let event = build_from_fields(record.kind, &mut fields);
    if event.is_none() {
        warn!(
            record = record.id,
            kind = ?record.kind,
            "record fired but a required payload field has no extraction; not emitting"
        );
    }
    let leftover = fields.field_names();
    if !leftover.is_empty() {
        debug!(
            record = record.id,
            ?leftover,
            "extraction fields with no SignalEvent slot (supplementary evidence)"
        );
    }
    event
}

/// The canonical `SignalKind` → variant assembly. Exhaustive: adding a
/// taxonomy member must force a compile error here.
fn build_from_fields(kind: SignalKind, f: &mut ResolvedFields) -> Option<SignalEvent> {
    let event = match kind {
        SignalKind::UsageCapApproaching => SignalEvent::UsageCapApproaching {
            window: f.take_window("window"),
            resets_at: f.take_datetime("resets_at"),
            remaining: f.take_quantity("remaining"),
            message: f.take_string("message"),
        },
        SignalKind::UsageCapped => SignalEvent::UsageCapped {
            window: f.take_window("window"),
            lifts_at: f.take_datetime("lifts_at"),
            remaining: f.take_quantity("remaining"),
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
