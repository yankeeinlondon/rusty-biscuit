//! ingest classification bridge tests.

use super::*;

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

