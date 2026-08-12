//! E5 migration-bridge equivalence: the parser-computed [`RateLimitInfo`]
//! and the engine+projection path must agree on the claude rate-limit
//! fixtures for the structured fields (`is_throttled`, `retry_after_ms`,
//! `reset_at`) and on `message` wherever the provider supplied a raw one.
//!
//! Documented divergence: the parser SYNTHESIZES a rendered message when
//! the payload carries none (`render_claude_rate_limit_message`); the
//! projection deliberately does not — rendering is a consumer concern
//! (design §Sink), and the consumer-side migration bridge
//! (`IterationSummarySignals::apply_projected_rate_limit`) fills the gap
//! from the parser value.

use chrono::{TimeZone, Utc};
use claudine_catalog_types::SignalSource;
use serde_json::Value;

use super::{SignalHub, detection_table, project};
use crate::provider_id::Provider;
use crate::stream::semantic::NullSemanticSink;
use crate::stream::summary::RateLimitInfo;
use crate::stream::{ParserConfig, create_semantic_parser};

const THROTTLED: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-throttled-message.jsonl"
);
const NOT_THROTTLED: &str =
    include_str!("../../../docs/research/signals/fixtures/claude/rate-limit-not-throttled.jsonl");
const APPROACHING: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-info-approaching.jsonl"
);
const ALLOWED_WARNING: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/rate-limit-info-allowed-warning-seven-day.jsonl"
);
const ERROR_BILLING: &str =
    include_str!("../../../docs/research/signals/fixtures/claude/error-billing.jsonl");
const BILLING_SYNTHETIC: &str = include_str!(
    "../../../docs/research/signals/fixtures/claude/billing-error-synthetic-result.jsonl"
);

/// The legacy path: claude semantic parser → `summary.rate_limit`.
fn parser_rate_limit(fixture: &str) -> Option<RateLimitInfo> {
    let mut parser =
        create_semantic_parser(Provider::Claude, NullSemanticSink, ParserConfig::default());
    for line in fixture.lines().filter(|line| !line.trim().is_empty()) {
        parser.feed_line(line);
    }
    parser.finish(0).rate_limit
}

/// The engine path: hub-observed stream payloads → sink → projection.
fn projected_rate_limit(fixture: &str) -> Option<RateLimitInfo> {
    let hub = SignalHub::new(detection_table("claude").expect("claude table"));
    for line in fixture.lines().filter(|line| !line.trim().is_empty()) {
        let payload: Value = serde_json::from_str(line).expect("fixture line is JSON");
        hub.observe_json(SignalSource::Stream, &payload);
    }
    project::rate_limit_info(&hub.drain())
}

#[test]
fn throttled_message_fixture_agrees_on_all_fields() {
    let parser = parser_rate_limit(THROTTLED).expect("parser records the throttle");
    let projected = projected_rate_limit(THROTTLED).expect("projection observes the throttle");
    // The fixture carries a raw provider message, so even `message` must
    // agree exactly.
    assert_eq!(projected, parser);
    assert_eq!(projected.is_throttled, Some(true));
    assert_eq!(projected.retry_after_ms, Some(5_000));
    assert_eq!(projected.message.as_deref(), Some("slow"));
    assert_eq!(projected.reset_at, None);
}

#[test]
fn approaching_fixture_agrees_on_structured_fields() {
    let parser = parser_rate_limit(APPROACHING).expect("parser records the advisory");
    let projected = projected_rate_limit(APPROACHING).expect("projection observes the advisory");
    assert_eq!(projected.is_throttled, parser.is_throttled);
    assert_eq!(projected.is_throttled, Some(false));
    assert_eq!(projected.retry_after_ms, parser.retry_after_ms);
    assert_eq!(projected.reset_at, parser.reset_at);
    assert_eq!(
        projected.reset_at,
        Some(Utc.timestamp_opt(1_712_000_000, 0).unwrap())
    );
    // Rendering divergence (module docs): the payload has no raw message,
    // so the parser synthesizes one and the projection stays silent.
    assert!(parser.message.is_some());
    assert_eq!(projected.message, None);
}

#[test]
fn allowed_warning_fixture_agrees_on_structured_fields() {
    let parser = parser_rate_limit(ALLOWED_WARNING).expect("parser records the advisory");
    let projected =
        projected_rate_limit(ALLOWED_WARNING).expect("projection observes the advisory");
    assert_eq!(projected.is_throttled, parser.is_throttled);
    assert_eq!(projected.retry_after_ms, parser.retry_after_ms);
    assert_eq!(projected.reset_at, parser.reset_at);
    assert!(parser.message.is_some());
    assert_eq!(projected.message, None);
}

#[test]
fn not_throttled_fixture_reports_no_throttle_on_both_paths() {
    // The parser records the advisory event (`is_throttled: Some(false)`);
    // the engine has no record for a non-throttle heartbeat, so the
    // projection is `None`. Agreement is semantic: neither path reports an
    // active throttle.
    let parser = parser_rate_limit(NOT_THROTTLED);
    let projected = projected_rate_limit(NOT_THROTTLED);
    assert_ne!(
        parser.as_ref().and_then(|info| info.is_throttled),
        Some(true)
    );
    assert_eq!(projected, None);
}

#[test]
fn billing_fixtures_produce_no_rate_limit_info_on_either_path() {
    for fixture in [ERROR_BILLING, BILLING_SYNTHETIC] {
        assert_eq!(parser_rate_limit(fixture), None);
        assert_eq!(projected_rate_limit(fixture), None);
    }
}
