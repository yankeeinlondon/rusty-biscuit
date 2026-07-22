//! Tests for OpenCode reasoning-log analysis.

use super::*;
use super::stall_guard::MAX_GENERATIONS_WITHOUT_PROGRESS;
use crate::stream::logs::opencode::state::merge_stderr_state_into_summary;
use crate::stream::summary::StderrDiagnostics;

const STALL_BUDGET: Duration = Duration::from_secs(600);
const STREAMED_LLM_CALL: &str = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";
const GENERIC_STREAM_ERROR: &str = r#"timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=acme modelID=m1 session.id=ses_x small=false agent=build mode=primary error.error="AI_APICallError: connection reset by peer""#;

#[derive(Default)]
struct RecordingSink {
    events: Vec<SemanticEvent>,
}

impl SemanticEventSink for RecordingSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.push(event);
    }
}

fn stdout_seen() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(true))
}

fn stdout_unseen() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn armed_bridge() -> OpenCodeLogBridge<RecordingSink> {
    OpenCodeLogBridge::new(
        RecordingSink::default(),
        stdout_seen(),
        None,
        Some(STALL_BUDGET),
    )
}

/// Uses a zero silence budget so stalled-generation tests can isolate the
/// mandatory churn-count condition without advancing the monotonic clock.
fn count_only_bridge(tx: Sender<EarlyTermination>) -> OpenCodeLogBridge<RecordingSink> {
    OpenCodeLogBridge::new(
        RecordingSink::default(),
        stdout_seen(),
        Some(tx),
        Some(Duration::ZERO),
    )
}

fn assert_string(extra: &Value, key: &str, expected: &str) {
    let actual = extra
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing {key} in extra: {extra}"));
    assert_eq!(actual, expected, "extra.{key} mismatch: {extra}");
}


mod ingest_classification;
mod session_lifecycle;
mod signal_projection;
mod stalled_generation_progress;
mod stdout_stderr_coordination;
mod usage_retry_guards;
