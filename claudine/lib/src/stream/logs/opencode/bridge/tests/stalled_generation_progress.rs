//! stalled generation progress bridge tests.

use super::*;

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
