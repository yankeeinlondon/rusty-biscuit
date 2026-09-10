//! stdout stderr coordination bridge tests.

use super::*;

/// Builds the stdout-side observer over the bridge's shared progress cell,
/// mirroring the production sink wiring.
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

