//! Phase 6 — content-detector driving from the live semantic sink.
//!
//! These tests exercise the sink seam directly (`on_semantic_event`): a sink
//! armed with a [`ContentDetector`] and a trip [`Sender`] should fire on a
//! guard breach observed in `OutputText`/`Reasoning`, never on tool payloads,
//! and suppress further output rendering once a trip has fired.

use super::*;
use claudine::runaway::{
    CompiledExitExpressions, ContentDetector, DetectorConfig, ExitExpressionInput, PatternKind,
};
use claudine::stream::logs::EarlyTermination;
use serde_json::json;
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
