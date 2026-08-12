//! Phase 6 — content-detector driving from the live semantic sink.
//!
//! These tests exercise the sink seam directly (`on_semantic_event`): a sink
//! armed with a [`ContentDetector`] and a trip [`Sender`] should fire on a
//! guard breach observed in `OutputText`/`Reasoning`, never on tool payloads,
//! and suppress further output rendering once a trip has fired.

use super::*;
use crate::commands::wrap::runaway_guard::ResolvedGuardInputs;
use claudine::runaway::{
    CompiledExitExpressions, ContentDetector, DetectorConfig, ExitExpressionEntry,
    ExitExpressionInput, GuardSettings, PatternKind,
};
use claudine::stream::logs::EarlyTermination;
use serde_json::json;
use std::sync::Arc as StdArc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc::{Receiver, channel};

/// Build a sink with an armed detector + trip sender; returns the sink and
/// the trip receiver the caller asserts on.
fn armed_sink(
    detector: ContentDetector,
) -> (LiveSemanticSink, Receiver<EarlyTermination>) {
    let dispatch = Box::new(|_event: AgenticEvent, _meta: DispatchEventMeta| {});
    let emit = Box::new(|_line: &str| {});
    let mut sink = LiveSemanticSink::new(
        Provider::OpenCode,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.set_content_detector(Some(detector));
    let (tx, rx) = channel();
    sink.set_trip_sender(tx);
    (sink, rx)
}

fn detector_with_patterns(inputs: &[ExitExpressionInput]) -> ContentDetector {
    let compiled = CompiledExitExpressions::compile(inputs).expect("patterns compile");
    ContentDetector::new(DetectorConfig::default(), compiled)
}

fn output(text: &str) -> SemanticEvent {
    SemanticEvent::OutputText {
        text: text.to_string(),
        extra: json!({}),
    }
}

// ---------------------------------------------------------------------------
// VC-6.1 — exit-expression trip end-to-end (streaming sink).
// ---------------------------------------------------------------------------

#[test]
fn vc_6_1_exit_expression_trips_and_sends_on_channel() {
    let detector = detector_with_patterns(&[ExitExpressionInput {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: Some("opencode".to_string()),
    }]);
    let (mut sink, rx) = armed_sink(detector);

    sink.on_semantic_event(output("working on it\n"));
    assert!(rx.try_recv().is_err(), "no trip before the matching line");

    sink.on_semantic_event(output("STOP.\n"));
    match rx.try_recv() {
        Ok(EarlyTermination::ExitExpression { pattern, scope }) => {
            assert_eq!(pattern, "STOP.");
            assert_eq!(scope.as_deref(), Some("opencode"));
        }
        other => panic!("expected ExitExpression trip, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// VC-6.2 — repetition trip end-to-end (streaming sink).
// ---------------------------------------------------------------------------

#[test]
fn vc_6_2_repetition_trips_at_threshold() {
    let detector = ContentDetector::new(DetectorConfig::default(), CompiledExitExpressions::empty());
    let (mut sink, rx) = armed_sink(detector);

    // The captured 6-line cycle. Feed 29 cycles (under threshold): no trip.
    let cycle = "Done.\nNo more.\nEnd.\nSTOP.\nOK.\nBye.\n";
    let mut under = String::new();
    for _ in 0..29 {
        under.push_str(cycle);
    }
    sink.on_semantic_event(output(&under));
    assert!(rx.try_recv().is_err(), "29 cycles must not trip");

    // The 30th cycle crosses the threshold.
    sink.on_semantic_event(output(cycle));
    match rx.try_recv() {
        Ok(EarlyTermination::RunawayRepetition { cycle_len, repeats }) => {
            assert_eq!(cycle_len, 6);
            assert!(repeats >= 30, "repeats should reach the threshold: {repeats}");
        }
        other => panic!("expected RunawayRepetition trip, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// VC-6.3 — per-turn volume reset across TurnComplete.
// ---------------------------------------------------------------------------

#[test]
fn vc_6_3_per_turn_volume_resets_on_turn_complete() {
    // Small line cap so the test stays fast; bytes effectively unbounded.
    let cfg = DetectorConfig {
        repetition_enabled: false,
        max_lines: 100,
        max_bytes: u64::MAX,
        ..DetectorConfig::default()
    };
    let detector = ContentDetector::new(cfg, CompiledExitExpressions::empty());
    let (mut sink, rx) = armed_sink(detector);

    // 100 distinct lines: exactly at the cap, no trip yet.
    let mut chunk = String::new();
    for i in 0..100u32 {
        chunk.push_str(&format!("line {i}\n"));
    }
    sink.on_semantic_event(output(&chunk));
    assert!(rx.try_recv().is_err(), "at-cap must not trip");

    // TurnComplete resets the per-turn counters.
    sink.on_semantic_event(SemanticEvent::TurnComplete {
        provider_status: None,
        token_usage: None,
        cost_usd: None,
        duration_ms: None,
        extra: json!({}),
    });

    // Another 100 distinct lines after reset still does not trip.
    let mut chunk2 = String::new();
    for i in 0..100u32 {
        chunk2.push_str(&format!("post {i}\n"));
    }
    sink.on_semantic_event(output(&chunk2));
    assert!(
        rx.try_recv().is_err(),
        "post-reset turn must not accumulate into a trip"
    );

    // One more line tips past the cap in this turn.
    sink.on_semantic_event(output("one more\n"));
    assert!(
        matches!(rx.try_recv(), Ok(EarlyTermination::RunawayVolume { .. })),
        "exceeding the per-turn cap should trip volume"
    );
}

// ---------------------------------------------------------------------------
// VC-6.5 — output rendering is suppressed once a trip fires.
// ---------------------------------------------------------------------------

#[test]
fn vc_6_5_rendering_suppressed_after_trip() {
    let detector = detector_with_patterns(&[ExitExpressionInput {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }]);
    let (mut sink, _rx) = armed_sink(detector);

    let forwarded: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
    {
        let sink_forwarded = forwarded.clone();
        sink.set_output_text_sink(Box::new(move |chunk: &str| {
            sink_forwarded.lock().unwrap().push(chunk.to_string());
        }));
    }

    sink.on_semantic_event(output("hello\n"));
    sink.on_semantic_event(output("STOP.\n")); // trips here
    sink.on_semantic_event(output("tail that must not echo\n"));

    let seen = forwarded.lock().unwrap().concat();
    assert!(seen.contains("hello"), "pre-trip text must render: {seen:?}");
    assert!(
        !seen.contains("STOP."),
        "the tripping chunk must be suppressed: {seen:?}"
    );
    assert!(
        !seen.contains("tail that must not echo"),
        "post-trip text must be suppressed: {seen:?}"
    );
}

// ---------------------------------------------------------------------------
// VC-6.6 (sink half) — only one trip is sent even on continued flooding.
// ---------------------------------------------------------------------------

#[test]
fn trip_fires_exactly_once() {
    let detector = detector_with_patterns(&[ExitExpressionInput {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }]);
    let (mut sink, rx) = armed_sink(detector);

    sink.on_semantic_event(output("STOP.\n"));
    sink.on_semantic_event(output("STOP.\n"));
    sink.on_semantic_event(output("STOP.\n"));

    assert!(rx.try_recv().is_ok(), "first trip is sent");
    assert!(
        rx.try_recv().is_err(),
        "a trip is terminal — no further signals after the first"
    );
}

// ---------------------------------------------------------------------------
// VC-6.7 — tool payloads are never scanned (A2).
// ---------------------------------------------------------------------------

#[test]
fn vc_6_7_tool_payloads_never_trip() {
    let detector = detector_with_patterns(&[ExitExpressionInput {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }]);
    let (mut sink, rx) = armed_sink(detector);

    // A ToolResult whose output repeats the exit-expression must NOT trip:
    // only OutputText/Reasoning are scanned.
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: Some("t1".into()),
        status: Some("ok".into()),
        exit_code: Some(0),
        output: Some(json!("STOP.\nSTOP.\nSTOP.\n")),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t2".into()),
        input: Some(json!({ "command": "echo STOP." })),
        extra: json!({}),
    });

    assert!(
        rx.try_recv().is_err(),
        "tool payloads must never feed the content detector"
    );
}

// ---------------------------------------------------------------------------
// Reasoning text is scanned just like OutputText.
// ---------------------------------------------------------------------------

#[test]
fn reasoning_text_is_scanned() {
    let detector = detector_with_patterns(&[ExitExpressionInput {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }]);
    let (mut sink, rx) = armed_sink(detector);

    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "thinking... STOP.\n".to_string(),
        extra: json!({}),
    });
    assert!(
        matches!(rx.try_recv(), Ok(EarlyTermination::ExitExpression { .. })),
        "reasoning text must be scanned for exit expressions"
    );
}

// ---------------------------------------------------------------------------
// A sink with no detector armed never trips (opt-out parity).
// ---------------------------------------------------------------------------

#[test]
fn unarmed_sink_never_trips() {
    let dispatch = Box::new(|_event: AgenticEvent, _meta: DispatchEventMeta| {});
    let emit = Box::new(|_line: &str| {});
    let mut sink = LiveSemanticSink::new(
        Provider::OpenCode,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    assert!(!sink.has_content_detector());
    // Without a detector, feeding obvious runaway text is a no-op.
    sink.on_semantic_event(output("STOP.\nSTOP.\nSTOP.\n"));
    // No panic, no trip channel — behavior identical to pre-guard claudine.
}

// ---------------------------------------------------------------------------
// Provider-reported model re-scope (Review 2 High #2).
//
// An exit expression scoped to `{agent}/{model}` is INACTIVE when the
// detector is first compiled with no launch-time model hint, then becomes
// ACTIVE once `SessionStart { model }` supplies the matching model.
// ---------------------------------------------------------------------------

/// One agent/model-scoped exit-expression entry (`opencode/k2p7`).
fn agent_model_entry(pattern: &str, scope: &str) -> ExitExpressionEntry {
    ExitExpressionEntry {
        patterns: vec![pattern.to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: Some(scope.to_string()),
    }
}

/// Build a sink armed by compiling `inputs` for the (possibly absent)
/// `launch_model`, with the re-scope source wired so a later `SessionStart`
/// can rebuild the in-scope set. Returns the sink and trip receiver.
fn rescope_sink(
    inputs: ResolvedGuardInputs,
    launch_model: Option<&str>,
) -> (LiveSemanticSink, Receiver<EarlyTermination>) {
    let resolved = inputs
        .compile_for_model(launch_model)
        .expect("launch-time compile");
    let inputs = StdArc::new(inputs);

    let dispatch = Box::new(|_event: AgenticEvent, _meta: DispatchEventMeta| {});
    let emit = Box::new(|_line: &str| {});
    let mut sink = LiveSemanticSink::new(
        Provider::OpenCode,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.set_content_detector(resolved.detector);
    sink.set_guard_rescope_source(inputs, launch_model);
    let (tx, rx) = channel();
    sink.set_trip_sender(tx);
    (sink, rx)
}

fn session_start(model: Option<&str>) -> SemanticEvent {
    SemanticEvent::SessionStart {
        session_id: Some("sess-1".to_string()),
        model: model.map(str::to_string),
        extra: json!({}),
    }
}

#[test]
fn agent_model_scope_activates_after_session_start_reports_model() {
    // Guards otherwise off so the only thing that can trip is the scoped
    // exit expression — isolating the re-scope behavior.
    let guards = GuardSettings {
        repetition: claudine::runaway::RepetitionGuardSettings {
            enabled: false,
            ..Default::default()
        },
        volume: claudine::runaway::VolumeGuardSettings {
            enabled: false,
            ..Default::default()
        },
    };
    let inputs = ResolvedGuardInputs::from_parts(
        Provider::OpenCode,
        vec![agent_model_entry("k2-only-stop", "opencode/k2p7")],
        guards,
    );
    // No launch-time model hint: the scoped expression is inactive.
    let (mut sink, rx) = rescope_sink(inputs, None);

    sink.on_semantic_event(output("k2-only-stop\n"));
    assert!(
        rx.try_recv().is_err(),
        "agent/model scope must be inactive before SessionStart reports the model"
    );

    // The provider reports its actual model; the scoped expression activates.
    sink.on_semantic_event(session_start(Some("k2p7")));
    sink.on_semantic_event(output("k2-only-stop\n"));
    match rx.try_recv() {
        Ok(EarlyTermination::ExitExpression { pattern, scope }) => {
            assert_eq!(pattern, "k2-only-stop");
            assert_eq!(scope.as_deref(), Some("opencode/k2p7"));
        }
        other => panic!("expected ExitExpression trip after re-scope, got {other:?}"),
    }
}

#[test]
fn agent_model_scope_stays_inactive_for_non_matching_reported_model() {
    let guards = GuardSettings {
        repetition: claudine::runaway::RepetitionGuardSettings {
            enabled: false,
            ..Default::default()
        },
        volume: claudine::runaway::VolumeGuardSettings {
            enabled: false,
            ..Default::default()
        },
    };
    let inputs = ResolvedGuardInputs::from_parts(
        Provider::OpenCode,
        vec![agent_model_entry("k2-only-stop", "opencode/k2p7")],
        guards,
    );
    let (mut sink, rx) = rescope_sink(inputs, None);

    // SessionStart reports a DIFFERENT model — the scope must not activate.
    sink.on_semantic_event(session_start(Some("some-other-model")));
    sink.on_semantic_event(output("k2-only-stop\n"));
    assert!(
        rx.try_recv().is_err(),
        "agent/model scope must stay inactive for a non-matching reported model"
    );
}

#[test]
fn global_scope_is_unaffected_by_session_start() {
    // A global (unscoped) exit expression is active from the first compile,
    // regardless of any later SessionStart.
    let inputs = ResolvedGuardInputs::from_parts(
        Provider::OpenCode,
        vec![ExitExpressionEntry {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }],
        GuardSettings::default(),
    );
    let (mut sink, rx) = rescope_sink(inputs, None);

    // Active before any SessionStart.
    sink.on_semantic_event(output("STOP.\n"));
    assert!(
        matches!(rx.try_recv(), Ok(EarlyTermination::ExitExpression { .. })),
        "global scope must be active from the start"
    );
}

#[test]
fn session_start_echoing_launch_model_is_a_noop() {
    // The launch-time model already matches the scoped entry, so the entry
    // is active from the first compile. A SessionStart echoing that same
    // model must not rebuild the set or drop accumulated repetition state.
    let inputs = ResolvedGuardInputs::from_parts(
        Provider::OpenCode,
        // A global single-line spam entry would trip via repetition; here we
        // assert the repetition counter survives the (no-op) SessionStart.
        vec![agent_model_entry("k2-only-stop", "opencode/k2p7")],
        GuardSettings::default(),
    );
    // Launch-time model matches — the scoped entry is active immediately.
    let (mut sink, rx) = rescope_sink(inputs, Some("k2p7"));

    // Feed 29 single-line cycles of a DIFFERENT line to build repetition
    // state (under the threshold).
    let mut under = String::new();
    for _ in 0..29 {
        under.push_str("spam\n");
    }
    sink.on_semantic_event(output(&under));
    assert!(rx.try_recv().is_err(), "29 cycles must not trip yet");

    // SessionStart echoes the same model — must be a no-op (no rebuild, no
    // state reset).
    sink.on_semantic_event(session_start(Some("k2p7")));

    // One more cycle crosses the repetition threshold — proving the counter
    // was preserved across the no-op SessionStart.
    sink.on_semantic_event(output("spam\n"));
    assert!(
        matches!(rx.try_recv(), Ok(EarlyTermination::RunawayRepetition { .. })),
        "repetition state must survive a same-model SessionStart"
    );

    // And the matching-model scoped entry is still active afterwards.
    // (A fresh sink avoids the already-tripped terminal state above.)
    let inputs2 = ResolvedGuardInputs::from_parts(
        Provider::OpenCode,
        vec![agent_model_entry("k2-only-stop", "opencode/k2p7")],
        GuardSettings::default(),
    );
    let (mut sink2, rx2) = rescope_sink(inputs2, Some("k2p7"));
    sink2.on_semantic_event(session_start(Some("k2p7")));
    sink2.on_semantic_event(output("k2-only-stop\n"));
    assert!(
        matches!(rx2.try_recv(), Ok(EarlyTermination::ExitExpression { .. })),
        "matching launch-time model stays active after same-model SessionStart"
    );
}
