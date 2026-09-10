//! session lifecycle bridge tests.

use super::*;

#[test]
fn new_format_bridge_consumes_lifecycle_lines() {
    let fixture =
        include_str!("../../../../../../tests/fixtures/logs/opencode-new-format-lifecycle.txt");
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
        include_str!("../../../../../../tests/fixtures/logs/opencode-new-format-serviceless.txt");
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

