//! Fixture-driven semantics tests for the detection engine and event
//! builder. The corpus fixtures under `docs/research/signals/fixtures/` are
//! the ground truth for payload shapes.

use chrono::DateTime;
use claudine_catalog_types::{CapScope, Quantity, SignalEvent, SignalKind, SignalSource, Unit};
use serde_json::Value;

use super::detection_table;
use super::engine::SignalEngine;
use super::event_builder::{ResolvedValue, resolve_extractions};

const CLAUDE_INIT: &str =
    include_str!("../../../docs/research/signals/fixtures/claude/init-model.jsonl");
const CLAUDE_RESULT_COSTS: &str =
    include_str!("../../../docs/research/signals/fixtures/claude/result-usage-cost-fields.jsonl");
const CLAUDE_THROTTLED: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-throttled-message.jsonl"
);
const CLAUDE_APPROACHING: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-info-approaching.jsonl"
);
const CLAUDE_WARNING_SEVEN_DAY: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-info-allowed-warning-seven-day.jsonl"
);
const CLAUDE_SEVEN_DAY_OPUS: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-info-seven-day-opus.jsonl"
);
const CLAUDE_BILLING_SYNTHETIC: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/billing-error-synthetic-result.jsonl"
);
const KIMI_AUTH_EXPIRED: &str =
    include_str!("../../../docs/research/signals/fixtures/kimi/wire-auth-expired.jsonl");
const KIMI_STEP_RETRY: &str =
    include_str!("../../../docs/research/signals/fixtures/kimi/step-retry-rate-limit.jsonl");

fn engine(slug: &str) -> SignalEngine {
    SignalEngine::new(detection_table(slug).expect("provider table"))
}

fn line(fixture: &str, index: usize) -> Value {
    serde_json::from_str(fixture.lines().nth(index).expect("fixture line"))
        .expect("fixture line is JSON")
}

/// Per-KIND first-match-wins: the claude `init` frame carries the resolved
/// model AND the auth kind, so both kinds co-fire on one payload.
#[test]
fn claude_init_cofires_model_resolved_and_auth_kind() {
    let mut engine = engine("claude");
    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_INIT, 0));

    let kinds: Vec<SignalKind> = events.iter().map(SignalEvent::kind).collect();
    assert!(kinds.contains(&SignalKind::ModelResolved), "kinds: {kinds:?}");
    assert!(kinds.contains(&SignalKind::AuthKindDetected), "kinds: {kinds:?}");
    assert!(events.contains(&SignalEvent::ModelResolved {
        requested: None,
        resolved: "claude-sonnet-4-20250514".to_string(),
    }));
    assert!(events.contains(&SignalEvent::AuthKindDetected {
        auth_kind: "ANTHROPIC_API_KEY".to_string(),
    }));
}

/// Same-kind fallback pair: with BOTH cost spellings present only the
/// higher-priority `total_cost_usd` record fires, and its extraction
/// carries the cost as a USD quantity.
#[test]
fn claude_cost_field_fallback_prefers_total_cost_usd() {
    let engine = engine("claude");
    // The fixture proves each spelling on its own line; merge them to
    // exercise the both-present tiebreak the record priorities exist for.
    let mut payload = line(CLAUDE_RESULT_COSTS, 1);
    payload["cost_usd"] = serde_json::json!(0.0042);

    let fired = engine.fired_records(SignalSource::Stream, &payload);
    let tokens: Vec<&str> = fired
        .iter()
        .filter(|r| r.kind == SignalKind::TokensConsumed)
        .map(|r| r.id)
        .collect();
    assert_eq!(tokens, ["stream-tokens_consumed-total_cost_usd"]);

    let record = fired
        .iter()
        .find(|r| r.id == "stream-tokens_consumed-total_cost_usd")
        .unwrap();
    let fields = resolve_extractions(record, &payload);
    assert_eq!(
        fields.get("cost"),
        Some(&ResolvedValue::Measure(Quantity {
            value: 0.0042,
            unit: Unit::Usd,
        }))
    );
}

/// The `cache_read` and `cost` extractions now land on the typed
/// `TokensConsumed` event instead of the debug-drop path (more-struture
/// Cat 1A).
#[test]
fn tokens_consumed_event_carries_cache_read_and_cost() {
    let mut engine = engine("claude");
    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_RESULT_COSTS, 1));
    let carried = events.iter().any(|event| {
        matches!(
            event,
            SignalEvent::TokensConsumed {
                cache_read: Some(Quantity { value: cr, unit: Unit::Tokens }),
                cost: Some(Quantity { value: c, unit: Unit::Usd }),
                ..
            } if *cr == 200.0 && *c == 0.0042
        )
    });
    assert!(
        carried,
        "TokensConsumed should carry cache_read and cost; got {events:?}"
    );
}

/// Extraction fields with no variant slot (`provider`, `model`) ride on the
/// observation as supplementary context instead of being debug-dropped
/// (more-struture Cat 1B).
#[test]
fn unmapped_fields_ride_as_supplementary_context() {
    let mut engine = engine("opencode");
    let payload = serde_json::json!({
        "kind": "RateLimited",
        "provider_error": "429 Too Many Requests",
        "provider_id": "zai-coding-plan",
        "model_id": "glm-5.1",
    });
    let observations = engine.observe_with_context(SignalSource::StderrPromoted, &payload);
    let context = observations
        .iter()
        .find(|(event, _)| matches!(event, SignalEvent::RateLimited { .. }))
        .map(|(_, context)| context)
        .expect("rate_limited fires on the stderr-promoted 429 shape");
    assert_eq!(context.get("provider"), Some(&"zai-coding-plan".to_string()));
    assert_eq!(context.get("model"), Some(&"glm-5.1".to_string()));
}

/// Coercion pins: eq against a JSON boolean, regex against JSON numbers
/// (`-32004` keeps its sign, `429` serializes without a decimal point).
#[test]
fn coercion_matches_booleans_and_numbers() {
    let mut claude = engine("claude");
    let events = claude.observe(SignalSource::Stream, &line(CLAUDE_THROTTLED, 0));
    assert!(
        events.iter().any(|e| e.kind() == SignalKind::RateLimited),
        "is_throttled: true must satisfy eq \"true\""
    );

    let mut kimi = engine("kimi");
    let events = kimi.observe(SignalSource::Stream, &line(KIMI_AUTH_EXPIRED, 1));
    assert!(
        events.iter().any(|e| e.kind() == SignalKind::AuthInvalid),
        "error.code -32004 must satisfy regex ^-32004$"
    );

    let events = kimi.observe(SignalSource::Stream, &line(KIMI_STEP_RETRY, 0));
    let kinds: Vec<SignalKind> = events.iter().map(SignalEvent::kind).collect();
    assert!(
        kinds.contains(&SignalKind::RateLimited),
        "status_code 429 must satisfy regex ^429$: {kinds:?}"
    );
    // Different kind on the same payload co-fires (per-kind rule).
    assert!(kinds.contains(&SignalKind::GenerationRetried), "kinds: {kinds:?}");
}

/// `exists` = present AND non-null; a present object DOES fire.
#[test]
fn exists_requires_present_non_null() {
    let engine = engine("claude");
    let fires = |payload: Value| {
        engine
            .fired_records(SignalSource::Stream, &payload)
            .iter()
            .any(|r| r.kind == SignalKind::TokensConsumed)
    };

    assert!(fires(serde_json::json!({"total_cost_usd": 0.01})));
    assert!(fires(serde_json::json!({"total_cost_usd": {"amount": 1}})));
    assert!(!fires(serde_json::json!({"total_cost_usd": null})));
    assert!(!fires(serde_json::json!({"other": 1})));
}

/// Version-range selection on the opencode usage-cap pair: union mode picks
/// the lower-priority-number legacy record; an observed version narrows to
/// the admitting record (bounds inclusive).
#[test]
fn opencode_usage_cap_pair_narrows_on_observed_version() {
    // LogClassification-shaped payload matching the stderr_promoted records.
    let payload = serde_json::json!({
        "kind": "UsageCap",
        "provider_error": "Usage limit reached. Your limit will reset at 2026-04-16 04:18:56",
        "provider_id": "zai-coding-plan",
        "model_id": "glm-5.1",
        "reset_at": "2026-04-16T04:18:56Z",
    });
    let mut engine = engine("opencode");
    let cap_record = |engine: &SignalEngine| {
        engine
            .fired_records(SignalSource::StderrPromoted, &payload)
            .iter()
            .find(|r| r.kind == SignalKind::UsageCapped)
            .map(|r| r.id)
    };

    assert_eq!(cap_record(&engine), Some("stderr_promoted-usage_capped-legacy"));

    engine.observe_provider_version("1.17.8");
    assert_eq!(cap_record(&engine), Some("stderr_promoted-usage_capped-1178"));

    engine.observe_provider_version("1.17.7");
    assert_eq!(cap_record(&engine), Some("stderr_promoted-usage_capped-legacy"));

    // The built event carries the ISO reset time as lifts_at, the extracted
    // `model_id` as a specific cap scope, and the fired-cap remaining default.
    let events = engine.observe(SignalSource::StderrPromoted, &payload);
    assert!(events.contains(&SignalEvent::UsageCapped {
        model: CapScope::Specific("glm-5.1".into()),
        timeframe: None,
        remaining: Some(Quantity {
            value: 0.0,
            unit: Unit::Percent,
        }),
        lifts_at: Some(DateTime::parse_from_rfc3339("2026-04-16T04:18:56Z").unwrap().to_utc()),
        message: Some(
            "Usage limit reached. Your limit will reset at 2026-04-16 04:18:56".to_string()
        ),
    }));
}

/// Bracket-index walking against the synthetic-assistant billing fixture.
#[test]
fn walker_reaches_bracket_indexed_message_text() {
    let mut engine = engine("claude");
    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_BILLING_SYNTHETIC, 0));
    assert!(events.contains(&SignalEvent::NoFunds {
        message: Some("Credit balance is too low".to_string()),
        top_up_url: None,
    }));
}

/// Bespoke records are documentation: the claude session-taint record
/// matches `is_error: true` on this result line but must not fire from the
/// generic matcher (tokens_consumed still does).
#[test]
fn bespoke_records_are_skipped() {
    let engine = engine("claude");
    let fired = engine.fired_records(SignalSource::Stream, &line(CLAUDE_BILLING_SYNTHETIC, 1));
    let kinds: Vec<SignalKind> = fired.iter().map(|r| r.kind).collect();
    assert!(!kinds.contains(&SignalKind::SessionTainted), "kinds: {kinds:?}");
    assert!(kinds.contains(&SignalKind::TokensConsumed), "kinds: {kinds:?}");
}

/// Builder unit conversions: `retry_after_ms` → a millisecond quantity, and
/// epoch-second `resetsAt` → `DateTime<Utc>` on the renamed `resets_at`
/// extraction rows.
#[test]
fn builder_converts_durations_and_epoch_seconds() {
    let mut engine = engine("claude");

    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_THROTTLED, 0));
    assert!(events.contains(&SignalEvent::RateLimited {
        status_code: None,
        reset_at: None,
        retry_after: Some(Quantity {
            value: 5000.0,
            unit: Unit::DurationMillis,
        }),
        message: Some("slow".to_string()),
    }));

    let resets_at = DateTime::from_timestamp(1_712_000_000, 0);
    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_APPROACHING, 0));
    assert!(events.contains(&SignalEvent::UsageCapApproaching {
        // The observed claude spelling "usage" is a generic account-wide
        // marker: no model tier, no window duration.
        model: CapScope::All,
        timeframe: None,
        remaining: None,
        resets_at,
        message: None,
    }));

    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_WARNING_SEVEN_DAY, 0));
    assert!(events.contains(&SignalEvent::UsageCapApproaching {
        // "seven_day" is account-wide with a 7-day (604800s) window.
        model: CapScope::All,
        timeframe: Some(Quantity {
            value: 604_800.0,
            unit: Unit::DurationSecs,
        }),
        remaining: None,
        resets_at,
        message: None,
    }));
}

/// A claude cap record decomposes the combined `rateLimitType` wire token
/// into the provider-neutral cap axes via `ExtractStrategy::Literal`:
/// `seven_day_opus` → `{model: Specific(opus), timeframe: 7d}` (more-struture
/// Cluster 0). This is the mechanism the whole cap reshape exists to enable.
#[test]
fn claude_cap_record_decomposes_model_and_timeframe_via_literal() {
    let mut engine = engine("claude");
    let events = engine.observe(SignalSource::Stream, &line(CLAUDE_SEVEN_DAY_OPUS, 0));
    assert!(
        events.contains(&SignalEvent::UsageCapApproaching {
            model: CapScope::Specific("opus".into()),
            timeframe: Some(Quantity {
                value: 604_800.0,
                unit: Unit::DurationSecs,
            }),
            remaining: None,
            resets_at: DateTime::from_timestamp(1_712_000_000, 0),
            message: None,
        }),
        "got {events:?}"
    );
}
