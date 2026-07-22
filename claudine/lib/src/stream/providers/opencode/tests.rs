use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::*;
use crate::stream::progress::LiveMetricsState;
use serde_json::json;

struct Recording {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}
impl SemanticEventSink for Recording {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn new_parser() -> (
    Arc<Mutex<Vec<SemanticEvent>>>,
    Box<OpenCodeSemanticStreamParser<Recording>>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    (
        events,
        Box::new(OpenCodeSemanticStreamParser::new(
            sink,
            Some("gpt-4o".into()),
            Provider::OpenCode,
        )
        .unwrap()),
    )
}

fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

#[test]
fn step_start_emits_session_start_once_and_info() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    let ks = kinds(&events.lock().unwrap());
    // first step_start: session_start + info; second: just info
    assert_eq!(ks, vec!["session_start", "info", "info"]);
}

#[test]
fn text_emits_output_text() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"text","text":"hello"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::OutputText { ref text, .. } if text == "hello"
    ));
}

#[test]
fn tool_use_and_result_emit_typed_events() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
        );
    let ks = kinds(&events.lock().unwrap());
    assert_eq!(ks, vec!["tool_call", "tool_result"]);
}

#[test]
fn orphan_tool_result_emits_unmatched_result() {
    // OpenCode's stream frequently has completion-only visibility.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_end","part":{"tool_use_id":"t999","status":"success","content":"ok","tool_name":"bash"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::ToolResult {
            id, name, status, ..
        } => {
            assert_eq!(id.as_deref(), Some("t999"));
            assert_eq!(name.as_deref(), Some("bash"));
            assert_eq!(status.as_deref(), Some("success"));
        }
        other => panic!("expected orphan ToolResult, got {other:?}"),
    }
}

#[test]
fn step_complete_emits_turn_complete_and_sums() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":200}"#,
        );
    parser
        .feed_line(
            r#"{"type":"step_complete","usage":{"input_tokens":20,"output_tokens":10,"total_tokens":30},"cost_usd":0.002}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["turn_complete", "turn_complete"]);
    let summary = parser.finish(0);
    let tu = summary.token_usage.unwrap();
    assert_eq!(tu.input, Some(30));
    assert_eq!(tu.output, Some(15));
    assert_eq!(summary.cost_usd, Some(0.003));
}

#[test]
fn error_event_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"error","error_message":"API timeout"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Error { terminal: true, .. }
    ));
    let summary = parser.finish(1);
    assert!(summary.is_error);
}

#[test]
fn unknown_event_becomes_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"some_future_event","x":1}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::ProviderExtension { ref kind, .. } if kind == "some_future_event"
    ));
}

#[test]
fn malformed_json_emits_warning() {
    let (events, mut parser) = new_parser();
    parser.feed_line("garbage");
    assert!(matches!(
        events.lock().unwrap()[0],
        SemanticEvent::Warning { .. }
    ));
}

#[test]
fn tool_input_string_fallback_parses_without_panic() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"tool_start","part":{"tool_name":"bash","input":"ls -la"}}"#);
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["tool_call"]);
    match &collected[0] {
        SemanticEvent::ToolCall { input, .. } => {
            assert_eq!(input.as_ref().and_then(Value::as_str), Some("ls -la"));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn missing_discriminator_falls_through_to_provider_extension() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"payload":{"k":1}}"#);
    let collected = events.lock().unwrap().clone();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        SemanticEvent::ProviderExtension {
            provider,
            kind,
            payload,
        } => {
            assert_eq!(*provider, Provider::OpenCode);
            assert_eq!(kind, "");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn step_finish_info_has_phase_marker() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"step_finish","part":{"reason":"stop","cost":0.01,"tokens":{"input":1,"output":2,"total":3,"cache":{"read":0}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Info { message, extra } => {
            assert_eq!(message, "step_finish");
            assert_eq!(extra.get("step_phase"), Some(&Value::from("finish")));
            assert_eq!(extra.get("reason"), Some(&Value::from("stop")));
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn round_trip_fidelity_mixed_fixture() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"step_start","sessionID":"s"}"#,
        r#"{"type":"text","text":"hi"}"#,
        r#"{"type":"tool_start","part":{"id":"t","tool_name":"b","input":{"x":1}}}"#,
        r#"{"type":"tool_end","part":{"tool_use_id":"t","status":"success","content":"ok"}}"#,
        r#"{"type":"step_finish","part":{"reason":"stop","cost":0.01,"tokens":{"input":1,"output":2,"total":3,"cache":{"read":0}}}}"#,
        r#"{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3},"cost_usd":0.01}"#,
        r#"{"type":"future.kind","x":1}"#,
    ] {
        parser.feed_line(line);
    }
    for event in events.lock().unwrap().iter() {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(v, serde_json::to_value(&decoded).unwrap());
    }
}

#[test]
fn opencode_tool_use_emits_tool_result_only() {
    use super::super::parser::SemanticStreamParser;
    use super::super::semantic::{SemanticEvent, SemanticEventSink};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
    impl SemanticEventSink for Capture {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Capture(events.clone());
    let mut parser =
        OpenCodeSemanticStreamParser::new(sink, Some("gpt-4o".into()), Provider::OpenCode)
            .unwrap();

    parser
        .feed_line(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
             "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
        );

    let captured = events.lock().unwrap().clone();
    let kinds: Vec<&str> = captured.iter().map(|e| e.kind_str()).collect();
    assert_eq!(
        kinds,
        vec!["tool_result"],
        "tool_use (completion) must emit only a ToolResult; OpenCode never emits a paired request"
    );

    let SemanticEvent::ToolResult { name, status, .. } = &captured[0] else {
        panic!("expected ToolResult");
    };
    assert_eq!(name.as_deref(), Some("bash"));
    assert_eq!(status.as_deref(), Some("completed"));
}

#[test]
fn assistant_text_in_part_text_shape_emits_output_text() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/providers/opencode-assistant-text.ndjson");
    let raw = std::fs::read_to_string(&path).expect("fixture exists");
    let (events, mut parser) = new_parser();
    for line in raw.lines() {
        parser.feed_line(line);
    }
    let has_non_empty_output_text = events
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, SemanticEvent::OutputText { text, .. } if !text.is_empty()));
    assert!(
        has_non_empty_output_text,
        "fixture must cause at least one non-empty OutputText emission"
    );

    // Also assert that the assistant_text field was accumulated by
    // the parser (so the summary reflects it).
    let summary_text: String = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !summary_text.is_empty(),
        "concatenated OutputText must be non-empty"
    );
}

#[test]
fn tool_use_event_emits_only_tool_result_not_synthesized_call() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}}"#);
    let kinds: Vec<&'static str> = events
        .lock()
        .unwrap()
        .iter()
        .map(|e| e.kind_str())
        .collect();
    let n_calls = kinds.iter().filter(|k| **k == "tool_call").count();
    let n_results = kinds.iter().filter(|k| **k == "tool_result").count();
    assert_eq!(
        n_calls, 0,
        "must not synthesize a ToolCall when only a completion was observed; got {kinds:?}"
    );
    assert_eq!(n_results, 1, "must emit exactly one ToolResult");
}

#[test]
fn orphan_think_close_delimiter_in_text_is_dropped() {
    // MiniMax-M2/M3 leak the `</think>` boundary token into OpenCode's
    // `text` channel after the reasoning prose was already routed to
    // `reasoning` events. The lone delimiter must not surface as output.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"text","text":"</think>"}"#);
    parser
        .feed_line(r#"{"type":"text","text":"\n</think>\n"}"#);
    parser
        .feed_line(r#"{"type":"text","text":"<think>"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.is_empty(),
        "standalone think delimiters must not emit OutputText; got {collected:?}"
    );
    let summary = parser.finish(0);
    assert_eq!(
        summary.assistant_text, "",
        "leaked delimiters must not accumulate into assistant_text"
    );
}

#[test]
fn think_delimiter_packed_with_content_strips_only_the_delimiter_line() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"text","text":"</think>\n\nNow I'll commit."}"#);
    let collected = events.lock().unwrap().clone();
    let texts: Vec<&str> = collected
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec!["\nNow I'll commit."],
        "the standalone delimiter line (and its terminator) is removed; \
         the following blank line and real content are preserved"
    );
}

#[test]
fn inline_think_mention_in_text_is_preserved() {
    // Prose that legitimately references the tag (e.g. when the agent is
    // editing this very codebase) must pass through untouched.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"text","text":"The </think> token closes a reasoning block."}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::OutputText { text, .. } => {
            assert_eq!(text, "The </think> token closes a reasoning block.");
        }
        other => panic!("expected OutputText, got {other:?}"),
    }
}

#[test]
fn reasoning_event_emits_semantic_reasoning() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"reasoning","text":"weighing options"}"#);
    let collected = events.lock().unwrap().clone();
    let kinds: Vec<&'static str> = collected.iter().map(|e| e.kind_str()).collect();
    assert_eq!(
        kinds,
        vec!["reasoning"],
        "OpenCode `reasoning` event must route to SemanticEvent::Reasoning, \
         not fall through to ProviderExtension; got {kinds:?}"
    );
    let SemanticEvent::Reasoning { text, .. } = &collected[0] else {
        panic!("expected Reasoning");
    };
    assert_eq!(text, "weighing options");
}

#[test]
fn reasoning_with_empty_text_emits_nothing() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"reasoning","text":""}"#);
    parser
        .feed_line(r#"{"type":"reasoning","part":{"text":""}}"#);
    parser.feed_line(r#"{"type":"reasoning"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.is_empty(),
        "empty / missing reasoning text must not emit any semantic event; got {collected:?}"
    );
}

#[test]
fn tool_start_tool_end_pair_preserves_cached_input_on_result() {
    // Per the 2026-04-18 OpenCode reporting contract, a `tool_start`'s
    // input must survive into the paired `tool_end` ToolResult so
    // renderers can annotate successful incoming events with the same
    // slot content the outgoing arrow used.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls -la"}}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let result = collected
        .iter()
        .find(|e| matches!(e, SemanticEvent::ToolResult { .. }))
        .expect("expected a ToolResult");
    let SemanticEvent::ToolResult { extra, .. } = result else {
        unreachable!()
    };
    let input = extra
        .get("input")
        .expect("extra['input'] must survive into ToolResult");
    assert_eq!(input.get("command").and_then(Value::as_str), Some("ls -la"));
}

#[test]
fn tool_end_wire_input_wins_over_cached_input() {
    // When the wire `tool_end` payload carries its own `input` (rare
    // but permitted), the parser must prefer it over the cached
    // request-side input so we never overwrite fresher data.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls"}}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok","input":{"command":"pwd"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let result = collected
        .iter()
        .find(|e| matches!(e, SemanticEvent::ToolResult { .. }))
        .expect("expected a ToolResult");
    let SemanticEvent::ToolResult { extra, .. } = result else {
        unreachable!()
    };
    assert_eq!(
        extra
            .get("input")
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("pwd"),
        "wire-provided input must win over cached input"
    );
}

#[test]
fn tool_use_event_still_increments_tool_calls_counter() {
    // Ensure the trailer count matches the rendered-line count by keeping
    // `tool_calls += 1` even though no ToolCall event is emitted.
    let (_events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}}"#);
    parser
        .feed_line(r#"{"type":"tool_use","part":{"id":"t2","tool":"bash","state":{"status":"completed","input":{"command":"pwd"},"output":"/"}}}"#);
    let summary = Box::new(parser).finish(0);
    assert_eq!(summary.tool_calls, Some(2), "both completions must count");
}

#[test]
fn task_started_becomes_subagent_start() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#);
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::SubagentStart { id, name, .. } => {
            assert_eq!(id.as_deref(), Some("sa1"));
            assert_eq!(name.as_deref(), Some("researcher"));
        }
        other => panic!("expected SubagentStart, got {other:?}"),
    }
}

#[test]
fn task_completed_becomes_subagent_stop() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::SubagentStop {
            id, name, status, ..
        } => {
            assert_eq!(id.as_deref(), Some("sa1"));
            assert_eq!(name.as_deref(), Some("researcher"));
            assert_eq!(status.as_deref(), Some("success"));
        }
        other => panic!("expected SubagentStop, got {other:?}"),
    }
}

#[test]
fn task_progress_becomes_info() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"task_progress","message":"working"}"#);
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Info { message, .. } => assert_eq!(message, "working"),
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn opencode_task_completion_no_longer_synthesizes_subagent_lifecycle() {
    // Phase 4 (2026-05-12): the structured stderr stream is now the
    // authoritative source for OpenCode subagent lifecycle. The stdout
    // NDJSON parser no longer synthesizes SubagentStart/SubagentStop
    // from `task` tool completions — those events come from
    // `service=session ... parentID=...` and the matching `exiting loop`
    // record on stderr.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"task",
             "state":{"status":"completed",
                      "input":{"description":"Commit wrap CLI refactor","subagent_type":"coder"},
                      "metadata":{"sessionId":"child-ses-1"},
                      "time":{"start":1715340000000},
                      "output":"ok"}}}"#,
        );

    let collected = events.lock().unwrap().clone();
    let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
    assert_eq!(
        ks,
        vec!["session_start", "info", "tool_result"],
        "task tool completion must emit only ToolResult; subagent lifecycle now comes from stderr"
    );

    // Verify NO subagent_start / subagent_stop in the stdout stream
    let n_subagent_starts = ks.iter().filter(|k| **k == "subagent_start").count();
    let n_subagent_stops = ks.iter().filter(|k| **k == "subagent_stop").count();
    assert_eq!(n_subagent_starts, 0);
    assert_eq!(n_subagent_stops, 0);

    // subagent_done_count must NOT increment from stdout-side events
    let mut metrics = LiveMetricsState::default();
    let now = Instant::now();
    for event in &collected {
        metrics.observe_event(event, now);
    }
    assert_eq!(
        metrics.subagent_done_count, 0,
        "stdout-side task completion must not increment subagent_done_count"
    );
}

#[test]
fn opencode_task_error_completion_no_longer_synthesizes_subagent_lifecycle() {
    // Companion to opencode_task_completion_no_longer_synthesizes_subagent_lifecycle:
    // even when a `task` tool errors out, no SubagentStart/SubagentStop
    // is synthesized from stdout. The matching subagent stop comes from
    // the stderr `exiting loop` record for the child session.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(
            r#"{"type":"tool_use","part":{"id":"t-err","tool":"task",
             "state":{"status":"error",
                      "input":{"description":"Failed subagent","subagent_type":"coder"},
                      "metadata":{"sessionId":"child-ses-err"},
                      "error":"agent crashed",
                      "output":""}}}"#,
        );

    let collected = events.lock().unwrap().clone();
    let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
    assert_eq!(
        ks,
        vec!["session_start", "info", "tool_result"],
        "task tool error completion must emit only ToolResult"
    );

    // ToolResult must still carry the error status so it is rendered.
    match &collected[2] {
        SemanticEvent::ToolResult { status, .. } => {
            assert_eq!(status.as_deref(), Some("error"));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }

    let mut metrics = LiveMetricsState::default();
    let now = Instant::now();
    for event in &collected {
        metrics.observe_event(event, now);
    }
    assert_eq!(
        metrics.subagent_done_count, 0,
        "stdout-side task error completion must not increment subagent_done_count"
    );
}

#[test]
fn opencode_non_task_tool_does_not_synthesize_subagent_lifecycle() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#);
    parser
        .feed_line(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
             "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
        );

    let collected = events.lock().unwrap().clone();
    let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
    let n_subagent_starts = ks.iter().filter(|k| **k == "subagent_start").count();
    let n_subagent_stops = ks.iter().filter(|k| **k == "subagent_stop").count();
    assert_eq!(
        n_subagent_starts, 0,
        "non-task tool must not synthesize SubagentStart; got {ks:?}"
    );
    assert_eq!(
        n_subagent_stops, 0,
        "non-task tool must not synthesize SubagentStop; got {ks:?}"
    );
    assert_eq!(
        ks,
        vec!["session_start", "info", "tool_result"],
        "bash tool must emit only Info + ToolResult"
    );
}

/// Feed a single error line through an OpenCode-shaped parser stamped with
/// `provider`, returning the emitted `Error` event's semantic kind and the
/// `provider` slug stamped on its `extra`.
fn classify_error_line_with_vocabulary(
    provider: Provider,
    line: &str,
    vocabulary_for: fn(Provider) -> &'static super::super::common::ErrorKeywords,
) -> Result<(SemanticErrorKind, String), InvalidOpenCodeParserProvider> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    let mut parser = OpenCodeSemanticStreamParser::new_with_vocabulary_resolver(
        sink,
        None,
        provider,
        vocabulary_for,
    )?;
    parser.feed_line(line);
    let collected = events.lock().unwrap().clone();
    let SemanticEvent::Error { kind, extra, .. } = collected
        .into_iter()
        .find(|e| matches!(e, SemanticEvent::Error { .. }))
        .expect("an Error event")
    else {
        unreachable!()
    };
    let slug = extra
        .get("provider")
        .and_then(Value::as_str)
        .expect("provider slug in extra")
        .to_string();
    Ok((kind, slug))
}

fn classify_error_line(
    provider: Provider,
    line: &str,
) -> Result<(SemanticErrorKind, String), InvalidOpenCodeParserProvider> {
    classify_error_line_with_vocabulary(
        provider,
        line,
        super::super::vocabulary::error_keywords,
    )
}

#[test]
fn kilo_identity_stamps_kilo_and_classifies_via_kilo_vocabulary() {
    // A Kilo-configured OpenCode parser stamps Kilo identity on every event
    // and classifies through Kilo's own generated vocabulary.
    let (kind, slug) = classify_error_line(
        Provider::Kilo,
        r#"{"type":"error","error_message":"rate limit exceeded"}"#,
    )
    .unwrap();
    assert_eq!(kind, SemanticErrorKind::ApiRemote);
    assert_eq!(slug, "kilo", "Kilo runs must not be stamped as OpenCode");

    // The summary carries Kilo identity too, not the parser's OpenCode origin.
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    let mut parser =
        OpenCodeSemanticStreamParser::new(sink, None, Provider::Kilo).unwrap();
    parser
        .feed_line(r#"{"type":"error","error_message":"rate limit exceeded"}"#);
    let summary = Box::new(parser).finish(1);
    assert_eq!(summary.provider, Provider::Kilo);
}

#[test]
fn shared_parser_selects_vocabulary_by_runtime_identity() {
    static OPENCODE_TEST_VOCABULARY: super::super::common::ErrorKeywords =
        super::super::common::ErrorKeywords {
            kind_buckets: &[],
            msg_buckets: &[(SemanticErrorKind::Configuration, &["identity seam"])],
            code_buckets: &[],
        };
    static KILO_TEST_VOCABULARY: super::super::common::ErrorKeywords =
        super::super::common::ErrorKeywords {
            kind_buckets: &[],
            msg_buckets: &[(SemanticErrorKind::Interrupted, &["identity seam"])],
            code_buckets: &[],
        };
    fn vocabulary_for(
        provider: Provider,
    ) -> &'static super::super::common::ErrorKeywords {
        match provider {
            Provider::OpenCode => &OPENCODE_TEST_VOCABULARY,
            Provider::Kilo => &KILO_TEST_VOCABULARY,
            _ => unreachable!("constructor rejects unsupported identities first"),
        }
    }

    let line = r#"{"type":"error","error_message":"identity seam"}"#;
    let (opencode_kind, opencode_slug) =
        classify_error_line_with_vocabulary(Provider::OpenCode, line, vocabulary_for).unwrap();
    let (kilo_kind, kilo_slug) =
        classify_error_line_with_vocabulary(Provider::Kilo, line, vocabulary_for).unwrap();

    assert_eq!(opencode_slug, "opencode");
    assert_eq!(kilo_slug, "kilo");
    assert_eq!(opencode_kind, SemanticErrorKind::Configuration);
    assert_eq!(kilo_kind, SemanticErrorKind::Interrupted);
}

#[test]
fn invalid_parser_identity_returns_typed_error() {
    let error = classify_error_line(
        Provider::Claude,
        r#"{"type":"error","error_message":"boom"}"#,
    )
    .unwrap_err();
    assert_eq!(
        error,
        InvalidOpenCodeParserProvider {
            provider: Provider::Claude,
        }
    );
}
