//! Tests for OpenCode reasoning-log analysis.

use super::*;
use super::stall_guard::MAX_GENERATIONS_WITHOUT_PROGRESS;
use crate::stream::logs::opencode::state::merge_stderr_state_into_summary;
use crate::stream::summary::StderrDiagnostics;

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

fn assert_string(extra: &Value, key: &str, expected: &str) {
    let actual = extra
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing {key} in extra: {extra}"));
    assert_eq!(actual, expected, "extra.{key} mismatch: {extra}");
}

#[test]
fn unclassified_structured_line_is_consumed_without_emitting_event() {
    // Structured OpenCode log records are owned by the bridge — even
    // when they don't classify into a promoted semantic event we
    // suppress the raw line so it doesn't leak to the user's terminal
    // as debug output.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let outcome = bridge.ingest("INFO 2026-04-15T21:28:30 +0ms service=default msg=hello");
    assert_eq!(outcome, StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 0);
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.log_records_parsed, 1);
}

#[test]
fn unstructured_noise_returns_not_consumed_and_is_not_counted() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let outcome = bridge.ingest("just some chatter");
    assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.log_records_parsed, 0);
    assert_eq!(state.diagnostics.uncaught_errors, 0);
}

#[test]
fn malformed_command_emits_warning_and_consumes() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT failed to load command";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert!(message.contains("command"), "{message}");
            assert!(message.contains("/tmp/foo.md"), "{message}");
            assert_string(extra, "provider", "opencode");
            assert_string(extra, "source", "stderr_log");
            assert_string(extra, "classification", "malformed_asset");
            assert_string(extra, "asset_type", "command");
            assert_string(extra, "path", "/tmp/foo.md");
            assert_string(extra, "service", "config");
            assert!(
                extra.get("raw").and_then(Value::as_str).is_some(),
                "raw must be present in extra: {extra}",
            );
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.malformed_asset_events, 1);
}

#[test]
fn usage_cap_after_stdout_emits_terminal_error_and_early_terminate() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);
    let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"), "{message}");
            // We no longer assert the exact timestamp because it's converted to local time
            assert!(message.contains("2026-04-"), "{message}");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for UsageCap even when stdout already seen",
    );
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.rate_limit_events, 1);
    assert!(state.diagnostics.rate_limit_reset_at.is_some());
    assert_eq!(state.rate_limit.as_ref().unwrap().is_throttled, Some(true));
}

#[test]
fn usage_cap_before_stdout_emits_terminal_error_and_early_terminate() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"));
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(EarlyTermination::RateLimit { message, reset_at }) => {
            assert!(message.to_lowercase().contains("usage limit"));
            let reset = reset_at.expect("reset_at should be forwarded");
            assert_eq!(
                reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                "2026-04-16 04:18:56",
            );
        }
        other => panic!("expected EarlyTermination::RateLimit, got {other:?}"),
    }
}

// The 1.17.8 `message="stream error"` + `error.error=` shape must drive the
// same terminal early-termination as the legacy JSON form, otherwise the
// wrapper hangs through OpenCode's unbounded backoff retries.
// Regression for fixes/2026-06-21-opencode-log-fix (session ses_1127ec2f).
#[test]
fn opencode_1178_stream_error_usage_cap_terminates() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);
    let line = r#"timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_1127ec2fdffepaJc2kEnX093eo small=false agent=build mode=primary error.error="AI_APICallError: Usage limit reached for 5 hour. Your limit will reset at 2026-06-22 13:59:38""#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"), "{message}");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for the 1.17.8 stream-error cap",
    );
}

// A generic `stream error` the classifier treats as a non-fatal API
// failure (no cap/429/auth vocabulary) must still trip the backstop after
// MAX_CONSECUTIVE_STREAM_ERRORS with no step advance, so an unknown future
// format cannot retry forever. Regression for fixes/2026-06-21-opencode-log-fix.
const GENERIC_STREAM_ERROR: &str = r#"timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=acme modelID=m1 session.id=ses_x small=false agent=build mode=primary error.error="AI_APICallError: connection reset by peer""#;

#[test]
fn repeated_stream_errors_trip_backstop_and_terminate() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);

    // First MAX-1 errors are non-fatal warnings; the channel stays quiet.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 1) {
        assert_eq!(bridge.ingest(GENERIC_STREAM_ERROR), StderrIngestOutcome::Consumed);
    }
    assert!(rx.try_recv().is_err(), "backstop must not fire below threshold");

    // The threshold-crossing error trips the terminal abort.
    assert_eq!(bridge.ingest(GENERIC_STREAM_ERROR), StderrIngestOutcome::Consumed);
    match bridge.sink.events.last().expect("expected an event") {
        SemanticEvent::Error { terminal, kind, .. } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
        }
        other => panic!("expected terminal Error, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(EarlyTermination::RepeatedStreamError { count }) => {
            assert_eq!(count, MAX_CONSECUTIVE_STREAM_ERRORS);
        }
        other => panic!("expected RepeatedStreamError, got {other:?}"),
    }
}

#[test]
fn step_advance_resets_stream_error_backstop() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);
    let step_line = r#"timestamp=2026-06-22T04:07:20.000Z level=INFO run=da37e0dd message=loop session.id=ses_x step=7"#;

    // One below threshold, then a genuine step transition clears the count.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 1) {
        bridge.ingest(GENERIC_STREAM_ERROR);
    }
    assert_eq!(bridge.ingest(step_line), StderrIngestOutcome::Consumed);

    // A fresh run of MAX-1 errors must still stay under the threshold.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 1) {
        bridge.ingest(GENERIC_STREAM_ERROR);
    }
    assert!(
        rx.try_recv().is_err(),
        "step advance should have reset the backstop counter",
    );
}

/// Build a distinct (non-identical) `stream error` line whose error text
/// varies by `n`. Used to prove the backstop only accumulates *identical*
/// failures; the cap needles do not match `transient glitch N`.
fn distinct_stream_error(n: usize) -> String {
    format!(
        r#"timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=acme modelID=m1 session.id=ses_x small=false agent=build mode=primary error.error="AI_APICallError: transient glitch {n}""#,
    )
}

#[test]
fn distinct_stream_errors_do_not_trip_backstop() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);

    // MAX distinct errors in one step: each resets the counter to 1, so the
    // threshold is never crossed and the channel stays quiet.
    for n in 0..MAX_CONSECUTIVE_STREAM_ERRORS {
        assert_eq!(
            bridge.ingest(&distinct_stream_error(n as usize)),
            StderrIngestOutcome::Consumed,
        );
    }
    assert!(
        rx.try_recv().is_err(),
        "distinct stream errors must not accumulate toward the backstop",
    );
}

#[test]
fn fingerprint_change_resets_run_then_new_run_can_trip() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None);
    let first = distinct_stream_error(1);
    let second = distinct_stream_error(2);

    // (MAX-1) identical "first" errors — one short of the threshold.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 1) {
        bridge.ingest(&first);
    }
    // One different error resets the run to 1.
    bridge.ingest(&second);
    // (MAX-1) more identical-to-"second" errors bring the new run to MAX-1.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 2) {
        bridge.ingest(&second);
    }
    assert!(
        rx.try_recv().is_err(),
        "fingerprint change must reset the counter mid-run",
    );

    // One more matching "second" reaches MAX of the new fingerprint and trips.
    bridge.ingest(&second);
    match rx.try_recv() {
        Ok(EarlyTermination::RepeatedStreamError { count }) => {
            assert_eq!(count, MAX_CONSECUTIVE_STREAM_ERRORS);
        }
        other => panic!("expected RepeatedStreamError, got {other:?}"),
    }
}

#[test]
fn provider_limit_fires_early_termination_only_once() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert!(rx.try_recv().is_ok(), "first rate-limit fires channel");
    assert!(
        rx.try_recv().is_err(),
        "second rate-limit must not re-fire the channel",
    );
}

#[test]
fn auth_failure_emits_terminal_error() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert_string(extra, "classification", "auth_failure");
            assert_eq!(message, "AuthenticationError: Invalid API key");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.auth_failures, 1);
}

#[test]
fn api_failure_emits_warning_if_not_fatal() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert_string(extra, "classification", "api_failure");
            assert_string(extra, "error_name", "AI_APICallError");
            assert_eq!(extra.get("status_code"), Some(&json!(500)));
            assert_eq!(
                message,
                "AI_APICallError (500: Internal Server Error): upstream boom"
            );
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.api_failures, 1);
}

#[test]
fn uncaught_structured_error_emits_unknown_error_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "ERROR 2026-04-15T21:28:30 +33ms service=default name=TypeError message=U.split is not a function fatal";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            extra,
            ..
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::Unknown);
            assert_string(extra, "classification", "uncaught_error");
            assert_string(extra, "error_name", "TypeError");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.uncaught_errors, 1);
}

#[test]
fn raw_ansi_error_line_emits_unknown_error_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error, check log file";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::Unknown);
            assert!(
                !message.contains('\u{1b}'),
                "ANSI must be stripped: {message}"
            );
            assert_string(extra, "classification", "uncaught_error");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn early_termination_channel_works_without_receiver_dropped_panic() {
    // Receiver is dropped before the bridge ingests; bridge should not
    // panic, just log a warning and continue.
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    drop(rx);
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
}

#[test]
fn merge_stderr_state_applies_diagnostics_without_emitting_config_badge() {
    use crate::provider_id::Provider;
    use crate::stream::badges::BadgeCategory;
    use crate::stream::summary::StreamExecutionSummary;

    let state = Arc::new(Mutex::new(SharedStderrState {
        diagnostics: StderrDiagnostics {
            log_records_parsed: 3,
            malformed_asset_events: 2,
            ..Default::default()
        },
        rate_limit: None,
        ..Default::default()
    }));
    let mut summary = StreamExecutionSummary {
        provider: Provider::OpenCode,
        ..Default::default()
    };

    merge_stderr_state_into_summary(&state, &mut summary);

    // Per the 2026-04-18 OpenCode reporting contract, the malformed
    // asset counter is preserved on the summary (so JSONL/dashboards
    // can observe it) but the trailer Config badge is removed —
    // each malformed asset is already surfaced as a per-line Warning.
    let diagnostics = summary
        .stderr_diagnostics
        .as_ref()
        .expect("diagnostics should be attached");
    assert_eq!(diagnostics.log_records_parsed, 3);
    assert_eq!(diagnostics.malformed_asset_events, 2);
    assert!(
        !summary
            .badges
            .iter()
            .any(|b| b.category == BadgeCategory::Config),
        "Config trailer badge must NOT be emitted for malformed assets: {:?}",
        summary.badges,
    );
}

#[test]
fn merge_stderr_state_merges_rate_limit_and_yields_rate_limit_badge() {
    use crate::provider_id::Provider;
    use crate::stream::badges::BadgeCategory;
    use crate::stream::summary::StreamExecutionSummary;
    use chrono::TimeZone;

    let reset = Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap();
    let state = Arc::new(Mutex::new(SharedStderrState {
        diagnostics: StderrDiagnostics {
            log_records_parsed: 1,
            rate_limit_events: 1,
            rate_limit_reset_at: Some(reset),
            ..Default::default()
        },
        rate_limit: Some(RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: None,
            message: Some("Usage limit reached".into()),
            reset_at: Some(reset),
        }),
        ..Default::default()
    }));
    let mut summary = StreamExecutionSummary {
        provider: Provider::OpenCode,
        ..Default::default()
    };

    merge_stderr_state_into_summary(&state, &mut summary);

    let rate_limit = summary
        .rate_limit
        .as_ref()
        .expect("rate_limit should be merged onto summary");
    assert_eq!(rate_limit.is_throttled, Some(true));
    assert_eq!(rate_limit.reset_at, Some(reset));
    assert!(
        summary
            .badges
            .iter()
            .any(|b| b.category == BadgeCategory::RateLimit),
        "RateLimit badge should be recomputed after merge",
    );
}

#[test]
fn merge_stderr_state_without_records_does_not_attach_diagnostics() {
    use crate::provider_id::Provider;
    use crate::stream::summary::StreamExecutionSummary;

    let state = Arc::new(Mutex::new(SharedStderrState::default()));
    let mut summary = StreamExecutionSummary {
        provider: Provider::OpenCode,
        ..Default::default()
    };

    merge_stderr_state_into_summary(&state, &mut summary);
    assert!(summary.stderr_diagnostics.is_none());
    assert!(summary.rate_limit.is_none());
    assert!(summary.badges.is_empty());
}

#[test]
fn bus_line_is_consumed_silently_and_counts_as_parsed() {
    // `service=bus` is the noisiest source on the stderr stream
    // (~70-75% of INFO volume) and carries nothing the user cares
    // about. Bus lines are counted in diagnostics but produce no
    // semantic event and must NEVER leak to the user as raw stderr —
    // so the bridge returns `Consumed` to suppress raw passthrough.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO 2026-04-15T21:28:30 +5ms service=bus msg=internal chatter";
    let outcome = bridge.ingest(line);
    assert_eq!(outcome, StderrIngestOutcome::Consumed);
    assert_eq!(
        bridge.sink.events.len(),
        0,
        "bus lines must not emit semantic events"
    );
    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.diagnostics.log_records_parsed, 1,
        "bus lines must still be counted as parsed"
    );
}

#[test]
fn non_bus_line_classifies_normally_after_bus_filter() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    // First, a bus line that is silently dropped (but still consumed
    // so it never reaches raw stderr passthrough).
    let bus_line = "INFO 2026-04-15T21:28:30 +5ms service=bus msg=ignored";
    assert_eq!(bridge.ingest(bus_line), StderrIngestOutcome::Consumed);
    // Then, a malformed asset line that should classify normally.
    let real_line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT failed to load command";
    assert_eq!(bridge.ingest(real_line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.log_records_parsed, 2);
    assert_eq!(state.diagnostics.malformed_asset_events, 1);
}

#[test]
fn new_format_bridge_consumes_lifecycle_lines() {
    let fixture =
        include_str!("../../../../../tests/fixtures/logs/opencode-new-format-lifecycle.txt");
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None, None);

    for line in fixture.lines() {
        assert_eq!(
            bridge.ingest(line),
            StderrIngestOutcome::Consumed,
            "new-format lifecycle line must be consumed: {line}",
        );
    }

    let actual: Vec<&str> = bridge
        .sink
        .events
        .iter()
        .map(|event| match event {
            SemanticEvent::SessionStart { .. } => "session_start",
            SemanticEvent::SubagentStart { .. } => "subagent_start",
            SemanticEvent::SubagentStop { .. } => "subagent_stop",
            SemanticEvent::Info { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("info"),
            SemanticEvent::Warning { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("warning"),
            SemanticEvent::Error { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("error"),
            _ => "other",
        })
        .collect();

    assert_eq!(
        actual,
        vec![
            "session_start",
            "llm_call",
            "step_loop",
            "permission_evaluated",
            "subagent_start",
            "llm_call",
            "step_loop",
            "step_exit",
            "subagent_stop",
            "step_exit",
            "http_response",
        ],
    );

    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.diagnostics.log_records_parsed,
        fixture.lines().count() as u32,
    );
}

#[test]
fn new_format_bridge_consumes_serviceless_lifecycle_lines() {
    let fixture =
        include_str!("../../../../../tests/fixtures/logs/opencode-new-format-serviceless.txt");
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None, None);

    for line in fixture.lines() {
        assert_eq!(
            bridge.ingest(line),
            StderrIngestOutcome::Consumed,
            "new-format serviceless lifecycle line must be consumed: {line}",
        );
    }

    let actual: Vec<&str> = bridge
        .sink
        .events
        .iter()
        .map(|event| match event {
            SemanticEvent::SessionStart { .. } => "session_start",
            SemanticEvent::SubagentStart { .. } => "subagent_start",
            SemanticEvent::SubagentStop { .. } => "subagent_stop",
            SemanticEvent::Info { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("info"),
            SemanticEvent::Warning { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("warning"),
            SemanticEvent::Error { extra, .. } => extra
                .get("classification")
                .and_then(Value::as_str)
                .unwrap_or("error"),
            _ => "other",
        })
        .collect();

    assert_eq!(
        actual,
        vec![
            "session_start",
            "llm_call",
            "step_loop",
            "permission_evaluated",
            "subagent_start",
            "llm_call",
            "step_loop",
            "step_exit",
            "subagent_stop",
            "step_exit",
            "http_response",
        ],
    );

    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.diagnostics.log_records_parsed,
        fixture.lines().count() as u32,
    );
}

// ------------------------------------------------------------------
// Phase-3 semantic event promotion
// ------------------------------------------------------------------

#[test]
fn session_created_without_parent_emits_session_start() {
    // Phase 4 dedup gate: bridge emits a primary SessionStart only
    // when stdout has not yet produced any semantic event.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary slug=happy-panda version=1.14.48 title=New session created";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::SessionStart {
            session_id,
            model,
            extra,
        } => {
            assert_eq!(session_id.as_deref(), Some("ses_primary"));
            assert!(model.is_none());
            assert_string(extra, "classification", "session_created");
            assert_string(extra, "session_id", "ses_primary");
            assert_string(extra, "title", "New session");
        }
        other => panic!("expected SessionStart, got {other:?}"),
    }
}

#[test]
fn duplicate_primary_session_created_is_not_re_emitted() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary title=A created";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    let line2 =
        "INFO  2026-05-12T20:00:13 +21ms service=session id=ses_primary_other title=B created";
    assert_eq!(bridge.ingest(line2), StderrIngestOutcome::Consumed);
    assert_eq!(
        bridge.sink.events.len(),
        1,
        "second primary session-created must be suppressed",
    );
}

#[test]
fn session_created_with_parent_emits_subagent_start() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child slug=lucky-orchid version=1.14.48 parentID=ses_parent title=Count letters in 'banana' created";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::SubagentStart { name, id, extra } => {
            assert_eq!(id.as_deref(), Some("ses_child"));
            assert!(name.is_some());
            assert_string(extra, "classification", "session_created");
            assert_string(extra, "session_id", "ses_child");
            assert_string(extra, "parent_id", "ses_parent");
        }
        other => panic!("expected SubagentStart, got {other:?}"),
    }
    assert!(bridge.child_sessions.contains_key("ses_child"));
}

#[test]
fn llm_call_emits_info_event_with_provider_model_mode() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(
                message,
                "llm_call_start kimi-for-coding/k2p6 (mode=primary, agent=build)"
            );
            assert_string(extra, "provider_id", "kimi-for-coding");
            assert_string(extra, "model_id", "k2p6");
            assert_string(extra, "mode", "primary");
            assert_string(extra, "agent", "build");
            assert_string(extra, "session_id", "ses_a");
            assert_eq!(extra.get("is_stream"), Some(&json!(true)));
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn step_loop_emits_info_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=3 logSpan.http.span.4=55ms loop";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(message, "step_loop step=3 session=ses_a");
            assert_string(extra, "session_id", "ses_a");
            assert_eq!(extra.get("step"), Some(&json!(3)));
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn step_loop_dedups_repeated_step_in_same_session() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let same_step = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
    let later_span = "INFO  2026-05-12T20:00:13 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=2143ms loop";
    let new_step = "INFO  2026-05-12T20:00:14 +0ms service=session.prompt session.id=ses_a step=1 logSpan.http.span.4=2200ms loop";

    assert_eq!(bridge.ingest(same_step), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(later_span), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(new_step), StderrIngestOutcome::Consumed);

    let step_loops: Vec<_> = bridge
        .sink
        .events
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::Info { message, .. } if message.starts_with("step_loop ") => {
                Some(message.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        step_loops,
        vec![
            "step_loop step=0 session=ses_a".to_string(),
            "step_loop step=1 session=ses_a".to_string(),
        ],
    );
}

#[test]
fn step_loop_dedup_resets_after_step_exit() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let first = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
    let exit = "INFO  2026-05-12T20:00:19 +0ms service=session.prompt session.id=ses_a logSpan.http.span.4=7437ms exiting loop";
    let after = "INFO  2026-05-12T20:01:00 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.5=10ms loop";

    assert_eq!(bridge.ingest(first), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(after), StderrIngestOutcome::Consumed);

    let step_loops = bridge
        .sink
        .events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::Info { message, .. } if message.starts_with("step_loop ")))
        .count();
    assert_eq!(step_loops, 2, "exit should reset dedup so the next step=0 emits again");
}

#[test]
fn step_exit_for_non_child_session_emits_only_info_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:19 +1ms service=session.prompt session.id=ses_a logSpan.http.span.4=7437ms exiting loop";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(message, "exiting_loop session=ses_a");
            assert_string(extra, "session_id", "ses_a");
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn step_exit_for_child_session_emits_info_then_subagent_stop() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);

    let start = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
    assert_eq!(bridge.ingest(start), StderrIngestOutcome::Consumed);
    let exit = "INFO  2026-05-12T20:05:30 +0ms service=session.prompt session.id=ses_child logSpan.http.span.4=4000ms exiting loop";
    assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);

    assert_eq!(bridge.sink.events.len(), 3);
    match &bridge.sink.events[0] {
        SemanticEvent::SubagentStart { id, .. } => {
            assert_eq!(id.as_deref(), Some("ses_child"));
        }
        other => panic!("expected SubagentStart first, got {other:?}"),
    }
    match &bridge.sink.events[1] {
        SemanticEvent::Info { message, .. } => {
            assert!(
                message.starts_with("exiting_loop session="),
                "expected enriched exiting_loop message, got {message:?}",
            );
        }
        other => panic!("expected Info second, got {other:?}"),
    }
    match &bridge.sink.events[2] {
        SemanticEvent::SubagentStop { id, extra, .. } => {
            assert_eq!(id.as_deref(), Some("ses_child"));
            assert_string(extra, "parent_id", "ses_parent");
            assert_string(extra, "classification", "subagent_stop");
        }
        other => panic!("expected SubagentStop third, got {other:?}"),
    }
}

#[test]
fn step_exit_for_child_session_emits_subagent_stop_only_once() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);

    let start = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
    assert_eq!(bridge.ingest(start), StderrIngestOutcome::Consumed);
    let exit = "INFO  2026-05-12T20:05:30 +0ms service=session.prompt session.id=ses_child logSpan.http.span.4=4000ms exiting loop";
    assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);

    let subagent_stops = bridge
        .sink
        .events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SubagentStop { .. }))
        .count();
    assert_eq!(
        subagent_stops, 1,
        "SubagentStop must fire only once per child session",
    );
}

#[test]
fn permission_evaluated_emits_info_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = r#"INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(message, "permission_evaluated task:general → allow");
            assert_string(extra, "permission", "task");
            assert_string(extra, "pattern", "general");
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn http_response_emits_info_event() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/x/message http.status=500 logSpan.http.span.4=99ms Sent HTTP response";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(message, "http_response POST /session/x/message 500 (99ms)");
            assert_string(extra, "method", "POST");
            assert_string(extra, "url", "/session/x/message");
            assert_eq!(extra.get("status"), Some(&json!(500)));
            assert_eq!(extra.get("duration_ms"), Some(&json!(99)));
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn snapshot_warn_line_emits_warning_with_message_context() {
    // OpenCode's stderr body parser absorbs trailing message text into
    // the last bare-valued tag (a known quirk handled elsewhere via
    // `has_trailing_keyword`). Either shape — a clean trailing message
    // OR an absorbed tag value — must surface enough context for the
    // user to know which snapshot subsystem operation failed.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    // JSON-array `files=[…]` form: parser cleanly separates trailing message.
    let line = r#"WARN  2026-05-12T20:05:26 +0ms service=snapshot session.id=ses_a files=["/repo/.env",".npmrc"] failed to add snapshot files"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert!(
                message.starts_with("snapshot: failed to add snapshot files"),
                "expected snapshot message prefix, got {message:?}",
            );
            assert!(
                message.contains("files=") || message.contains("session.id="),
                "expected tag summary in message, got {message:?}",
            );
            assert_string(extra, "level", "WARN");
            assert_string(extra, "classification", "snapshot");
            assert!(
                extra.get("files").is_some() || extra.get("session.id").is_some(),
                "expected files or session.id in extra, got {extra:?}",
            );
        }
        other => panic!("expected Warning, got {other:?}"),
    }
}

#[test]
fn snapshot_warn_line_with_absorbed_trailing_still_carries_context() {
    // Even when the body parser absorbs the trailing message into a
    // bare-valued tag (e.g. `file=/repo/.env failed to add snapshot files`),
    // the rendered Warning must still surface enough information for
    // the operator to know what was being snapshotted — the file path
    // and the absorbed diagnostic both ride along in the tag summary.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "WARN  2026-05-12T20:05:26 +0ms service=snapshot session.id=ses_a file=/repo/.env failed to add snapshot files";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert!(
                message.contains("/repo/.env")
                    && message.contains("failed to add snapshot files"),
                "expected file path and diagnostic in message, got {message:?}",
            );
            assert_string(extra, "level", "WARN");
        }
        other => panic!("expected Warning, got {other:?}"),
    }
}

#[test]
fn snapshot_info_line_is_silently_consumed() {
    // Routine snapshot maintenance — INFO/DEBUG `taking snapshot`,
    // `prune=7.days cleanup`, etc. — is parsed and counted but emits
    // no semantic event. They dominate snapshot-line volume and
    // carry no user-actionable signal; only WARN/ERROR-level
    // snapshot lines (`failed to add snapshot files`, etc.) surface
    // as Warning events.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let lines = [
        r#"INFO  2026-05-12T20:05:26 +0ms service=snapshot id=snap_abc files=["/repo/x.rs"] taking snapshot"#,
        "INFO  2026-05-12T20:05:27 +0ms service=snapshot prune=7.days cleanup",
        "DEBUG 2026-05-12T20:05:28 +0ms service=snapshot id=snap_xyz noop",
    ];
    for line in lines {
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    }
    assert_eq!(
        bridge.sink.events.len(),
        0,
        "routine snapshot lines must not emit events; got {:?}",
        bridge.sink.events,
    );
    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.diagnostics.log_records_parsed, 3,
        "INFO/DEBUG snapshot lines must still count as parsed records"
    );
}

#[test]
fn boot_banner_is_parsed_and_consumed_without_emitting_event() {
    // The boot banner is parsed and counted but Phase 3 deliberately
    // does not promote it to a `SessionStart` (the NDJSON stream's
    // own session event remains the anchor). It still must NOT be
    // proxied to the user's terminal — return `Consumed` so the raw
    // stderr passthrough does not echo it as debug output.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:11 +97ms service=default version=1.14.48 args=[\"run\"] opencode";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 0);
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.log_records_parsed, 1);
}

// ------------------------------------------------------------------
// Phase-4 cross-stream dedup and summary enrichment
// ------------------------------------------------------------------

#[test]
fn primary_llm_call_captures_provider_and_model_on_first_observation() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.primary_provider_id.as_deref(),
        Some("kimi-for-coding")
    );
    assert_eq!(state.primary_model_id.as_deref(), Some("k2p6"));
}

#[test]
fn primary_llm_call_only_captures_first_mode_primary_observation() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let first = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a mode=primary stream";
    let second = "INFO  2026-05-12T20:01:00 +0ms service=llm providerID=anthropic modelID=claude-4 session.id=ses_a mode=primary stream";
    assert_eq!(bridge.ingest(first), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(second), StderrIngestOutcome::Consumed);

    let state = bridge.state.lock().unwrap();
    assert_eq!(
        state.primary_provider_id.as_deref(),
        Some("kimi-for-coding")
    );
    assert_eq!(state.primary_model_id.as_deref(), Some("k2p6"));
}

#[test]
fn non_primary_llm_call_does_not_capture_provider_and_model() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6-small session.id=ses_a mode=subagent stream";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

    let state = bridge.state.lock().unwrap();
    assert!(state.primary_provider_id.is_none());
    assert!(state.primary_model_id.is_none());
}

#[test]
fn merge_stderr_state_backfills_summary_model_from_primary_llm_call() {
    use crate::provider_id::Provider;
    use crate::stream::summary::StreamExecutionSummary;

    let state = Arc::new(Mutex::new(SharedStderrState {
        diagnostics: StderrDiagnostics {
            log_records_parsed: 1,
            ..Default::default()
        },
        rate_limit: None,
        primary_provider_id: Some("kimi-for-coding".into()),
        primary_model_id: Some("k2p6".into()),
    }));
    let mut summary = StreamExecutionSummary {
        provider: Provider::OpenCode,
        model: None,
        ..Default::default()
    };

    merge_stderr_state_into_summary(&state, &mut summary);
    assert_eq!(summary.model.as_deref(), Some("k2p6"));
}

#[test]
fn merge_stderr_state_does_not_overwrite_existing_summary_model() {
    use crate::provider_id::Provider;
    use crate::stream::summary::StreamExecutionSummary;

    let state = Arc::new(Mutex::new(SharedStderrState {
        diagnostics: StderrDiagnostics {
            log_records_parsed: 1,
            ..Default::default()
        },
        rate_limit: None,
        primary_provider_id: Some("kimi-for-coding".into()),
        primary_model_id: Some("k2p6".into()),
    }));
    let mut summary = StreamExecutionSummary {
        provider: Provider::OpenCode,
        model: Some("preexisting-model".into()),
        ..Default::default()
    };

    merge_stderr_state_into_summary(&state, &mut summary);
    assert_eq!(
        summary.model.as_deref(),
        Some("preexisting-model"),
        "stdout-derived model must win over the stderr backfill",
    );
}

#[test]
fn primary_session_start_is_suppressed_when_stdout_already_emitted() {
    // Cross-stream dedup: if stdout has already emitted any semantic
    // event, the stderr-derived primary SessionStart is redundant and
    // must be dropped. The `primary_session_emitted` flag is still set
    // so subsequent stderr session_created lines for other ids are
    // also suppressed.
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line =
        "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary title=Primary created";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(
        bridge.sink.events.len(),
        0,
        "stderr SessionStart must be suppressed when stdout has already emitted",
    );
    assert!(
        bridge.primary_session_emitted,
        "primary_session_emitted must be set so future duplicates are also skipped",
    );
}

#[test]
fn subagent_start_is_not_dedup_gated_by_stdout_event_seen() {
    // The dedup gate only applies to the primary SessionStart;
    // child session_created lines must always promote to SubagentStart
    // because the stdout NDJSON stream no longer synthesizes them
    // (Phase 4 removed the synthesis path).
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    assert!(matches!(
        bridge.sink.events[0],
        SemanticEvent::SubagentStart { .. }
    ));
}

#[test]
fn usage_cap_without_retry_error_still_terminates() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);

    // This is a 1308 but NOT wrapped in AI_RetryError
    let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached.\"}}"}}"#;

    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"), "{message}");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }

    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for UsageCap before stdout",
    );
}

#[test]
fn overload_emits_warning_no_early_terminate() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = r#"ERROR 2026-05-15T19:26:02 +3054ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"}}","isRetryable":true}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert_eq!(message, "server overloaded; will retry");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "Overloaded");
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.rate_limit_events, 1);
    assert!(
        state.rate_limit.is_none(),
        "overload must not set state.rate_limit"
    );
}

#[test]
fn throttled_emits_warning_no_early_terminate() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"message":"Too many requests"}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert_eq!(message, "request throttled; will retry");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "RateLimited");
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.rate_limit_events, 1);
    assert!(
        state.rate_limit.is_none(),
        "throttle must not set state.rate_limit"
    );
}

#[test]
fn retries_exhausted_emits_terminal_error_and_early_terminate() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert_eq!(message, "provider 429s did not clear after retries");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "RetriesExhausted");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for RetriesExhausted",
    );
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.rate_limit_events, 1);
    assert!(
        state.rate_limit.is_some(),
        "RetriesExhausted must set state.rate_limit"
    );
}

#[test]
fn exceeded_quota_emits_terminal_error_and_early_terminate() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"exceeded_current_quota_error\",\"message\":\"Quota exceeded\"}}"}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"), "{message}");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for UsageCap",
    );
}

#[test]
fn cap_phrase_without_error_tag_emits_advisory_warning_no_terminate() {
    let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let line = "ERROR 2026-05-15T19:26:02 +100ms service=llm dummy={} Usage limit reached for k2p6";
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert_eq!(message, "Usage limit reached for k2p6");
            assert_string(extra, "classification", "api_failure");
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let state = bridge.state.lock().unwrap();
    assert_eq!(state.diagnostics.api_failures, 1);
    assert!(
        state.rate_limit.is_none(),
        "advisory cap must not set state.rate_limit"
    );
}

#[test]
fn cap_wins_over_retries_exhausted_in_bridge() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx), None);
    let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached\"}}"}]}}"#;
    assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.sink.events.len(), 1);
    match &bridge.sink.events[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            extra,
        } => {
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.to_lowercase().contains("usage limit"), "{message}");
            assert_string(extra, "classification", "rate_limit");
            assert_string(extra, "kind", "UsageCap");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_ok(),
        "early-termination signal expected for UsageCap",
    );
}

// --- EarlyTermination runaway-guard variant parity (VC-1.2) ---
//
// The three new content-guard variants must clone/compare cleanly and
// carry their fields verbatim. The summary-mapping behavior (error_kind
// routing through the CLI termination layer) is proven in Phase 4.

#[test]
fn exit_expression_variant_clones_and_compares() {
    let original = EarlyTermination::ExitExpression {
        pattern: "STOP.".to_string(),
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    // Different pattern must not compare equal — guards against a
    // future derive mistake that ignores fields.
    let other = EarlyTermination::ExitExpression {
        pattern: "HALT.".to_string(),
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
    };
    assert_ne!(original, other);
}

#[test]
fn runaway_repetition_variant_clones_and_compares() {
    let original = EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 30,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    let other = EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 31,
    };
    assert_ne!(original, other);
}

#[test]
fn runaway_volume_variant_clones_and_compares() {
    let original = EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 32 * 1024 * 1024,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    let other = EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 32 * 1024 * 1024 + 1,
    };
    assert_ne!(original, other);
}

#[test]
fn new_variants_are_distinct_from_legacy_terminations() {
    // Exhaustive-distinctness smoke test: none of the three new
    // variants may compare equal to a legacy variant or to each other.
    let exit = EarlyTermination::ExitExpression {
        pattern: "x".to_string(),
        scope: None,
    };
    let rep = EarlyTermination::RunawayRepetition {
        cycle_len: 1,
        repeats: 30,
    };
    let vol = EarlyTermination::RunawayVolume { lines: 1, bytes: 1 };
    let rate = EarlyTermination::RateLimit {
        message: "m".to_string(),
        reset_at: None,
    };
    assert_ne!(exit, rep);
    assert_ne!(exit, vol);
    assert_ne!(rep, vol);
    assert_ne!(exit, rate);
    assert_ne!(rep, rate);
    assert_ne!(vol, rate);
}

// --- Stalled-generation detector (Phase 2 sanity; full matrix in Phase 5) ---
//
// These exercise the two private detector helpers directly with an
// injected `now: Instant` so no real time passes. The `on_llm_call`
// handler reads `Instant::now()` itself; the count/time logic it delegates
// to is what these lock down.

const STALL_BUDGET: Duration = Duration::from_secs(600);

fn armed_bridge() -> OpenCodeLogBridge<RecordingSink> {
    OpenCodeLogBridge::new(
        RecordingSink::default(),
        stdout_seen(),
        None,
        Some(STALL_BUDGET),
    )
}

#[test]
fn four_streamed_generations_past_budget_trip_stalled_generation() {
    let mut bridge = armed_bridge();
    let base = Instant::now();
    bridge.reset_stalled_generation_progress(base);
    let past = base + STALL_BUDGET + Duration::from_secs(1);
    let ctx = StalledGenerationContext {
        session_id: Some("ses_a".into()),
        ..Default::default()
    };

    // First three accumulate churn but the count condition is not yet met.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        assert!(
            bridge
                .record_llm_call_and_check_trip(past, ctx.clone())
                .is_none()
        );
    }
    match bridge.record_llm_call_and_check_trip(past, ctx.clone()) {
        Some(EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        }) => {
            assert_eq!(generation_count, MAX_GENERATIONS_WITHOUT_PROGRESS);
            assert!(stall_duration >= STALL_BUDGET);
            assert_eq!(context.session_id.as_deref(), Some("ses_a"));
        }
        other => panic!("expected StalledGeneration, got {other:?}"),
    }
}

#[test]
fn three_generations_past_budget_do_not_trip() {
    let mut bridge = armed_bridge();
    let base = Instant::now();
    bridge.reset_stalled_generation_progress(base);
    let past = base + STALL_BUDGET + Duration::from_secs(1);
    let ctx = StalledGenerationContext::default();

    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        assert!(
            bridge
                .record_llm_call_and_check_trip(past, ctx.clone())
                .is_none(),
            "count condition must remain unmet below the threshold",
        );
    }
}

#[test]
fn four_generations_under_budget_do_not_trip() {
    let mut bridge = armed_bridge();
    let base = Instant::now();
    bridge.reset_stalled_generation_progress(base);
    let recent = base + Duration::from_secs(60);
    let ctx = StalledGenerationContext::default();

    for _ in 0..MAX_GENERATIONS_WITHOUT_PROGRESS {
        assert!(
            bridge
                .record_llm_call_and_check_trip(recent, ctx.clone())
                .is_none(),
            "progress-silence condition must remain unmet under the budget",
        );
    }
}

#[test]
fn progress_reset_restarts_the_generation_count() {
    let mut bridge = armed_bridge();
    let base = Instant::now();
    bridge.reset_stalled_generation_progress(base);
    let past = base + STALL_BUDGET + Duration::from_secs(1);
    let ctx = StalledGenerationContext::default();

    // Churn up to one below the threshold, then a progress event resets.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        bridge.record_llm_call_and_check_trip(past, ctx.clone());
    }
    bridge.reset_stalled_generation_progress(past);

    // A fresh run of (threshold - 1) calls must stay under the count even
    // though wall-clock silence since the reset is now zero.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        assert!(
            bridge
                .record_llm_call_and_check_trip(past, ctx.clone())
                .is_none(),
            "reset should have cleared the churn count",
        );
    }
}

#[test]
fn disabled_guard_never_trips_even_with_churn_past_budget() {
    // `None` stall_timeout disables the guard. Encodes Design Decision 3's
    // anti-correlation lock at the unit level: churn alone never terminates.
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None);
    let base = Instant::now();
    bridge.reset_stalled_generation_progress(base);
    let past = base + STALL_BUDGET + Duration::from_secs(1);
    let ctx = StalledGenerationContext::default();

    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS + 2) {
        assert!(
            bridge
                .record_llm_call_and_check_trip(past, ctx.clone())
                .is_none(),
            "a disabled guard must never trip regardless of churn",
        );
    }
}

// ------------------------------------------------------------------
// Phase-5 stalled-generation spec matrix (ingest-level)
//
// The helper-level count/time logic is locked above with an injected
// `now`. These exercise the full `ingest` path so the reset taxonomy, the
// terminal-event shape, the long-tool exemption, and independence from the
// `RepeatedStreamError` backstop are proven end-to-end. `on_llm_call` reads
// `Instant::now()` itself, so the time condition is forced two ways:
// a generous `STALL_BUDGET` keeps the guard from tripping while we observe
// counter/clock state, and a `Duration::ZERO` budget makes the
// progress-silence condition trivially true so a trip turns purely on the
// (still-mandatory) churn count.
// ------------------------------------------------------------------

const STREAMED_LLM_CALL: &str = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";

/// Bridge armed with a `Duration::ZERO` budget: the progress-silence
/// condition is trivially satisfied, so a trip turns purely on the churn
/// count reaching `MAX_GENERATIONS_WITHOUT_PROGRESS` (which stays
/// mandatory). Used to make the count-driven event path deterministic
/// without advancing the monotonic clock.
fn count_only_bridge(
    tx: Sender<EarlyTermination>,
) -> OpenCodeLogBridge<RecordingSink> {
    OpenCodeLogBridge::new(
        RecordingSink::default(),
        stdout_seen(),
        Some(tx),
        Some(Duration::ZERO),
    )
}

#[test]
fn genuine_step_advance_via_ingest_resets_generation_count() {
    let mut bridge = armed_bridge();
    // Two streamed generations accumulate churn; the generous budget keeps
    // the guard from tripping while we observe the counter.
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.generation_count_since_progress(), 2);

    let step = "INFO  2026-05-12T20:00:13 +0ms service=session.prompt session.id=ses_a step=5 logSpan.http.span.4=55ms loop";
    assert_eq!(bridge.ingest(step), StderrIngestOutcome::Consumed);
    assert_eq!(
        bridge.generation_count_since_progress(), 0,
        "a genuine step advance must reset the churn count",
    );
}

#[test]
fn deduped_step_loop_does_not_reset_but_genuine_advance_does() {
    let mut bridge = armed_bridge();
    let step0 = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
    let step0_again = "INFO  2026-05-12T20:00:13 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=99ms loop";
    let step1 = "INFO  2026-05-12T20:00:14 +0ms service=session.prompt session.id=ses_a step=1 logSpan.http.span.4=120ms loop";

    // Establish step=0 (resets), then churn two generations.
    assert_eq!(bridge.ingest(step0), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.generation_count_since_progress(), 2);

    // A deduped repeat of the same (session, step) returns early and must
    // NOT reset the churn count.
    assert_eq!(bridge.ingest(step0_again), StderrIngestOutcome::Consumed);
    assert_eq!(
        bridge.generation_count_since_progress(), 2,
        "a deduped step-loop repeat must not reset the churn count",
    );

    // A genuine step advance does reset.
    assert_eq!(bridge.ingest(step1), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.generation_count_since_progress(), 0);
}

#[test]
fn liveness_only_events_do_not_reset_stalled_generation_state() {
    let mut bridge = armed_bridge();
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.generation_count_since_progress(), 2);
    let progress_at_before = bridge.last_progress_at();

    // None of these handler paths are progress-class events, so neither the
    // churn count nor the silence clock may move.
    let http = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/x/message http.status=200 logSpan.http.span.4=99ms Sent HTTP response";
    let permission = r#"INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated"#;
    let bus = "INFO 2026-04-15T21:28:30 +5ms service=bus msg=internal chatter";
    assert_eq!(bridge.ingest(http), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(permission), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(bus), StderrIngestOutcome::Consumed);
    // Raw, unstructured stderr bytes likewise leave the state untouched.
    assert_eq!(bridge.ingest("just some chatter"), StderrIngestOutcome::NotConsumed);

    assert_eq!(
        bridge.generation_count_since_progress(), 2,
        "liveness-only events and raw bytes must not reset the churn count",
    );
    assert_eq!(
        bridge.last_progress_at(), progress_at_before,
        "liveness-only events and raw bytes must not advance the silence clock",
    );
}

#[test]
fn long_tool_shape_never_trips_even_past_budget() {
    // AC6: a long-running tool that emits step loops and HTTP responses but
    // no `llm_call_start` records must never trip this guard, even when the
    // progress-silence condition is trivially satisfied (ZERO budget).
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = count_only_bridge(tx);

    let step = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
    let http = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/x/message http.status=200 logSpan.http.span.4=99ms Sent HTTP response";
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS + 4) {
        assert_eq!(bridge.ingest(step), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(http), StderrIngestOutcome::Consumed);
    }

    assert_eq!(
        bridge.generation_count_since_progress(), 0,
        "no llm_call_start means the churn count never accumulates",
    );
    assert!(
        !bridge
            .sink
            .events
            .iter()
            .any(|e| matches!(e, SemanticEvent::Error { .. })),
        "long-tool shape must not emit a terminal error",
    );
    assert!(
        rx.try_recv().is_err(),
        "long-tool shape must not request early termination",
    );
}

#[test]
fn stalled_generation_emits_agent_native_terminal_event_with_safe_context() {
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = count_only_bridge(tx);

    // The count condition is still mandatory: only the fourth streamed
    // generation crosses MAX_GENERATIONS_WITHOUT_PROGRESS.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    }
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);

    let error = bridge
        .sink
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            SemanticEvent::Error {
                message,
                terminal,
                kind,
                extra,
            } => Some((message, *terminal, *kind, extra)),
            _ => None,
        })
        .expect("a terminal stalled-generation error must be emitted");
    let (message, terminal, kind, extra) = error;
    assert!(terminal, "stalled-generation error must be terminal");
    assert_eq!(kind, SemanticErrorKind::AgentNative);
    assert!(
        message.to_lowercase().contains("stalled generation"),
        "message must classify the failure: {message}",
    );
    assert_string(extra, "classification", "stalled_generation");
    assert_string(extra, "label", "Stalled Generation");
    assert_eq!(
        extra.get("generation_count"),
        Some(&json!(MAX_GENERATIONS_WITHOUT_PROGRESS)),
    );
    assert!(
        extra.get("stall_duration_ms").and_then(Value::as_u64).is_some(),
        "stall_duration_ms must be a number: {extra}",
    );
    // Safe context only — identity, never payloads.
    assert_string(extra, "session_id", "ses_a");
    assert_string(extra, "agent", "build");
    assert_string(extra, "provider_id", "kimi-for-coding");
    assert_string(extra, "model_id", "k2p6");
    assert_string(extra, "mode", "primary");
    let extra_obj = extra.as_object().expect("extra is an object");
    for forbidden in ["prompt", "prompt_text", "tool", "tool_input", "tool_output", "input"] {
        assert!(
            !extra_obj.contains_key(forbidden),
            "extra must not leak `{forbidden}`: {extra}",
        );
    }

    match rx.try_recv() {
        Ok(EarlyTermination::StalledGeneration {
            generation_count,
            context,
            ..
        }) => {
            assert_eq!(generation_count, MAX_GENERATIONS_WITHOUT_PROGRESS);
            assert_eq!(context.session_id.as_deref(), Some("ses_a"));
            assert_eq!(context.agent.as_deref(), Some("build"));
        }
        other => panic!("expected EarlyTermination::StalledGeneration, got {other:?}"),
    }
}

#[test]
fn repeated_stream_error_is_independent_of_llm_call_churn() {
    // The two backstops keep separate counters. Interleaving streamed
    // generations between `stream error` records must NOT clear the
    // consecutive-stream-error count (only a genuine step advance does),
    // and a `stream error` must NOT clear the stalled-generation count.
    // A generous budget keeps the stalled guard from firing so we observe
    // the RepeatedStreamError path in isolation.
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = OpenCodeLogBridge::new(
        RecordingSink::default(),
        stdout_seen(),
        Some(tx),
        Some(STALL_BUDGET),
    );

    // Below threshold: alternate a generation with a stream error. The
    // generation must not reset the stream-error count.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS - 1) {
        bridge.ingest(STREAMED_LLM_CALL);
        bridge.ingest(GENERIC_STREAM_ERROR);
    }
    assert!(
        rx.try_recv().is_err(),
        "interleaved generations must not let the backstop fire early",
    );
    let churn_before_error = bridge.generation_count_since_progress();
    assert!(churn_before_error > 0, "generations should have accumulated");

    // The threshold-crossing stream error trips RepeatedStreamError; it
    // must not have been reset by the interleaved generations.
    bridge.ingest(GENERIC_STREAM_ERROR);
    match rx.try_recv() {
        Ok(EarlyTermination::RepeatedStreamError { count }) => {
            assert_eq!(count, MAX_CONSECUTIVE_STREAM_ERRORS);
        }
        other => panic!("expected RepeatedStreamError, got {other:?}"),
    }
    assert_eq!(
        bridge.generation_count_since_progress(), churn_before_error,
        "a stream error must not clear the stalled-generation churn count",
    );
}

#[test]
fn early_termination_fires_at_most_once_when_both_guards_could_trip() {
    // With a ZERO budget the stalled guard trips on the fourth generation;
    // a flood of stream errors afterward could also trip RepeatedStreamError,
    // but `early_terminate_fired` idempotency must hold — exactly one signal.
    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = count_only_bridge(tx);

    for _ in 0..MAX_GENERATIONS_WITHOUT_PROGRESS {
        bridge.ingest(STREAMED_LLM_CALL);
    }
    assert!(
        matches!(
            rx.try_recv(),
            Ok(EarlyTermination::StalledGeneration { .. })
        ),
        "the stalled guard should have fired first",
    );

    // A subsequent stream-error flood must not re-fire the channel.
    for _ in 0..(MAX_CONSECUTIVE_STREAM_ERRORS + 2) {
        bridge.ingest(GENERIC_STREAM_ERROR);
    }
    assert!(
        rx.try_recv().is_err(),
        "fire_early_termination must fire at most once per bridge",
    );
}

// ------------------------------------------------------------------
// Stdout-origin progress reset (real producer/sink wiring)
//
// AC5: ANY progress-class event resets the generation count and moves
// `last_progress_at` forward — including stdout NDJSON semantic events
// (`OutputText`, `ToolCall`, …) that never touch the stderr bridge. These
// exercise the *real* wiring: the `StalledProgressObserverSink` built from
// the bridge's shared progress cell, exactly as `policy.rs` wires it, rather
// than the private bridge reset helper.
// ------------------------------------------------------------------

/// Build the stdout-side observer over the bridge's shared progress cell,
/// mirroring the `policy.rs` plumbing. Events forwarded through the returned
/// sink reset the same counter the bridge accumulates against.
fn stdout_progress_observer(
    bridge: &OpenCodeLogBridge<RecordingSink>,
) -> crate::stream::semantic::StalledProgressObserverSink<RecordingSink> {
    crate::stream::semantic::StalledProgressObserverSink::new(
        RecordingSink::default(),
        bridge.stalled_generation_progress(),
    )
}

#[test]
fn stdout_progress_event_resets_stalled_generation_state() {
    use crate::stream::semantic::SemanticEventSink;

    let mut bridge = armed_bridge();
    let mut stdout = stdout_progress_observer(&bridge);

    // Accumulate retry churn on the stderr side.
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.ingest(STREAMED_LLM_CALL), StderrIngestOutcome::Consumed);
    assert_eq!(bridge.generation_count_since_progress(), 2);
    let progress_before = bridge.last_progress_at();

    // A genuine stdout-origin progress event — model output — must reset the
    // shared counter and advance the silence clock even though it arrives on
    // a different producer than the stderr bridge.
    stdout.on_semantic_event(SemanticEvent::OutputText {
        text: "real progress".into(),
        extra: Value::Null,
    });

    assert_eq!(
        bridge.generation_count_since_progress(),
        0,
        "stdout-origin progress must reset the churn count",
    );
    assert!(
        bridge.last_progress_at() >= progress_before,
        "stdout-origin progress must advance the silence clock",
    );
}

#[test]
fn stdout_tool_lifecycle_events_reset_stalled_generation_state() {
    use crate::stream::semantic::SemanticEventSink;

    // Each progress-class stdout variant in the spec's reset taxonomy must
    // clear churn accumulated on the stderr side.
    let progress_events = [
        SemanticEvent::OutputText {
            text: "t".into(),
            extra: Value::Null,
        },
        SemanticEvent::Reasoning {
            text: "r".into(),
            extra: Value::Null,
        },
        SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: None,
            input: None,
            extra: Value::Null,
        },
        SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: None,
            status: None,
            exit_code: None,
            output: None,
            extra: Value::Null,
        },
        SemanticEvent::FileChange {
            path: Some("a.rs".into()),
            change_kind: None,
            extra: Value::Null,
        },
        SemanticEvent::PlanUpdate {
            message: None,
            extra: Value::Null,
        },
        SemanticEvent::SubagentStart {
            name: None,
            id: None,
            extra: Value::Null,
        },
        SemanticEvent::SubagentStop {
            name: None,
            id: None,
            status: None,
            extra: Value::Null,
        },
    ];

    for event in progress_events {
        let kind = event.kind_str();
        let mut bridge = armed_bridge();
        let mut stdout = stdout_progress_observer(&bridge);
        bridge.ingest(STREAMED_LLM_CALL);
        bridge.ingest(STREAMED_LLM_CALL);
        assert_eq!(bridge.generation_count_since_progress(), 2);

        stdout.on_semantic_event(event);
        assert_eq!(
            bridge.generation_count_since_progress(),
            0,
            "stdout {kind} must reset the churn count",
        );
    }
}

#[test]
fn stdout_liveness_only_events_do_not_reset_stalled_generation_state() {
    use crate::stream::semantic::SemanticEventSink;

    let mut bridge = armed_bridge();
    let mut stdout = stdout_progress_observer(&bridge);
    bridge.ingest(STREAMED_LLM_CALL);
    bridge.ingest(STREAMED_LLM_CALL);
    assert_eq!(bridge.generation_count_since_progress(), 2);
    let progress_before = bridge.last_progress_at();

    // Liveness-only stdout events (diagnostics, session/turn envelope) must
    // not reset the guard — only forward progress does.
    let liveness = [
        SemanticEvent::Info {
            message: "heartbeat".into(),
            extra: Value::Null,
        },
        SemanticEvent::Warning {
            message: "soft warning".into(),
            extra: Value::Null,
        },
        SemanticEvent::Error {
            message: "non-terminal".into(),
            terminal: false,
            kind: SemanticErrorKind::Unknown,
            extra: Value::Null,
        },
        SemanticEvent::TurnStart { extra: Value::Null },
        SemanticEvent::SessionStart {
            session_id: None,
            model: None,
            extra: Value::Null,
        },
    ];
    for event in liveness {
        stdout.on_semantic_event(event);
    }

    assert_eq!(
        bridge.generation_count_since_progress(),
        2,
        "liveness-only stdout events must not reset the churn count",
    );
    assert_eq!(
        bridge.last_progress_at(),
        progress_before,
        "liveness-only stdout events must not advance the silence clock",
    );
}

#[test]
fn stdout_progress_keeps_a_progressing_run_from_tripping_the_guard() {
    // The end-to-end finding: a run accumulates churn, makes REAL stdout
    // progress, and must NOT trip on a later `llm_call_start`. With a ZERO
    // budget the progress-silence condition is trivially true, so only the
    // (mandatory) churn count can trip — and the stdout reset is what keeps
    // it below the threshold.
    use crate::stream::semantic::SemanticEventSink;

    let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let mut bridge = count_only_bridge(tx);
    let mut stdout = stdout_progress_observer(&bridge);

    // Churn up to one below the threshold, then make stdout progress.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        bridge.ingest(STREAMED_LLM_CALL);
    }
    stdout.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("ok".into()),
        exit_code: Some(0),
        output: None,
        extra: Value::Null,
    });
    assert_eq!(
        bridge.generation_count_since_progress(),
        0,
        "stdout progress must have reset the churn count",
    );

    // A fresh run of (threshold - 1) generations must stay under the count,
    // so the guard does not trip on the progressing run.
    for _ in 0..(MAX_GENERATIONS_WITHOUT_PROGRESS - 1) {
        bridge.ingest(STREAMED_LLM_CALL);
    }
    assert!(
        rx.try_recv().is_err(),
        "a run that made stdout progress must not trip the stalled-generation guard",
    );

    // One more generation (no intervening progress) reaches the threshold
    // and now trips, proving the guard is still armed after the reset.
    bridge.ingest(STREAMED_LLM_CALL);
    assert!(
        matches!(rx.try_recv(), Ok(EarlyTermination::StalledGeneration { .. })),
        "the guard must still trip once churn resumes past the threshold",
    );
}

// ── E5 glue-mode signal shim ────────────────────────────────────────

use claudine_catalog_types::SignalKind;

const USAGE_CAP_1178: &str = include_str!(
    "../../../../../../docs/research/signals/fixtures/opencode/stream-error-1178-usage-cap.txt"
);
const VERSION_ANNOUNCEMENT: &str = include_str!(
    "../../../../../../docs/research/signals/fixtures/opencode/version-announcement.txt"
);

/// Bridge wired to a fresh hub over the compiled opencode table.
fn shim_bridge() -> (OpenCodeLogBridge<RecordingSink>, Arc<SignalHub>) {
    let hub = Arc::new(SignalHub::new(
        crate::signals::detection_table("opencode").expect("opencode table"),
    ));
    let bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None)
        .with_signal_hub(Arc::clone(&hub));
    (bridge, hub)
}

#[test]
fn shim_promotes_usage_cap_stderr_line_to_usage_capped_signal() {
    let (mut bridge, hub) = shim_bridge();
    for line in USAGE_CAP_1178.lines().filter(|l| !l.trim().is_empty()) {
        bridge.ingest(line);
    }

    let signals = hub.drain();
    let capped = signals
        .iter()
        .find(|s| s.event.kind() == SignalKind::UsageCapped)
        .expect("usage_capped signal from promoted stderr");
    assert_eq!(capped.source, SignalSource::StderrPromoted);
    let TaxonomySignalEvent::UsageCapped {
        message, lifts_at, ..
    } = &capped.event
    else {
        panic!("kind checked above");
    };
    assert!(
        message
            .as_deref()
            .is_some_and(|m| m.contains("Usage limit reached for 5 hour")),
        "message must carry the provider error: {message:?}"
    );
    assert!(
        lifts_at.is_some(),
        "lifts_at must be extracted from the classifier's reset_at"
    );
}

#[test]
fn shim_boot_banner_emits_provider_version_and_narrows_selection() {
    let (mut bridge, hub) = shim_bridge();
    let banner = VERSION_ANNOUNCEMENT
        .lines()
        .next()
        .expect("fixture has a banner line");
    bridge.ingest(banner);
    // A usage-cap line after the banner still fires: 1.14.48 admits the
    // un-bounded/legacy records, proving version narrowing did not
    // wipe the candidate set.
    for line in USAGE_CAP_1178.lines().filter(|l| !l.trim().is_empty()) {
        bridge.ingest(line);
    }

    let signals = hub.drain();
    let version = signals
        .iter()
        .find(|s| s.event.kind() == SignalKind::ProviderVersion)
        .expect("provider_version signal from the boot banner");
    assert_eq!(
        version.event,
        TaxonomySignalEvent::ProviderVersion {
            version: "1.14.48".to_string()
        }
    );
    assert!(
        signals
            .iter()
            .any(|s| s.event.kind() == SignalKind::UsageCapped),
        "usage cap must still fire under the narrowed (1.14.x) selection"
    );
}

#[test]
fn fire_early_termination_mirrors_the_bespoke_signal_once() {
    let (tx, _rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let hub = Arc::new(SignalHub::without_table());
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None)
            .with_signal_hub(Arc::clone(&hub));

    bridge.fire_early_termination(EarlyTermination::RepeatedStreamError { count: 5 });
    // Second fire is idempotent — no second signal.
    bridge.fire_early_termination(EarlyTermination::RepeatedStreamError { count: 6 });

    let signals = hub.drain();
    assert_eq!(signals.len(), 1);
    assert_eq!(
        signals[0].event,
        TaxonomySignalEvent::RepeatedStreamError { count: 5 }
    );
    assert_eq!(signals[0].source, SignalSource::StderrPromoted);
}

/// EarlyTermination → SignalKind, one row per variant. The mapping fn's
/// match is exhaustive, so a new variant fails compilation there; this
/// test pins the KIND each existing variant maps to.
#[test]
fn early_termination_to_signal_event_covers_every_variant() {
    let cases: Vec<(EarlyTermination, SignalKind)> = vec![
        (
            EarlyTermination::RateLimit {
                message: "cap".into(),
                reset_at: None,
            },
            // Terminal-cap semantics (see `to_signal_event`), not a
            // transient rate_limited.
            SignalKind::UsageCapped,
        ),
        (
            EarlyTermination::Timeout {
                message: "wall".into(),
            },
            SignalKind::Timeout,
        ),
        (
            EarlyTermination::StepTimeout {
                message: "silent".into(),
                outstanding: Vec::new(),
            },
            SignalKind::StepTimeout,
        ),
        (
            EarlyTermination::ExitExpression {
                pattern: "FATAL".into(),
                scope: Some("opencode/kimi".into()),
            },
            SignalKind::ExitExpression,
        ),
        (
            EarlyTermination::RunawayRepetition {
                cycle_len: 3,
                repeats: 12,
            },
            SignalKind::RunawayRepetition,
        ),
        (
            EarlyTermination::RunawayVolume {
                lines: 50_000,
                bytes: 33_554_432,
            },
            SignalKind::RunawayVolume,
        ),
        (
            EarlyTermination::RepeatedStreamError { count: 5 },
            SignalKind::RepeatedStreamError,
        ),
        (
            EarlyTermination::StalledGeneration {
                generation_count: 4,
                stall_duration: Duration::from_secs(600),
                context: StalledGenerationContext::default(),
            },
            SignalKind::StalledGeneration,
        ),
    ];
    for (termination, expected) in cases {
        assert_eq!(
            termination.to_signal_event().kind(),
            expected,
            "mapping drifted for {termination:?}"
        );
    }
}

#[test]
fn exit_expression_mapping_carries_pattern_and_scope() {
    let event = EarlyTermination::ExitExpression {
        pattern: "unrecoverable".into(),
        scope: Some("opencode/kimi-for-coding/k2p7".into()),
    }
    .to_signal_event();
    assert_eq!(
        event,
        TaxonomySignalEvent::ExitExpression {
            pattern: "unrecoverable".into(),
            scope: Some("opencode/kimi-for-coding/k2p7".into()),
        }
    );
}
