//! usage retry guards bridge tests.

use super::*;

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

